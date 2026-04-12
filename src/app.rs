use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[derive(Clone, Serialize, Deserialize)]
struct AppSettings {
    api_key: String,
    recording_shortcut: String,
    transcribe_shortcut: String,
    cancel_shortcut: String,
    play_sound: bool,
}

#[component]
fn ShortcutRecorder(value: ReadSignal<String>, set_value: WriteSignal<String>) -> impl IntoView {
    let (recording, set_recording) = signal(false);

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        ev.prevent_default();
        ev.stop_propagation();

        let key = ev.key();

        // Ignore bare modifier key presses
        if matches!(
            key.as_str(),
            "Meta" | "Control" | "Shift" | "Alt" | "CapsLock" | "Tab"
        ) {
            return;
        }

        // Escape cancels without saving
        if key == "Escape" {
            set_recording.set(false);
            return;
        }

        let meta = ev.meta_key();
        let ctrl = ev.ctrl_key();
        let shift = ev.shift_key();
        let alt = ev.alt_key();

        // Require at least one modifier
        if !meta && !ctrl && !shift && !alt {
            return;
        }

        let mut parts: Vec<String> = Vec::new();
        if meta || ctrl {
            parts.push("CmdOrCtrl".to_string());
        }
        if shift {
            parts.push("Shift".to_string());
        }
        if alt {
            parts.push("Alt".to_string());
        }

        let main_key = match key.as_str() {
            " " => "Space".to_string(),
            k if k.len() == 1 => k.to_uppercase(),
            k => k.to_string(), // F1–F12, ArrowUp, etc.
        };
        parts.push(main_key);

        set_value.set(parts.join("+"));
        set_recording.set(false);
    };

    view! {
        <input
            class="input shortcut-recorder"
            class:is-recording=move || recording.get()
            type="text"
            readonly=true
            prop:value=move || {
                if recording.get() {
                    "Press shortcut…".to_string()
                } else {
                    let v = value.get();
                    if v.is_empty() { "Click to record…".to_string() } else { v }
                }
            }
            on:focus=move |_| set_recording.set(true)
            on:keydown=move |ev| {
                if recording.get_untracked() {
                    handle_keydown(ev);
                }
            }
            on:blur=move |_| set_recording.set(false)
        />
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (api_key, set_api_key) = signal(String::new());
    let (rec_shortcut, set_rec_shortcut) = signal(String::new());
    let (tx_shortcut, set_tx_shortcut) = signal(String::new());
    let (cancel_shortcut, set_cancel_shortcut) = signal(String::new());
    let (play_sound, set_play_sound) = signal(true);
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(None::<String>);

    // Load settings on mount
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(val) = invoke("get_settings", JsValue::NULL).await {
                if let Ok(s) = serde_wasm_bindgen::from_value::<AppSettings>(val) {
                    set_api_key.set(s.api_key);
                    set_rec_shortcut.set(s.recording_shortcut);
                    set_tx_shortcut.set(s.transcribe_shortcut);
                    set_cancel_shortcut.set(s.cancel_shortcut);
                    set_play_sound.set(s.play_sound);
                }
            }
        });
    });

    let save = move |_| {
        set_error.set(None);
        set_saving.set(true);
        let s = AppSettings {
            api_key: api_key.get_untracked(),
            recording_shortcut: rec_shortcut.get_untracked(),
            transcribe_shortcut: tx_shortcut.get_untracked(),
            cancel_shortcut: cancel_shortcut.get_untracked(),
            play_sound: play_sound.get_untracked(),
        };
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "settings": s })).unwrap();
            match invoke("save_settings", args).await {
                Ok(_) => {
                    let _ = invoke("hide_settings_window", JsValue::NULL).await;
                }
                Err(e) => {
                    set_error.set(Some(
                        e.as_string().unwrap_or("Error saving settings".into()),
                    ));
                }
            }
            set_saving.set(false);
        });
    };

    let cancel = move |_| {
        spawn_local(async move {
            let _ = invoke("hide_settings_window", JsValue::NULL).await;
        });
    };

    view! {
        <div class="settings">
            <h1 class="settings-title">"Henry Whisper"</h1>

            <div class="field">
                <label class="label">"Gemini API Key"</label>
                <input
                    class="input"
                    type="password"
                    placeholder="AIza..."
                    prop:value=move || api_key.get()
                    on:input=move |ev| set_api_key.set(event_target_value(&ev))
                />
            </div>

            <div class="field">
                <label class="label">"Recording Shortcut"</label>
                <ShortcutRecorder value=rec_shortcut set_value=set_rec_shortcut />
            </div>

            <div class="field">
                <label class="label">"Transcribe Shortcut"</label>
                <ShortcutRecorder value=tx_shortcut set_value=set_tx_shortcut />
            </div>

            <div class="field">
                <label class="label">"Cancel Shortcut"</label>
                <ShortcutRecorder value=cancel_shortcut set_value=set_cancel_shortcut />
            </div>

            <div class="field toggle-field">
                <label class="label">"Play sound after transcription"</label>
                <label class="toggle">
                    <input
                        type="checkbox"
                        prop:checked=move || play_sound.get()
                        on:change=move |ev| set_play_sound.set(event_target_checked(&ev))
                    />
                    <span class="toggle-track"></span>
                </label>
            </div>

            {move || error.get().map(|e| view! {
                <div class="error">{e}</div>
            })}

            <div class="actions">
                <button class="btn-cancel" on:click=cancel>"Cancel"</button>
                <button
                    class="btn-save"
                    on:click=save
                    disabled=move || saving.get()
                >
                    {move || if saving.get() { "Saving…" } else { "Save" }}
                </button>
            </div>
        </div>
    }
}
