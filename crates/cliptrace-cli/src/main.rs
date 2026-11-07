use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use cliptrace_core::{
    CaptureConfig, NewClip, Paths, Store, UpsertOutcome, load_or_create, run_listener,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("watch") => watch(),
        _ => {
            eprintln!("Usage: cliptrace watch");
            std::process::exit(2);
        }
    }
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

fn preview(text: &str) -> String {
    text.chars().take(60).collect()
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
