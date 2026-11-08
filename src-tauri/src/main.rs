#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cliptrace_core::{
    CaptureConfig, Embedder, NewClip, Store, UpsertOutcome, format_local_ms, hybrid_search,
    load_or_create, local_from_unix_ms, parse_time_phrase, relative_time, run_listener,
    set_clipboard_text,
};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
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
}

static SINGLETON: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[derive(Serialize)]
struct HitDto {
    id: i64,
    preview: String,
    source_exe: String,
    abs_time: String,
    rel_time: String,
}

#[derive(Serialize)]
struct SearchPayload {
    hits: Vec<HitDto>,
    note: Option<String>,
}

fn main() {
    if !acquire_singleton() {
        std::process::exit(0);
    }

    tauri::Builder::default()
        .setup(|app| {
            let paused = Arc::new(AtomicBool::new(false));
            spawn_backend(paused.clone());
            app.manage(AppState {
                embedder: Mutex::new(None),
            });
            build_tray(app.handle(), paused)?;
            spawn_hotkey_thread(app.handle().clone());
            warmup_embedder(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![search_clips, copy_clip, hide_popup])
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

fn spawn_backend(paused: Arc<AtomicBool>) {
    std::thread::Builder::new()
        .name("cliptrace-capture".into())
        .spawn(move || {
            if let Err(err) = run_backend(paused) {
                eprintln!("backend: {err}");
            }
        })
        .ok();
}

fn run_backend(paused: Arc<AtomicBool>) -> anyhow::Result<()> {
    let paths = cliptrace_core::Paths::resolve()?;
    paths.ensure_data_dir()?;
    let config = load_or_create(&paths)?;
    let store = Store::open(&paths.db_path)?;
    let ttl = config.secret_ttl_hours;
    let retention = config.retention_days;
    let _ = store.purge(unix_ms(), retention, ttl);
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

    run_listener(
        CaptureConfig {
            blocklist: config.blocklist,
        },
        move |event| {
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
            Err(err) => eprintln!("embed warmup: {err}"),
        }
    });
}

fn build_tray(app: &AppHandle, paused: Arc<AtomicBool>) -> tauri::Result<()> {
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
            }
            "quit" => app.exit(0),
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
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.eval("window.cliptraceOnOpen && window.cliptraceOnOpen()");
    }
}

fn hide_popup_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("popup") {
        let _ = win.hide();
    }
}

#[tauri::command]
fn hide_popup(app: AppHandle) {
    hide_popup_window(&app);
}

#[tauri::command]
fn search_clips(query: String, state: tauri::State<AppState>) -> Result<SearchPayload, String> {
    let paths = cliptrace_core::Paths::resolve().map_err(|e| e.to_string())?;
    let store = Store::open(&paths.db_path).map_err(|e| e.to_string())?;
    let now_ms = unix_ms();
    let mut note = None;
    let parsed = parse_time_phrase(&query, local_from_unix_ms(now_ms));
    let mut vector = None;
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
                        }
                    }
                }
                if let Some(embedder) = slot.as_mut() {
                    match embedder.embed_query(&parsed.text, true) {
                        Ok(v) => vector = Some(v),
                        Err(err) => note = Some(format!("keyword-only: {err}")),
                    }
                }
            }
            Err(_) => note = Some("keyword-only: embedder lock".into()),
        }
    }
    let outcome = hybrid_search(&store, &query, now_ms, 8, vector.as_deref())
        .map_err(|e| e.to_string())?;
    if outcome.keyword_only && note.is_none() && vector.is_none() {
        if let Ok(config) = load_or_create(&paths) {
            let dll = config.resolved_onnxruntime_path(&paths);
            if !dll.is_file() {
                note = Some("keyword-only: onnxruntime.dll missing".into());
            }
        }
    }
    let hits = outcome
        .hits
        .into_iter()
        .map(|h| HitDto {
            id: h.id,
            preview: h.preview,
            source_exe: h.source_exe,
            abs_time: format_local_ms(h.last_seen_at),
            rel_time: relative_time(h.last_seen_at, now_ms),
        })
        .collect();
    Ok(SearchPayload { hits, note })
}

#[tauri::command]
fn copy_clip(id: i64, app: AppHandle) -> Result<(), String> {
    let paths = cliptrace_core::Paths::resolve().map_err(|e| e.to_string())?;
    let store = Store::open(&paths.db_path).map_err(|e| e.to_string())?;
    let content = store.clip_content(id).map_err(|e| e.to_string())?;
    set_clipboard_text(&content).map_err(|e| e.to_string())?;
    hide_popup_window(&app);
    Ok(())
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
