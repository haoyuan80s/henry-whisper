# Henry Whisper

Henry Whisper is a tiny desktop dictation app. It sits in the menu bar, records from your microphone, sends the audio to a single OpenAI-compatible model, applies transcript cleanup in the same request, and copies the result to your clipboard.

Press `CmdOrCtrl+1` to start recording, press it again to transcribe, then paste anywhere. Press `CmdOrCtrl+2` to cancel.

## Features

- Menu bar / system tray app
- Global shortcuts
- Single OpenAI-compatible audio model
- Automatic clipboard copy
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

Open the settings window from the tray menu to configure the model endpoint, model name, shortcuts, and sound cues.

Default:

- Model: `google/gemma-4-E4B-it` at `https://gemini.gooseread.com/v1`

Settings are saved automatically to the app config directory.

## Notes

- On macOS, Henry Whisper runs as an accessory app in the menu bar.
- The first recording may ask for microphone permission.
- Transcripts are copied to the clipboard; no transcript history is stored.

## License

Apache-2.0

# TODOS

- [ ] add polish model so that I can inject user's inputs
