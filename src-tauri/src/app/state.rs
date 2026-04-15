use std::sync::Mutex;

use arboard::Clipboard;

use super::settings::AppSettings;
use crate::ai::AiModel;
use crate::audio::AudioPlayer;

pub struct RecordingHandle {
    pub sample_rx: crossbeam_channel::Receiver<Vec<f32>>,
    pub stop_tx: tokio::sync::oneshot::Sender<()>,
    pub join_handle: tokio::task::JoinHandle<()>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub struct AppState {
    pub recording: Mutex<Option<RecordingHandle>>,
    pub settings: Mutex<AppSettings>,
    pub clipboard: Mutex<Clipboard>,
    pub audio: AudioPlayer,
    pub transcription_model: AiModel,
    pub polish_model: AiModel,
}
