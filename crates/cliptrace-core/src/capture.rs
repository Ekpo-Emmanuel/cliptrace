use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use thiserror::Error;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND, LPARAM, LRESULT, MAX_PATH, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardOwner,
    IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
    RemoveClipboardFormatListener, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::SystemInformation::GetSystemTime;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow,
    GetMessageW, GetWindowTextW, GetWindowThreadProcessId, GWLP_USERDATA, HWND_MESSAGE, MSG,
    PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, TranslateMessage, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_CLIPBOARDUPDATE, WM_DESTROY, WNDCLASSEXW,
};
use windows::core::{PCWSTR, w};

const CF_UNICODETEXT: u32 = 13;

const MAX_CLIP_BYTES: usize = 1024 * 1024;
const OPEN_RETRIES: u32 = 8;
const OPEN_BACKOFF: Duration = Duration::from_millis(15);
const EXCLUDE_MONITOR: PCWSTR = w!("ExcludeClipboardContentFromMonitorProcessing");
const CAN_INCLUDE_HISTORY: PCWSTR = w!("CanIncludeInClipboardHistory");
const CLASS_NAME: PCWSTR = w!("CliptraceClipboardListener");

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("failed to register listener window class")]
    RegisterClass,
    #[error("failed to create message-only listener window")]
    CreateWindow,
    #[error("AddClipboardFormatListener failed")]
    AddListener,
    #[error("failed to open clipboard")]
    OpenClipboard,
    #[error("failed to write clipboard text")]
    SetClipboard,
    #[error(transparent)]
    Windows(#[from] windows::core::Error),
}

#[derive(Clone)]
pub struct ClipEvent {
    pub timestamp: String,
    pub source_exe: String,
    pub window_title: String,
    pub text: String,
}

impl std::fmt::Debug for ClipEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipEvent")
            .field("timestamp", &self.timestamp)
            .field("source_exe", &self.source_exe)
            .field("window_title", &self.window_title)
            .field("text_len", &self.text.len())
            .finish()
    }
}

#[derive(Clone, Debug, Default)]
pub struct CaptureConfig {
    pub blocklist: Vec<String>,
}

struct ListenerState {
    tx: mpsc::Sender<ClipEvent>,
    blocklist: Vec<String>,
}

pub fn run_listener(
    config: CaptureConfig,
    mut on_clip: impl FnMut(ClipEvent),
) -> Result<(), CaptureError> {
    let (tx, rx) = mpsc::channel();
    let hwnd = create_message_window(ListenerState {
        tx,
        blocklist: config
            .blocklist
            .into_iter()
            .map(|s| s.to_ascii_lowercase())
            .collect(),
    })?;

    // Safety: hwnd is a window we created; the listener is removed before destroy.
    unsafe {
        AddClipboardFormatListener(hwnd).map_err(|_| CaptureError::AddListener)?;
    }

    let mut msg = MSG::default();
    loop {
        // Safety: standard UI thread message pump for our hidden window.
        let got = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if !got.as_bool() {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        while let Ok(event) = rx.try_recv() {
            on_clip(event);
        }
    }

    unsafe {
        let _ = RemoveClipboardFormatListener(hwnd);
    }
    Ok(())
}

fn create_message_window(state: ListenerState) -> Result<HWND, CaptureError> {
    let hinstance = unsafe { GetModuleHandleW(None)? };
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wnd_proc),
        hInstance: hinstance.into(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };

    let atom = unsafe { RegisterClassExW(&class) };
    if atom == 0 {
        return Err(CaptureError::RegisterClass);
    }

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS_NAME,
            w!("Cliptrace"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            hinstance,
            None,
        )
    }
    .map_err(|_| CaptureError::CreateWindow)?;
    if hwnd.is_invalid() {
        return Err(CaptureError::CreateWindow);
    }

    let boxed = Box::into_raw(Box::new(state));
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, boxed as isize);
    }
    Ok(hwnd)
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLIPBOARDUPDATE => {
            let state = unsafe { window_state(hwnd) };
            if let Some(state) = state {
                if let Some(event) = capture_once(&state.blocklist) {
                    let _ = state.tx.send(event);
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) } as *mut ListenerState;
            if !ptr.is_null() {
                // Safety: ptr came from Box::into_raw in create_message_window.
                drop(unsafe { Box::from_raw(ptr) });
            }
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn window_state(hwnd: HWND) -> Option<&'static ListenerState> {
    let ptr = unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_USERDATA) }
        as *const ListenerState;
    if ptr.is_null() {
        None
    } else {
        // Safety: userdata is a leaked Box<ListenerState> until WM_DESTROY.
        Some(unsafe { &*ptr })
    }
}

fn capture_once(blocklist: &[String]) -> Option<ClipEvent> {
    let source_exe = source_exe_name().unwrap_or_default();
    let exe_lower = source_exe.to_ascii_lowercase();
    if !exe_lower.is_empty() && blocklist.iter().any(|b| b == &exe_lower) {
        return None;
    }

    if !open_clipboard_with_retry() {
        return None;
    }

    let event = (|| {
        if clipboard_excluded() {
            return None;
        }
        let text = read_unicode_text()?;
        if text.trim().is_empty() || text.len() > MAX_CLIP_BYTES {
            return None;
        }
        Some(ClipEvent {
            timestamp: utc_timestamp(),
            source_exe,
            window_title: foreground_title(),
            text,
        })
    })();

    unsafe {
        let _ = CloseClipboard();
    }
    event
}

fn open_clipboard_with_retry() -> bool {
    for attempt in 0..OPEN_RETRIES {
        let ok = unsafe { OpenClipboard(None) };
        if ok.is_ok() {
            return true;
        }
        if attempt + 1 < OPEN_RETRIES {
            thread::sleep(OPEN_BACKOFF);
        }
    }
    false
}

fn clipboard_excluded() -> bool {
    let exclude = unsafe { RegisterClipboardFormatW(EXCLUDE_MONITOR) };
    if exclude != 0 && unsafe { IsClipboardFormatAvailable(exclude).is_ok() } {
        return true;
    }

    let can_include = unsafe { RegisterClipboardFormatW(CAN_INCLUDE_HISTORY) };
    if can_include != 0 && unsafe { IsClipboardFormatAvailable(can_include).is_ok() } {
        if read_clipboard_u32(can_include) == Some(0) {
            return true;
        }
    }
    false
}

fn read_clipboard_u32(format: u32) -> Option<u32> {
    let handle = unsafe { GetClipboardData(format).ok()? };
    if handle.is_invalid() {
        return None;
    }
    let hglobal = windows::Win32::Foundation::HGLOBAL(handle.0);
    let ptr = unsafe { GlobalLock(hglobal) };
    if ptr.is_null() {
        return None;
    }
    // Safety: clipboard data for this format is a DWORD-sized value while locked.
    let value = unsafe { std::ptr::read_unaligned(ptr as *const u32) };
    unsafe {
        let _ = GlobalUnlock(hglobal);
    }
    Some(value)
}

fn read_unicode_text() -> Option<String> {
    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() } {
        return None;
    }
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT).ok()? };
    if handle.is_invalid() {
        return None;
    }
    let hglobal = windows::Win32::Foundation::HGLOBAL(handle.0);
    let size = unsafe { GlobalSize(hglobal) };
    if size == 0 {
        return None;
    }
    let ptr = unsafe { GlobalLock(hglobal) };
    if ptr.is_null() {
        return None;
    }
    let utf16_len = size as usize / 2;
    // Safety: GlobalLock returned utf-16 clipboard bytes of `size`.
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u16, utf16_len) };
    let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    let text = String::from_utf16_lossy(&slice[..end]);
    unsafe {
        let _ = GlobalUnlock(hglobal);
    }
    Some(text)
}

fn source_exe_name() -> Option<String> {
    let owner = unsafe { GetClipboardOwner() }.ok();
    let hwnd = match owner {
        Some(h) if !h.is_invalid() => h,
        _ => unsafe { GetForegroundWindow() },
    };
    if hwnd.is_invalid() {
        return None;
    }
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        let fg = unsafe { GetForegroundWindow() };
        if fg.is_invalid() {
            return None;
        }
        unsafe { GetWindowThreadProcessId(fg, Some(&mut pid)) };
    }
    if pid == 0 {
        return None;
    }
    exe_name_from_pid(pid)
}

fn exe_name_from_pid(pid: u32) -> Option<String> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let name = query_exe_name(process);
    unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(process);
    }
    name
}

fn query_exe_name(process: HANDLE) -> Option<String> {
    let mut buf = [0u16; MAX_PATH as usize];
    let mut size = buf.len() as u32;
    unsafe { QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut size) }
        .ok()?;
    let path = String::from_utf16_lossy(&buf[..size as usize]);
    Path::new(&path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
}

fn foreground_title() -> String {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return String::new();
    }
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}

pub fn set_clipboard_text(text: &str) -> Result<(), CaptureError> {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let bytes = wide.len() * std::mem::size_of::<u16>();

    if !open_clipboard_with_retry() {
        return Err(CaptureError::OpenClipboard);
    }

    let result = (|| {
        unsafe { EmptyClipboard()? };
        let hglobal = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes)? };
        let ptr = unsafe { GlobalLock(hglobal) };
        if ptr.is_null() {
            unsafe {
                let _ = GlobalFree(hglobal);
            }
            return Err(CaptureError::SetClipboard);
        }
        // Safety: hglobal is GMEM_MOVEABLE of `bytes`; we copy utf-16 including NUL.
        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr().cast::<u8>(), ptr.cast::<u8>(), bytes);
            let _ = GlobalUnlock(hglobal);
        }
        let handle = windows::Win32::Foundation::HANDLE(hglobal.0);
        match unsafe { SetClipboardData(CF_UNICODETEXT, handle) } {
            Ok(_) => Ok(()),
            Err(_) => {
                unsafe {
                    let _ = GlobalFree(hglobal);
                }
                Err(CaptureError::SetClipboard)
            }
        }
    })();

    unsafe {
        let _ = CloseClipboard();
    }
    result
}

fn utc_timestamp() -> String {
    let st = unsafe { GetSystemTime() };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
    )
}
