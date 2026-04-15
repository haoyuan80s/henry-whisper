# Henry Whisper

Henry Whisper is a tiny desktop dictation app. It sits in the menu bar, records from your microphone, sends the audio to an OpenAI-compatible transcription endpoint, optionally polishes the text, and copies the result to your clipboard.

Press `CmdOrCtrl+1` to start recording, press it again to transcribe, then paste anywhere. Press `CmdOrCtrl+2` to cancel.

## Features

- Menu bar / system tray app
- Global shortcuts
- OpenAI-compatible transcription and polishing endpoints
- Automatic clipboard copy
- Optional transcript polishing
- Optional sound cues
- Auto-saved settings

## Building

Install the Rust WASM target, Trunk, and the Tauri CLI:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install tauri-cli --locked
```

Run the app:

```sh
cargo tauri dev
```

Build a release:

```sh
cargo tauri build
```

## Settings

Open the settings window from the tray menu to configure model endpoints, model names, shortcuts, sound cues, and transcript polishing.

Defaults:

- Transcription: `Qwen/Qwen3-ASR-0.6B` at `https://qwen.gooseread.com/v1`
- Polish: `google/gemma-4-E4B-it` at `https://gemini.gooseread.com/v1`

Settings are saved automatically to the app config directory.

## Notes

- On macOS, Henry Whisper runs as an accessory app in the menu bar.
- The first recording may ask for microphone permission.
- Transcripts are copied to the clipboard; no transcript history is stored.

## License

Apache-2.0
