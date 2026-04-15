use serde::Deserialize;
use serde::Serialize;
use tauri::Manager;

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub transcription_model: AiModelSetting,
    pub polish_model: AiModelSetting,
    pub shortcut: ShortcutSetting,
    pub polish: bool,
    pub play_sound: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            transcription_model: AiModelSetting {
                base_url: "https://qwen.gooseread.com/v1".to_string(),
                model: "Qwen/Qwen3-ASR-0.6B".to_string(),
            },
            polish_model: AiModelSetting {
                base_url: "https://gemini.gooseread.com/v1".to_string(),
                model: "google/gemma-4-E4B-it".to_string(),
            },
            shortcut: ShortcutSetting {
                recording: "CmdOrCtrl+1".to_string(),
                cancel: "CmdOrCtrl+2".to_string(),
            },
            polish: true,
            play_sound: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AiModelSetting {
    pub base_url: String,
    pub model: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShortcutSetting {
    pub recording: String,
    pub cancel: String,
}

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
    if let Ok(settings) = serde_json::from_value::<AppSettings>(value.clone()) {
        return Some(settings);
    }

    let object = value.as_object()?;
    let mut settings = AppSettings::default();

    if let Some(base_url) = object.get("base_url").and_then(|v| v.as_str()) {
        settings.transcription_model.base_url = base_url.to_string();
    }
    if let Some(model) = object.get("transcription_model").and_then(|v| v.as_str()) {
        settings.transcription_model.model = model.to_string();
    }
    if let Some(recording) = object.get("recording_shortcut").and_then(|v| v.as_str()) {
        settings.shortcut.recording = recording.to_string();
    }
    if let Some(cancel) = object.get("cancel_shortcut").and_then(|v| v.as_str()) {
        settings.shortcut.cancel = cancel.to_string();
    }
    if let Some(polish) = object.get("polish").and_then(|v| v.as_bool()) {
        settings.polish = polish;
    }
    if let Some(play_sound) = object.get("play_sound").and_then(|v| v.as_bool()) {
        settings.play_sound = play_sound;
    }

    Some(settings)
}
