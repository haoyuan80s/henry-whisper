# Henry Whisper

Tiny Tauri desktop dictation app: record from the tray, transcribe with an OpenAI-compatible audio endpoint, and copy the result straight to the clipboard.

## Run

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install tauri-cli --locked
cargo tauri dev
```

Build a release with:

```sh
cargo tauri build
```

## Use

- `CmdOrCtrl+1`: start recording, then stop and transcribe
- `CmdOrCtrl+2`: cancel recording
- Open Settings from the tray to change the endpoint, model, shortcuts, and sound cues

Default transcription config:

- Base URL: `https://lulu.gooseread.com/v1`
- Model: `CohereLabs/cohere-transcribe-03-2026`

## Notes

- The app stores settings in the app config directory as `settings.json`
- The first recording may trigger microphone permission prompts
- Transcripts are copied to the clipboard; no transcript history is stored
- If your endpoint requires auth, set the appropriate API key in the environment before launch

## License

Apache-2.0
