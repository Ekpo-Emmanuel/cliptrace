//! Search benchmark on synthetic data.
//!
//! Fills a scratch database (in the system temp folder, deleted at the end) with N synthetic clips
//! and random unit vectors, then times the real search code at several sizes:
//! the FTS5 keyword search, the brute-force vector scan, and the full hybrid pipeline.
//!
//! Run it with:
//!
//! ```text
//! cargo run --release -p cliptrace-core --example bench_search
//! ```
//!
//! What the numbers do and do not tell you:
//! - Vector search cost does not depend on the vector values, so it is representative.
//! - The synthetic text uses only 72 distinct words, so most OR queries match most rows. That makes
//!   the keyword numbers pessimistic compared with real, more varied clipboard text.
//! - It does not include the time to embed the query with the ONNX model.
//! - Results depend on your machine. Compare runs on the same machine.
use std::time::Instant;

use cliptrace_core::{BrowseFilter, Store, hybrid_search};
use rusqlite::{Connection, params};

/// Small deterministic generator, so every run builds the same data. No `rand` dependency.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

const WORDS: &[&str] = &[
    "invoice", "contract", "payment", "meeting", "address", "email", "report", "budget", "client", "project",
    "deploy", "error", "stack", "trace", "query", "table", "index", "server", "token", "config", "release",
    "design", "review", "ticket", "sprint", "customer", "refund", "order", "shipping", "tracking", "number",
    "password", "reset", "login", "session", "cache", "build", "test", "branch", "merge", "commit", "issue",
    "weekly", "monthly", "update", "schedule", "calendar", "flight", "hotel", "booking", "receipt", "tax",
    "bank", "account", "transfer", "salary", "offer", "resume", "interview", "recruiter", "founder", "demo",
    "slack", "notion", "figma", "github", "docker", "rust", "react", "sqlite", "windows", "clipboard",
];

const QUERIES: &[&str] = &[
    "invoice payment",
    "deploy error stack",
    "contract client budget",
    "flight hotel booking",
    "reset password login",
];

fn fake_text(rng: &mut Lcg) -> String {
    let words = 6 + (rng.next() % 14) as usize;
    let mut parts: Vec<String> = (0..words)
        .map(|_| WORDS[(rng.next() as usize) % WORDS.len()].to_string())
        .collect();
    // A unique token, so every row is distinct.
    parts.push(format!("ref{}", rng.next() % 1_000_000));
    parts.join(" ")
}

fn unit_vector(rng: &mut Lcg) -> Vec<f32> {
    let mut v: Vec<f32> = (0..384)
        .map(|_| (rng.next() % 2000) as f32 / 1000.0 - 1.0)
        .collect();
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    for x in &mut v {
        *x /= norm;
    }
    v
}

fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn median(mut ms: Vec<f64>) -> f64 {
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ms[ms.len() / 2]
}

fn main() {
    let dir = std::env::temp_dir().join("cliptrace-bench");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let db = dir.join("bench.db");

    // Opening the store creates the real schema, triggers and indexes.
    let store = Store::open(&db).expect("open store");
    // Bulk inserts go through a second connection in one transaction per batch, because
    // `upsert_clip` commits (and syncs) one row at a time.
    let mut conn = Connection::open(&db).expect("open bulk connection");

    let mut rng = Lcg(42);
    let now_ms: i64 = 1_800_000_000_000;
    let filter = BrowseFilter::default();
    let mut have = 0usize;

    println!("clips,db_mb,vectors_mb,keyword_ms,vector_ms,hybrid_ms");
    for target in [1_000usize, 10_000, 50_000, 100_000] {
        let tx = conn.transaction().expect("begin");
        {
            let mut insert_clip = tx
                .prepare(
                    "INSERT INTO clips (hash, first_seen_at, last_seen_at, copy_count, kind, is_secret,
                                        content, search_text, source_exe, window_title)
                     VALUES (?1, ?2, ?3, 1, 'other', 0, ?4, ?4, 'app.exe', 'title')",
                )
                .expect("prepare clip insert");
            let mut insert_vec = tx
                .prepare(
                    "INSERT INTO clip_embeddings (clip_id, model, embedding) VALUES (?1, 'bge-small-en-v1.5', ?2)",
                )
                .expect("prepare vector insert");
            while have < target {
                let text = fake_text(&mut rng);
                let t = now_ms - (have as i64) * 60_000;
                insert_clip
                    .execute(params![format!("h{have}"), t, t, text])
                    .expect("insert clip");
                let id = tx.last_insert_rowid();
                insert_vec
                    .execute(params![id, to_blob(&unit_vector(&mut rng))])
                    .expect("insert vector");
                have += 1;
            }
        }
        tx.commit().expect("commit");

        let query_vector = unit_vector(&mut rng);
        let (mut keyword, mut vector, mut hybrid) = (Vec::new(), Vec::new(), Vec::new());
        for round in 0..10 {
            for query in QUERIES {
                let t = Instant::now();
                store
                    .search_keyword_scoped(query, 50, None, &filter)
                    .expect("keyword search");
                keyword.push(t.elapsed().as_secs_f64() * 1000.0);
                // The vector scan is slow at large sizes, so it gets fewer repetitions.
                if round < 3 {
                    let t = Instant::now();
                    store
                        .search_vector_scoped(&query_vector, 50, None, &filter)
                        .expect("vector search");
                    vector.push(t.elapsed().as_secs_f64() * 1000.0);
                    let t = Instant::now();
                    hybrid_search(&store, query, now_ms, 50, Some(&query_vector), &filter)
                        .expect("hybrid search");
                    hybrid.push(t.elapsed().as_secs_f64() * 1000.0);
                }
            }
        }
        let db_mb = std::fs::metadata(&db).map(|m| m.len()).unwrap_or(0) as f64 / 1_048_576.0;
        let vectors_mb = (have * 384 * 4) as f64 / 1_048_576.0;
        println!(
            "{have},{db_mb:.0},{vectors_mb:.0},{:.1},{:.1},{:.1}",
            median(keyword),
            median(vector),
            median(hybrid)
        );
    }

    // Close both connections first: Windows will not delete an open database file.
    drop(conn);
    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
}
