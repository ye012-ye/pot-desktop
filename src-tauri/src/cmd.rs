use crate::config::get;
use crate::config::StoreWrapper;
use crate::error::Error;
use crate::StringWrapper;
use crate::APP;
use log::{error, info};
use serde_json::{json, Value};
use std::io::Read;
use tauri::Manager;

#[tauri::command]
pub fn get_text(state: tauri::State<StringWrapper>) -> String {
    return state.0.lock().unwrap().to_string();
}

#[tauri::command]
pub fn reload_store() {
    let state = APP.get().unwrap().state::<StoreWrapper>();
    let mut store = state.0.lock().unwrap();
    store.load().unwrap();
}

#[tauri::command]
pub fn cut_image(left: u32, top: u32, width: u32, height: u32, app_handle: tauri::AppHandle) {
    use dirs::cache_dir;
    use image::GenericImage;
    info!("Cut image: {}x{}+{}+{}", width, height, left, top);
    let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
    app_cache_dir_path.push(&app_handle.config().tauri.bundle.identifier);
    app_cache_dir_path.push("pot_screenshot.png");
    if !app_cache_dir_path.exists() {
        return;
    }
    let mut img = match image::open(&app_cache_dir_path) {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e.to_string());
            return;
        }
    };
    let img2 = img.sub_image(left, top, width, height);
    app_cache_dir_path.pop();
    app_cache_dir_path.push("pot_screenshot_cut.png");
    match img2.to_image().save(&app_cache_dir_path) {
        Ok(_) => {}
        Err(e) => {
            error!("{:?}", e.to_string());
        }
    }
}

#[tauri::command]
pub fn get_base64(app_handle: tauri::AppHandle) -> String {
    use base64::{engine::general_purpose, Engine as _};
    use dirs::cache_dir;
    use std::fs::File;
    use std::io::Read;
    let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
    app_cache_dir_path.push(&app_handle.config().tauri.bundle.identifier);
    app_cache_dir_path.push("pot_screenshot_cut.png");
    if !app_cache_dir_path.exists() {
        return "".to_string();
    }
    let mut file = File::open(app_cache_dir_path).unwrap();
    let mut vec = Vec::new();
    match file.read_to_end(&mut vec) {
        Ok(_) => {}
        Err(e) => {
            error!("{:?}", e.to_string());
            return "".to_string();
        }
    }
    let base64 = general_purpose::STANDARD.encode(&vec);
    base64.replace("\r\n", "")
}

#[tauri::command]
pub fn copy_img(app_handle: tauri::AppHandle, width: usize, height: usize) -> Result<(), Error> {
    use arboard::{Clipboard, ImageData};
    use dirs::cache_dir;
    use image::ImageReader;
    use std::borrow::Cow;

    let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
    app_cache_dir_path.push(&app_handle.config().tauri.bundle.identifier);
    app_cache_dir_path.push("pot_screenshot_cut.png");
    let data = ImageReader::open(app_cache_dir_path)?.decode()?;

    let img = ImageData {
        width,
        height,
        bytes: Cow::from(data.as_bytes()),
    };
    let result = Clipboard::new()?.set_image(img)?;
    Ok(result)
}

#[tauri::command]
pub fn set_proxy() -> Result<bool, ()> {
    let host = match get("proxy_host") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => return Err(()),
    };
    let port = match get("proxy_port") {
        Some(v) => v.as_i64().unwrap(),
        None => return Err(()),
    };
    let no_proxy = match get("no_proxy") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => return Err(()),
    };
    let proxy = format!("http://{}:{}", host, port);

    std::env::set_var("http_proxy", &proxy);
    std::env::set_var("https_proxy", &proxy);
    std::env::set_var("all_proxy", &proxy);
    std::env::set_var("no_proxy", &no_proxy);
    Ok(true)
}

#[tauri::command]
pub fn unset_proxy() -> Result<bool, ()> {
    std::env::remove_var("http_proxy");
    std::env::remove_var("https_proxy");
    std::env::remove_var("all_proxy");
    std::env::remove_var("no_proxy");
    Ok(true)
}

#[tauri::command]
pub fn install_plugin(path_list: Vec<String>) -> Result<i32, Error> {
    let mut success_count = 0;

    for path in path_list {
        if !path.ends_with("potext") {
            continue;
        }
        let path = std::path::Path::new(&path);
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let file_name = file_name.replace(".potext", "");
        if !file_name.starts_with("plugin") {
            return Err(Error::Error(
                "Invalid Plugin: file name must start with plugin".into(),
            ));
        }

        let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        #[allow(unused_mut)]
        let mut plugin_type: String;
        if let Ok(mut info) = zip.by_name("info.json") {
            let mut content = String::new();
            info.read_to_string(&mut content)?;
            let json: serde_json::Value = serde_json::from_str(&content)?;
            plugin_type = json["plugin_type"]
                .as_str()
                .ok_or(Error::Error("can't find plugin type in info.json".into()))?
                .to_string();
        } else {
            return Err(Error::Error("Invalid Plugin: miss info.json".into()));
        }
        if zip.by_name("main.js").is_err() {
            return Err(Error::Error("Invalid Plugin: miss main.js".into()));
        }
        let config_path = dirs::config_dir().unwrap();
        let config_path =
            config_path.join(APP.get().unwrap().config().tauri.bundle.identifier.clone());
        let config_path = config_path.join("plugins");
        let config_path = config_path.join(plugin_type);
        let plugin_path = config_path.join(file_name);
        std::fs::create_dir_all(&config_path)?;
        zip.extract(&plugin_path)?;

        success_count += 1;
    }
    Ok(success_count)
}

#[tauri::command]
pub fn run_binary(
    plugin_type: String,
    plugin_name: String,
    cmd_name: String,
    args: Vec<String>,
) -> Result<Value, Error> {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let config_path = dirs::config_dir().unwrap();
    let config_path = config_path.join(APP.get().unwrap().config().tauri.bundle.identifier.clone());
    let config_path = config_path.join("plugins");
    let config_path = config_path.join(plugin_type);
    let plugin_path = config_path.join(plugin_name);

    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("cmd");
    #[cfg(target_os = "windows")]
    let cmd = cmd.creation_flags(0x08000000);
    #[cfg(target_os = "windows")]
    let cmd = cmd.args(["/c", &cmd_name]);
    #[cfg(not(target_os = "windows"))]
    let mut cmd = Command::new(&cmd_name);

    let output = cmd.args(args).current_dir(plugin_path).output()?;
    Ok(json!({
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        "status": output.status.code().unwrap_or(-1),
    }))
}

#[tauri::command]
pub fn font_list() -> Result<Vec<String>, Error> {
    use font_kit::source::SystemSource;
    let source = SystemSource::new();

    Ok(source.all_families()?)
}

#[tauri::command]
pub fn start_ollama_serve() {
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;

    // 候选可执行文件：先试 PATH，再试常见安装位置
    let mut candidates: Vec<String> = vec!["ollama".to_string()];

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let mut p = PathBuf::from(local_app);
            p.push("Programs");
            p.push("Ollama");
            p.push("ollama.exe");
            candidates.push(p.to_string_lossy().into_owned());
        }
        candidates.push(r"C:\Program Files\Ollama\ollama.exe".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push("/usr/local/bin/ollama".to_string());
        candidates.push("/opt/homebrew/bin/ollama".to_string());
        candidates.push("/Applications/Ollama.app/Contents/Resources/ollama".to_string());
    }
    #[cfg(target_os = "linux")]
    {
        candidates.push("/usr/local/bin/ollama".to_string());
        candidates.push("/usr/bin/ollama".to_string());
    }

    for exe in candidates {
        let mut cmd = Command::new(&exe);
        cmd.arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());

        // CREATE_NO_WINDOW + DETACHED_PROCESS：黑框不闪 + pot 退出不带走 ollama
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000 | 0x00000008);

        match cmd.spawn() {
            Ok(_) => {
                info!("Spawned `ollama serve` via {}", exe);
                return;
            }
            Err(e) => info!("Skip {}: {}", exe, e),
        }
    }
    info!("All ollama candidates failed; ollama may not be installed");
}

fn lmstudio_cli_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates = vec![std::path::PathBuf::from("lms")];
    if let Some(home_dir) = dirs::home_dir() {
        #[cfg(target_os = "windows")]
        candidates.push(home_dir.join(".lmstudio").join("bin").join("lms.exe"));
        #[cfg(not(target_os = "windows"))]
        candidates.push(home_dir.join(".lmstudio").join("bin").join("lms"));
    }
    candidates
}

fn lmstudio_server_command(executable: &std::path::Path, port: u16) -> std::process::Command {
    use std::process::{Command, Stdio};
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;

    let mut command = Command::new(executable);
    command
        .args(["server", "start", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000 | 0x00000008);

    command
}

#[tauri::command]
pub fn start_lmstudio_server(port: u16) -> Result<(), Error> {
    let mut last_error = None;
    for executable in lmstudio_cli_candidates() {
        match lmstudio_server_command(&executable, port).spawn() {
            Ok(_) => {
                info!(
                    "Spawned LM Studio server on port {} via {:?}",
                    port, executable
                );
                return Ok(());
            }
            Err(error) => {
                info!("Skip {:?}: {}", executable, error);
                last_error = Some(error);
            }
        }
    }
    Err(Error::Io(last_error.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "LM Studio CLI was not found")
    })))
}

#[cfg(test)]
mod lmstudio_tests {
    use super::lmstudio_server_command;
    use std::path::Path;

    #[test]
    fn lmstudio_server_command_uses_requested_port() {
        let command = lmstudio_server_command(Path::new("lms"), 1234);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["server", "start", "--port", "1234"]);
    }
}

#[tauri::command]
pub fn open_devtools(window: tauri::Window) {
    if !window.is_devtools_open() {
        window.open_devtools();
    } else {
        window.close_devtools();
    }
}

// Write text to the Windows clipboard while marking it with
// `ExcludeClipboardContentFromMonitorProcessing` so the Cloud / Win+V
// clipboard-history service skips this entry. Used by `inline_paste` so
// the translated text we briefly stage on the clipboard does not pile up
// in the user's clipboard history.
#[cfg(windows)]
fn set_clipboard_text_no_history(text: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HANDLE, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
        SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_UNICODETEXT: u32 = 13;

    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let bytes = wide.len() * std::mem::size_of::<u16>();

    let exclude_name: Vec<u16> = "ExcludeClipboardContentFromMonitorProcessing\0"
        .encode_utf16()
        .collect();

    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return Err("OpenClipboard failed".into());
        }
        let _ = EmptyClipboard();

        let h_mem = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|e| {
            let _ = CloseClipboard();
            format!("GlobalAlloc failed: {}", e)
        })?;
        let dst = GlobalLock(h_mem) as *mut u16;
        if dst.is_null() {
            let _ = CloseClipboard();
            return Err("GlobalLock failed".into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
        let _ = GlobalUnlock(h_mem);

        if SetClipboardData(CF_UNICODETEXT, HANDLE(h_mem.0)).is_err() {
            let _ = CloseClipboard();
            return Err("SetClipboardData(CF_UNICODETEXT) failed".into());
        }

        let fmt = RegisterClipboardFormatW(PCWSTR(exclude_name.as_ptr()));
        if fmt != 0 {
            // The format only needs to be present — value can be a null
            // handle. Clipboard history scans for the format name and
            // skips the entry when it is registered.
            let _ = SetClipboardData(fmt, HANDLE(std::ptr::null_mut()));
        }

        let _ = CloseClipboard();
    }
    Ok(())
}

#[tauri::command]
pub fn inline_paste(text: String) -> Result<(), Error> {
    use arboard::Clipboard;
    use enigo::{Enigo, Key, Keyboard, Settings};
    use enigo::Direction::{Press, Release};
    use std::thread;
    use std::time::Duration;

    log::info!("inline_paste: received text length: {}", text.len());
    // Save original clipboard text (if any).
    let mut old_clipboard: Option<String> = None;
    if let Ok(mut clipboard) = Clipboard::new() {
        old_clipboard = clipboard.get_text().ok();
    }
    // Stage the translated text WITHOUT polluting Win+V clipboard history.
    #[cfg(windows)]
    {
        if let Err(e) = set_clipboard_text_no_history(&text) {
            log::warn!("inline_paste: history-exclude set failed ({}), falling back to arboard", e);
            if let Ok(mut clipboard) = Clipboard::new() {
                let _ = clipboard.set_text(&text);
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(&text);
        }
    }

    // Small delay to ensure clipboard is updated
    thread::sleep(Duration::from_millis(50));

    // Simulate paste: Ctrl+V (Cmd+V on macOS)
    #[cfg(target_os = "macos")]
    let paste_key = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let paste_key = Key::Control;

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| Error::Error(Box::new(e)))?;

    enigo
        .key(paste_key, Press)
        .map_err(|e| Error::Error(Box::new(e)))?;
    enigo
        .key(Key::Unicode('v'), Press)
        .map_err(|e| Error::Error(Box::new(e)))?;
    enigo
        .key(Key::Unicode('v'), Release)
        .map_err(|e| Error::Error(Box::new(e)))?;
    enigo
        .key(paste_key, Release)
        .map_err(|e| Error::Error(Box::new(e)))?;

    // Restore original clipboard after paste (only if we saved text).
    // Also mark as history-excluded so the restore doesn't appear in Win+V.
    thread::sleep(Duration::from_millis(200));
    if let Some(old) = old_clipboard {
        #[cfg(windows)]
        {
            if let Err(e) = set_clipboard_text_no_history(&old) {
                log::warn!("inline_paste: restore history-exclude failed ({}), falling back", e);
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(&old);
                }
            }
        }
        #[cfg(not(windows))]
        {
            if let Ok(mut clipboard) = Clipboard::new() {
                let _ = clipboard.set_text(&old);
            }
        }
    }

    Ok(())
}
