use crate::config::{get, set};
use crate::window::{input_translate, inline_translate, ocr_recognize, ocr_translate, selection_translate};
use crate::APP;
use log::{info, warn};
use tauri::{AppHandle, GlobalShortcutManager};

fn register<F>(app_handle: &AppHandle, name: &str, handler: F, key: &str) -> Result<(), String>
where
    F: Fn() + Send + 'static,
{
    let hotkey = {
        if key.is_empty() {
            match get(name) {
                Some(v) => v.as_str().unwrap().to_string(),
                None => {
                    set(name, "");
                    String::new()
                }
            }
        } else {
            key.to_string()
        }
    };

    if !hotkey.is_empty() {
        match app_handle
            .global_shortcut_manager()
            .register(hotkey.as_str(), handler)
        {
            Ok(()) => {
                info!("Registered global shortcut: {} for {}", hotkey, name);
            }
            Err(e) => {
                warn!("Failed to register global shortcut: {} {:?}", hotkey, e);
                return Err(e.to_string());
            }
        };
    }
    Ok(())
}

// Register global shortcuts
pub fn register_shortcut(shortcut: &str) -> Result<(), String> {
    let app_handle = APP.get().unwrap();
    match shortcut {
        "hotkey_selection_translate" => register(
            app_handle,
            "hotkey_selection_translate",
            selection_translate,
            "",
        )?,
        "hotkey_input_translate" => {
            register(app_handle, "hotkey_input_translate", input_translate, "")?
        }
        "hotkey_ocr_recognize" => register(app_handle, "hotkey_ocr_recognize", ocr_recognize, "")?,
        "hotkey_ocr_translate" => register(app_handle, "hotkey_ocr_translate", ocr_translate, "")?,
        "all" => {
            // Register each shortcut independently so that one failure (e.g.
            // the key is already taken by another app) does not skip the
            // remaining registrations. Previously the `?` operator caused any
            // earlier failure to short-circuit `hotkey_inline_translate`,
            // leaving Alt+E silently unbound after restart.
            let mut errors: Vec<String> = Vec::new();
            let mut try_register = |name: &str, handler: fn()| {
                if let Err(e) = register(app_handle, name, handler, "") {
                    errors.push(format!("{}: {}", name, e));
                }
            };
            try_register("hotkey_selection_translate", selection_translate);
            try_register("hotkey_input_translate", input_translate);
            try_register("hotkey_ocr_recognize", ocr_recognize);
            try_register("hotkey_ocr_translate", ocr_translate);
            try_register("hotkey_inline_translate", inline_translate);
            if !errors.is_empty() {
                return Err(errors.join("\n"));
            }
        }
        "hotkey_inline_translate" => register(
            app_handle,
            "hotkey_inline_translate",
            inline_translate,
            "",
        )?,
        _ => {}
    }
    Ok(())
}

#[tauri::command]
pub fn register_shortcut_by_frontend(name: &str, shortcut: &str) -> Result<(), String> {
    let app_handle = APP.get().unwrap();
    match name {
        "hotkey_selection_translate" => register(
            app_handle,
            "hotkey_selection_translate",
            selection_translate,
            shortcut,
        )?,
        "hotkey_input_translate" => register(
            app_handle,
            "hotkey_input_translate",
            input_translate,
            shortcut,
        )?,
        "hotkey_ocr_recognize" => {
            register(app_handle, "hotkey_ocr_recognize", ocr_recognize, shortcut)?
        }
        "hotkey_ocr_translate" => {
            register(app_handle, "hotkey_ocr_translate", ocr_translate, shortcut)?
        }
        "hotkey_inline_translate" => {
            register(app_handle, "hotkey_inline_translate", inline_translate, shortcut)?
        }
        _ => {}
    }
    Ok(())
}
