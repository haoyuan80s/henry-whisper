use serde::Deserialize;
use serde::Serialize;
use tauri::Manager;

const DEFAULT_RECORD_TRANSCRIBE_SHORTCUT: &str = "CmdOrCtrl+Shift+R";

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub api_key: String,
    pub recording_shortcut: String,
    pub cancel_shortcut: String,
    pub play_sound: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_default(),
            recording_shortcut: DEFAULT_RECORD_TRANSCRIBE_SHORTCUT.to_string(),
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
