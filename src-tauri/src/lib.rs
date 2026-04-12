use arboard::Clipboard;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tauri::Manager;
use tauri::menu::Menu;
use tauri::menu::MenuItem;
use tauri::menu::PredefinedMenuItem;
use tauri::tray::TrayIconBuilder;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::ShortcutState;
use tauri_plugin_notification::NotificationExt;
use tracing::debug;

// ── Settings ─────────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub api_key: String,
    pub recording_shortcut: String,
    pub transcribe_shortcut: String,
    pub play_sound: bool,
    pub show_notification: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_default(),
            recording_shortcut: "CmdOrCtrl+Shift+R".to_string(),
            transcribe_shortcut: "CmdOrCtrl+Shift+T".to_string(),
            play_sound: true,
            show_notification: true,
        }
    }
}

fn settings_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("settings.json")
}

fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    let path = settings_path(app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn persist_settings(app: &tauri::AppHandle, settings: &AppSettings) {
    let path = settings_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        std::fs::write(&path, json).ok();
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct RecordingHandle {
    samples: Arc<Mutex<Vec<f32>>>,
    stop_tx: tokio::sync::oneshot::Sender<()>,
    join_handle: tokio::task::JoinHandle<()>,
    sample_rate: u32,
    channels: u16,
}

struct AppState {
    recording: Mutex<Option<RecordingHandle>>,
    settings: Mutex<AppSettings>,
}

// ── Audio ─────────────────────────────────────────────────────────────────────

static NOTIFICATION_SOUND: &[u8] = include_bytes!("../resources/notification.wav");

fn play_sound() {
    thread::spawn(|| {
        let mut device_sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[play_sound] open_default_sink failed: {e}");
                return;
            }
        };
        device_sink.log_on_drop(false);
        let cursor = std::io::Cursor::new(NOTIFICATION_SOUND);
        let player = match rodio::play(device_sink.mixer(), cursor) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[play_sound] play failed: {e}");
                return;
            }
        };
        player.sleep_until_end();
    });
}

// ── Tray helpers ──────────────────────────────────────────────────────────────

fn set_tray_title(app: &tauri::AppHandle, title: Option<&str>) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(Some(title.unwrap_or("Henry Whisper")));
    }
}

// ── Global shortcuts ──────────────────────────────────────────────────────────

fn register_shortcuts(app: &tauri::AppHandle, rec: &str, tx: &str) {
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
                if let Err(e) = do_stop_and_transcribe(&app).await {
                    eprintln!("stop_and_transcribe: {e}");
                }
            });
        }
    }) {
        eprintln!("Failed to register transcribe shortcut '{tx}': {e}");
    }
}

// ── Core logic ────────────────────────────────────────────────────────────────

async fn do_start_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut recording = state.recording.lock().unwrap();
    if recording.is_some() {
        return Err("Already recording".to_string());
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;
    let sample_rate = config.sample_rate();
    let channels = config.channels() as u16;

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_cb = samples.clone();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();

    let join_handle = tokio::task::spawn_blocking(move || {
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    samples_cb.lock().unwrap().extend_from_slice(data);
                },
                |err| eprintln!("Stream error: {err}"),
                None,
            )
            .expect("Failed to build input stream");
        stream.play().expect("Failed to start stream");
        stop_rx.blocking_recv().ok();
        drop(stream);
        // 所有 samples 已写完，任务结束
    });

    *recording = Some(RecordingHandle {
        samples,
        stop_tx,
        join_handle,
        sample_rate,
        channels,
    });
    drop(recording);

    let settings = state.settings.lock().unwrap().clone();

    if settings.play_sound {
        play_sound();
    }

    set_tray_title(&app, Some("Recording..."));
    Ok(())
}

async fn do_stop_and_transcribe(app: &tauri::AppHandle) -> Result<String, String> {
    set_tray_title(app, Some("Transcribing..."));
    let state = app.state::<AppState>();

    let handle = {
        let mut recording = state.recording.lock().unwrap();
        recording.take().ok_or("Not currently recording")?
    };

    handle.stop_tx.send(()).ok();
    handle.join_handle.await.ok();

    let samples = handle.samples.lock().unwrap().clone();
    if samples.is_empty() {
        set_tray_title(app, None);
        return Err("No audio recorded".to_string());
    }

    let wav_bytes =
        encode_wav(&samples, handle.sample_rate, handle.channels).map_err(|e| e.to_string())?;

    if std::env::var("HENRY_WHISPER_DEBUG_AUDIO").as_deref() == Ok("1") {
        let dir = std::path::Path::new("/tmp/henry-whisper");
        std::fs::create_dir_all(dir).ok();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("{ts}.wav"));
        std::fs::write(&path, &wav_bytes).ok();
        debug!(path = %path.display(), "saved debug audio");
    }

    let audio_b64 = BASE64.encode(&wav_bytes);

    let api_key = state.settings.lock().unwrap().api_key.clone();
    if api_key.is_empty() {
        set_tray_title(app, None);
        return Err("API key not configured. Open Settings from the tray.".to_string());
    }

    let client = reqwest::Client::new();
    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&json!({
            "model": "google/gemini-3.1-flash-lite-preview",
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "Transcribe this audio exactly. Output only the spoken words, with no extra commentary, labels, or formatting. If no voice is audible, output exactly: don't hear"
                    },
                    {
                        "type": "input_audio",
                        "input_audio": { "data": audio_b64, "format": "wav" }
                    }
                ]
            }]
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if let Some(err) = body.get("error") {
        set_tray_title(app, None);
        return Err(format!("API error: {err}"));
    }

    let transcript = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    debug!(transcript = %transcript, "transcription result");

    // Always copy to clipboard
    if let Ok(mut cb) = Clipboard::new() {
        let _ = cb.set_text(&transcript);
    }

    let settings = state.settings.lock().unwrap().clone();

    if settings.show_notification && !transcript.is_empty() {
        let _ = app
            .notification()
            .builder()
            .title("Transcribed")
            .body(&transcript)
            .show();
    }

    if settings.play_sound {
        play_sound();
    }
    set_tray_title(app, None);
    Ok(transcript)
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
async fn start_recording(app: tauri::AppHandle) -> Result<(), String> {
    do_start_recording(app).await
}

#[tauri::command]
async fn stop_and_transcribe(app: tauri::AppHandle) -> Result<String, String> {
    let transcribe = do_stop_and_transcribe(&app).await?;
    Ok(transcribe)
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    settings: AppSettings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let rec = settings.recording_shortcut.clone();
    let tx = settings.transcribe_shortcut.clone();
    *state.settings.lock().unwrap() = settings.clone();
    persist_settings(&app, &settings);
    register_shortcuts(&app, &rec, &tx);
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn hide_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── WAV encoding ──────────────────────────────────────────────────────────────

fn encode_wav(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut writer = hound::WavWriter::new(cursor, spec)?;
        for &s in samples {
            let pcm = (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            writer.write_sample(pcm)?;
        }
        writer.finalize()?;
    }
    Ok(buf)
}

// ── Entry point ───────────────────────────────────────────────────────────────

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
            let record = MenuItem::with_id(app, "record", "record", true, None::<&str>)?;
            let transcribe =
                MenuItem::with_id(app, "transcribe", "transcribe", true, None::<&str>)?;

            let cancel = MenuItem::with_id(app, "transcribe", "cancel", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Henry Whisper", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[&record, &transcribe, &sep1, &settings_item, &sep2, &quit],
            )?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .title("Henry Whisper")
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
                            if let Err(e) = do_start_recording(app).await {
                                eprintln!("start_recording: {e}");
                            }
                        });
                    }
                    "transcribe" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = do_stop_and_transcribe(&app).await {
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
            start_recording,
            stop_and_transcribe,
            get_settings,
            save_settings,
            hide_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
