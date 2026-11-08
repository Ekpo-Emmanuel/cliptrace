#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cliptrace_core::{
    BrowseFilter, CaptureConfig, ClipCard, Embedder, NewClip, Store, UpsertOutcome, hybrid_search,
    load_or_create, run_listener, set_clipboard_text,
};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HWND};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, TranslateMessage, WM_HOTKEY,
};
use windows::core::PCWSTR;

struct AppState {
    embedder: Mutex<Option<Embedder>>,
    paused: Arc<AtomicBool>,
    keyword_only: Arc<AtomicBool>,
    skip_clipboard: Arc<AtomicBool>,
    recent: Arc<Mutex<Vec<ClipCard>>>,
    pending_deletes: Arc<Mutex<Vec<i64>>>,
    ignore_blur_until: AtomicU64,
}

#[derive(Clone, Serialize)]
struct StatusDto {
    paused: bool,
    keyword_only: bool,
}

static SINGLETON: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[derive(Serialize)]
struct BrowsePayload {
    cards: Vec<ClipCard>,
    pinned: i64,
    total: i64,
    note: Option<String>,
    paused: bool,
}

#[derive(Serialize)]
struct TagDto {
    name: String,
    count: i64,
}

fn main() {
    if !acquire_singleton() {
        std::process::exit(0);
    }

    tauri::Builder::default()
        .setup(|app| {
            let paused = Arc::new(AtomicBool::new(false));
            let keyword_only = Arc::new(AtomicBool::new(false));
            let skip_clipboard = Arc::new(AtomicBool::new(false));
            let recent = Arc::new(Mutex::new(Vec::new()));
            let pending_deletes = Arc::new(Mutex::new(Vec::new()));
            spawn_backend(
                app.handle().clone(),
                paused.clone(),
                skip_clipboard.clone(),
                recent.clone(),
            );
            app.manage(AppState {
                embedder: Mutex::new(None),
                paused: paused.clone(),
                keyword_only: keyword_only.clone(),
                skip_clipboard: skip_clipboard.clone(),
                recent: recent.clone(),
                pending_deletes: pending_deletes.clone(),
                ignore_blur_until: AtomicU64::new(0),
            });
            build_tray(app.handle(), paused, keyword_only)?;
            spawn_hotkey_thread(app.handle().clone());
            warmup_embedder(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            browse_clips,
            list_recent,
            search_clips,
            list_tags,
            set_pinned,
            add_tag,
            remove_tag,
            mark_clip_secret,
            delete_clip,
            stage_delete,
            unstage_delete,
            clip_text,
            copy_clip,
            hide_popup,
            status,
        ])
        .on_window_event(|window, event| {
            if window.label() != "popup" {
                return;
            }
            let tauri::WindowEvent::Focused(false) = event else {
                return;
            };
            if let Some(state) = window.try_state::<AppState>() {
                let until = state.ignore_blur_until.load(Ordering::SeqCst);
                if (unix_ms() as u64) < until {
                    return;
                }
            }
            hide_popup_now(window.app_handle());
        })
        .run(tauri::generate_context!())
        .expect("cliptrace-app");
}

fn acquire_singleton() -> bool {
    let name: Vec<u16> = "Local\\CliptraceSingleInstance\0".encode_utf16().collect();
    // Safety: name is a valid NUL-terminated UTF-16 buffer that lives for the call.
    // The mutex handle is leaked on purpose so it lasts the process lifetime.
    let result = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) };
    match result {
        Ok(handle) => {
            SINGLETON.store(handle.0 as isize, std::sync::atomic::Ordering::SeqCst);
            let err = unsafe { GetLastError() };
            err != ERROR_ALREADY_EXISTS
        }
        Err(_) => {
            let err = unsafe { GetLastError() };
            err != ERROR_ALREADY_EXISTS
        }
    }
}

fn spawn_backend(
    app: AppHandle,
    paused: Arc<AtomicBool>,
    skip_clipboard: Arc<AtomicBool>,
    recent: Arc<Mutex<Vec<ClipCard>>>,
) {
    std::thread::Builder::new()
        .name("cliptrace-capture".into())
        .spawn(move || {
            if let Err(err) = run_backend(app, paused, skip_clipboard, recent) {
                eprintln!("backend: {err}");
            }
        })
        .ok();
}

fn refresh_recent(store: &Store, recent: &Mutex<Vec<ClipCard>>) {
    if let Ok(cards) = store.list_cards(0, 50, &BrowseFilter::default()) {
        if let Ok(mut slot) = recent.lock() {
            *slot = cards.into_iter().map(for_ipc).collect();
        }
    }
}

fn run_backend(
    app: AppHandle,
    paused: Arc<AtomicBool>,
    skip_clipboard: Arc<AtomicBool>,
    recent: Arc<Mutex<Vec<ClipCard>>>,
) -> anyhow::Result<()> {
    let paths = cliptrace_core::Paths::resolve()?;
    paths.ensure_data_dir()?;
    let config = load_or_create(&paths)?;
    let store = Store::open(&paths.db_path)?;
    let ttl = config.secret_ttl_hours;
    let retention = config.retention_days;
    let _ = store.purge(unix_ms(), retention, ttl);
    refresh_recent(&store, &recent);
    let db_purge = paths.db_path.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(3600));
        if let Ok(store) = Store::open(&db_purge) {
            let _ = store.purge(unix_ms(), retention, ttl);
        }
    });

    let dll = config.resolved_onnxruntime_path(&paths);
    let worker = cliptrace_core::EmbedWorker::spawn(
        paths.models_dir.clone(),
        paths.db_path.clone(),
        dll,
    );

    let recent_cache = recent.clone();
    run_listener(
        CaptureConfig {
            blocklist: config.blocklist,
        },
        move |event| {
            if skip_clipboard.swap(false, Ordering::SeqCst) {
                return;
            }
            if paused.load(Ordering::SeqCst) {
                return;
            }
            let now_ms = unix_ms();
            match store.upsert_clip(
                &NewClip {
                    content: event.text,
                    source_exe: event.source_exe.clone(),
                    window_title: event.window_title,
                },
                now_ms,
                ttl,
            ) {
                Ok(UpsertOutcome::Inserted { id }) | Ok(UpsertOutcome::Duplicate { id, .. }) => {
                    refresh_recent(&store, &recent_cache);
                    let _ = app.emit("clips-changed", id);
                    if store.has_embedding(id).ok() == Some(false) {
                        if let Ok(search_text) = store.clip_search_text(id) {
                            worker.submit(id, search_text);
                        }
                    }
                }
                Err(err) => {
                    eprintln!(
                        "store error exe={} text_len skipped: {err}",
                        event.source_exe
                    );
                }
            }
        },
    )?;
    Ok(())
}

fn warmup_embedder(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        let Ok(paths) = cliptrace_core::Paths::resolve() else {
            return;
        };
        let Ok(config) = load_or_create(&paths) else {
            return;
        };
        let dll = config.resolved_onnxruntime_path(&paths);
        match Embedder::load(&paths.models_dir, &dll) {
            Ok(embedder) => {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(mut slot) = state.embedder.lock() {
                        *slot = Some(embedder);
                    }
                }
            }
            Err(err) => {
                eprintln!("embed warmup: {err}");
                if let Some(state) = app.try_state::<AppState>() {
                    state.keyword_only.store(true, Ordering::SeqCst);
                }
            }
        }
    });
}

fn build_tray(
    app: &AppHandle,
    paused: Arc<AtomicBool>,
    keyword_only: Arc<AtomicBool>,
) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause capture", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &pause, &quit])?;
    let pause_item = pause.clone();
    TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Cliptrace")
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => show_popup(app),
            "pause" => {
                let now = !paused.load(Ordering::SeqCst);
                paused.store(now, Ordering::SeqCst);
                let label = if now {
                    "Resume capture"
                } else {
                    "Pause capture"
                };
                let _ = pause_item.set_text(label);
                let _ = app.emit(
                    "status-changed",
                    StatusDto {
                        paused: now,
                        keyword_only: keyword_only.load(Ordering::SeqCst),
                    },
                );
            }
            "quit" => {
                flush_pending(app);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn spawn_hotkey_thread(app: AppHandle) {
    std::thread::Builder::new()
        .name("cliptrace-hotkey".into())
        .spawn(move || {
            let hotkey = load_hotkey_string();
            let Some((modifiers, vk)) = parse_hotkey(&hotkey) else {
                eprintln!("hotkey parse failed: {hotkey}");
                notify_hotkey_failed(&app, &hotkey);
                return;
            };
            // Safety: no HWND; this thread owns the hotkey message loop.
            let ok = unsafe { RegisterHotKey(HWND::default(), 1, modifiers | MOD_NOREPEAT, vk) };
            if ok.is_err() {
                eprintln!("RegisterHotKey failed for {hotkey}");
                notify_hotkey_failed(&app, &hotkey);
                return;
            }
            loop {
                let mut msg = MSG::default();
                let status = unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) };
                if status.0 <= 0 {
                    break;
                }
                if msg.message == WM_HOTKEY {
                    show_popup(&app);
                } else {
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        })
        .ok();
}

fn load_hotkey_string() -> String {
    cliptrace_core::Paths::resolve()
        .ok()
        .and_then(|p| load_or_create(&p).ok())
        .map(|c| c.hotkey)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Ctrl+Alt+V".to_string())
}

fn parse_hotkey(spec: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let mut mods = HOT_KEY_MODIFIERS(0);
    let mut vk = None;
    for part in spec.split('+') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "alt" => mods |= MOD_ALT,
            "shift" => mods |= MOD_SHIFT,
            "win" | "super" | "meta" => mods |= MOD_WIN,
            one if one.len() == 1 => {
                vk = Some(u32::from(one.as_bytes()[0].to_ascii_uppercase()));
            }
            _ => return None,
        }
    }
    Some((mods, vk?))
}

fn notify_hotkey_failed(app: &AppHandle, hotkey: &str) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&format!(
            "Cliptrace: hotkey {hotkey} failed, tray Open still works"
        )));
    }
}

fn show_popup(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("popup") {
        let cached = app
            .try_state::<AppState>()
            .and_then(|state| state.recent.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        let paused = app
            .try_state::<AppState>()
            .map(|state| state.paused.load(Ordering::SeqCst))
            .unwrap_or(false);
        let keyword_only = app
            .try_state::<AppState>()
            .map(|state| state.keyword_only.load(Ordering::SeqCst))
            .unwrap_or(false);
        let payload = serde_json::json!({
            "cards": cached,
            "paused": paused,
            "keyword_only": keyword_only
        });
        let script = format!(
            "window.cliptraceOnOpen && window.cliptraceOnOpen({payload})"
        );
        if let Some(state) = app.try_state::<AppState>() {
            state
                .ignore_blur_until
                .store(unix_ms() as u64 + 400, Ordering::SeqCst);
        }
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.eval(script);
    }
}

fn hide_popup_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("popup") {
        let _ = win.eval("window.cliptraceOnHide && window.cliptraceOnHide()");
        let _ = win.hide();
    }
}

fn for_ipc(mut card: ClipCard) -> ClipCard {
    if card.kind != "secret" {
        card.body.clear();
    }
    card
}

fn flush_pending(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let ids = state
        .pending_deletes
        .lock()
        .map(|mut v| std::mem::take(&mut *v))
        .unwrap_or_default();
    if ids.is_empty() {
        return;
    }
    if let Ok(store) = open_store() {
        for id in ids {
            let _ = store.delete_clip(id);
        }
        refresh_recent(&store, &state.recent);
    }
}

fn hide_popup_now(app: &AppHandle) {
    flush_pending(app);
    hide_popup_window(app);
}

#[tauri::command]
fn hide_popup(app: AppHandle) {
    hide_popup_now(&app);
}

fn open_store() -> Result<Store, String> {
    let paths = cliptrace_core::Paths::resolve().map_err(|e| e.to_string())?;
    Store::open(&paths.db_path).map_err(|e| e.to_string())
}

fn scope_for(tab: &str, tag: &str) -> BrowseFilter {
    let mut filter = BrowseFilter::default();
    if tab == "pinned" {
        filter.pinned_only = true;
    }
    if !tag.trim().is_empty() {
        filter.tags.push(tag.trim().to_string());
    }
    filter
}

#[tauri::command]
fn browse_clips(
    query: String,
    tab: String,
    tag: String,
    offset: i64,
    state: tauri::State<AppState>,
) -> Result<BrowsePayload, String> {
    let paths = cliptrace_core::Paths::resolve().map_err(|e| e.to_string())?;
    let store = Store::open(&paths.db_path).map_err(|e| e.to_string())?;
    let filter = scope_for(&tab, &tag);
    let pinned = store.pinned_count().map_err(|e| e.to_string())?;
    let total = store.clip_count().map_err(|e| e.to_string())?;
    let paused = state.paused.load(Ordering::SeqCst);
    let trimmed = query.trim();
    if trimmed.is_empty() {
        let cards: Vec<ClipCard> = store
            .list_cards(offset.max(0), 50, &filter)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(for_ipc)
            .collect();
        if offset <= 0 && tab == "recent" && tag.trim().is_empty() {
            if let Ok(mut slot) = state.recent.lock() {
                *slot = cards.clone();
            }
        }
        return Ok(BrowsePayload {
            cards,
            pinned,
            total,
            note: None,
            paused,
        });
    }
    let now_ms = unix_ms();
    let mut note = None;
    let mut vector = None;
    let (stripped, _) = cliptrace_core::parse_hash_tags(trimmed);
    let parsed = cliptrace_core::parse_time_phrase(
        &stripped,
        cliptrace_core::local_from_unix_ms(now_ms),
    );
    if cliptrace_core::fts_or_query(&parsed.text).is_some() {
        match state.embedder.lock() {
            Ok(mut slot) => {
                if slot.is_none() {
                    let config = load_or_create(&paths).map_err(|e| e.to_string())?;
                    let dll = config.resolved_onnxruntime_path(&paths);
                    match Embedder::load(&paths.models_dir, &dll) {
                        Ok(e) => *slot = Some(e),
                        Err(err) => {
                            note = Some(format!("keyword-only: {err}"));
                            state.keyword_only.store(true, Ordering::SeqCst);
                        }
                    }
                }
                if let Some(embedder) = slot.as_mut() {
                    match embedder.embed_query(&parsed.text, true) {
                        Ok(v) => {
                            vector = Some(v);
                            state.keyword_only.store(false, Ordering::SeqCst);
                        }
                        Err(err) => {
                            note = Some(format!("keyword-only: {err}"));
                            state.keyword_only.store(true, Ordering::SeqCst);
                        }
                    }
                }
            }
            Err(_) => {
                note = Some("keyword-only: embedder lock".into());
                state.keyword_only.store(true, Ordering::SeqCst);
            }
        }
    }
    let outcome = hybrid_search(&store, trimmed, now_ms, 50, vector.as_deref(), &filter)
        .map_err(|e| e.to_string())?;
    if outcome.keyword_only && note.is_none() && vector.is_none() {
        if let Ok(config) = load_or_create(&paths) {
            let dll = config.resolved_onnxruntime_path(&paths);
            if !dll.is_file() {
                note = Some("keyword-only: onnxruntime.dll missing".into());
                state.keyword_only.store(true, Ordering::SeqCst);
            }
        }
    }
    let ids: Vec<i64> = outcome.hits.iter().map(|h| h.id).collect();
    let cards = store
        .cards_for_ids(&ids)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(for_ipc)
        .collect();
    Ok(BrowsePayload {
        cards,
        pinned,
        total,
        note,
        paused,
    })
}

#[tauri::command]
fn list_recent(
    offset: i64,
    limit: i64,
    tab: String,
    tag: String,
    state: tauri::State<AppState>,
) -> Result<BrowsePayload, String> {
    browse_clips(String::new(), tab, tag, offset, state).map(|mut p| {
        if limit > 0 {
            p.cards.truncate(limit as usize);
        }
        p
    })
}

#[tauri::command]
fn search_clips(
    query: String,
    tab: String,
    tag: String,
    state: tauri::State<AppState>,
) -> Result<BrowsePayload, String> {
    browse_clips(query, tab, tag, 0, state)
}

#[tauri::command]
fn list_tags() -> Result<Vec<TagDto>, String> {
    let store = open_store()?;
    let tags = store.list_tags().map_err(|e| e.to_string())?;
    Ok(tags
        .into_iter()
        .map(|t| TagDto {
            name: t.name,
            count: t.count,
        })
        .collect())
}

#[tauri::command]
fn set_pinned(id: i64, pinned: bool) -> Result<ClipCard, String> {
    let paths = cliptrace_core::Paths::resolve().map_err(|e| e.to_string())?;
    let config = load_or_create(&paths).map_err(|e| e.to_string())?;
    let store = Store::open(&paths.db_path).map_err(|e| e.to_string())?;
    if pinned {
        store.pin_clip(id, unix_ms()).map_err(|e| e.to_string())?;
    } else {
        store
            .unpin_clip(id, unix_ms(), config.secret_ttl_hours)
            .map_err(|e| e.to_string())?;
    }
    one_card(&store, id)
}

#[tauri::command]
fn add_tag(id: i64, tag: String) -> Result<ClipCard, String> {
    let store = open_store()?;
    store.tag_clip(id, &tag).map_err(|e| e.to_string())?;
    one_card(&store, id)
}

#[tauri::command]
fn remove_tag(id: i64, tag: String) -> Result<ClipCard, String> {
    let store = open_store()?;
    store.untag_clip(id, &tag).map_err(|e| e.to_string())?;
    one_card(&store, id)
}

#[tauri::command]
fn mark_clip_secret(id: i64) -> Result<ClipCard, String> {
    let paths = cliptrace_core::Paths::resolve().map_err(|e| e.to_string())?;
    let config = load_or_create(&paths).map_err(|e| e.to_string())?;
    let store = Store::open(&paths.db_path).map_err(|e| e.to_string())?;
    store
        .mark_secret(id, unix_ms(), config.secret_ttl_hours)
        .map_err(|e| e.to_string())?;
    one_card(&store, id)
}

fn one_card(store: &Store, id: i64) -> Result<ClipCard, String> {
    let mut cards = store.cards_for_ids(&[id]).map_err(|e| e.to_string())?;
    let card = cards.pop().ok_or_else(|| format!("clip {id} not found"))?;
    Ok(for_ipc(card))
}

#[tauri::command]
fn clip_text(id: i64) -> Result<String, String> {
    let store = open_store()?;
    store.clip_plain_text(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn status(state: tauri::State<AppState>) -> StatusDto {
    StatusDto {
        paused: state.paused.load(Ordering::SeqCst),
        keyword_only: state.keyword_only.load(Ordering::SeqCst),
    }
}

#[tauri::command]
fn copy_clip(id: i64, state: tauri::State<AppState>) -> Result<(), String> {
    let store = open_store()?;
    let content = store.clip_content(id).map_err(|e| e.to_string())?;
    state.skip_clipboard.store(true, Ordering::SeqCst);
    if let Err(err) = set_clipboard_text(&content) {
        state.skip_clipboard.store(false, Ordering::SeqCst);
        return Err(err.to_string());
    }
    Ok(())
}

#[tauri::command]
fn delete_clip(id: i64, state: tauri::State<AppState>) -> Result<(), String> {
    if let Ok(mut pending) = state.pending_deletes.lock() {
        pending.retain(|x| *x != id);
    }
    let store = open_store()?;
    store.delete_clip(id).map_err(|e| e.to_string())?;
    refresh_recent(&store, &state.recent);
    Ok(())
}

#[tauri::command]
fn stage_delete(id: i64, state: tauri::State<AppState>) -> Result<(), String> {
    let mut pending = state
        .pending_deletes
        .lock()
        .map_err(|_| "pending deletes lock".to_string())?;
    if !pending.contains(&id) {
        pending.push(id);
    }
    Ok(())
}

#[tauri::command]
fn unstage_delete(id: i64, state: tauri::State<AppState>) -> Result<(), String> {
    let mut pending = state
        .pending_deletes
        .lock()
        .map_err(|_| "pending deletes lock".to_string())?;
    pending.retain(|x| *x != id);
    Ok(())
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
