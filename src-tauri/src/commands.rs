use tauri::Manager;

use crate::recording::do_start_recording;
use crate::recording::do_stop_and_transcribe;
use crate::settings::AppSettings;
use crate::settings::persist_settings;
use crate::shortcuts::register_shortcuts;
use crate::state::AppState;

#[tauri::command]
pub async fn start_recording(app: tauri::AppHandle) -> Result<(), String> {
    do_start_recording(app).await
}

#[tauri::command]
pub async fn stop_and_transcribe(app: tauri::AppHandle) -> Result<String, String> {
    do_stop_and_transcribe(app).await
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    settings: AppSettings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let rec = settings.recording_shortcut.clone();
    let tx = settings.transcribe_shortcut.clone();
    let cancel = settings.cancel_shortcut.clone();
    *state.settings.lock().unwrap() = settings.clone();
    persist_settings(&app, &settings);
    register_shortcuts(&app, &rec, &tx, &cancel);
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
