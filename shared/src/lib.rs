use serde::{Deserialize, Serialize};

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub transcription_model: TranscriptionModelSetting,
    pub shortcut: ShortcutSetting,
    pub play_sound: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionModelSetting {
    pub base_url: String,
    pub model: String,
}

impl Default for TranscriptionModelSetting {
    fn default() -> Self {
        Self {
            base_url: "https://lulu.gooseread.com/v1".to_string(),
            model: "CohereLabs/cohere-transcribe-03-2026".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortcutSetting {
    pub recording: String,
    pub cancel: String,
}

impl Default for ShortcutSetting {
    fn default() -> Self {
        Self {
            recording: "CmdOrCtrl+1".to_string(),
            cancel: "CmdOrCtrl+2".to_string(),
        }
    }
}
