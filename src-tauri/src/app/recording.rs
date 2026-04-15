use anyhow::Result;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::Manager;

use super::state::AppState;
use super::state::RecordingHandle;
use super::tray::set_tray_title;
use crate::audio::SoundEffect;
use crate::audio::encode_transcription_mp3;
use crate::audio::play_sound;

const VOICE_THRESHOLD: f32 = 0.0001;
/// How long to wait for the first voice chunk before starting anyway.
const WARMUP_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait for the audio thread to exit after stopping.
const STREAM_STOP_TIMEOUT: Duration = Duration::from_secs(3);
/// How long to wait for the transcription API.
const TRANSCRIPTION_TIMEOUT: Duration = Duration::from_secs(60);
/// How long to wait for the polish API.
const POLISH_TIMEOUT: Duration = Duration::from_secs(60);

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

    // Shared flag so we can force-start after the warmup timeout.
    let warmed_up = Arc::new(AtomicBool::new(false));
    let warmed_up_cb = warmed_up.clone();

    let join_handle = tokio::task::spawn_blocking(move || {
        let ready_tx = Mutex::new(Some(ready_tx));

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if !warmed_up_cb.load(Ordering::Relaxed) {
                        if rms(data) < VOICE_THRESHOLD {
                            return;
                        }
                        warmed_up_cb.store(true, Ordering::Relaxed);
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

    match tokio::time::timeout(WARMUP_TIMEOUT, ready_rx).await {
        Ok(Ok(Ok(()))) => {}             // got voice, normal path
        Ok(Ok(Err(e))) => return Err(e), // stream setup failed
        Ok(Err(_)) => return Err(anyhow::anyhow!("Audio stream thread exited unexpectedly")),
        Err(_) => {
            // Timeout: no voice detected yet — force the warmup flag so the
            // callback starts forwarding samples from now on and proceed.
            tracing::warn!("No voice detected within {WARMUP_TIMEOUT:?}, starting anyway");
            warmed_up.store(true, Ordering::Relaxed);
        }
    }

    *state.recording.lock().unwrap() = Some(RecordingHandle {
        sample_rx,
        stop_tx,
        join_handle,
        sample_rate,
        channels,
    });

    let settings = state.settings.lock().expect("lock settings").clone();

    if settings.play_sound {
        play_sound(SoundEffect::Record);
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
    let _ = tokio::time::timeout(STREAM_STOP_TIMEOUT, handle.join_handle).await;

    let settings = state.settings.lock().unwrap().clone();
    if settings.play_sound {
        play_sound(SoundEffect::TranscribeStart);
    }

    let samples: Vec<f32> = handle.sample_rx.try_iter().flatten().collect();
    if samples.is_empty() {
        set_tray_title(&app, None);
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

    // if std::env::var("HENRY_WHISPER_DEBUG_AUDIO").as_deref() == Ok("1") {
    //     let dir = std::path::Path::new("/tmp/henry-whisper");
    //     std::fs::create_dir_all(dir).ok();
    //     let ts = std::time::SystemTime::now()
    //         .duration_since(std::time::UNIX_EPOCH)
    //         .map(|d| d.as_secs())
    //         .unwrap_or(0);
    //     let path = dir.join(format!("{ts}.mp3"));
    //     std::fs::write(&path, &mp3_bytes).ok();
    //     debug!(path = %path.display(), "saved debug audio");
    // }

    tracing::debug!(
        "Transcribing MP3 audio with {} input samples at {} Hz across {} channel(s)",
        samples.len(),
        handle.sample_rate,
        handle.channels
    );
    let now = std::time::Instant::now();
    let transcription_model = state.transcription_model.lock().unwrap().clone();
    let transcript = match tokio::time::timeout(
        TRANSCRIPTION_TIMEOUT,
        transcription_model.transcribe_mp3(mp3_bytes),
    )
    .await
    {
        Ok(Ok(t)) => t,
        Ok(Err(err)) => {
            set_tray_title(&app, None);
            return Err(err);
        }
        Err(_) => {
            set_tray_title(&app, None);
            return Err(anyhow::anyhow!("Transcription timed out"));
        }
    };
    let elapsed = now.elapsed();
    tracing::debug!("Transcription completed in {:.2?}: {transcript}", elapsed);

    let mut polished_transcript = None;
    if state.settings.lock().unwrap().polish {
        tracing::debug!("Polishing transcript: {transcript}");
        let polish_model = state.polish_model.lock().unwrap().clone();
        polished_transcript = Some(
            tokio::time::timeout(
                POLISH_TIMEOUT,
                polish_model.chat(include_str!("./polish.md"), &transcript),
            )
            .await
            .map_err(|_| anyhow::anyhow!("Polish timed out"))??,
        );
        let elapsed_total = now.elapsed();
        tracing::debug!(
            "Polishing completed in {:.2?}, total time: {:.2?}",
            elapsed_total - elapsed,
            elapsed_total
        );
    };

    state
        .clipboard
        .lock()
        .expect("lock clipboard")
        .set_text(polished_transcript.unwrap_or(transcript))?;

    if settings.play_sound {
        play_sound(SoundEffect::Transcribe);
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
    let _ = tokio::time::timeout(STREAM_STOP_TIMEOUT, handle.join_handle).await;
    let settings = state.settings.lock().unwrap().clone();
    if settings.play_sound {
        play_sound(SoundEffect::Cancel);
    }
    Ok(())
}
