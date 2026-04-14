use anyhow::Result;
use arboard::Clipboard;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use std::sync::Mutex;
use tauri::Manager;
use tracing::debug;

use crate::audio::SoundEffect;
use crate::audio::encode_wav;
use crate::state::AppState;
use crate::state::RecordingHandle;
use crate::tray::set_tray_title;

const VOICE_THRESHOLD: f32 = 0.0001;

fn rms(data: &[f32]) -> f32 {
    let sum = data.iter().map(|x| x * x).sum::<f32>();
    (sum / data.len() as f32).sqrt()
}

pub async fn do_start_recording(app: tauri::AppHandle) -> Result<()> {
    let state = app.state::<AppState>();
    {
        let recording = state.recording.lock().unwrap();
        if recording.is_some() {
            return Err(anyhow::anyhow!("Already recording"));
        }
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device available"))?;
    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate();
    let channels = config.channels() as u16;
    let (sample_tx, sample_rx) = crossbeam_channel::unbounded::<Vec<f32>>();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<()>>();

    let join_handle = tokio::task::spawn_blocking(move || {
        let ready_tx = Mutex::new(Some(ready_tx));
        let warmed_up = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let warmed_up_cb = warmed_up.clone();

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if !warmed_up_cb.load(std::sync::atomic::Ordering::Relaxed) {
                        if rms(data) < VOICE_THRESHOLD {
                            return;
                        }
                        warmed_up_cb.store(true, std::sync::atomic::Ordering::Relaxed);
                    }

                    let _ = sample_tx.send(data.to_vec());

                    if let Some(tx) = ready_tx.lock().unwrap().take() {
                        let _ = tx.send(Ok(()));
                    }
                },
                |err| eprintln!("Stream error: {err}"),
                None,
            )
            .unwrap();
        stream.play().unwrap();
        stop_rx.blocking_recv().ok();
        drop(stream);
    });

    ready_rx.await??;

    *state.recording.lock().unwrap() = Some(RecordingHandle {
        sample_rx,
        stop_tx,
        join_handle,
        sample_rate,
        channels,
    });

    let settings = state.settings.lock().expect("lock settings").clone();

    if settings.play_sound {
        state.audio.play(SoundEffect::Record);
    }

    set_tray_title(&app, Some("Recording..."));
    Ok(())
}

pub async fn do_stop_and_transcribe(app: tauri::AppHandle) -> Result<()> {
    let state = app.state::<AppState>();
    let handle = {
        let mut recording = state.recording.lock().unwrap();
        recording
            .take()
            .ok_or_else(|| anyhow::anyhow!("Not currently recording"))?
    };

    set_tray_title(&app, Some("Transcribing..."));
    handle.stop_tx.send(()).ok();
    handle.join_handle.await.ok();

    let settings = state.settings.lock().unwrap().clone();
    if settings.play_sound {
        state.audio.play(SoundEffect::TranscribeStart);
    }

    let samples: Vec<f32> = handle.sample_rx.try_iter().flatten().collect();
    if samples.is_empty() {
        set_tray_title(&app, None);
        return Err(anyhow::anyhow!("No audio captured"));
    }

    let wav_bytes = encode_wav(&samples, handle.sample_rate, handle.channels)?;

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

    let transcript = match state.ai.transcribe_wav(wav_bytes).await {
        Ok(transcript) => transcript,
        Err(err) => {
            set_tray_title(&app, None);
            return Err(err);
        }
    };
    debug!(transcript = %transcript, "transcription result");

    // Always copy to clipboard
    if let Ok(mut cb) = Clipboard::new() {
        let _ = cb.set_text(&transcript);
    }

    if settings.play_sound {
        state.audio.play(SoundEffect::Transcribe);
    }
    set_tray_title(&app, None);
    Ok(())
}

pub async fn do_record_or_transcribe(app: tauri::AppHandle) -> Result<()> {
    let is_recording = {
        let state = app.state::<AppState>();
        state.recording.lock().unwrap().is_some()
    };

    if is_recording {
        do_stop_and_transcribe(app).await
    } else {
        do_start_recording(app).await
    }
}

pub async fn do_cancel_recording(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let handle = {
        let mut recording = state.recording.lock().unwrap();
        recording.take().ok_or("Not currently recording")?
    };
    set_tray_title(&app, None);
    handle.stop_tx.send(()).ok();
    handle.join_handle.await.ok();
    let settings = state.settings.lock().unwrap().clone();
    if settings.play_sound {
        state.audio.play(SoundEffect::Cancel);
    }
    Ok(())
}
