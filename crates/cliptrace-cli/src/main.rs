use anyhow::Result;
use cliptrace_core::{CaptureConfig, run_listener};

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
    eprintln!("cliptrace watch: listening for clipboard copies (Ctrl+C to stop)");
    run_listener(CaptureConfig::default(), |event| {
        let preview: String = event.text.chars().take(60).collect();
        println!(
            "{}\t{}\t{}\t{}",
            event.timestamp, event.source_exe, event.window_title, preview
        );
    })?;
    Ok(())
}
