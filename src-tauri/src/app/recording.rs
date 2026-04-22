use anyhow::Result;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use enigo::{
    Direction,
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::Manager;

use super::state::AppState;
use super::state::RecordingHandle;
use super::tray::set_tray_title;
use crate::app::tray::TrayTitleGuard;
use crate::audio::SoundEffect;
use crate::audio::encode_transcription_mp3;
use crate::audio::play_sound;

const VOICE_THRESHOLD: f32 = 0.0001;
const STREAM_STOP_TIMEOUT: Duration = Duration::from_secs(3);
const TRANSCRIPTION_TIMEOUT: Duration = Duration::from_secs(60);

fn rms(data: &[f32]) -> f32 {
    let sum = data.iter().map(|x| x * x).sum::<f32>();
    (sum / data.len() as f32).sqrt()
}

fn paste_modifier() -> Key {
    if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    }
}

fn send_key(enigo: &mut Enigo, key: Key, direction: Direction) -> Result<()> {
    catch_unwind(AssertUnwindSafe(|| enigo.key(key, direction)))
        .map_err(|_| anyhow::anyhow!("Input simulation panicked"))??;
    Ok(())
}

fn paste_clipboard() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())?;
    let modifier = paste_modifier();

    send_key(&mut enigo, modifier, Press)?;
    std::thread::sleep(Duration::from_millis(20));
    let paste_result = send_key(&mut enigo, Key::Unicode('v'), Click);
    std::thread::sleep(Duration::from_millis(20));
    let release_result = send_key(&mut enigo, modifier, Release);

    paste_result?;
    release_result?;
    Ok(())
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
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_cb = cancelled.clone();
    let warmed_up = Arc::new(AtomicBool::new(false));
    let warmed_up_cb = warmed_up.clone();

    let settings = state.settings.lock().expect("lock settings").clone();
    let is_play_sound = settings.play_sound;

    let join_handle = tokio::task::spawn_blocking(move || {
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if cancelled_cb.load(Ordering::Relaxed) {
                        tracing::debug!("Audio callback: cancelled, ignoring data");
                        return;
                    }
                    tracing::trace!("Audio callback: received {} samples", data.len());
                    if !warmed_up_cb.load(Ordering::Relaxed) {
                        if rms(data) < VOICE_THRESHOLD {
                            return;
                        }
                        if is_play_sound {
                            play_sound(SoundEffect::Record);
                        }
                        warmed_up_cb.store(true, Ordering::Relaxed);
                    }

                    let _ = sample_tx.send(data.to_vec());
                },
                |err| eprintln!("Stream error: {err}"),
                None,
            )
            .unwrap();
        stream.play().unwrap();
        stop_rx.blocking_recv().ok();
        drop(stream);
    });

    *state.recording.lock().unwrap() = Some(RecordingHandle {
        sample_rx,
        stop_tx,
        join_handle,
        sample_rate,
        cancelled,
        channels,
    });

    set_tray_title(&app, Some("Recording..."));
    Ok(())
}

pub async fn do_stop_and_transcribe(app: tauri::AppHandle) -> Result<()> {
    let _title_guard = TrayTitleGuard::new(&app);
    let state = app.state::<AppState>();
    let handle = {
        let mut recording = state.recording.lock().unwrap();
        recording
            .take()
            .ok_or_else(|| anyhow::anyhow!("Not currently recording"))?
    };

    set_tray_title(&app, Some("Transcribing..."));
    handle.stop_tx.send(()).ok();
    let _ = tokio::time::timeout(STREAM_STOP_TIMEOUT, handle.join_handle).await;

    let settings = state.settings.lock().unwrap().clone();
    if settings.play_sound {
        play_sound(SoundEffect::TranscribeStart);
    }

    let samples: Vec<f32> = handle.sample_rx.try_iter().flatten().collect();
    if samples.is_empty() {
        return Err(anyhow::anyhow!("No audio captured"));
    }

    let mp3_bytes = encode_transcription_mp3(&samples, handle.sample_rate, handle.channels)?;
    let input_duration_secs =
        samples.len() as f64 / handle.sample_rate as f64 / f64::from(handle.channels);

    tracing::debug!(
        input_samples = samples.len(),
        input_sample_rate = handle.sample_rate,
        input_channels = handle.channels,
        duration_secs = format_args!("{input_duration_secs:.2}"),
        mp3_bytes = mp3_bytes.len(),
        mp3_kib = format_args!("{:.2}", mp3_bytes.len() as f64 / 1024.0),
        "Prepared transcription audio"
    );

    tracing::debug!(
        "Transcribing MP3 audio with {} input samples at {} Hz across {} channel(s)",
        samples.len(),
        handle.sample_rate,
        handle.channels
    );
    let now = std::time::Instant::now();
    let settings = state.settings.lock().unwrap().clone();
    let model = state.model.lock().unwrap().clone();
    let transcript =
        match tokio::time::timeout(TRANSCRIPTION_TIMEOUT, model.transcribe_mp3(mp3_bytes)).await {
            Ok(Ok(t)) => t,
            Ok(Err(err)) => {
                return Err(err);
            }
            Err(_) => {
                return Err(anyhow::anyhow!("Transcription timed out"));
            }
        };
    let elapsed = now.elapsed();
    tracing::debug!(
        "Single-model transcription completed in {:.2?}: {transcript}",
        elapsed
    );

    state
        .clipboard
        .lock()
        .expect("lock clipboard")
        .set_text(transcript)?;

    if settings.auto_paste {
        // TSMGetInputSourceProperty (used by enigo) requires the main thread on macOS.
        // Dispatch paste to the main thread to avoid EXC_BREAKPOINT crash.
        if let Err(err) = app.run_on_main_thread(|| {
            if let Err(e) = paste_clipboard() {
                tracing::warn!("Auto-paste failed: {e}");
            }
        }) {
            tracing::warn!("Failed to dispatch paste to main thread: {err}");
        }
    }

    if settings.play_sound {
        play_sound(SoundEffect::Transcribe);
    }
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
    let _title_guard = TrayTitleGuard::new(&app);
    let state = app.state::<AppState>();
    let handle = {
        let mut recording = state.recording.lock().unwrap();
        recording.take().ok_or("Not currently recording")?
    };
    tracing::debug!("Cancelling recording");
    handle.cancelled.store(true, Ordering::Relaxed);
    handle.stop_tx.send(()).ok();
    let _ = tokio::time::timeout(STREAM_STOP_TIMEOUT, handle.join_handle).await;
    let settings = state.settings.lock().unwrap().clone();
    if settings.play_sound {
        play_sound(SoundEffect::Cancel);
    }
    Ok(())
}
