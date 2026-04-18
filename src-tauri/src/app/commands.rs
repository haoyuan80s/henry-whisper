use tauri::Manager;

use crate::ai::AiModel;

use super::settings::AppSettings;
use super::settings::persist_settings;
use super::shortcuts::register_shortcuts;
use super::state::AppState;

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
    let shortcut_setting = settings.shortcut.clone();
    let model = AiModel::new(
        &settings.transcription_model.base_url,
        &settings.transcription_model.model,
    );

    *state.settings.lock().unwrap() = settings.clone();
    *state.model.lock().unwrap() = model;
    persist_settings(&app, &settings)?;
    // Re-register shortcuts on the async runtime so we don't block the IPC
    // response (macOS Carbon APIs dispatch to the main thread internally).
    tauri::async_runtime::spawn(async move {
        register_shortcuts(&app, &shortcut_setting);
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
