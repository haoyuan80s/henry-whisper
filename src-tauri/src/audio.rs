use std::thread;

pub static NOTIFICATION_SOUND: &[u8] = include_bytes!("../resources/notification.wav");

pub fn play_sound() {
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

pub fn encode_wav(
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
