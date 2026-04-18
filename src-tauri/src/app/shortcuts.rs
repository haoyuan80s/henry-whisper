use super::recording::do_cancel_recording;
use super::recording::do_record_or_transcribe;
use crate::app::settings::ShortcutSetting;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::ShortcutState;

pub fn register_shortcuts(app: &tauri::AppHandle, setting: &ShortcutSetting) {
    let gs = app.global_shortcut();
    gs.unregister_all().ok();

    let recording: &str = &setting.recording;
    let cancel: &str = &setting.cancel;

    if let Err(e) = gs.on_shortcut(recording, |app, _, event| {
        if event.state == ShortcutState::Pressed {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = do_record_or_transcribe(app).await {
                    eprintln!("record_or_transcribe: {e}");
                }
            });
        }
    }) {
        eprintln!("Failed to register record/transcribe shortcut '{recording}': {e}");
    }

    if let Err(e) = gs.on_shortcut(cancel, |app, _, event| {
        if event.state == ShortcutState::Pressed {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = do_cancel_recording(app).await {
                    eprintln!("do_cancel_recording: {e}");
                }
            });
        }
    }) {
        eprintln!("Failed to register cancel shortcut '{cancel}': {e}");
    }
}
