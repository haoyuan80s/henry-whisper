mod ai;
mod app;
mod audio;
mod commands;
mod recording;
mod settings;
mod shortcuts;
mod state;
mod tray;

use audio::AudioPlayer;
use recording::do_record_or_transcribe;
use settings::load_settings;
use shortcuts::register_shortcuts;
use state::AppState;
use std::sync::Mutex;
use tauri::Manager;
use tauri::menu::Menu;
use tauri::menu::MenuItem;
use tauri::menu::PredefinedMenuItem;
use tauri::tray::TrayIconBuilder;

use crate::ai::Ai;
use crate::recording::do_cancel_recording;
use crate::tray::DEFAULT_TRAY_TITLE;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let ai = Ai::new("http://192.168.86.29:8001/v1", "Qwen/Qwen3-ASR-0.6B");

            // Load persisted settings
            let settings = load_settings(app.handle());
            app.manage(AppState {
                recording: Mutex::new(None),
                settings: Mutex::new(settings.clone()),
                audio: AudioPlayer::new(),
                ai,
            });

            // Build tray context menu
            let record =
                MenuItem::with_id(app, "record", "Record / Transcribe", true, None::<&str>)?;
            let cancel = MenuItem::with_id(app, "cancel", "Cancel", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let settings_item = MenuItem::with_id(app, "settings", "Setting", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[&record, &cancel, &sep1, &settings_item, &sep2, &quit],
            )?;
            let tray_icon =
                tauri::image::Image::new(include_bytes!("../icons/tray-icon.rgba"), 64, 64);

            TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(true)
                .title(DEFAULT_TRAY_TITLE)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "settings" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "record" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = do_record_or_transcribe(app).await {
                                eprintln!("record_or_transcribe: {e}");
                            }
                        });
                    }
                    "cancel" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = do_cancel_recording(app).await {
                                eprintln!("do_cancel_recording: {e}");
                            }
                        });
                    }
                    _ => {}
                })
                .build(app)?;

            // Closing the settings window hides it instead of quitting
            if let Some(win) = app.get_webview_window("main") {
                let win2 = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        win2.hide().ok();
                        api.prevent_close();
                    }
                });
            }

            // Register global shortcuts
            register_shortcuts(
                app.handle(),
                &settings.recording_shortcut,
                &settings.cancel_shortcut,
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::hide_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
