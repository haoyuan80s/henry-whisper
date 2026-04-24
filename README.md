# Henry Whisper

Tray-first desktop dictation app built with Tauri.

- Record from anywhere
- Transcribe with any OpenAI-compatible API
- Copy the transcript to the clipboard automatically

## Compatible APIs

Henry Whisper works with OpenAI-compatible endpoints that support chat completions with audio input.

Example: OpenRouter

- `AI Base URL`: `https://openrouter.ai/api/v1`
- `AI Model`: `google/gemini-2.5-flash-preview`
- `AI API Key`: your OpenRouter key

## Run

```sh
cargo tauri dev
```

## Build

```sh
cargo tauri build
```

## Defaults

- Record / Transcribe: `Ctrl+1`
- Cancel: `Ctrl+2`

## Notes

- Settings are stored in `settings.json` under the app config directory.
- The app keeps no transcript history.

## License

Apache-2.0
