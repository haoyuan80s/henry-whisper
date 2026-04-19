use super::recording::{do_cancel_recording, do_record_or_transcribe};
use crate::app::settings::ShortcutSetting;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

pub(super) fn handle_shortcut(app: &tauri::AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }

    let settings = app
        .state::<super::state::AppState>()
        .settings
        .lock()
        .unwrap()
        .shortcut
        .clone();

    let Ok(record_sc) = settings.recording.parse::<Shortcut>() else {
        return;
    };
    let Ok(cancel_sc) = settings.cancel.parse::<Shortcut>() else {
        return;
    };

    let app = app.clone();
    if shortcut == &record_sc {
        tracing::warn!("{shortcut} pressed");
        tauri::async_runtime::spawn(async move {
            if let Err(e) = do_record_or_transcribe(app).await {
                eprintln!("record_or_transcribe: {e}");
            }
        });
    } else if shortcut == &cancel_sc {
        tracing::warn!("{shortcut} pressed");
        tauri::async_runtime::spawn(async move {
            if let Err(e) = do_cancel_recording(app).await {
                eprintln!("do_cancel_recording: {e}");
            }
        });
    }
}

pub fn register_shortcuts(app: &tauri::AppHandle, setting: &ShortcutSetting) {
    let gs = app.global_shortcut();
    gs.unregister_all().ok();

    if let Err(e) = gs.register(setting.recording.as_str()) {
        eprintln!(
            "Failed to register record/transcribe shortcut '{}': {e}",
            setting.recording
        );
    }

    if let Err(e) = gs.register(setting.cancel.as_str()) {
        eprintln!(
            "Failed to register cancel shortcut '{}': {e}",
            setting.cancel
        );
    }
}
