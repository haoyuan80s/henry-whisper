use henry_whisper_ipc_gen::ipc_command;
use tauri::Manager;

use super::settings::AppSettings;
use super::settings::persist_settings;
use super::shortcuts::register_shortcuts;
use super::state::AppState;
use crate::ai::AiModel;

#[ipc_command]
#[tauri::command]
pub fn frontend_trace(message: String) {
    tracing::trace!("frontend: {}", message);
}

#[ipc_command]
#[tauri::command]
pub fn frontend_debug(message: String) {
    tracing::debug!("frontend: {}", message);
}

#[ipc_command]
#[tauri::command]
pub fn frontend_info(message: String) {
    tracing::info!("frontend: {}", message);
}

#[ipc_command]
#[tauri::command]
pub fn frontend_warn(message: String) {
    tracing::warn!("frontend: {}", message);
}

#[ipc_command]
#[tauri::command]
pub fn frontend_error(message: String) {
    tracing::error!("frontend: {}", message);
}

#[ipc_command]
#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

#[ipc_command]
#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let shortcut_setting = settings.shortcut.clone();
    let model = AiModel::from_settings(&settings);

    *state.settings.lock().unwrap() = settings.clone();
    *state.model.lock().unwrap() = model;
    persist_settings(&app, &settings)?;
    tauri::async_runtime::spawn(async move {
        register_shortcuts(&app, &shortcut_setting);
    });
    Ok(())
}

#[ipc_command]
#[tauri::command]
pub fn hide_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
