use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
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
    play_sound: bool,
    show_notification: bool,
}

#[component]
pub fn App() -> impl IntoView {
    let (api_key, set_api_key) = signal(String::new());
    let (rec_shortcut, set_rec_shortcut) = signal(String::new());
    let (tx_shortcut, set_tx_shortcut) = signal(String::new());
    let (play_sound, set_play_sound) = signal(true);
    let (show_notif, set_show_notif) = signal(true);
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
                    set_play_sound.set(s.play_sound);
                    set_show_notif.set(s.show_notification);
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
            play_sound: play_sound.get_untracked(),
            show_notification: show_notif.get_untracked(),
        };
        spawn_local(async move {
            let args =
                serde_wasm_bindgen::to_value(&serde_json::json!({ "settings": s })).unwrap();
            match invoke("save_settings", args).await {
                Ok(_) => {}
                Err(e) => {
                    set_error
                        .set(Some(e.as_string().unwrap_or("Error saving settings".into())));
                    set_saving.set(false);
                }
            }
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
                <label class="label">"OpenRouter API Key"</label>
                <input
                    class="input"
                    type="password"
                    placeholder="sk-or-..."
                    prop:value=move || api_key.get()
                    on:input=move |ev| set_api_key.set(event_target_value(&ev))
                />
            </div>

            <div class="field">
                <label class="label">"Recording Shortcut"</label>
                <input
                    class="input"
                    type="text"
                    placeholder="CmdOrCtrl+Shift+R"
                    prop:value=move || rec_shortcut.get()
                    on:input=move |ev| set_rec_shortcut.set(event_target_value(&ev))
                />
            </div>

            <div class="field">
                <label class="label">"Transcribe Shortcut"</label>
                <input
                    class="input"
                    type="text"
                    placeholder="CmdOrCtrl+Shift+T"
                    prop:value=move || tx_shortcut.get()
                    on:input=move |ev| set_tx_shortcut.set(event_target_value(&ev))
                />
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

            <div class="field toggle-field">
                <label class="label">"Show notification after transcription"</label>
                <label class="toggle">
                    <input
                        type="checkbox"
                        prop:checked=move || show_notif.get()
                        on:change=move |ev| set_show_notif.set(event_target_checked(&ev))
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
