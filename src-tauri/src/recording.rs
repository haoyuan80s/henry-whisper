use arboard::Clipboard;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;
use tracing::debug;

use crate::audio::encode_wav;
use crate::audio::play_sound;
use crate::state::AppState;
use crate::state::RecordingHandle;
use crate::tray::set_tray_title;

pub async fn do_start_recording(app: tauri::AppHandle) -> Result<(), String> {
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

pub async fn do_stop_and_transcribe(app: tauri::AppHandle) -> Result<String, String> {
    set_tray_title(&app, Some("Transcribing..."));
    let state = app.state::<AppState>();

    let handle = {
        let mut recording = state.recording.lock().unwrap();
        recording.take().ok_or("Not currently recording")?
    };

    handle.stop_tx.send(()).ok();
    handle.join_handle.await.ok();

    let samples = handle.samples.lock().unwrap().clone();
    if samples.is_empty() {
        set_tray_title(&app, None);
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
        set_tray_title(&app, None);
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
        set_tray_title(&app, None);
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
    set_tray_title(&app, None);
    Ok(transcript)
}

pub async fn do_cancel_recording(app: tauri::AppHandle) -> Result<(), String> {
    set_tray_title(&app, None);
    let state = app.state::<AppState>();

    let handle = {
        let mut recording = state.recording.lock().unwrap();
        recording.take().ok_or("Not currently recording")?
    };

    handle.stop_tx.send(()).ok();
    handle.join_handle.await.ok();

    Ok(())
}
