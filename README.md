# Henry Whisper

Tiny Tauri desktop dictation app: record from the tray, transcribe with an OpenAI-compatible audio endpoint, and copy the result straight to the clipboard.

## Run

### Dev

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install tauri-cli --locked
cargo install trunk
rustup target add wasm32-unknown-unknown
cargo tauri dev
```

### Build app

Build a release with:

```sh
cargo tauri build
```

## Use

- `Ctrl+1`: start recording, then stop and transcribe
- `Ctrl+2`: cancel recording
- Open Settings from the tray to change the endpoint, model, shortcuts, and sound cues

Default transcription config:

- Base URL: `https://lulu.gooseread.com/v1`
- Model: `CohereLabs/cohere-transcribe-03-2026`

## Notes

- The app stores settings in the app config directory as `settings.json`
- The first recording may trigger microphone permission prompts
- Transcripts are copied to the clipboard; no transcript history is stored
- If your endpoint requires auth, set the appropriate API key in the environment before launch

## Todo

- [ ] polish model
- [ ] test on window/linux

## License

Apache-2.0
