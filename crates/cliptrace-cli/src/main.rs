use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
#[cfg(feature = "embeddings")]
use cliptrace_core::{EmbedWorker, Embedder, run_embed_test};
use cliptrace_core::{
    CaptureConfig, NewClip, Paths, Store, UpsertOutcome, load_or_create, relative_time,
    run_listener, set_clipboard_text,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("watch") => watch(),
        Some("search") => search_cmd(&args.collect::<Vec<_>>()),
        Some("vsearch") => vsearch_cmd(&args.collect::<Vec<_>>()),
        Some("backfill") => backfill_cmd(),
        Some("embed-test") => {
            let text = args.next().unwrap_or_else(|| "hello".to_string());
            embed_test_cmd(&text)
        }
        Some("copy") => {
            let Some(id) = args.next() else {
                bail!("Usage: cliptrace copy <id>");
            };
            copy_cmd(&id)
        }
        _ => {
            eprintln!(
                "Usage: cliptrace <watch | search [--limit N] <query> | vsearch [--limit N] <query> | backfill | embed-test <text> | copy <id>>"
            );
            std::process::exit(2);
        }
    }
}

fn open_paths() -> Result<Paths> {
    let paths = Paths::resolve().context("resolve data dir")?;
    paths.ensure_data_dir().context("create data dir")?;
    Ok(paths)
}

fn open_store() -> Result<Store> {
    let paths = open_paths()?;
    Store::open(&paths.db_path).context("open cliptrace.db")
}

fn embed_test_cmd(text: &str) -> Result<()> {
    #[cfg(feature = "embeddings")]
    {
        let paths = open_paths()?;
        eprintln!(
            "embed-test: cache={} text_len={}",
            paths.models_dir.display(),
            text.len()
        );
        let reports = run_embed_test(&paths.models_dir, text).context("onnx/fastembed embed-test")?;
        for r in reports {
            println!(
                "{}\tdims={}\tload_ms={}\tembed_ms={}\tfirst3={:.6} {:.6} {:.6}",
                r.model, r.dims, r.load_ms, r.embed_ms, r.first3[0], r.first3[1], r.first3[2]
            );
        }
        Ok(())
    }
    #[cfg(not(feature = "embeddings"))]
    {
        let _ = text;
        bail!("rebuild with --features embeddings");
    }
}

fn watch() -> Result<()> {
    let paths = open_paths()?;
    let config = load_or_create(&paths.config_path).context("load config.toml")?;
    let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
    #[cfg(feature = "embeddings")]
    let worker = EmbedWorker::spawn(paths.models_dir.clone(), paths.db_path.clone())
        .context("start embedding worker")?;

    eprintln!(
        "cliptrace watch: storing clips in {} (Ctrl+C to stop)",
        paths.db_path.display()
    );

    run_listener(
        CaptureConfig {
            blocklist: config.blocklist,
        },
        |event| {
            let now_ms = unix_ms();
            let outcome = store.upsert_clip(
                &NewClip {
                    content: event.text.clone(),
                    source_exe: event.source_exe.clone(),
                    window_title: event.window_title.clone(),
                },
                now_ms,
            );
            match outcome {
                Ok(UpsertOutcome::Inserted { id }) => {
                    println!(
                        "stored\t{}\t{}\t{}\t{}\t{}",
                        id,
                        event.timestamp,
                        event.source_exe,
                        event.window_title,
                        preview(&event.text)
                    );
                    #[cfg(feature = "embeddings")]
                    match store.clip_search_text(id) {
                        Ok(search_text) => worker.submit(id, search_text),
                        Err(err) => eprintln!("embed skip id={id}: {err}"),
                    }
                }
                Ok(UpsertOutcome::Duplicate { id, copy_count }) => {
                    println!(
                        "duplicate\t{}\t{}\t{}\t{}\t{}\tcount={}",
                        id,
                        event.timestamp,
                        event.source_exe,
                        event.window_title,
                        preview(&event.text),
                        copy_count
                    );
                }
                Err(err) => {
                    eprintln!(
                        "store error exe={} title_len={} text_len={}: {err}",
                        event.source_exe,
                        event.window_title.len(),
                        event.text.len()
                    );
                }
            }
        },
    )
    .context("clipboard listener")?;
    Ok(())
}

fn search_cmd(args: &[String]) -> Result<()> {
    let (query, limit) = parse_search_args(args, "search")?;
    let store = open_store()?;
    print_hits(store.search_keyword(&query, limit).context("fts5 search")?)
}

fn vsearch_cmd(args: &[String]) -> Result<()> {
    #[cfg(feature = "embeddings")]
    {
        let (query, limit) = parse_search_args(args, "vsearch")?;
        let paths = open_paths()?;
        let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
        let mut embedder = Embedder::load(&paths.models_dir).context("load embedding model")?;
        let vector = embedder.embed_one(&query).context("embed query")?;
        print_hits(store.search_vector(&vector, limit).context("vector search")?)
    }
    #[cfg(not(feature = "embeddings"))]
    {
        let _ = args;
        bail!("rebuild with --features embeddings");
    }
}

fn backfill_cmd() -> Result<()> {
    #[cfg(feature = "embeddings")]
    {
        let paths = open_paths()?;
        let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
        let missing = store
            .clips_missing_embeddings()
            .context("list clips missing vectors")?;
        let total = missing.len();
        if total == 0 {
            eprintln!("backfill: 0 missing");
            return Ok(());
        }
        eprintln!("backfill: {total} missing vectors");
        let mut embedder = Embedder::load(&paths.models_dir).context("load embedding model")?;
        let mut done = 0usize;
        for (id, search_text) in missing {
            let text_len = search_text.len();
            let vector = embedder
                .embed_one(&search_text)
                .with_context(|| format!("embed id={id} text_len={text_len}"))?;
            store
                .upsert_embedding(id, &vector)
                .with_context(|| format!("store vector id={id}"))?;
            done += 1;
            eprintln!("backfill {done}/{total} id={id} dims=384 text_len={text_len}");
        }
        eprintln!("backfill done embedded={done}");
        Ok(())
    }
    #[cfg(not(feature = "embeddings"))]
    {
        bail!("rebuild with --features embeddings");
    }
}

fn print_hits(hits: Vec<cliptrace_core::SearchHit>) -> Result<()> {
    if hits.is_empty() {
        eprintln!("no matches");
        return Ok(());
    }
    let now_ms = unix_ms();
    for hit in hits {
        println!(
            "{}\t{}\t{}\t{}",
            hit.id,
            relative_time(hit.last_seen_at, now_ms),
            hit.source_exe,
            hit.preview
        );
    }
    Ok(())
}

fn parse_search_args(args: &[String], cmd: &str) -> Result<(String, usize)> {
    let mut limit = 5usize;
    let mut query_parts: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--limit" {
            i += 1;
            let Some(raw) = args.get(i) else {
                bail!("Usage: cliptrace {cmd} [--limit N] <query>");
            };
            limit = parse_limit(raw)?;
            i += 1;
        } else if let Some(raw) = args[i].strip_prefix("--limit=") {
            limit = parse_limit(raw)?;
            i += 1;
        } else {
            query_parts.push(args[i].as_str());
            i += 1;
        }
    }
    let query = query_parts.join(" ");
    if query.trim().is_empty() {
        bail!("Usage: cliptrace {cmd} [--limit N] <query>");
    }
    Ok((query, limit))
}

fn parse_limit(raw: &str) -> Result<usize> {
    let n: usize = raw.parse().context("--limit must be a number")?;
    if n == 0 {
        bail!("--limit must be >= 1");
    }
    Ok(n)
}

fn copy_cmd(id_raw: &str) -> Result<()> {
    let id: i64 = id_raw.parse().context("id must be an integer")?;
    let store = open_store()?;
    let content = store.clip_content(id)?;
    set_clipboard_text(&content).context("set clipboard")?;
    eprintln!("copied id={id} ({len} chars)", len = content.len());
    Ok(())
}

fn preview(text: &str) -> String {
    text.chars().take(60).collect()
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
