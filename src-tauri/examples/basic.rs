use std::io::Write;
use std::io::{self};
use std::sync::Arc;
use std::sync::Mutex;

use arboard::Clipboard;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY must be set");

    println!("Press Enter to start recording...");
    wait_for_enter();

    println!("Recording... Press Enter to stop.");
    io::stdout().flush()?;

    // Set up audio capture
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as u16;

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_cb = samples.clone();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _| {
            samples_cb.lock().unwrap().extend_from_slice(data);
        },
        |err| eprintln!("Stream error: {err}"),
        None,
    )?;

    stream.play()?;
    wait_for_enter();
    drop(stream);

    print!("Transcribing...");
    io::stdout().flush()?;

    // Encode captured samples to WAV
    let samples = samples.lock().unwrap().clone();
    let wav_bytes = encode_wav(&samples, sample_rate, channels)?;
    let audio_b64 = BASE64.encode(&wav_bytes);

    // Call OpenRouter
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
        .await?;

    let body: serde_json::Value = resp.json().await?;

    if let Some(err) = body.get("error") {
        eprintln!("\nAPI error: {err}");
        std::process::exit(1);
    }

    let transcript = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    println!(" done.");
    println!("Transcript: {transcript}");

    // Copy to clipboard
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(&transcript)?;

    // Notify with system sound
    let _ = std::process::Command::new("afplay")
        .arg("/System/Library/Sounds/Glass.aiff")
        .status();

    println!("Copied to clipboard.");
    Ok(())
}

fn wait_for_enter() {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
}

fn encode_wav(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Cursor;

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut writer = hound::WavWriter::new(cursor, spec)?;
        for &s in samples {
            let pcm = (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            writer.write_sample(pcm)?;
        }
        writer.finalize()?;
    }
    Ok(buf)
}
