use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub shortcut: ShortcutSetting,
    pub play_sound: bool,
    #[serde(default)]
    pub auto_paste: bool,
    #[serde(default = "default_ai_model")]
    pub ai_model: String,
    #[serde(default = "default_ai_api_base")]
    pub ai_api_base: String,
    #[serde(default)]
    pub ai_api_key: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortcutSetting {
    pub recording: String,
    pub cancel: String,
}

impl Default for ShortcutSetting {
    fn default() -> Self {
        Self {
            recording: "Ctrl+1".to_string(),
            cancel: "Ctrl+2".to_string(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            shortcut: ShortcutSetting::default(),
            play_sound: false,
            auto_paste: false,
            ai_model: default_ai_model(),
            ai_api_base: default_ai_api_base(),
            ai_api_key: String::new(),
        }
    }
}

fn default_ai_model() -> String {
    "Qwen/Qwen3-ASR-1.7B".to_string()
}

fn default_ai_api_base() -> String {
    "https://henry.gooseread.com/v1".to_string()
}
