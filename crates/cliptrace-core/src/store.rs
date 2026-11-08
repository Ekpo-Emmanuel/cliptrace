use std::path::{Path, PathBuf};

use crate::search::fts_or_query;
use crate::secrets::{
    SecretKind, detect_span, dpapi_protect, dpapi_unprotect, mask_secret, secret_hint,
    secret_search_text,
};
use crate::tags::{BrowseFilter, MAX_TAGS_PER_CLIP, normalize_tag};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const SEARCH_TEXT_CHARS: usize = 2000;
pub const EMBED_DIMS: usize = 384;
pub const EMBED_MODEL: &str = "bge-small-en-v1.5";

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS clips (
  id            INTEGER PRIMARY KEY,
  hash          TEXT NOT NULL UNIQUE,
  first_seen_at INTEGER NOT NULL,
  last_seen_at  INTEGER NOT NULL,
  copy_count    INTEGER NOT NULL DEFAULT 1,
  kind          TEXT NOT NULL,
  is_secret     INTEGER NOT NULL DEFAULT 0,
  content       TEXT,
  content_enc   BLOB,
  search_text   TEXT NOT NULL,
  source_exe    TEXT,
  window_title  TEXT,
  expires_at    INTEGER
);
CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
  search_text, content='clips', content_rowid='id', tokenize='porter unicode61'
);
CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
  INSERT INTO clips_fts(rowid, search_text) VALUES (new.id, new.search_text);
END;
CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, search_text) VALUES('delete', old.id, old.search_text);
END;
CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, search_text) VALUES('delete', old.id, old.search_text);
  INSERT INTO clips_fts(rowid, search_text) VALUES (new.id, new.search_text);
END;
CREATE TABLE IF NOT EXISTS clip_embeddings (
  clip_id INTEGER PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
  model TEXT NOT NULL,
  embedding BLOB NOT NULL
);
"#;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("clip {0} not found")]
    NotFound(i64),
    #[error("embedding error: {0}")]
    Embedding(String),
    #[error("dpapi error: {0}")]
    Dpapi(String),
    #[error("invalid tag")]
    InvalidTag,
    #[error("clip already has 10 tags")]
    TooManyTags,
    #[error("clip {0} is already a secret")]
    AlreadySecret(i64),
}

#[derive(Clone, Debug)]
pub struct StoredEmbedding {
    pub clip_id: i64,
    pub last_seen_at: i64,
    pub source_exe: String,
    pub preview: String,
    pub model: String,
    pub embedding: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SearchHit {
    pub id: i64,
    pub last_seen_at: i64,
    pub source_exe: String,
    pub preview: String,
    pub tags: String,
    pub pinned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TagCount {
    pub name: String,
    pub count: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ClipCard {
    pub id: i64,
    pub last_seen_at: i64,
    pub source_exe: String,
    pub window_title: String,
    pub preview: String,
    pub body: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub copy_count: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewClip {
    pub content: String,
    pub source_exe: String,
    pub window_title: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpsertOutcome {
    Inserted { id: i64 },
    Duplicate { id: i64, copy_count: i64 },
}

pub struct Store {
    conn: Connection,
    db_path: PathBuf,
}

impl Store {
    pub fn open(db_path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "secure_delete", "ON")?;
        conn.execute_batch(SCHEMA)?;
        migrate_secret_hint(&conn)?;
        migrate_pins_and_tags(&conn)?;
        Ok(Self {
            conn,
            db_path: db_path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.db_path
    }

    pub fn upsert_clip(
        &self,
        clip: &NewClip,
        now_ms: i64,
        secret_ttl_hours: u32,
    ) -> Result<UpsertOutcome, StoreError> {
        let normalized = normalize_text(&clip.content);
        let hash = clip_hash(&normalized);
        if let Some((id, copy_count)) = self.lookup_hash(&hash)? {
            let copy_count = copy_count + 1;
            self.conn.execute(
                "UPDATE clips SET last_seen_at = ?1, copy_count = ?2 WHERE id = ?3",
                params![now_ms, copy_count, id],
            )?;
            return Ok(UpsertOutcome::Duplicate { id, copy_count });
        }

        if let Some((secret_kind, start, end)) = detect_span(&normalized) {
            let matched = if end <= normalized.len() && start < end {
                &normalized[start..end]
            } else {
                normalized.as_str()
            };
            return self.insert_secret(
                clip,
                &normalized,
                matched,
                &hash,
                now_ms,
                secret_ttl_hours,
                secret_kind,
            );
        }

        let search_text: String = normalized.chars().take(SEARCH_TEXT_CHARS).collect();
        let kind = classify_kind(&normalized);
        self.conn.execute(
            "INSERT INTO clips (
                hash, first_seen_at, last_seen_at, copy_count, kind, is_secret,
                content, content_enc, search_text, source_exe, window_title, expires_at, secret_hint
            ) VALUES (?1, ?2, ?3, 1, ?4, 0, ?5, NULL, ?6, ?7, ?8, NULL, NULL)",
            params![
                hash,
                now_ms,
                now_ms,
                kind,
                normalized,
                search_text,
                clip.source_exe,
                clip.window_title,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(UpsertOutcome::Inserted { id })
    }

    fn insert_secret(
        &self,
        clip: &NewClip,
        normalized: &str,
        matched: &str,
        hash: &str,
        now_ms: i64,
        secret_ttl_hours: u32,
        secret_kind: SecretKind,
    ) -> Result<UpsertOutcome, StoreError> {
        let enc = dpapi_protect(normalized.as_bytes()).map_err(|e| StoreError::Dpapi(e.to_string()))?;
        let search_text = secret_search_text(secret_kind, &clip.source_exe, &clip.window_title);
        let hint = secret_hint(matched);
        let expires_at = secret_expiry(now_ms, secret_ttl_hours);
        self.conn.execute(
            "INSERT INTO clips (
                hash, first_seen_at, last_seen_at, copy_count, kind, is_secret,
                content, content_enc, search_text, source_exe, window_title, expires_at, secret_hint
            ) VALUES (?1, ?2, ?3, 1, ?4, 1, NULL, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                hash,
                now_ms,
                now_ms,
                secret_kind.slug(),
                enc,
                search_text,
                clip.source_exe,
                clip.window_title,
                expires_at,
                hint,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(UpsertOutcome::Inserted { id })
    }

    fn lookup_hash(&self, hash: &str) -> Result<Option<(i64, i64)>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT id, copy_count FROM clips WHERE hash = ?1",
                [hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    pub fn search_keyword(
        &self,
        query: &str,
        limit: usize,
        range: Option<(i64, i64)>,
    ) -> Result<Vec<SearchHit>, StoreError> {
        self.search_keyword_scoped(query, limit, range, &BrowseFilter::default())
    }

    pub fn search_keyword_scoped(
        &self,
        query: &str,
        limit: usize,
        range: Option<(i64, i64)>,
        filter: &BrowseFilter,
    ) -> Result<Vec<SearchHit>, StoreError> {
        let limit = limit.max(1) as i64;
        let (start, end) = match range {
            Some((s, e)) => (opt_i64(Some(s)), opt_i64(Some(e))),
            None => (Value::Null, Value::Null),
        };
        let overlap = "(? IS NULL OR (c.first_seen_at <= ? AND c.last_seen_at >= ?))";
        let extra = browse_clause(filter);
        let cols = HIT_COLS;
        if let Some(match_query) = fts_or_query(query) {
            let sql = format!(
                "SELECT {cols}
                 FROM clips_fts
                 JOIN clips c ON c.id = clips_fts.rowid
                 WHERE clips_fts MATCH ? AND {overlap}{extra}
                 ORDER BY bm25(clips_fts, 1.0, 3.0) ASC, c.last_seen_at DESC
                 LIMIT ?"
            );
            let mut binds = vec![Value::Text(match_query), start.clone(), end, start];
            push_tag_binds(&mut binds, filter);
            binds.push(Value::Integer(limit));
            let mut stmt = self.conn.prepare(&sql)?;
            return map_hits(stmt.query_map(params_from_iter(binds), hit_from_row)?);
        }
        let scoped = range.is_some() || filter.pinned_only || !filter.tags.is_empty();
        if scoped {
            let sql = format!(
                "SELECT {cols}
                 FROM clips c
                 WHERE {overlap}{extra}
                 ORDER BY c.last_seen_at DESC
                 LIMIT ?"
            );
            let mut binds = vec![start.clone(), end, start];
            push_tag_binds(&mut binds, filter);
            binds.push(Value::Integer(limit));
            let mut stmt = self.conn.prepare(&sql)?;
            return map_hits(stmt.query_map(params_from_iter(binds), hit_from_row)?);
        }
        Ok(Vec::new())
    }

    pub fn list_recent(
        &self,
        offset: i64,
        limit: i64,
        filter: &BrowseFilter,
    ) -> Result<Vec<SearchHit>, StoreError> {
        let extra = browse_clause(filter);
        let sql = format!(
            "SELECT {HIT_COLS}
             FROM clips c
             WHERE 1=1{extra}
             ORDER BY c.last_seen_at DESC
             LIMIT ? OFFSET ?"
        );
        let mut binds = Vec::new();
        push_tag_binds(&mut binds, filter);
        binds.push(Value::Integer(limit.max(1)));
        binds.push(Value::Integer(offset.max(0)));
        let mut stmt = self.conn.prepare(&sql)?;
        map_hits(stmt.query_map(params_from_iter(binds), hit_from_row)?)
    }

    pub fn list_cards(
        &self,
        offset: i64,
        limit: i64,
        filter: &BrowseFilter,
    ) -> Result<Vec<ClipCard>, StoreError> {
        let extra = browse_clause(filter);
        let sql = format!(
            "SELECT {CARD_COLS}
             FROM clips c
             WHERE 1=1{extra}
             ORDER BY c.last_seen_at DESC
             LIMIT ? OFFSET ?"
        );
        let mut binds = Vec::new();
        push_tag_binds(&mut binds, filter);
        binds.push(Value::Integer(limit.max(1)));
        binds.push(Value::Integer(offset.max(0)));
        self.query_cards(&sql, binds)
    }

    pub fn cards_for_ids(&self, ids: &[i64]) -> Result<Vec<ClipCard>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let marks = vec!["?"; ids.len()].join(",");
        let sql = format!("SELECT {CARD_COLS} FROM clips c WHERE c.id IN ({marks})");
        let binds = ids.iter().copied().map(Value::Integer).collect();
        let mut cards = self.query_cards(&sql, binds)?;
        cards.sort_by_key(|card| ids.iter().position(|id| *id == card.id).unwrap_or(usize::MAX));
        Ok(cards)
    }

    pub fn clip_count(&self) -> Result<i64, StoreError> {
        let n = self.conn.query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))?;
        Ok(n)
    }

    pub fn pinned_count(&self) -> Result<i64, StoreError> {
        let n = self.conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE pinned_at IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(n)
    }

    fn query_cards(&self, sql: &str, binds: Vec<Value>) -> Result<Vec<ClipCard>, StoreError> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params_from_iter(binds), card_from_row)?;
        let mut cards = Vec::new();
        for row in rows {
            let mut card = row?;
            card.tags = self.clip_tags(card.id)?;
            cards.push(card);
        }
        Ok(cards)
    }

    pub fn list_tags(&self) -> Result<Vec<TagCount>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, COUNT(ct.clip_id) AS n
             FROM tags t
             JOIN clip_tags ct ON ct.tag_id = t.id
             GROUP BY t.id, t.name
             ORDER BY n DESC, t.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TagCount {
                name: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn pin_clip(&self, id: i64, now_ms: i64) -> Result<(), StoreError> {
        self.require_clip(id)?;
        self.conn.execute(
            "UPDATE clips SET pinned_at = ?1,
                 expires_at = CASE WHEN is_secret = 1 THEN NULL ELSE expires_at END
             WHERE id = ?2",
            params![now_ms, id],
        )?;
        Ok(())
    }

    pub fn unpin_clip(
        &self,
        id: i64,
        now_ms: i64,
        secret_ttl_hours: u32,
    ) -> Result<(), StoreError> {
        self.require_clip(id)?;
        let expires = secret_expiry(now_ms, secret_ttl_hours);
        self.conn.execute(
            "UPDATE clips SET pinned_at = NULL,
                 expires_at = CASE WHEN is_secret = 1 THEN ?1 ELSE expires_at END
             WHERE id = ?2",
            params![expires, id],
        )?;
        Ok(())
    }

    pub fn tag_clip(&self, id: i64, raw: &str) -> Result<Vec<String>, StoreError> {
        self.require_clip(id)?;
        let Some(name) = normalize_tag(raw) else {
            return Err(StoreError::InvalidTag);
        };
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clip_tags WHERE clip_id = ?1",
            [id],
            |row| row.get(0),
        )?;
        let already: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clip_tags ct JOIN tags t ON t.id = ct.tag_id
             WHERE ct.clip_id = ?1 AND t.name = ?2 COLLATE NOCASE",
            params![id, name],
            |row| row.get(0),
        )?;
        if already == 0 && n >= MAX_TAGS_PER_CLIP as i64 {
            return Err(StoreError::TooManyTags);
        }
        self.conn.execute(
            "INSERT INTO tags(name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            [&name],
        )?;
        let tag_id: i64 = self.conn.query_row(
            "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
            [&name],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO clip_tags(clip_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )?;
        self.sync_tags_text(id)?;
        self.delete_embedding(id)?;
        self.clip_tags(id)
    }

    pub fn untag_clip(&self, id: i64, raw: &str) -> Result<Vec<String>, StoreError> {
        self.require_clip(id)?;
        let Some(name) = normalize_tag(raw) else {
            return Err(StoreError::InvalidTag);
        };
        self.conn.execute(
            "DELETE FROM clip_tags WHERE clip_id = ?1 AND tag_id IN (
                SELECT id FROM tags WHERE name = ?2 COLLATE NOCASE
             )",
            params![id, name],
        )?;
        self.conn.execute(
            "DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM clip_tags)",
            [],
        )?;
        self.sync_tags_text(id)?;
        self.delete_embedding(id)?;
        self.clip_tags(id)
    }

    pub fn clip_tags(&self, id: i64) -> Result<Vec<String>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name FROM clip_tags ct JOIN tags t ON t.id = ct.tag_id
             WHERE ct.clip_id = ?1 ORDER BY t.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([id], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn mark_secret(&self, id: i64, now_ms: i64, secret_ttl_hours: u32) -> Result<(), StoreError> {
        let row: Option<(Option<String>, i64, String, String)> = self
            .conn
            .query_row(
                "SELECT content, is_secret, COALESCE(source_exe, ''), COALESCE(window_title, '')
                 FROM clips WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let Some((content, is_secret, exe, title)) = row else {
            return Err(StoreError::NotFound(id));
        };
        if is_secret == 1 {
            return Err(StoreError::AlreadySecret(id));
        }
        let content = content.ok_or(StoreError::NotFound(id))?;
        let (kind, matched) = if let Some((kind, start, end)) = detect_span(&content) {
            let matched = if end <= content.len() && start < end {
                content[start..end].to_string()
            } else {
                content.clone()
            };
            (kind, matched)
        } else {
            (crate::secrets::SecretKind::HighEntropy, content.clone())
        };
        self.convert_plain_to_secret(
            id,
            &content,
            &matched,
            &exe,
            &title,
            now_ms,
            secret_ttl_hours,
            kind,
        )?;
        self.rebuild_fts()?;
        self.checkpoint()?;
        self.vacuum()?;
        self.checkpoint()?;
        Ok(())
    }

    pub fn delete_clip(&self, id: i64) -> Result<(), StoreError> {
        let n = self.conn.execute("DELETE FROM clips WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(StoreError::NotFound(id));
        }
        self.conn.execute(
            "DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM clip_tags)",
            [],
        )?;
        self.rebuild_fts()?;
        self.checkpoint()?;
        self.vacuum()?;
        self.checkpoint()?;
        Ok(())
    }

    fn require_clip(&self, id: i64) -> Result<(), StoreError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM clips WHERE id = ?1", [id], |row| {
                row.get(0)
            })?;
        if n == 0 {
            Err(StoreError::NotFound(id))
        } else {
            Ok(())
        }
    }

    fn sync_tags_text(&self, id: i64) -> Result<(), StoreError> {
        let text: String = self.conn.query_row(
            "SELECT IFNULL((
                SELECT group_concat(name, ' ') FROM (
                    SELECT t.name AS name FROM clip_tags ct
                    JOIN tags t ON t.id = ct.tag_id
                    WHERE ct.clip_id = ?1
                    ORDER BY t.name COLLATE NOCASE
                )
            ), '')",
            [id],
            |row| row.get(0),
        )?;
        self.conn
            .execute("UPDATE clips SET tags_text = ?1 WHERE id = ?2", params![text, id])?;
        Ok(())
    }

    pub fn clip_content(&self, id: i64) -> Result<String, StoreError> {
        let row: Option<(Option<String>, Option<Vec<u8>>, i64)> = self
            .conn
            .query_row(
                "SELECT content, content_enc, is_secret FROM clips WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let Some((content, enc, is_secret)) = row else {
            return Err(StoreError::NotFound(id));
        };
        if is_secret == 1 {
            let blob = enc.ok_or_else(|| StoreError::Dpapi("missing content_enc".into()))?;
            let plain = dpapi_unprotect(&blob).map_err(|e| StoreError::Dpapi(e.to_string()))?;
            return String::from_utf8(plain)
                .map_err(|e| StoreError::Dpapi(e.to_string()));
        }
        content.ok_or(StoreError::NotFound(id))
    }

    pub fn display_preview(&self, id: i64) -> Result<String, StoreError> {
        self.conn
            .query_row(
                "SELECT COALESCE(content, ''), is_secret, COALESCE(kind, ''), COALESCE(secret_hint, '')
                 FROM clips WHERE id = ?1",
                [id],
                |row| {
                    let content: String = row.get(0)?;
                    let is_secret: i64 = row.get(1)?;
                    let kind: String = row.get(2)?;
                    let hint: String = row.get(3)?;
                    Ok(hit_preview(is_secret, &kind, &hint, &content))
                },
            )
            .map_err(|_| StoreError::NotFound(id))
    }

    pub fn clip_search_text(&self, id: i64) -> Result<String, StoreError> {
        self.conn
            .query_row(
                "SELECT TRIM(search_text || CASE WHEN tags_text = '' THEN '' ELSE ' ' || tags_text END)
                 FROM clips WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::NotFound(id))
    }

    pub fn has_embedding(&self, id: i64) -> Result<bool, StoreError> {
        let found: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM clip_embeddings WHERE clip_id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(found.is_some())
    }

    pub fn clips_missing_embeddings(&self) -> Result<Vec<(i64, String)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, TRIM(c.search_text || CASE WHEN c.tags_text = '' THEN '' ELSE ' ' || c.tags_text END)
             FROM clips c
             LEFT JOIN clip_embeddings e ON e.clip_id = c.id
             WHERE e.clip_id IS NULL
             ORDER BY c.id",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn upsert_embedding(&self, id: i64, vector: &[f32]) -> Result<(), StoreError> {
        if vector.len() != EMBED_DIMS {
            return Err(StoreError::Embedding(format!(
                "embedding len {} != {EMBED_DIMS}",
                vector.len()
            )));
        }
        let normalized = l2_normalize(vector).ok_or_else(|| {
            StoreError::Embedding("cannot L2-normalize a zero vector".into())
        })?;
        let blob = f32_le_blob(&normalized);
        self.conn.execute(
            "INSERT INTO clip_embeddings(clip_id, model, embedding) VALUES (?1, ?2, ?3)
             ON CONFLICT(clip_id) DO UPDATE SET model = excluded.model, embedding = excluded.embedding",
            params![id, EMBED_MODEL, blob],
        )?;
        Ok(())
    }

    /// `range` is inclusive overlap: first_seen_at <= end AND last_seen_at >= start.
    pub fn list_embeddings(
        &self,
        range: Option<(i64, i64)>,
    ) -> Result<Vec<StoredEmbedding>, StoreError> {
        self.list_embeddings_scoped(range, &BrowseFilter::default())
    }

    pub fn list_embeddings_scoped(
        &self,
        range: Option<(i64, i64)>,
        filter: &BrowseFilter,
    ) -> Result<Vec<StoredEmbedding>, StoreError> {
        let extra = browse_clause(filter);
        let sql = format!(
            "SELECT e.clip_id, c.last_seen_at, COALESCE(c.source_exe, ''),
                    COALESCE(c.content, ''), e.model, e.embedding,
                    c.is_secret, COALESCE(c.kind, ''), COALESCE(c.secret_hint, '')
             FROM clip_embeddings e
             JOIN clips c ON c.id = e.clip_id
             WHERE (? IS NULL OR (c.first_seen_at <= ? AND c.last_seen_at >= ?)){extra}"
        );
        let (start, end) = match range {
            Some((s, e)) => (opt_i64(Some(s)), opt_i64(Some(e))),
            None => (Value::Null, Value::Null),
        };
        let mut binds = vec![start.clone(), end, start];
        push_tag_binds(&mut binds, filter);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(binds), |row| {
            let blob: Vec<u8> = row.get(5)?;
            let content: String = row.get(3)?;
            let is_secret: i64 = row.get(6)?;
            let kind: String = row.get(7)?;
            let hint: String = row.get(8)?;
            Ok(StoredEmbedding {
                clip_id: row.get(0)?,
                last_seen_at: row.get(1)?,
                source_exe: row.get(2)?,
                preview: hit_preview(is_secret, &kind, &hint, &content),
                model: row.get(4)?,
                embedding: f32_le_from_blob(&blob),
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn search_vector(
        &self,
        vector: &[f32],
        limit: usize,
        range: Option<(i64, i64)>,
    ) -> Result<Vec<SearchHit>, StoreError> {
        self.search_vector_scoped(vector, limit, range, &BrowseFilter::default())
    }

    pub fn search_vector_scoped(
        &self,
        vector: &[f32],
        limit: usize,
        range: Option<(i64, i64)>,
        filter: &BrowseFilter,
    ) -> Result<Vec<SearchHit>, StoreError> {
        if vector.len() != EMBED_DIMS {
            return Err(StoreError::Embedding(format!(
                "query embedding len {} != {EMBED_DIMS}",
                vector.len()
            )));
        }
        let query = l2_normalize(vector).ok_or_else(|| {
            StoreError::Embedding("cannot L2-normalize a zero query vector".into())
        })?;
        let rows = self.list_embeddings_scoped(range, filter)?;
        let scored: Vec<(f32, &StoredEmbedding)> = rows
            .iter()
            .filter(|row| row.embedding.len() == EMBED_DIMS)
            .map(|row| (dot(&query, &row.embedding), row))
            .collect();
        let ranked = rank_by_score(scored, limit);
        let mut hits = Vec::new();
        for row in ranked {
            let (tags, pinned) = self.hit_meta(row.clip_id)?;
            hits.push(SearchHit {
                id: row.clip_id,
                last_seen_at: row.last_seen_at,
                source_exe: row.source_exe.clone(),
                preview: row.preview.clone(),
                tags,
                pinned,
            });
        }
        Ok(hits)
    }

    fn hit_meta(&self, id: i64) -> Result<(String, bool), StoreError> {
        self.conn
            .query_row(
                "SELECT COALESCE(tags_text, ''), pinned_at IS NOT NULL FROM clips WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get::<_, i64>(1)? == 1)),
            )
            .map_err(|_| StoreError::NotFound(id))
    }

    pub fn delete_embedding(&self, id: i64) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM clip_embeddings WHERE clip_id = ?1", [id])?;
        Ok(())
    }

    /// Delete expired secrets and normal clips older than retention. FTS triggers + CASCADE embeddings.
    pub fn purge(
        &self,
        now_ms: i64,
        retention_days: u32,
        secret_ttl_hours: u32,
    ) -> Result<u64, StoreError> {
        let retention_ms = i64::from(retention_days) * 86_400_000;
        let cutoff = now_ms.saturating_sub(retention_ms);
        let expired = if secret_ttl_hours == 0 {
            self.conn.execute(
                "DELETE FROM clips WHERE is_secret = 1 AND pinned_at IS NULL",
                [],
            )?
        } else {
            self.conn.execute(
                "DELETE FROM clips WHERE expires_at IS NOT NULL AND expires_at < ?1 AND pinned_at IS NULL",
                [now_ms],
            )?
        };
        let old = self.conn.execute(
            "DELETE FROM clips WHERE is_secret = 0 AND pinned_at IS NULL AND last_seen_at < ?1",
            [cutoff],
        )?;
        let deleted = expired as u64 + old as u64;
        if deleted > 0 {
            self.rebuild_fts()?;
        }
        self.checkpoint()?;
        Ok(deleted)
    }

    pub fn rescan_secrets(
        &self,
        now_ms: i64,
        secret_ttl_hours: u32,
    ) -> Result<RescanReport, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, COALESCE(source_exe, ''), COALESCE(window_title, '') FROM clips WHERE is_secret = 0 AND content IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut scanned = 0usize;
        let mut converted = Vec::new();
        let mut pending = Vec::new();
        for row in rows {
            let (id, content, exe, title) = row?;
            scanned += 1;
            if let Some((kind, start, end)) = detect_span(&content) {
                let matched = if end <= content.len() && start < end {
                    content[start..end].to_string()
                } else {
                    content.clone()
                };
                pending.push((id, content, exe, title, kind, matched));
            }
        }
        drop(stmt);
        for (id, content, exe, title, kind, matched) in pending {
            self.convert_plain_to_secret(
                id,
                &content,
                &matched,
                &exe,
                &title,
                now_ms,
                secret_ttl_hours,
                kind,
            )?;
            let search_text = secret_search_text(kind, &exe, &title);
            converted.push((id, search_text));
        }
        let converted_n = converted.len();
        if converted_n > 0 {
            self.rebuild_fts()?;
            self.checkpoint()?;
            self.vacuum()?;
        } else {
            self.checkpoint()?;
        }
        Ok(RescanReport {
            scanned,
            converted: converted_n,
            reembed: converted,
        })
    }

    pub fn scrub(&self) -> Result<(), StoreError> {
        self.checkpoint()?;
        self.rebuild_fts()?;
        self.vacuum()?;
        self.checkpoint()?;
        Ok(())
    }

    fn checkpoint(&self) -> Result<(), StoreError> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    fn rebuild_fts(&self) -> Result<(), StoreError> {
        self.conn
            .execute("INSERT INTO clips_fts(clips_fts) VALUES('rebuild')", [])?;
        Ok(())
    }

    fn vacuum(&self) -> Result<(), StoreError> {
        self.conn.execute_batch("VACUUM")?;
        Ok(())
    }

    fn convert_plain_to_secret(
        &self,
        id: i64,
        content: &str,
        matched: &str,
        exe: &str,
        title: &str,
        now_ms: i64,
        secret_ttl_hours: u32,
        kind: SecretKind,
    ) -> Result<(), StoreError> {
        let enc = dpapi_protect(content.as_bytes()).map_err(|e| StoreError::Dpapi(e.to_string()))?;
        let search_text = secret_search_text(kind, exe, title);
        let hint = secret_hint(matched);
        let pinned: i64 = self.conn.query_row(
            "SELECT CASE WHEN pinned_at IS NOT NULL THEN 1 ELSE 0 END FROM clips WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;
        let expires_at = if pinned == 1 {
            None
        } else {
            Some(secret_expiry(now_ms, secret_ttl_hours))
        };
        self.conn.execute(
            "UPDATE clips SET is_secret = 1, kind = ?1, content = NULL, content_enc = ?2,
             search_text = ?3, expires_at = ?4, secret_hint = ?5 WHERE id = ?6",
            params![kind.slug(), enc, search_text, expires_at, hint, id],
        )?;
        self.delete_embedding(id)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct RescanReport {
    pub scanned: usize,
    pub converted: usize,
    pub reembed: Vec<(i64, String)>,
}

const HIT_COLS: &str = "c.id, c.last_seen_at, COALESCE(c.source_exe, ''), COALESCE(c.content, ''),
                        c.is_secret, COALESCE(c.kind, ''), COALESCE(c.secret_hint, ''),
                        COALESCE(c.tags_text, ''), CASE WHEN c.pinned_at IS NOT NULL THEN 1 ELSE 0 END";

const CARD_COLS: &str = "c.id, c.last_seen_at, c.copy_count, COALESCE(c.source_exe, ''),
                         COALESCE(c.window_title, ''), COALESCE(c.content, ''),
                         c.is_secret, COALESCE(c.kind, ''), COALESCE(c.secret_hint, ''),
                         CASE WHEN c.pinned_at IS NOT NULL THEN 1 ELSE 0 END";

fn card_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipCard> {
    let content: String = row.get(5)?;
    let is_secret: i64 = row.get(6)?;
    let kind_col: String = row.get(7)?;
    let hint: String = row.get(8)?;
    let pinned: i64 = row.get(9)?;
    let title: String = row.get(4)?;
    let preview = hit_preview(is_secret, &kind_col, &hint, &content);
    let body = if is_secret == 1 {
        preview.clone()
    } else {
        content.clone()
    };
    let title = if is_secret == 1 {
        crate::secrets::redact_secrets(&title)
    } else {
        title
    };
    Ok(ClipCard {
        id: row.get(0)?,
        last_seen_at: row.get(1)?,
        copy_count: row.get(2)?,
        source_exe: row.get(3)?,
        window_title: title,
        preview,
        body,
        kind: display_kind(is_secret, &kind_col, &content),
        tags: Vec::new(),
        pinned: pinned == 1,
    })
}

fn display_kind(is_secret: i64, kind_col: &str, content: &str) -> String {
    if is_secret == 1 {
        return "secret".into();
    }
    if kind_col == "url"
        || content.starts_with("http://")
        || content.starts_with("https://")
    {
        return "link".into();
    }
    if looks_like_code(content) {
        return "code".into();
    }
    "text".into()
}

fn looks_like_code(text: &str) -> bool {
    let lines = text.lines().count();
    let has_block = text.contains('{') && text.contains('}');
    let indented = text.lines().filter(|l| l.starts_with("    ") || l.starts_with('\t')).count();
    has_block || (lines >= 3 && indented >= 2)
}

fn hit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchHit> {
    let content: String = row.get(3)?;
    let is_secret: i64 = row.get(4)?;
    let kind: String = row.get(5)?;
    let hint: String = row.get(6)?;
    let tags: String = row.get(7)?;
    let pinned: i64 = row.get(8)?;
    Ok(SearchHit {
        id: row.get(0)?,
        last_seen_at: row.get(1)?,
        source_exe: row.get(2)?,
        preview: hit_preview(is_secret, &kind, &hint, &content),
        tags,
        pinned: pinned == 1,
    })
}

fn hit_preview(is_secret: i64, kind: &str, hint: &str, content: &str) -> String {
    if is_secret == 1 {
        let label = SecretKind::from_slug(kind)
            .map(|k| mask_secret(k, hint))
            .unwrap_or_else(|| "[secret …]".to_string());
        return label;
    }
    content.chars().take(60).collect()
}

fn secret_expiry(now_ms: i64, secret_ttl_hours: u32) -> i64 {
    if secret_ttl_hours == 0 {
        now_ms.saturating_sub(1)
    } else {
        now_ms.saturating_add(i64::from(secret_ttl_hours) * 3_600_000)
    }
}

fn opt_i64(v: Option<i64>) -> Value {
    match v {
        Some(n) => Value::Integer(n),
        None => Value::Null,
    }
}

fn browse_clause(filter: &BrowseFilter) -> String {
    let mut extra = String::new();
    if filter.pinned_only {
        extra.push_str(" AND c.pinned_at IS NOT NULL");
    }
    for _ in &filter.tags {
        extra.push_str(
            " AND EXISTS (SELECT 1 FROM clip_tags ct JOIN tags t ON t.id = ct.tag_id \
             WHERE ct.clip_id = c.id AND t.name LIKE '%' || ? || '%' COLLATE NOCASE)",
        );
    }
    extra
}

fn push_tag_binds(binds: &mut Vec<Value>, filter: &BrowseFilter) {
    for tag in &filter.tags {
        binds.push(Value::Text(tag.clone()));
    }
}

fn migrate_pins_and_tags(conn: &Connection) -> Result<(), StoreError> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 2 {
        let mut stmt = conn.prepare("PRAGMA table_info(clips)")?;
        let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_pinned = false;
        let mut has_tags_text = false;
        for name in names {
            let name = name?;
            if name == "pinned_at" {
                has_pinned = true;
            }
            if name == "tags_text" {
                has_tags_text = true;
            }
        }
        drop(stmt);
        if !has_pinned {
            conn.execute("ALTER TABLE clips ADD COLUMN pinned_at INTEGER", [])?;
        }
        if !has_tags_text {
            conn.execute(
                "ALTER TABLE clips ADD COLUMN tags_text TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }
        conn.execute_batch(
            r#"
CREATE TABLE IF NOT EXISTS tags (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE
);
CREATE TABLE IF NOT EXISTS clip_tags (
  clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
  tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (clip_id, tag_id)
);
"#,
        )?;
        rebuild_fts_v2(conn)?;
        conn.pragma_update(None, "user_version", 2)?;
    } else {
        ensure_fts_triggers_v2(conn)?;
    }
    Ok(())
}

fn rebuild_fts_v2(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        r#"
DROP TRIGGER IF EXISTS clips_ai;
DROP TRIGGER IF EXISTS clips_ad;
DROP TRIGGER IF EXISTS clips_au;
DROP TABLE IF EXISTS clips_fts;
CREATE VIRTUAL TABLE clips_fts USING fts5(
  search_text, tags_text, content='clips', content_rowid='id', tokenize='porter unicode61'
);
"#,
    )?;
    ensure_fts_triggers_v2(conn)?;
    conn.execute("INSERT INTO clips_fts(clips_fts) VALUES('rebuild')", [])?;
    Ok(())
}

fn ensure_fts_triggers_v2(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        r#"
DROP TRIGGER IF EXISTS clips_ai;
DROP TRIGGER IF EXISTS clips_ad;
DROP TRIGGER IF EXISTS clips_au;
CREATE TRIGGER clips_ai AFTER INSERT ON clips BEGIN
  INSERT INTO clips_fts(rowid, search_text, tags_text) VALUES (new.id, new.search_text, new.tags_text);
END;
CREATE TRIGGER clips_ad AFTER DELETE ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, search_text, tags_text)
    VALUES('delete', old.id, old.search_text, old.tags_text);
END;
CREATE TRIGGER clips_au AFTER UPDATE ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, search_text, tags_text)
    VALUES('delete', old.id, old.search_text, old.tags_text);
  INSERT INTO clips_fts(rowid, search_text, tags_text)
    VALUES (new.id, new.search_text, new.tags_text);
END;
"#,
    )?;
    Ok(())
}

fn migrate_secret_hint(conn: &Connection) -> Result<(), StoreError> {
    let mut stmt = conn.prepare("PRAGMA table_info(clips)")?;
    let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_hint = false;
    for name in names {
        if name? == "secret_hint" {
            has_hint = true;
            break;
        }
    }
    if !has_hint {
        conn.execute("ALTER TABLE clips ADD COLUMN secret_hint TEXT", [])?;
    }
    Ok(())
}

fn map_hits(
    rows: impl Iterator<Item = rusqlite::Result<SearchHit>>,
) -> Result<Vec<SearchHit>, StoreError> {
    let mut hits = Vec::new();
    for row in rows {
        hits.push(row?);
    }
    Ok(hits)
}

pub fn l2_normalize(vector: &[f32]) -> Option<Vec<f32>> {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return None;
    }
    Some(vector.iter().map(|x| x / norm).collect())
}

pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Higher score first. Ties keep input order.
pub fn rank_by_score<'a>(
    mut scored: Vec<(f32, &'a StoredEmbedding)>,
    limit: usize,
) -> Vec<&'a StoredEmbedding> {
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(limit.max(1))
        .map(|(_, row)| row)
        .collect()
}

fn f32_le_blob(vector: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vector.len() * 4);
    for value in vector {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}

fn f32_le_from_blob(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

pub fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n").trim().to_string()
}

pub fn clip_hash(normalized: &str) -> String {
    let digest = Sha256::digest(normalized.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn classify_kind(text: &str) -> &'static str {
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        "url"
    } else {
        "other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db() -> (std::path::PathBuf, Store) {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cliptrace-m2-{n}"));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("cliptrace.db");
        let store = Store::open(&path).expect("open store");
        (dir, store)
    }

    fn clip(content: &str) -> NewClip {
        NewClip {
            content: content.to_string(),
            source_exe: "notepad.exe".to_string(),
            window_title: "Untitled".to_string(),
        }
    }

    #[test]
    fn normalize_trims_and_unifies_newlines() {
        assert_eq!(normalize_text("  hello\r\nworld\r  "), "hello\nworld");
    }

    #[test]
    fn hash_is_sha256_of_normalized_text() {
        let a = clip_hash(&normalize_text("  Hello\r\n"));
        let b = clip_hash(&normalize_text("Hello\n"));
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert_ne!(clip_hash("Hello"), clip_hash("hello"));
    }

    #[test]
    fn duplicate_hash_bumps_copy_count() {
        let (dir, store) = temp_db();
        let first = store.upsert_clip(&clip("same text"), 1_000, 24).expect("insert");
        let UpsertOutcome::Inserted { id } = first else {
            panic!("expected insert");
        };
        let second = store
            .upsert_clip(&clip("  same text  "), 2_000, 24)
            .expect("dedupe");
        assert_eq!(
            second,
            UpsertOutcome::Duplicate {
                id,
                copy_count: 2
            }
        );
        let (last_seen, copy_count, count): (i64, i64, i64) = store
            .conn
            .query_row(
                "SELECT last_seen_at, copy_count, COUNT(*) FROM clips",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query");
        assert_eq!(last_seen, 2_000);
        assert_eq!(copy_count, 2);
        assert_eq!(count, 1);

        let other = store.upsert_clip(&clip("other text"), 3_000, 24).expect("other");
        assert!(matches!(other, UpsertOutcome::Inserted { id: other_id } if other_id != id));

        drop(store);
        let db = dir.join("cliptrace.db");
        let reopened = Store::open(&db).expect("reopen");
        let copy_count: i64 = reopened
            .conn
            .query_row("SELECT copy_count FROM clips WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .expect("persisted");
        assert_eq!(copy_count, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keyword_search_ranks_by_bm25_and_escapes_specials() {
        let (dir, store) = temp_db();
        store
            .upsert_clip(&clip("unrelated banana muffin recipe"), 1_000, 24)
            .unwrap();
        let stripe_once = store
            .upsert_clip(&clip("pay with stripe once"), 2_000, 24)
            .unwrap();
        let stripe_many = store
            .upsert_clip(
                &clip("stripe stripe stripe payment dashboard for stripe keys"),
                3_000,
                24,
            )
            .unwrap();
        let UpsertOutcome::Inserted { id: once_id } = stripe_once else {
            panic!("once");
        };
        let UpsertOutcome::Inserted { id: many_id } = stripe_many else {
            panic!("many");
        };

        let hits = store.search_keyword("stripe", 5, None).expect("search");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, many_id, "higher term frequency ranks first");
        assert_eq!(hits[1].id, once_id);

        store
            .search_keyword(r#"stripe -foo "bar" a:b *"#, 5, None)
            .expect("special chars must not be FTS syntax errors");

        let limited = store.search_keyword("stripe", 1, None).expect("limit");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].id, many_id);

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn axis(dim: usize) -> Vec<f32> {
        let mut v = vec![0.0f32; EMBED_DIMS];
        v[dim] = 1.0;
        v
    }

    fn dummy_row(id: i64, embedding: Vec<f32>) -> StoredEmbedding {
        StoredEmbedding {
            clip_id: id,
            last_seen_at: 0,
            source_exe: String::new(),
            preview: String::new(),
            model: "test".into(),
            embedding,
        }
    }

    #[test]
    fn cosine_ranks_hand_built_vectors() {
        let query = axis(0);
        let near = dummy_row(1, axis(0));
        let mid = dummy_row(2, l2_normalize(&{
            let mut v = vec![0.0f32; EMBED_DIMS];
            v[0] = 0.8;
            v[1] = 0.2;
            v
        }).unwrap());
        let far = dummy_row(3, axis(1));
        let scored = vec![
            (dot(&query, &far.embedding), &far),
            (dot(&query, &near.embedding), &near),
            (dot(&query, &mid.embedding), &mid),
        ];
        let ranked = rank_by_score(scored, 3);
        assert_eq!(ranked[0].clip_id, 1);
        assert_eq!(ranked[1].clip_id, 2);
        assert_eq!(ranked[2].clip_id, 3);

        let (dir, store) = temp_db();
        let a = store.upsert_clip(&clip("axis zero"), 1_000, 24).unwrap();
        let b = store.upsert_clip(&clip("axis one"), 2_000, 24).unwrap();
        let UpsertOutcome::Inserted { id: id_a } = a else { panic!("a") };
        let UpsertOutcome::Inserted { id: id_b } = b else { panic!("b") };
        assert_eq!(store.clips_missing_embeddings().unwrap().len(), 2);
        store.upsert_embedding(id_a, &axis(0)).unwrap();
        store.upsert_embedding(id_b, &axis(1)).unwrap();
        assert!(store.clips_missing_embeddings().unwrap().is_empty());
        let in_range = store.list_embeddings(Some((1_500, 2_500))).unwrap();
        assert_eq!(in_range.len(), 1);
        assert_eq!(in_range[0].clip_id, id_b);
        let hits = store.search_vector(&axis(0), 2, None).unwrap();
        assert_eq!(hits[0].id, id_a);
        assert_eq!(hits[1].id, id_b);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn secret_raw_value_not_in_db_or_fts() {
        let (dir, store) = temp_db();
        let key = "sk_test_4eC39HqLyjWDarjtT1zdp7dc";
        let ins = store
            .upsert_clip(
                &NewClip {
                    content: key.to_string(),
                    source_exe: "chrome.exe".to_string(),
                    window_title: "Stripe Dashboard - API keys".to_string(),
                },
                10_000,
                24,
            )
            .unwrap();
        let UpsertOutcome::Inserted { id } = ins else {
            panic!("insert");
        };
        let preview = store.display_preview(id).unwrap();
        assert!(preview.contains("stripe key"));
        assert!(!preview.contains("4eC39"));
        let hits = store.search_keyword("stripe key", 5, None).unwrap();
        assert_eq!(hits[0].id, id);
        assert!(
            store
                .search_keyword("4eC39HqLyjWDarjtT1zdp7dc", 5, None)
                .unwrap()
                .is_empty()
        );
        store
            .conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
        drop(store);
        let db = dir.join("cliptrace.db");
        let bytes = std::fs::read(&db).unwrap();
        assert!(
            !bytes.windows(key.len()).any(|w| w == key.as_bytes()),
            "raw key leaked into db file"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn purge_honors_injected_now() {
        let (dir, store) = temp_db();
        let secret = store
            .upsert_clip(&clip("sk_test_abcdefghijklmnop"), 1_000, 0)
            .unwrap();
        let old = store
            .upsert_clip(&clip("old prose sentence here"), 1_000, 24)
            .unwrap();
        let fresh = store
            .upsert_clip(&clip("fresh prose sentence here"), 50_000, 24)
            .unwrap();
        let UpsertOutcome::Inserted { id: sid } = secret else {
            panic!("secret");
        };
        let UpsertOutcome::Inserted { id: oid } = old else {
            panic!("old");
        };
        let UpsertOutcome::Inserted { id: fid } = fresh else {
            panic!("fresh");
        };
        let n = store.purge(10_000, 0, 24).unwrap();
        assert!(n >= 2);
        let left: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM clips WHERE id = ?1", [fid], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(left, 1);
        let gone_s: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM clips WHERE id = ?1", [sid], |row| {
                row.get(0)
            })
            .unwrap();
        let gone_o: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM clips WHERE id = ?1", [oid], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(gone_s, 0);
        assert_eq!(gone_o, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn insert_pre_m6_plain(store: &Store, content: &str, now_ms: i64) {
        let normalized = normalize_text(content);
        let hash = clip_hash(&normalized);
        store
            .conn
            .execute(
                "INSERT INTO clips (
                    hash, first_seen_at, last_seen_at, copy_count, kind, is_secret,
                    content, content_enc, search_text, source_exe, window_title, expires_at
                ) VALUES (?1, ?2, ?2, 1, 'other', 0, ?3, NULL, ?3, 'chrome.exe', 'legacy', NULL)",
                params![hash, now_ms, normalized],
            )
            .expect("pre-m6 insert");
    }

    fn assert_key_absent_on_disk(store: &Store, key: &str) {
        let needle = key.as_bytes();
        for suffix in ["", "-wal", "-shm"] {
            let mut path = store.db_path.clone();
            if !suffix.is_empty() {
                let mut name = path.as_os_str().to_os_string();
                name.push(suffix);
                path = PathBuf::from(name);
            }
            if !path.exists() {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            assert!(
                !bytes.windows(needle.len()).any(|w| w == needle),
                "raw key leaked into {}",
                path.display()
            );
        }
    }

    #[test]
    fn rescan_then_purge_wipes_plaintext_key_from_db_files() {
        let (dir, store) = temp_db();
        let key = "sk_test_ScrubByteScanKey99xx";
        insert_pre_m6_plain(&store, key, 1_000);
        let found: String = store
            .conn
            .query_row("SELECT content FROM clips WHERE content = ?1", [key], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(found, key);

        let report = store.rescan_secrets(2_000, 24).unwrap();
        assert_eq!(report.converted, 1);
        let n = store.purge(3_000, 90, 0).unwrap();
        assert!(n >= 1);
        store.scrub().unwrap();
        assert_key_absent_on_disk(&store, key);
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_v1_schema_and_keeps_rows() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cliptrace-m8a-{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cliptrace.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
CREATE TABLE clips (
  id INTEGER PRIMARY KEY,
  hash TEXT NOT NULL UNIQUE,
  first_seen_at INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL,
  copy_count INTEGER NOT NULL DEFAULT 1,
  kind TEXT NOT NULL,
  is_secret INTEGER NOT NULL DEFAULT 0,
  content TEXT,
  content_enc BLOB,
  search_text TEXT NOT NULL,
  source_exe TEXT,
  window_title TEXT,
  expires_at INTEGER,
  secret_hint TEXT
);
CREATE VIRTUAL TABLE clips_fts USING fts5(
  search_text, content='clips', content_rowid='id', tokenize='porter unicode61'
);
INSERT INTO clips (hash, first_seen_at, last_seen_at, copy_count, kind, is_secret, content, search_text, source_exe, window_title)
VALUES ('abc', 1, 1, 1, 'other', 0, 'hello sample', 'hello sample', 'notepad.exe', 't');
"#,
        )
        .unwrap();
        drop(conn);
        let store = Store::open(&path).unwrap();
        let version: i64 = store
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let (content, tags, pinned): (String, String, Option<i64>) = store
            .conn
            .query_row(
                "SELECT content, tags_text, pinned_at FROM clips WHERE hash = 'abc'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(content, "hello sample");
        assert_eq!(tags, "");
        assert!(pinned.is_none());
        let sql: String = store
            .conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name = 'clips_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(sql.contains("tags_text"));
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn purge_skips_pinned_clips() {
        let (dir, store) = temp_db();
        let keep = store.upsert_clip(&clip("keep pinned prose"), 1_000, 24).unwrap();
        let gone = store.upsert_clip(&clip("drop old prose here"), 1_000, 24).unwrap();
        let UpsertOutcome::Inserted { id: kid } = keep else { panic!("keep") };
        let UpsertOutcome::Inserted { id: gid } = gone else { panic!("gone") };
        store.pin_clip(kid, 1_000).unwrap();
        let n = store.purge(10_000, 0, 24).unwrap();
        assert!(n >= 1);
        let left: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM clips WHERE id = ?1", [kid], |row| row.get(0))
            .unwrap();
        let dropped: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM clips WHERE id = ?1", [gid], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 1);
        assert_eq!(dropped, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fts_finds_clip_by_tag_only() {
        let (dir, store) = temp_db();
        let ins = store.upsert_clip(&clip("hello world uniquebody"), 1_000, 24).unwrap();
        let UpsertOutcome::Inserted { id } = ins else { panic!("ins") };
        store.tag_clip(id, "RBC app").unwrap();
        let hits = store.search_keyword("rbc", 5, None).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, id);
        let none = store.search_keyword("hunter2", 5, None).unwrap();
        assert!(none.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mark_secret_and_delete_wipe_plaintext_bytes() {
        let (dir, store) = temp_db();
        let key = "hunter2";
        let ins = store.upsert_clip(&clip(key), 1_000, 24).unwrap();
        let UpsertOutcome::Inserted { id } = ins else { panic!("ins") };
        store.tag_clip(id, "RBC app").unwrap();
        store.mark_secret(id, 2_000, 24).unwrap();
        let preview = store.display_preview(id).unwrap();
        assert!(!preview.contains(key));
        assert!(store.search_keyword(key, 5, None).unwrap().is_empty());
        let tagged = store.search_keyword("rbc", 5, None).unwrap();
        assert_eq!(tagged[0].id, id);
        assert_eq!(store.clip_content(id).unwrap(), key);
        store.delete_clip(id).unwrap();
        assert_key_absent_on_disk(&store, key);
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
