use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use cliptrace_core::{
    CaptureConfig, NewClip, Paths, Store, UpsertOutcome, load_or_create, relative_time,
    run_listener, set_clipboard_text,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("watch") => watch(),
        Some("search") => search_cmd(&args.collect::<Vec<_>>()),
        Some("copy") => {
            let Some(id) = args.next() else {
                bail!("Usage: cliptrace copy <id>");
            };
            copy_cmd(&id)
        }
        _ => {
            eprintln!("Usage: cliptrace <watch | search [--limit N] <query> | copy <id>>");
            std::process::exit(2);
        }
    }
}

fn open_store() -> Result<Store> {
    let paths = Paths::resolve().context("resolve data dir")?;
    paths.ensure_data_dir().context("create data dir")?;
    Store::open(&paths.db_path).context("open cliptrace.db")
}

fn watch() -> Result<()> {
    let paths = Paths::resolve().context("resolve data dir")?;
    paths.ensure_data_dir().context("create data dir")?;
    let config = load_or_create(&paths.config_path).context("load config.toml")?;
    let store = Store::open(&paths.db_path).context("open cliptrace.db")?;

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
    let (query, limit) = parse_search_args(args)?;
    let store = open_store()?;
    let now_ms = unix_ms();
    let hits = store
        .search_keyword(&query, limit)
        .context("fts5 search")?;
    if hits.is_empty() {
        eprintln!("no matches");
        return Ok(());
    }
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

fn parse_search_args(args: &[String]) -> Result<(String, usize)> {
    let mut limit = 5usize;
    let mut query_parts: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--limit" {
            i += 1;
            let Some(raw) = args.get(i) else {
                bail!("Usage: cliptrace search [--limit N] <query>");
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
        bail!("Usage: cliptrace search [--limit N] <query>");
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
