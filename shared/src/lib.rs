pub use henry_whisper_macros::tauri_commands;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub shortcut: ShortcutSetting,
    pub play_sound: bool,
    #[serde(default)]
    pub auto_paste: bool,
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
