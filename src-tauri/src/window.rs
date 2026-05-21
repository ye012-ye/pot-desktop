use std::fs;

use crate::config::get;
use crate::config::set;
use crate::StringWrapper;
use crate::APP;
use dirs::cache_dir;
use log::{info, warn};
use tauri::Manager;
use tauri::Monitor;
use tauri::Window;
use tauri::WindowBuilder;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use window_shadows::set_shadow;

// Get daemon window instance
fn get_daemon_window() -> Window {
    let app_handle = APP.get().unwrap();
    match app_handle.get_window("daemon") {
        Some(v) => v,
        None => {
            warn!("Daemon window not found, create new daemon window!");
            WindowBuilder::new(
                app_handle,
                "daemon",
                tauri::WindowUrl::App("daemon.html".into()),
            )
            .title("Daemon")
            .additional_browser_args("--disable-web-security")
            .visible(false)
            .build()
            .unwrap()
        }
    }
}

// Get monitor where the mouse is currently located
fn get_current_monitor(x: i32, y: i32) -> Monitor {
    info!("Mouse position: {}, {}", x, y);
    let daemon_window = get_daemon_window();
    let monitors = daemon_window.available_monitors().unwrap();

    for m in monitors {
        let size = m.size();
        let position = m.position();

        if x >= position.x
            && x <= (position.x + size.width as i32)
            && y >= position.y
            && y <= (position.y + size.height as i32)
        {
            info!("Current Monitor: {:?}", m);
            return m;
        }
    }
    warn!("Current Monitor not found, using primary monitor");
    daemon_window.primary_monitor().unwrap().unwrap()
}

// Creating a window on the mouse monitor
fn build_window(label: &str, title: &str) -> (Window, bool) {
    use mouse_position::mouse_position::{Mouse, Position};

    let mouse_position = match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => Position { x, y },
        Mouse::Error => {
            warn!("Mouse position not found, using (0, 0) as default");
            Position { x: 0, y: 0 }
        }
    };
    let current_monitor = get_current_monitor(mouse_position.x, mouse_position.y);
    let position = current_monitor.position();

    let app_handle = APP.get().unwrap();
    match app_handle.get_window(label) {
        Some(v) => {
            info!("Window existence: {}", label);
            v.set_focus().unwrap();
            (v, true)
        }
        None => {
            info!("Window not existence, Creating new window: {}", label);
            let mut builder = tauri::WindowBuilder::new(
                app_handle,
                label,
                tauri::WindowUrl::App("index.html".into()),
            )
            .position(position.x.into(), position.y.into())
            .additional_browser_args("--disable-web-security")
            .focused(true)
            .title(title)
            .visible(false);

            #[cfg(target_os = "macos")]
            {
                builder = builder
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .hidden_title(true);
            }
            #[cfg(not(target_os = "macos"))]
            {
                builder = builder.transparent(true).decorations(false);
            }
            let window = builder.build().unwrap();

            if label != "screenshot" {
                #[cfg(not(target_os = "linux"))]
                set_shadow(&window, true).unwrap_or_default();
            }
            let _ = window.current_monitor();
            (window, false)
        }
    }
}

pub fn config_window() {
    let (window, _exists) = build_window("config", "Config");
    window
        .set_min_size(Some(tauri::LogicalSize::new(800, 400)))
        .unwrap();
    window.set_size(tauri::LogicalSize::new(800, 600)).unwrap();
    window.center().unwrap();
}

fn translate_window() -> Window {
    use mouse_position::mouse_position::{Mouse, Position};
    // Mouse physical position
    let mut mouse_position = match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => Position { x, y },
        Mouse::Error => {
            warn!("Mouse position not found, using (0, 0) as default");
            Position { x: 0, y: 0 }
        }
    };
    let (window, exists) = build_window("translate", "Translate");
    // Get Translate Window Size
    let width = match get("translate_window_width") {
        Some(v) => v.as_i64().unwrap(),
        None => {
            set("translate_window_width", 350);
            350
        }
    };
    let height = match get("translate_window_height") {
        Some(v) => v.as_i64().unwrap(),
        None => {
            set("translate_window_height", 420);
            420
        }
    };

    let monitor = window.current_monitor().unwrap().unwrap();
    let dpi = monitor.scale_factor();
    if !exists {
        window.set_skip_taskbar(true).unwrap();
        window
            .set_size(tauri::PhysicalSize::new(
                (width as f64) * dpi,
                (height as f64) * dpi,
            ))
            .unwrap();
    } else {
        let size = window.outer_size().unwrap();
        if size.width < 100 || size.height < 100 {
            window
                .set_size(tauri::PhysicalSize::new(
                    (width as f64) * dpi,
                    (height as f64) * dpi,
                ))
                .unwrap();
        }
    }

    let position_type = match get("translate_window_position") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => "mouse".to_string(),
    };

    match position_type.as_str() {
        "mouse" => {
            // Adjust window position
            let monitor_size = monitor.size();
            let monitor_size_width = monitor_size.width as f64;
            let monitor_size_height = monitor_size.height as f64;
            let monitor_position = monitor.position();
            let monitor_position_x = monitor_position.x as f64;
            let monitor_position_y = monitor_position.y as f64;

            if mouse_position.x as f64 + width as f64 * dpi
                > monitor_position_x + monitor_size_width
            {
                mouse_position.x -= (width as f64 * dpi) as i32;
                if (mouse_position.x as f64) < monitor_position_x {
                    mouse_position.x = monitor_position_x as i32;
                }
            }
            if mouse_position.y as f64 + height as f64 * dpi
                > monitor_position_y + monitor_size_height
            {
                mouse_position.y -= (height as f64 * dpi) as i32;
                if (mouse_position.y as f64) < monitor_position_y {
                    mouse_position.y = monitor_position_y as i32;
                }
            }

            window
                .set_position(tauri::PhysicalPosition::new(
                    mouse_position.x,
                    mouse_position.y,
                ))
                .unwrap();
        }
        _ => {
            let position_x = match get("translate_window_position_x") {
                Some(v) => v.as_i64().unwrap(),
                None => 0,
            };
            let position_y = match get("translate_window_position_y") {
                Some(v) => v.as_i64().unwrap(),
                None => 0,
            };
            window
                .set_position(tauri::PhysicalPosition::new(
                    (position_x as f64) * dpi,
                    (position_y as f64) * dpi,
                ))
                .unwrap();
        }
    }

    window
}

// On Windows, poll GetAsyncKeyState until physical Alt / Ctrl / Shift / Win
// are all released. `selection::get_text()` falls back to simulating Ctrl+C
// with SendInput, but synthetic key-up events do NOT release the user's
// physical keys — if Alt is still held, the OS sees Ctrl+Alt+C and copy
// fails. We must wait for the user to actually let go of the hotkey.
#[cfg(windows)]
fn wait_for_modifiers_released(timeout_ms: u64) -> u64 {
    use std::time::{Duration, Instant};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    let start = Instant::now();
    let deadline = start + Duration::from_millis(timeout_ms);
    let high_bit = 0x8000u16 as i16;
    while Instant::now() < deadline {
        let any_held = unsafe {
            (GetAsyncKeyState(VK_MENU.0 as i32) & high_bit) != 0
                || (GetAsyncKeyState(VK_CONTROL.0 as i32) & high_bit) != 0
                || (GetAsyncKeyState(VK_SHIFT.0 as i32) & high_bit) != 0
                || (GetAsyncKeyState(VK_LWIN.0 as i32) & high_bit) != 0
                || (GetAsyncKeyState(VK_RWIN.0 as i32) & high_bit) != 0
        };
        if !any_held {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_millis(10));
    start.elapsed().as_millis() as u64
}

#[cfg(windows)]
fn foreground_window_info() -> (String, String) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowTextW,
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut title_buf = [0u16; 256];
        let mut class_buf = [0u16; 256];
        let tlen = GetWindowTextW(hwnd, &mut title_buf);
        let clen = GetClassNameW(hwnd, &mut class_buf);
        let title = String::from_utf16_lossy(&title_buf[..tlen as usize]);
        let class = String::from_utf16_lossy(&class_buf[..clen as usize]);
        (title, class)
    }
}

#[cfg(windows)]
fn clipboard_seq() -> u32 {
    use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
    unsafe { GetClipboardSequenceNumber() }
}

#[cfg(not(windows))]
fn wait_for_modifiers_released(_timeout_ms: u64) -> u64 {
    std::thread::sleep(std::time::Duration::from_millis(50));
    50
}

// Custom selected-text reader. The `selection` crate's Ctrl+C fallback only
// waits a fixed 100ms before checking the clipboard, which both feels slow
// on fast apps and fails on slow ones. This version polls the clipboard
// for changes from a sentinel value, so it returns as soon as the target
// app finishes putting the selected text on the clipboard.
#[cfg(windows)]
fn read_selected_text() -> String {
    use arboard::Clipboard;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    let wait_ms = wait_for_modifiers_released(500);
    let (fg_title, fg_class) = foreground_window_info();
    log::info!(
        "read_selected_text: foreground hwnd title=\"{}\" class=\"{}\", mod-wait={}ms",
        fg_title,
        fg_class,
        wait_ms
    );
    let seq_before = clipboard_seq();

    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            log::error!("read_selected_text: clipboard open failed: {}", e);
            return String::new();
        }
    };

    let original = clipboard.get_text().ok();

    // Sentinel so we can distinguish "Ctrl+C didn't fire" from "selection
    // really is the same text that was already on the clipboard".
    let sentinel = "\u{0001}__POT_GETSEL_PROBE__\u{0001}";
    let _ = clipboard.set_text(sentinel);

    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            log::error!("read_selected_text: enigo init failed: {}", e);
            if let Some(orig) = original {
                let _ = clipboard.set_text(orig);
            }
            return String::new();
        }
    };

    // Up to 3 Ctrl+C attempts. Each polls the clipboard for up to
    // `max_wait_ms` and returns immediately as soon as a non-sentinel
    // value appears. Subsequent attempts get more patience for slow apps.
    let attempts: [(u64, u64); 3] = [
        (10, 200),  // attempt 1: 10ms pre, poll up to 200ms
        (40, 400),  // attempt 2
        (80, 600),  // attempt 3
    ];

    let mut result = String::new();
    let total_start = Instant::now();
    let seq_after_sentinel = clipboard_seq();
    log::info!(
        "read_selected_text: clipboard seq before={} after_sentinel={} (delta={})",
        seq_before,
        seq_after_sentinel,
        seq_after_sentinel.wrapping_sub(seq_before)
    );
    for (idx, &(pre_wait, max_wait_ms)) in attempts.iter().enumerate() {
        sleep(Duration::from_millis(pre_wait));
        let seq_pre = clipboard_seq();
        let _ = enigo.key(Key::Control, Direction::Press);
        sleep(Duration::from_millis(5));
        let _ = enigo.key(Key::Unicode('c'), Direction::Click);
        sleep(Duration::from_millis(5));
        let _ = enigo.key(Key::Control, Direction::Release);

        let deadline = Instant::now() + Duration::from_millis(max_wait_ms);
        let mut got = String::new();
        while Instant::now() < deadline {
            if let Ok(text) = clipboard.get_text() {
                if text != sentinel && !text.is_empty() {
                    got = text;
                    break;
                }
            }
            sleep(Duration::from_millis(10));
        }
        let seq_post = clipboard_seq();

        if !got.is_empty() {
            let elapsed = total_start.elapsed().as_millis();
            log::info!(
                "read_selected_text: got text on attempt {} after {}ms, len={}, seq {}→{}",
                idx + 1,
                elapsed,
                got.len(),
                seq_pre,
                seq_post
            );
            result = got;
            break;
        } else {
            log::warn!(
                "read_selected_text: attempt {} timed out after {}ms, seq {}→{} (delta={})",
                idx + 1,
                max_wait_ms,
                seq_pre,
                seq_post,
                seq_post.wrapping_sub(seq_pre)
            );
        }
    }

    match original {
        Some(orig) => {
            let _ = clipboard.set_text(orig);
        }
        None => {
            let _ = clipboard.clear();
        }
    }

    if result.is_empty() {
        log::warn!("read_selected_text: all attempts failed, returning empty");
    }
    result
}

#[cfg(not(windows))]
fn read_selected_text() -> String {
    use selection::get_text;
    wait_for_modifiers_released(500);
    get_text()
}

pub fn selection_translate() {
    let text = read_selected_text();
    let app_handle = APP.get().unwrap();
    let state: tauri::State<StringWrapper> = app_handle.state();
    // Always overwrite the cached state — including with an empty string when
    // read_selected_text() fails — so the frontend's mount-time
    // `invoke('get_text')` does not resurrect the previous translation as if
    // it were a new selection.
    state.0.lock().unwrap().replace_range(.., &text);

    if text.trim().is_empty() {
        log::warn!("selection_translate: empty text, not opening translate window");
        let _ = tauri::api::notification::Notification::new(&app_handle.config().tauri.bundle.identifier)
            .title("Pot")
            .body("无法获取选中文本，请重新选中后再试")
            .show();
        return;
    }

    let window = translate_window();
    window.emit("new_text", text).unwrap();
}

pub fn inline_translate() {
    let text = read_selected_text();
    log::info!("inline_translate called, text length: {}", text.len());
    if text.trim().is_empty() {
        log::info!("inline_translate: empty text, returning");
        return;
    }
    let event_text = format!("[INLINE_TRANSLATE]{}", text);
    let app_handle = APP.get().unwrap();
    // Always overwrite the cached StringWrapper so the frontend's mount /
    // config-dep effect that calls `invoke('get_text')` can never resurrect
    // a stale value from a previous Alt+E / Alt+Q. Both selection_translate
    // and inline_translate must keep this state authoritative.
    {
        let state: tauri::State<StringWrapper> = app_handle.state();
        state.0.lock().unwrap().replace_range(.., &event_text);
    }
    // Get or create a hidden translate window for processing — must NOT
    // focus or show it, otherwise the paste goes to Pot instead of the
    // original application.
    let window = match app_handle.get_window("translate") {
        Some(w) => w,
        None => {
            let width = match get("translate_window_width") {
                Some(v) => v.as_i64().unwrap(),
                None => 350,
            };
            let height = match get("translate_window_height") {
                Some(v) => v.as_i64().unwrap(),
                None => 420,
            };
            // Create silently: no focus and no show. Use the normal translate
            // size because this window is later reused by selection translate.
            tauri::WindowBuilder::new(
                app_handle,
                "translate",
                tauri::WindowUrl::App("index.html".into()),
            )
            .visible(false)
            .focused(false)
            .transparent(true)
            .decorations(false)
            .inner_size(width as f64, height as f64)
            .skip_taskbar(true)
            .additional_browser_args("--disable-web-security")
            .build()
            .unwrap()
        }
    };
    log::info!("inline_translate: emitting event to translate window");
    window.emit("new_text", event_text).unwrap();
}

pub fn input_translate() {
    let app_handle = APP.get().unwrap();
    // Clear State
    let state: tauri::State<StringWrapper> = app_handle.state();
    state
        .0
        .lock()
        .unwrap()
        .replace_range(.., "[INPUT_TRANSLATE]");
    let window = translate_window();
    let position_type = match get("translate_window_position") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => "mouse".to_string(),
    };
    if position_type == "mouse" {
        window.center().unwrap();
    }

    window.emit("new_text", "[INPUT_TRANSLATE]").unwrap();
}

pub fn text_translate(text: String) {
    let app_handle = APP.get().unwrap();
    // Clear State
    let state: tauri::State<StringWrapper> = app_handle.state();
    state.0.lock().unwrap().replace_range(.., &text);
    let window = translate_window();
    window.emit("new_text", text).unwrap();
}

pub fn image_translate() {
    let app_handle = APP.get().unwrap();
    let state: tauri::State<StringWrapper> = app_handle.state();
    state
        .0
        .lock()
        .unwrap()
        .replace_range(.., "[IMAGE_TRANSLATE]");
    let window = translate_window();
    window.emit("new_text", "[IMAGE_TRANSLATE]").unwrap();
}

pub fn recognize_window() {
    let (window, exists) = build_window("recognize", "Recognize");
    if exists {
        window.emit("new_image", "").unwrap();
        return;
    }
    let width = match get("recognize_window_width") {
        Some(v) => v.as_i64().unwrap(),
        None => {
            set("recognize_window_width", 800);
            800
        }
    };
    let height = match get("recognize_window_height") {
        Some(v) => v.as_i64().unwrap(),
        None => {
            set("recognize_window_height", 400);
            400
        }
    };
    let monitor = window.current_monitor().unwrap().unwrap();
    let dpi = monitor.scale_factor();
    window
        .set_size(tauri::PhysicalSize::new(
            (width as f64) * dpi,
            (height as f64) * dpi,
        ))
        .unwrap();
    window.center().unwrap();
    window.emit("new_image", "").unwrap();
}

#[cfg(not(target_os = "macos"))]
fn screenshot_window() -> Window {
    let (window, _exists) = build_window("screenshot", "Screenshot");

    window.set_skip_taskbar(true).unwrap();
    #[cfg(target_os = "macos")]
    {
        let monitor = window.current_monitor().unwrap().unwrap();
        let size = monitor.size();
        window.set_decorations(false).unwrap();
        window.set_size(*size).unwrap();
    }

    #[cfg(not(target_os = "macos"))]
    window.set_fullscreen(true).unwrap();

    window.set_always_on_top(true).unwrap();
    window
}

pub fn ocr_recognize() {
    #[cfg(target_os = "macos")]
    {
        let app_handle = APP.get().unwrap();
        let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
        app_cache_dir_path.push(&app_handle.config().tauri.bundle.identifier);
        if !app_cache_dir_path.exists() {
            // 创建目录
            fs::create_dir_all(&app_cache_dir_path).expect("Create Cache Dir Failed");
        }
        app_cache_dir_path.push("pot_screenshot_cut.png");

        let path = app_cache_dir_path.to_string_lossy().replace("\\\\?\\", "");
        println!("Screenshot path: {}", path);
        if let Ok(_output) = std::process::Command::new("/usr/sbin/screencapture")
            .arg("-i")
            .arg("-r")
            .arg(path)
            .output()
        {
            recognize_window();
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let window = screenshot_window();
        let window_ = window.clone();
        window.listen("success", move |event| {
            recognize_window();
            window_.unlisten(event.id())
        });
    }
}
pub fn ocr_translate() {
    #[cfg(target_os = "macos")]
    {
        let app_handle = APP.get().unwrap();
        let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
        app_cache_dir_path.push(&app_handle.config().tauri.bundle.identifier);
        if !app_cache_dir_path.exists() {
            // 创建目录
            fs::create_dir_all(&app_cache_dir_path).expect("Create Cache Dir Failed");
        }
        app_cache_dir_path.push("pot_screenshot_cut.png");

        let path = app_cache_dir_path.to_string_lossy().replace("\\\\?\\", "");
        println!("Screenshot path: {}", path);
        if let Ok(_output) = std::process::Command::new("/usr/sbin/screencapture")
            .arg("-i")
            .arg("-r")
            .arg(path)
            .output()
        {
            image_translate();
            ();
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let window = screenshot_window();
        let window_ = window.clone();
        window.listen("success", move |event| {
            image_translate();
            window_.unlisten(event.id())
        });
    }
}

#[tauri::command(async)]
pub fn updater_window() {
    let (window, _exists) = build_window("updater", "Updater");
    window
        .set_min_size(Some(tauri::LogicalSize::new(600, 400)))
        .unwrap();
    window.set_size(tauri::LogicalSize::new(600, 400)).unwrap();
    window.center().unwrap();
}
