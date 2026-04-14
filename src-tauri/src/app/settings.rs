use serde::Deserialize;
use serde::Serialize;
use tauri::Manager;

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub api_key: String,
    pub base_url: String,
    pub transcription_model: String,
    pub recording_shortcut: String,
    pub cancel_shortcut: String,
    pub play_sound: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: "1234".to_string(),
            base_url: "http://192.168.86.29:8001/v1".to_string(),
            transcription_model: "Qwen/Qwen3-ASR-0.6B".to_string(),
            recording_shortcut: "CmdOrCtrl+Shift+R".to_string(),
            cancel_shortcut: "CmdOrCtrl+Shift+C".to_string(),
            play_sound: true,
        }
    }
}

pub fn settings_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("settings.json")
}

pub fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    let path = settings_path(app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| {
            let value: serde_json::Value = serde_json::from_str(&s).ok()?;
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}

pub fn persist_settings(app: &tauri::AppHandle, settings: &AppSettings) {
    let path = settings_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        std::fs::write(&path, json).ok();
    }
}
