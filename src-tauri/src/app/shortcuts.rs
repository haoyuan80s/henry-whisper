use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::ShortcutState;

use super::recording::do_cancel_recording;
use super::recording::do_record_or_transcribe;

pub fn register_shortcuts(app: &tauri::AppHandle, rec: &str, cancel: &str) {
    let gs = app.global_shortcut();
    gs.unregister_all().ok();

    if let Err(e) = gs.on_shortcut(rec, |app, _, event| {
        if event.state == ShortcutState::Pressed {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = do_record_or_transcribe(app).await {
                    eprintln!("record_or_transcribe: {e}");
                }
            });
        }
    }) {
        eprintln!("Failed to register record/transcribe shortcut '{rec}': {e}");
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
