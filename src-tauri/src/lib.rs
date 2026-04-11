use std::sync::{Arc, Mutex};

use arboard::Clipboard;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde_json::json;
use tauri::{menu::{Menu, MenuItem, PredefinedMenuItem, Submenu}, Emitter, State};

struct RecordingHandle {
    samples: Arc<Mutex<Vec<f32>>>,
    stop_tx: std::sync::mpsc::SyncSender<()>,
    sample_rate: u32,
    channels: u16,
}

struct AppState {
    recording: Mutex<Option<RecordingHandle>>,
    api_key: Mutex<String>,
}

#[tauri::command]
fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
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
    let (stop_tx, stop_rx) = std::sync::mpsc::sync_channel(1);

    std::thread::spawn(move || {
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
        stop_rx.recv().ok();
        drop(stream);
    });

    *recording = Some(RecordingHandle {
        samples,
        stop_tx,
        sample_rate,
        channels,
    });
    Ok(())
}

#[tauri::command]
async fn stop_and_transcribe(state: State<'_, AppState>) -> Result<String, String> {
    let handle = {
        let mut recording = state.recording.lock().unwrap();
        recording.take().ok_or("Not currently recording")?
    };

    handle.stop_tx.send(()).ok();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let samples = handle.samples.lock().unwrap().clone();
    if samples.is_empty() {
        return Err("No audio recorded".to_string());
    }

    let wav_bytes = encode_wav(&samples, handle.sample_rate, handle.channels)
        .map_err(|e| e.to_string())?;
    let audio_b64 = BASE64.encode(&wav_bytes);

    let api_key = state.api_key.lock().unwrap().clone();
    if api_key.is_empty() {
        return Err("API key not configured. Go to File → Settings.".to_string());
    }

    let client = reqwest::Client::new();
    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&json!({
            "model": "xiaomi/mimo-v2-omni",
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "Transcribe this audio exactly. Output only the transcription, no extra commentary."
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
        return Err(format!("API error: {err}"));
    }

    let transcript = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(transcript)
}

#[tauri::command]
fn copy_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(&text).map_err(|e| e.to_string())?;
    let _ = std::process::Command::new("afplay")
        .arg("/System/Library/Sounds/Glass.aiff")
        .status();
    Ok(())
}

#[tauri::command]
fn set_api_key(key: String, state: State<'_, AppState>) -> Result<(), String> {
    *state.api_key.lock().unwrap() = key;
    Ok(())
}

#[tauri::command]
fn get_api_key(state: State<'_, AppState>) -> String {
    state.api_key.lock().unwrap().clone()
}

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();

    tauri::Builder::default()
        .manage(AppState {
            recording: Mutex::new(None),
            api_key: Mutex::new(api_key),
        })
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "Quit Henry Whisper", true, Some("CmdOrCtrl+Q"))?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let settings = MenuItem::with_id(app, "settings", "Settings...", true, Some("CmdOrCtrl+Comma"))?;
            let file = Submenu::with_items(app, "File", true, &[&settings, &sep1, &quit])?;

            let record_start = MenuItem::with_id(app, "record_start", "Start Recording", true, Some("CmdOrCtrl+R"))?;
            let record_stop = MenuItem::with_id(app, "record_stop", "Stop & Transcribe", true, Some("CmdOrCtrl+T"))?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let copy = MenuItem::with_id(app, "copy_transcript", "Copy Transcript", true, Some("CmdOrCtrl+Shift+C"))?;
            let clear = MenuItem::with_id(app, "clear_transcript", "Clear", true, None::<&str>)?;
            let record = Submenu::with_items(app, "Record", true, &[
                &record_start, &record_stop, &sep2, &copy, &clear,
            ])?;

            let menu = Menu::with_items(app, &[&file, &record])?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "quit" => app.exit(0),
                id => {
                    let _ = app.emit(id, ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_and_transcribe,
            copy_to_clipboard,
            set_api_key,
            get_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
