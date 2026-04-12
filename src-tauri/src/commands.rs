use tauri::Manager;

use crate::settings::AppSettings;
use crate::settings::persist_settings;
use crate::shortcuts::register_shortcuts;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let rec = settings.recording_shortcut.clone();
    let tx = settings.transcribe_shortcut.clone();
    let cancel = settings.cancel_shortcut.clone();
    *state.settings.lock().unwrap() = settings.clone();
    persist_settings(&app, &settings);
    // Re-register shortcuts on the async runtime so we don't block the IPC
    // response (macOS Carbon APIs dispatch to the main thread internally).
    tauri::async_runtime::spawn(async move {
        register_shortcuts(&app, &rec, &tx, &cancel);
    });
    Ok(())
}

#[tauri::command]
pub fn hide_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
