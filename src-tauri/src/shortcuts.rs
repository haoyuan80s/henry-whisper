use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::ShortcutState;

use crate::recording::do_cancel_recording;
use crate::recording::do_start_recording;
use crate::recording::do_stop_and_transcribe;

pub fn register_shortcuts(app: &tauri::AppHandle, rec: &str, tx: &str, cancel: &str) {
    let gs = app.global_shortcut();
    gs.unregister_all().ok();

    if let Err(e) = gs.on_shortcut(rec, |app, _, event| {
        if event.state == ShortcutState::Pressed {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = do_start_recording(app).await {
                    eprintln!("start_recording: {e}");
                }
            });
        }
    }) {
        eprintln!("Failed to register recording shortcut '{rec}': {e}");
    }

    if let Err(e) = gs.on_shortcut(tx, |app, _, event| {
        if event.state == ShortcutState::Pressed {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = do_stop_and_transcribe(app).await {
                    eprintln!("stop_and_transcribe: {e}");
                }
            });
        }
    }) {
        eprintln!("Failed to register transcribe shortcut '{tx}': {e}");
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
        eprintln!("Failed to register transcribe shortcut '{tx}': {e}");
    }
}
