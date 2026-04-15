use anyhow::Result;
use std::io::Cursor;
use std::sync::mpsc;
use std::thread;

const TRANSCRIPTION_SAMPLE_RATE: u32 = 16_000;
const TRANSCRIPTION_CHANNELS: u16 = 1;

pub enum SoundEffect {
    Record,
    TranscribeStart,
    Transcribe,
    Cancel,
}

impl SoundEffect {
    fn bytes(&self) -> &'static [u8] {
        match self {
            SoundEffect::Record => include_bytes!("../assets/record.wav"),
            SoundEffect::TranscribeStart => include_bytes!("../assets/transcribe_start.wav"),
            SoundEffect::Transcribe => include_bytes!("../assets/transcribe.wav"),
            SoundEffect::Cancel => include_bytes!("../assets/cancel.wav"),
        }
    }
}

pub struct AudioPlayer {
    tx: mpsc::Sender<SoundEffect>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<SoundEffect>();
        thread::spawn(move || {
            let mut device_sink = match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[AudioPlayer] open_default_sink failed: {e}");
                    return;
                }
            };
            device_sink.log_on_drop(false);
            for effect in rx {
                let cursor = Cursor::new(effect.bytes());
                match rodio::play(device_sink.mixer(), cursor) {
                    Ok(player) => player.sleep_until_end(),
                    Err(e) => eprintln!("[AudioPlayer] play failed: {e}"),
                }
            }
        });
        Self { tx }
    }

    pub fn play(&self, effect: SoundEffect) {
        let _ = self.tx.send(effect);
    }
}

pub fn encode_transcription_wav(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>> {
    let mono = downmix_to_mono(samples, channels);
    let resampled = resample_linear(&mono, sample_rate, TRANSCRIPTION_SAMPLE_RATE);
    encode_wav(
        &resampled,
        TRANSCRIPTION_SAMPLE_RATE,
        TRANSCRIPTION_CHANNELS,
    )
}

pub fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>> {
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

fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let channels = usize::from(channels.max(1));
    if channels == 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn resample_linear(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == 0 || source_rate == target_rate {
        return samples.to_vec();
    }

    let output_len =
        (samples.len() as u64 * target_rate as u64).div_ceil(source_rate as u64) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let source_pos = i as f64 * source_rate as f64 / target_rate as f64;
        let index = source_pos.floor() as usize;
        let fraction = (source_pos - index as f64) as f32;
        let current = samples[index.min(samples.len() - 1)];
        let next = samples[(index + 1).min(samples.len() - 1)];
        output.push(current + (next - current) * fraction);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn downmixes_interleaved_stereo_to_mono() {
        let stereo = [1.0, 0.0, 0.25, 0.75, -0.5, 0.5];

        assert_eq!(downmix_to_mono(&stereo, 2), vec![0.5, 0.5, 0.0]);
    }

    #[test]
    fn resamples_to_target_rate() {
        let samples = vec![0.0; 48_000];

        let resampled = resample_linear(&samples, 48_000, TRANSCRIPTION_SAMPLE_RATE);

        assert_eq!(resampled.len(), TRANSCRIPTION_SAMPLE_RATE as usize);
    }

    #[test]
    fn encodes_transcription_wav_as_16khz_mono_i16() {
        let stereo = vec![0.25; 48_000 * 2];

        let wav = encode_transcription_wav(&stereo, 48_000, 2).unwrap();
        let reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
        let spec = reader.spec();

        assert_eq!(spec.channels, TRANSCRIPTION_CHANNELS);
        assert_eq!(spec.sample_rate, TRANSCRIPTION_SAMPLE_RATE);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);
        assert_eq!(reader.duration(), TRANSCRIPTION_SAMPLE_RATE);
    }
}
