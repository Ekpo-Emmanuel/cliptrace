use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
#[cfg(feature = "embeddings")]
use cliptrace_core::{
    EmbedWorker, Embedder, backfill_batch_size, backfill_sleep, run_embed_test, warn_once,
};
use cliptrace_core::{
    BrowseFilter, CaptureConfig, NewClip, Paths, RRF_K, SearchHit, Store, UpsertOutcome,
    format_local_ms, load_or_create, local_from_unix_ms, parse_hash_tags, parse_time_phrase,
    rank_index, reciprocal_rank_fusion, redact_secrets, relative_time, run_listener,
    set_clipboard_text,
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
        Some("purge") => purge_cmd(),
        Some("rescan") => rescan_cmd(),
        Some("scrub") => scrub_cmd(),
        Some("autostart") => autostart_cmd(true),
        Some("no-autostart") => autostart_cmd(false),
        Some("pin") => pin_cmd(true, args.next().as_deref()),
        Some("unpin") => pin_cmd(false, args.next().as_deref()),
        Some("tag") => tag_cmd(true, args.next().as_deref(), args.next().as_deref()),
        Some("untag") => tag_cmd(false, args.next().as_deref(), args.next().as_deref()),
        Some("tags") => tags_cmd(),
        Some("list") => list_cmd(&args.collect::<Vec<_>>()),
        Some("mark-secret") => mark_secret_cmd(args.next().as_deref()),
        Some("delete") => delete_cmd(args.next().as_deref()),
        _ => {
            eprintln!(
                "Usage: cliptrace <watch | search [--limit N] [--keyword-only] [--vector-only] [--explain] [--pinned] [--tag X] <query> | vsearch [--limit N] <query> | backfill | embed-test <text> | copy <id> | purge | rescan | scrub | pin <id> | unpin <id> | tag <id> \"<tag>\" | untag <id> \"<tag>\" | tags | list [--pinned] [--tag X] [--offset N] [--limit N] | mark-secret <id> | delete <id> | autostart | no-autostart>"
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
        let config = load_or_create(&paths).context("load config.toml")?;
        let dll = config.resolved_onnxruntime_path(&paths);
        eprintln!(
            "embed-test: cache={} dll={} text_len={}",
            paths.models_dir.display(),
            dll.display(),
            text.len()
        );
        match run_embed_test(&paths.models_dir, &dll, text) {
            Ok(r) => {
                println!(
                    "{}\tdims={}\tload_ms={}\tembed1_ms={}\tembed2_ms={}\tfirst3={:.6} {:.6} {:.6}",
                    r.model,
                    r.dims,
                    r.load_ms,
                    r.embed1_ms,
                    r.embed2_ms,
                    r.first3[0],
                    r.first3[1],
                    r.first3[2]
                );
            }
            Err(err) => warn_once(&err.to_string()),
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
    let config = load_or_create(&paths).context("load config.toml")?;
    let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
    let secret_ttl_hours = config.secret_ttl_hours;
    let retention_days = config.retention_days;
    match store.purge(unix_ms(), retention_days, secret_ttl_hours) {
        Ok(n) => eprintln!("purge deleted={n}"),
        Err(err) => eprintln!("purge error: {err}"),
    }
    {
        let db_path = paths.db_path.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
                if let Ok(store) = Store::open(&db_path) {
                    match store.purge(unix_ms(), retention_days, secret_ttl_hours) {
                        Ok(n) => eprintln!("purge deleted={n}"),
                        Err(err) => eprintln!("purge error: {err}"),
                    }
                }
            }
        });
    }
    #[cfg(feature = "embeddings")]
    let worker = {
        let dll = config.resolved_onnxruntime_path(&paths);
        if !dll.is_file() {
            warn_once(&format!("onnxruntime.dll missing at {}", dll.display()));
        }
        EmbedWorker::spawn(
            paths.models_dir.clone(),
            paths.db_path.clone(),
            dll,
        )
    };

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
                secret_ttl_hours,
            );
            match outcome {
                Ok(UpsertOutcome::Inserted { id }) => {
                    let preview = store
                        .display_preview(id)
                        .unwrap_or_else(|_| "[clip]".to_string());
                    println!(
                        "stored\t{}\t{}\t{}\t{}\t{}",
                        id,
                        event.timestamp,
                        event.source_exe,
                        redact_secrets(&event.window_title),
                        preview
                    );
                    #[cfg(feature = "embeddings")]
                    if let Ok(search_text) = store.clip_search_text(id) {
                        worker.submit(id, search_text);
                    }
                }
                Ok(UpsertOutcome::Duplicate { id, copy_count }) => {
                    let preview = store
                        .display_preview(id)
                        .unwrap_or_else(|_| "[clip]".to_string());
                    println!(
                        "duplicate\t{}\t{}\t{}\t{}\t{}\tcount={}",
                        id,
                        event.timestamp,
                        event.source_exe,
                        redact_secrets(&event.window_title),
                        preview,
                        copy_count
                    );
                    #[cfg(feature = "embeddings")]
                    if store.has_embedding(id).ok() == Some(false) {
                        if let Ok(search_text) = store.clip_search_text(id) {
                            worker.submit(id, search_text);
                        }
                    }
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
    let opts = parse_search_args(args)?;
    let now_ms = unix_ms();
    let (stripped, hash_tags) = parse_hash_tags(&opts.query);
    let parsed = parse_time_phrase(&stripped, local_from_unix_ms(now_ms));
    let mut filter = BrowseFilter {
        pinned_only: opts.pinned,
        tags: opts.tags,
    };
    filter.tags.extend(hash_tags);
    let store = open_store()?;
    let pool = opts.limit.max(50);

    let keyword_hits = store
        .search_keyword_scoped(&parsed.text, pool, parsed.range, &filter)
        .context("fts5 search")?;
    let keyword_ids: Vec<i64> = keyword_hits.iter().map(|h| h.id).collect();

    let mut vector_hits = Vec::new();
    let mut vector_ids: Vec<i64> = Vec::new();
    let mut used_vector = false;
    let mut prefix_plain_ids: Option<Vec<i64>> = None;

    if !opts.keyword_only && cliptrace_core::fts_or_query(&parsed.text).is_some() {
        match embed_search_vectors(&parsed.text, opts.explain) {
            Ok(None) => {}
            Ok(Some(emb)) => {
                used_vector = true;
                vector_hits = store
                    .search_vector_scoped(&emb.prefixed, pool, parsed.range, &filter)
                    .context("vector search")?;
                vector_ids = vector_hits.iter().map(|h| h.id).collect();
                if let Some(plain) = emb.plain {
                    prefix_plain_ids = store
                        .search_vector_scoped(&plain, 5, parsed.range, &filter)
                        .ok()
                        .map(|h| h.into_iter().map(|x| x.id).collect());
                }
            }
            Err(err) => {
                #[cfg(feature = "embeddings")]
                warn_once(&err);
                #[cfg(not(feature = "embeddings"))]
                eprintln!("warning: {err}");
            }
        }
    }

    if opts.vector_only && !used_vector {
        return print_hits(Vec::new(), now_ms, None);
    }

    let hits = if opts.keyword_only || !used_vector {
        keyword_hits.iter().take(opts.limit).cloned().collect()
    } else if opts.vector_only {
        vector_hits.iter().take(opts.limit).cloned().collect()
    } else {
        let merged = reciprocal_rank_fusion(&[keyword_ids.clone(), vector_ids.clone()], RRF_K, opts.limit);
        let mut by_id = std::collections::HashMap::new();
        for hit in keyword_hits.iter().chain(vector_hits.iter()) {
            by_id.entry(hit.id).or_insert_with(|| hit.clone());
        }
        merged
            .into_iter()
            .filter_map(|id| by_id.remove(&id))
            .collect()
    };

    let prefix_note = prefix_plain_ids.map(|plain_ids| {
        let pref: Vec<i64> = vector_ids.iter().copied().take(5).collect();
        if pref == plain_ids {
            format!("BGE query prefix did not change top-5 ids={pref:?}")
        } else {
            format!("BGE query prefix changed ranking prefixed={pref:?} plain={plain_ids:?}")
        }
    });

    let explain = if opts.explain {
        Some(Explain {
            parsed_text: parsed.text.clone(),
            range: parsed.range,
            keyword_ids,
            vector_ids,
            prefix_note,
        })
    } else {
        None
    };
    print_hits(hits, now_ms, explain.as_ref())
}

struct SearchOpts {
    query: String,
    limit: usize,
    keyword_only: bool,
    vector_only: bool,
    explain: bool,
    pinned: bool,
    tags: Vec<String>,
}

struct Explain {
    parsed_text: String,
    range: Option<(i64, i64)>,
    keyword_ids: Vec<i64>,
    vector_ids: Vec<i64>,
    prefix_note: Option<String>,
}

struct EmbeddedQuery {
    prefixed: Vec<f32>,
    plain: Option<Vec<f32>>,
}

fn embed_search_vectors(text: &str, also_plain: bool) -> Result<Option<EmbeddedQuery>, String> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    #[cfg(feature = "embeddings")]
    {
        let paths = open_paths().map_err(|e| e.to_string())?;
        let config = load_or_create(&paths).map_err(|e| e.to_string())?;
        let dll = config.resolved_onnxruntime_path(&paths);
        let mut embedder = cliptrace_core::Embedder::load(&paths.models_dir, &dll)
            .map_err(|e| e.to_string())?;
        let prefixed = embedder.embed_query(text, true).map_err(|e| e.to_string())?;
        let plain = if also_plain {
            Some(embedder.embed_query(text, false).map_err(|e| e.to_string())?)
        } else {
            None
        };
        Ok(Some(EmbeddedQuery { prefixed, plain }))
    }
    #[cfg(not(feature = "embeddings"))]
    {
        let _ = also_plain;
        Err("embeddings feature unavailable".into())
    }
}

fn vsearch_cmd(args: &[String]) -> Result<()> {
    #[cfg(feature = "embeddings")]
    {
        let opts = parse_search_args(args)?;
        let paths = open_paths()?;
        let config = load_or_create(&paths).context("load config.toml")?;
        let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
        let dll = config.resolved_onnxruntime_path(&paths);
        let mut embedder = match Embedder::load(&paths.models_dir, &dll) {
            Ok(e) => e,
            Err(err) => {
                warn_once(&err.to_string());
                return Ok(());
            }
        };
        let vector = match embedder.embed_query(&opts.query, true) {
            Ok(v) => v,
            Err(err) => {
                warn_once(&err.to_string());
                return Ok(());
            }
        };
        print_hits(
            store
                .search_vector(&vector, opts.limit, None)
                .context("vector search")?,
            unix_ms(),
            None,
        )
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
        let config = load_or_create(&paths).context("load config.toml")?;
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
        let dll = config.resolved_onnxruntime_path(&paths);
        let mut embedder = match Embedder::load(&paths.models_dir, &dll) {
            Ok(e) => e,
            Err(err) => {
                warn_once(&err.to_string());
                eprintln!("backfill remaining={total}");
                return Ok(());
            }
        };
        let mut done = 0usize;
        let batch = backfill_batch_size();
        for (i, (id, search_text)) in missing.into_iter().enumerate() {
            let text_len = search_text.len();
            match embedder.embed_one(&search_text) {
                Ok(vector) => {
                    if let Err(err) = store.upsert_embedding(id, &vector) {
                        eprintln!("backfill store fail id={id} text_len={text_len}: {err}");
                    } else {
                        done += 1;
                    }
                }
                Err(err) => {
                    eprintln!("backfill fail id={id} text_len={text_len}: {err}");
                }
            }
            eprintln!("backfill {}/{total} id={id} dims=384 text_len={text_len}", i + 1);
            if (i + 1) % batch == 0 {
                std::thread::sleep(backfill_sleep());
            }
        }
        let remaining = store.clips_missing_embeddings()?.len();
        eprintln!("backfill done embedded={done} remaining={remaining}");
        Ok(())
    }
    #[cfg(not(feature = "embeddings"))]
    {
        bail!("rebuild with --features embeddings");
    }
}

fn print_hits(hits: Vec<SearchHit>, now_ms: i64, explain: Option<&Explain>) -> Result<()> {
    if let Some(ex) = explain {
        match ex.range {
            Some((start, end)) => eprintln!(
                "explain\trange\t{}\t{}\ttext={}",
                format_local_ms(start),
                format_local_ms(end),
                ex.parsed_text
            ),
            None => eprintln!("explain\trange\tnone\ttext={}", ex.parsed_text),
        }
        if let Some(note) = &ex.prefix_note {
            eprintln!("explain\tprefix\t{note}");
        }
    }
    if hits.is_empty() {
        eprintln!("no matches");
        return Ok(());
    }
    for hit in hits {
        print!(
            "{}\t{}\t{}\t{}\t{}",
            hit.id,
            format_local_ms(hit.last_seen_at),
            relative_time(hit.last_seen_at, now_ms),
            hit.source_exe,
            hit.preview
        );
        if hit.pinned {
            print!("\tpinned");
        }
        if !hit.tags.is_empty() {
            print!("\t{}", hit.tags);
        }
        if let Some(ex) = explain {
            let kr = rank_index(&ex.keyword_ids, hit.id);
            let vr = rank_index(&ex.vector_ids, hit.id);
            print!(
                "\tkw={}\tvec={}",
                kr.map(|r| r.to_string()).unwrap_or_else(|| "-".into()),
                vr.map(|r| r.to_string()).unwrap_or_else(|| "-".into())
            );
        }
        println!();
    }
    Ok(())
}

fn parse_search_args(args: &[String]) -> Result<SearchOpts> {
    let mut limit = 5usize;
    let mut query_parts: Vec<&str> = Vec::new();
    let mut keyword_only = false;
    let mut vector_only = false;
    let mut explain = false;
    let mut pinned = false;
    let mut tags = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--limit" {
            i += 1;
            let Some(raw) = args.get(i) else {
                bail!("Usage: cliptrace search [--limit N] [--keyword-only] [--vector-only] [--explain] [--pinned] [--tag X] <query>");
            };
            limit = parse_limit(raw)?;
            i += 1;
        } else if let Some(raw) = args[i].strip_prefix("--limit=") {
            limit = parse_limit(raw)?;
            i += 1;
        } else if args[i] == "--keyword-only" {
            keyword_only = true;
            i += 1;
        } else if args[i] == "--vector-only" {
            vector_only = true;
            i += 1;
        } else if args[i] == "--explain" {
            explain = true;
            i += 1;
        } else if args[i] == "--pinned" {
            pinned = true;
            i += 1;
        } else if args[i] == "--tag" {
            i += 1;
            let Some(raw) = args.get(i) else {
                bail!("--tag needs a value");
            };
            tags.push(raw.clone());
            i += 1;
        } else if let Some(raw) = args[i].strip_prefix("--tag=") {
            tags.push(raw.to_string());
            i += 1;
        } else {
            query_parts.push(args[i].as_str());
            i += 1;
        }
    }
    if keyword_only && vector_only {
        bail!("use only one of --keyword-only and --vector-only");
    }
    Ok(SearchOpts {
        query: query_parts.join(" "),
        limit,
        keyword_only,
        vector_only,
        explain,
        pinned,
        tags,
    })
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
    let len = content.len();
    set_clipboard_text(&content).context("set clipboard")?;
    eprintln!("copied id={id} ({len} chars)");
    Ok(())
}

fn purge_cmd() -> Result<()> {
    let paths = open_paths()?;
    let config = load_or_create(&paths).context("load config.toml")?;
    let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
    let n = store.purge(unix_ms(), config.retention_days, config.secret_ttl_hours)?;
    eprintln!("purge deleted={n}");
    Ok(())
}

fn scrub_cmd() -> Result<()> {
    let paths = open_paths()?;
    let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
    store.scrub().context("scrub")?;
    eprintln!("scrub ok db={}", paths.db_path.display());
    Ok(())
}

fn rescan_cmd() -> Result<()> {
    let paths = open_paths()?;
    let config = load_or_create(&paths).context("load config.toml")?;
    let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
    let report = store
        .rescan_secrets(unix_ms(), config.secret_ttl_hours)
        .context("rescan")?;
    let _ = report.reembed;
    eprintln!(
        "rescan scanned={} converted={}",
        report.scanned, report.converted
    );
    Ok(())
}

fn autostart_cmd(enabled: bool) -> Result<()> {
    let exe = app_exe_path()?;
    cliptrace_core::set_autostart(enabled, &exe).context("HKCU Run key")?;
    if enabled {
        eprintln!("autostart on exe={}", exe.display());
    } else {
        eprintln!("autostart off");
    }
    Ok(())
}

fn app_exe_path() -> Result<std::path::PathBuf> {
    if let Ok(p) = std::env::var("CLIPTRACE_APP_EXE") {
        let t = p.trim();
        if !t.is_empty() {
            return Ok(std::path::PathBuf::from(t));
        }
    }
    let here = std::env::current_exe().context("current exe")?;
    let sibling = here.with_file_name("cliptrace-app.exe");
    if sibling.is_file() {
        return Ok(sibling);
    }
    let paths = Paths::resolve().context("resolve data dir")?;
    Ok(paths.data_dir.join("cliptrace-app.exe"))
}

fn parse_id(raw: Option<&str>, usage: &str) -> Result<i64> {
    let Some(raw) = raw else {
        bail!("{usage}");
    };
    raw.parse().context("id must be an integer")
}

fn pin_cmd(pin: bool, id_raw: Option<&str>) -> Result<()> {
    let id = parse_id(id_raw, "Usage: cliptrace pin|unpin <id>")?;
    let paths = open_paths()?;
    let config = load_or_create(&paths).context("load config.toml")?;
    let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
    if pin {
        store.pin_clip(id, unix_ms())?;
        eprintln!("pinned id={id}");
    } else {
        store.unpin_clip(id, unix_ms(), config.secret_ttl_hours)?;
        eprintln!("unpinned id={id}");
    }
    Ok(())
}

fn tag_cmd(add: bool, id_raw: Option<&str>, tag_raw: Option<&str>) -> Result<()> {
    let id = parse_id(id_raw, "Usage: cliptrace tag|untag <id> \"<tag>\"")?;
    let Some(tag) = tag_raw else {
        bail!("Usage: cliptrace tag|untag <id> \"<tag>\"");
    };
    let store = open_store()?;
    let tags = if add {
        store.tag_clip(id, tag)?
    } else {
        store.untag_clip(id, tag)?
    };
    eprintln!("id={id} tags={}", tags.join(", "));
    Ok(())
}

fn tags_cmd() -> Result<()> {
    let store = open_store()?;
    let tags = store.list_tags()?;
    if tags.is_empty() {
        eprintln!("no tags");
        return Ok(());
    }
    for t in tags {
        println!("{}\t{}", t.count, t.name);
    }
    Ok(())
}

fn list_cmd(args: &[String]) -> Result<()> {
    let mut offset = 0i64;
    let mut limit = 20i64;
    let mut filter = BrowseFilter::default();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--pinned" {
            filter.pinned_only = true;
            i += 1;
        } else if args[i] == "--tag" {
            i += 1;
            let Some(raw) = args.get(i) else {
                bail!("--tag needs a value");
            };
            filter.tags.push(raw.clone());
            i += 1;
        } else if let Some(raw) = args[i].strip_prefix("--tag=") {
            filter.tags.push(raw.to_string());
            i += 1;
        } else if args[i] == "--offset" {
            i += 1;
            let Some(raw) = args.get(i) else {
                bail!("--offset needs a number");
            };
            offset = raw.parse().context("--offset")?;
            i += 1;
        } else if let Some(raw) = args[i].strip_prefix("--offset=") {
            offset = raw.parse().context("--offset")?;
            i += 1;
        } else if args[i] == "--limit" {
            i += 1;
            let Some(raw) = args.get(i) else {
                bail!("--limit needs a number");
            };
            limit = raw.parse().context("--limit")?;
            i += 1;
        } else if let Some(raw) = args[i].strip_prefix("--limit=") {
            limit = raw.parse().context("--limit")?;
            i += 1;
        } else {
            bail!("unknown list flag {}", args[i]);
        }
    }
    let store = open_store()?;
    print_hits(store.list_recent(offset, limit, &filter)?, unix_ms(), None)
}

fn mark_secret_cmd(id_raw: Option<&str>) -> Result<()> {
    let id = parse_id(id_raw, "Usage: cliptrace mark-secret <id>")?;
    let paths = open_paths()?;
    let config = load_or_create(&paths).context("load config.toml")?;
    let store = Store::open(&paths.db_path).context("open cliptrace.db")?;
    store.mark_secret(id, unix_ms(), config.secret_ttl_hours)?;
    let preview = store.display_preview(id)?;
    eprintln!("marked-secret id={id} {preview}");
    Ok(())
}

fn delete_cmd(id_raw: Option<&str>) -> Result<()> {
    let id = parse_id(id_raw, "Usage: cliptrace delete <id>")?;
    let store = open_store()?;
    store.delete_clip(id)?;
    eprintln!("deleted id={id}");
    Ok(())
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
