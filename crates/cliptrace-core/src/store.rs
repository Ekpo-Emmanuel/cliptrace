use std::path::Path;

use crate::search::escape_fts_query;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use thiserror::Error;

const SEARCH_TEXT_CHARS: usize = 2000;

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
"#;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("clip {0} not found")]
    NotFound(i64),
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

    pub fn search_keyword(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, StoreError> {
        let Some(match_query) = escape_fts_query(query) else {
            return Ok(Vec::new());
        };
        let limit = limit.max(1) as i64;
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.last_seen_at, COALESCE(c.source_exe, ''), COALESCE(c.content, '')
             FROM clips_fts
             JOIN clips c ON c.id = clips_fts.rowid
             WHERE clips_fts MATCH ?1
             ORDER BY bm25(clips_fts) ASC, c.last_seen_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![match_query, limit], |row| {
            let content: String = row.get(3)?;
            Ok(SearchHit {
                id: row.get(0)?,
                last_seen_at: row.get(1)?,
                source_exe: row.get(2)?,
                preview: content.chars().take(60).collect(),
            })
        })?;
        let mut hits = Vec::new();
        for row in rows {
            hits.push(row?);
        }
        Ok(hits)
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

        let hits = store.search_keyword("stripe", 5).expect("search");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, many_id, "higher term frequency ranks first");
        assert_eq!(hits[1].id, once_id);

        store
            .search_keyword(r#"stripe -foo "bar" a:b *"#, 5)
            .expect("special chars must not be FTS syntax errors");

        let limited = store.search_keyword("stripe", 1).expect("limit");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].id, many_id);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
