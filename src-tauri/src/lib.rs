mod audio;
mod commands;
mod recording;
mod settings;
mod shortcuts;
mod state;
mod tray;

use recording::do_start_recording;
use recording::do_stop_and_transcribe;
use settings::load_settings;
use shortcuts::register_shortcuts;
use state::AppState;
use std::sync::Mutex;
use tauri::Manager;
use tauri::menu::Menu;
use tauri::menu::MenuItem;
use tauri::menu::PredefinedMenuItem;
use tauri::tray::TrayIconBuilder;

use crate::recording::do_cancel_recording;
use crate::tray::DEFAULT_TRAY_TITLE;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Load persisted settings
            let settings = load_settings(app.handle());
            app.manage(AppState {
                recording: Mutex::new(None),
                settings: Mutex::new(settings.clone()),
            });

            // Build tray context menu
            let record = MenuItem::with_id(app, "record", "Record", true, None::<&str>)?;
            let transcribe =
                MenuItem::with_id(app, "transcribe", "Transcribe", true, None::<&str>)?;
            let cancel = MenuItem::with_id(app, "cancel", "Cancel", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let settings_item = MenuItem::with_id(app, "settingss", "Setting", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &record,
                    &transcribe,
                    &cancel,
                    &sep1,
                    &settings_item,
                    &sep2,
                    &quit,
                ],
            )?;

            let _ = cancel; // declared but not added to menu (preserved from original)

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
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
                            if let Err(e) = do_start_recording(app).await {
                                eprintln!("start_recording: {e}");
                            }
                        });
                    }
                    "cancel" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = do_cancel_recording(app).await {
                                eprintln!("start_recording: {e}");
                            }
                        });
                    }
                    "transcribe" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = do_stop_and_transcribe(app).await {
                                eprintln!("stop_and_transcribe: {e}");
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
                &settings.transcribe_shortcut,
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_and_transcribe,
            commands::get_settings,
            commands::save_settings,
            commands::hide_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
