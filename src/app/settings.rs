use henry_whisper_shared::{AppSettings, ShortcutSetting, TranscriptionModelSetting};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::ipc::{frontend_debug, frontend_error, save_settings};

pub fn current_settings(
    model_base_url: ReadSignal<String>,
    model_name: ReadSignal<String>,
    rec_shortcut: ReadSignal<String>,
    cancel_shortcut: ReadSignal<String>,
    play_sound: ReadSignal<bool>,
) -> AppSettings {
    AppSettings {
        transcription_model: TranscriptionModelSetting {
            base_url: model_base_url.get(),
            model: model_name.get(),
        },
        shortcut: ShortcutSetting {
            recording: rec_shortcut.get(),
            cancel: cancel_shortcut.get(),
        },
        play_sound: play_sound.get(),
    }
}

pub fn persist_settings(
    settings: AppSettings,
    request_id: u64,
    save_request_id: ReadSignal<u64>,
    set_error: WriteSignal<Option<String>>,
    set_last_saved: WriteSignal<Option<AppSettings>>,
    on_saved: impl FnOnce() + 'static,
) {
    spawn_local(async move {
        match save_settings(settings.clone()).await {
            Ok(()) => {
                let _ = frontend_debug("settings saved").await;
                if save_request_id.get_untracked() == request_id {
                    set_last_saved.set(Some(settings));
                    on_saved();
                }
            }
            Err(e) => {
                let msg = e.as_string().unwrap_or("Error saving settings".into());
                let _ = frontend_error(&format!("failed to save settings: {msg}")).await;
                if save_request_id.get_untracked() == request_id {
                    set_error.set(Some(msg));
                }
            }
        }
    });
}
