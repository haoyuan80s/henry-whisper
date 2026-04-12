use std::sync::Mutex;

use crate::audio::AudioPlayer;
use crate::settings::AppSettings;

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
    pub audio: AudioPlayer,
}
