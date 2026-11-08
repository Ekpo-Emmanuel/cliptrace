use std::path::Path;

use crate::search::fts_or_query;
use rusqlite::{Connection, OptionalExtension, params};
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub id: i64,
    pub last_seen_at: i64,
    pub source_exe: String,
    pub preview: String,
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
}

impl Store {
    pub fn open(db_path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn upsert_clip(&self, clip: &NewClip, now_ms: i64) -> Result<UpsertOutcome, StoreError> {
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

        let search_text: String = normalized.chars().take(SEARCH_TEXT_CHARS).collect();
        let kind = classify_kind(&normalized);
        self.conn.execute(
            "INSERT INTO clips (
                hash, first_seen_at, last_seen_at, copy_count, kind, is_secret,
                content, content_enc, search_text, source_exe, window_title, expires_at
            ) VALUES (?1, ?2, ?3, 1, ?4, 0, ?5, NULL, ?6, ?7, ?8, NULL)",
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
        let limit = limit.max(1) as i64;
        let (start, end) = match range {
            Some((s, e)) => (Some(s), Some(e)),
            None => (None, None),
        };
        let overlap = "(?2 IS NULL OR (c.first_seen_at <= ?3 AND c.last_seen_at >= ?2))";
        if let Some(match_query) = fts_or_query(query) {
            let sql = format!(
                "SELECT c.id, c.last_seen_at, COALESCE(c.source_exe, ''), COALESCE(c.content, '')
                 FROM clips_fts
                 JOIN clips c ON c.id = clips_fts.rowid
                 WHERE clips_fts MATCH ?1 AND {overlap}
                 ORDER BY bm25(clips_fts) ASC, c.last_seen_at DESC
                 LIMIT ?4"
            );
            let mut stmt = self.conn.prepare(&sql)?;
            return map_hits(stmt.query_map(params![match_query, start, end, limit], hit_from_row)?);
        }
        if range.is_some() {
            let sql = format!(
                "SELECT c.id, c.last_seen_at, COALESCE(c.source_exe, ''), COALESCE(c.content, '')
                 FROM clips c
                 WHERE {overlap}
                 ORDER BY c.last_seen_at DESC
                 LIMIT ?4"
            );
            let mut stmt = self.conn.prepare(&sql)?;
            return map_hits(stmt.query_map(params!["", start, end, limit], hit_from_row)?);
        }
        Ok(Vec::new())
    }

    pub fn clip_content(&self, id: i64) -> Result<String, StoreError> {
        let content: Option<String> = self
            .conn
            .query_row(
                "SELECT content FROM clips WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        content.ok_or(StoreError::NotFound(id))
    }

    pub fn clip_search_text(&self, id: i64) -> Result<String, StoreError> {
        self.conn
            .query_row(
                "SELECT search_text FROM clips WHERE id = ?1",
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
            "SELECT c.id, c.search_text
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
        let sql = "SELECT e.clip_id, c.last_seen_at, COALESCE(c.source_exe, ''),
                          COALESCE(c.content, ''), e.model, e.embedding
                   FROM clip_embeddings e
                   JOIN clips c ON c.id = e.clip_id
                   WHERE (?1 IS NULL OR (c.first_seen_at <= ?2 AND c.last_seen_at >= ?1))";
        let (start, end) = match range {
            Some((s, e)) => (Some(s), Some(e)),
            None => (None, None),
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![start, end], |row| {
            let blob: Vec<u8> = row.get(5)?;
            let content: String = row.get(3)?;
            Ok(StoredEmbedding {
                clip_id: row.get(0)?,
                last_seen_at: row.get(1)?,
                source_exe: row.get(2)?,
                preview: content.chars().take(60).collect(),
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
        if vector.len() != EMBED_DIMS {
            return Err(StoreError::Embedding(format!(
                "query embedding len {} != {EMBED_DIMS}",
                vector.len()
            )));
        }
        let query = l2_normalize(vector).ok_or_else(|| {
            StoreError::Embedding("cannot L2-normalize a zero query vector".into())
        })?;
        let rows = self.list_embeddings(range)?;
        let scored: Vec<(f32, &StoredEmbedding)> = rows
            .iter()
            .filter(|row| row.embedding.len() == EMBED_DIMS)
            .map(|row| (dot(&query, &row.embedding), row))
            .collect();
        let ranked = rank_by_score(scored, limit);
        Ok(ranked
            .into_iter()
            .map(|row| SearchHit {
                id: row.clip_id,
                last_seen_at: row.last_seen_at,
                source_exe: row.source_exe.clone(),
                preview: row.preview.clone(),
            })
            .collect())
    }
}

fn hit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchHit> {
    let content: String = row.get(3)?;
    Ok(SearchHit {
        id: row.get(0)?,
        last_seen_at: row.get(1)?,
        source_exe: row.get(2)?,
        preview: content.chars().take(60).collect(),
    })
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
        let first = store.upsert_clip(&clip("same text"), 1_000).expect("insert");
        let UpsertOutcome::Inserted { id } = first else {
            panic!("expected insert");
        };
        let second = store
            .upsert_clip(&clip("  same text  "), 2_000)
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

        let other = store.upsert_clip(&clip("other text"), 3_000).expect("other");
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
            .upsert_clip(&clip("unrelated banana muffin recipe"), 1_000)
            .unwrap();
        let stripe_once = store
            .upsert_clip(&clip("pay with stripe once"), 2_000)
            .unwrap();
        let stripe_many = store
            .upsert_clip(
                &clip("stripe stripe stripe payment dashboard for stripe keys"),
                3_000,
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
        let a = store.upsert_clip(&clip("axis zero"), 1_000).unwrap();
        let b = store.upsert_clip(&clip("axis one"), 2_000).unwrap();
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
}
