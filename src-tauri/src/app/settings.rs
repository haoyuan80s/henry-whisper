pub use henry_whisper_shared::AppSettings;
pub use henry_whisper_shared::ShortcutSetting;
use tauri::Manager;

pub fn settings_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("settings.json")
}

pub fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    let path = settings_path(app);
    if let Some(settings) = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .and_then(settings_from_value)
    {
        persist_settings(app, &settings).ok();
        settings
    } else {
        AppSettings::default()
    }
}

pub fn persist_settings(app: &tauri::AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

fn settings_from_value(value: serde_json::Value) -> Option<AppSettings> {
    serde_json::from_value(value).ok()
}
