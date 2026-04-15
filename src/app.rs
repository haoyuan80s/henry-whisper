use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct AppSettings {
    transcription_model: AiModelSetting,
    polish_model: AiModelSetting,
    shortcut: ShortcutSetting,
    polish: bool,
    play_sound: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct AiModelSetting {
    base_url: String,
    model: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct ShortcutSetting {
    recording: String,
    cancel: String,
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

fn current_settings(
    transcription_base_url: ReadSignal<String>,
    transcription_model: ReadSignal<String>,
    polish_base_url: ReadSignal<String>,
    polish_model: ReadSignal<String>,
    rec_shortcut: ReadSignal<String>,
    cancel_shortcut: ReadSignal<String>,
    polish: ReadSignal<bool>,
    play_sound: ReadSignal<bool>,
) -> AppSettings {
    AppSettings {
        transcription_model: AiModelSetting {
            base_url: transcription_base_url.get(),
            model: transcription_model.get(),
        },
        polish_model: AiModelSetting {
            base_url: polish_base_url.get(),
            model: polish_model.get(),
        },
        shortcut: ShortcutSetting {
            recording: rec_shortcut.get(),
            cancel: cancel_shortcut.get(),
        },
        polish: polish.get(),
        play_sound: play_sound.get(),
    }
}

fn persist_settings(
    settings: AppSettings,
    request_id: u64,
    save_request_id: ReadSignal<u64>,
    set_saving: WriteSignal<bool>,
    set_error: WriteSignal<Option<String>>,
    set_last_saved: WriteSignal<Option<AppSettings>>,
    on_saved: impl FnOnce() + 'static,
) {
    spawn_local(async move {
        let args =
            serde_wasm_bindgen::to_value(&serde_json::json!({ "settings": settings })).unwrap();
        match invoke("save_settings", args).await {
            Ok(_) => {
                if save_request_id.get_untracked() == request_id {
                    set_last_saved.set(Some(settings));
                    on_saved();
                }
            }
            Err(e) => {
                if save_request_id.get_untracked() == request_id {
                    set_error.set(Some(
                        e.as_string().unwrap_or("Error saving settings".into()),
                    ));
                }
            }
        }

        if save_request_id.get_untracked() == request_id {
            set_saving.set(false);
        }
    });
}

#[component]
pub fn App() -> impl IntoView {
    let (transcription_base_url, set_transcription_base_url) = signal(String::new());
    let (transcription_model, set_transcription_model) = signal(String::new());
    let (polish_base_url, set_polish_base_url) = signal(String::new());
    let (polish_model, set_polish_model) = signal(String::new());
    let (rec_shortcut, set_rec_shortcut) = signal(String::new());
    let (cancel_shortcut, set_cancel_shortcut) = signal(String::new());
    let (polish, set_polish) = signal(true);
    let (play_sound, set_play_sound) = signal(true);
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (loaded, set_loaded) = signal(false);
    let (last_saved, set_last_saved) = signal(None::<AppSettings>);
    let (save_request_id, set_save_request_id) = signal(0_u64);

    // Load settings on mount
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(val) = invoke("get_settings", JsValue::NULL).await {
                if let Ok(s) = serde_wasm_bindgen::from_value::<AppSettings>(val) {
                    set_last_saved.set(Some(s.clone()));
                    set_transcription_base_url.set(s.transcription_model.base_url);
                    set_transcription_model.set(s.transcription_model.model);
                    set_polish_base_url.set(s.polish_model.base_url);
                    set_polish_model.set(s.polish_model.model);
                    set_rec_shortcut.set(s.shortcut.recording);
                    set_cancel_shortcut.set(s.shortcut.cancel);
                    set_polish.set(s.polish);
                    set_play_sound.set(s.play_sound);
                    set_loaded.set(true);
                }
            }
        });
    });

    let mut schedule_save = leptos::leptos_dom::helpers::debounce(
        Duration::from_millis(550),
        move |settings: AppSettings| {
            set_error.set(None);
            set_saving.set(true);
            let request_id = save_request_id.get_untracked() + 1;
            set_save_request_id.set(request_id);

            persist_settings(
                settings,
                request_id,
                save_request_id,
                set_saving,
                set_error,
                set_last_saved,
                || {},
            );
        },
    );

    Effect::new(move |_| {
        if !loaded.get() {
            return;
        }

        let settings = current_settings(
            transcription_base_url,
            transcription_model,
            polish_base_url,
            polish_model,
            rec_shortcut,
            cancel_shortcut,
            polish,
            play_sound,
        );

        if last_saved.get().as_ref() == Some(&settings) {
            return;
        }

        schedule_save(settings);
    });

    let close = move |_| {
        let settings = current_settings(
            transcription_base_url,
            transcription_model,
            polish_base_url,
            polish_model,
            rec_shortcut,
            cancel_shortcut,
            polish,
            play_sound,
        );

        if loaded.get_untracked() && last_saved.get_untracked().as_ref() != Some(&settings) {
            set_error.set(None);
            set_saving.set(true);
            let request_id = save_request_id.get_untracked() + 1;
            set_save_request_id.set(request_id);
            persist_settings(
                settings,
                request_id,
                save_request_id,
                set_saving,
                set_error,
                set_last_saved,
                || {
                    spawn_local(async move {
                        let _ = invoke("hide_settings_window", JsValue::NULL).await;
                    });
                },
            );
        } else {
            spawn_local(async move {
                let _ = invoke("hide_settings_window", JsValue::NULL).await;
            });
        }
    };

    view! {
        <div class="settings">
            <h1 class="settings-title">"Henry Whisper"</h1>

            <div class="field">
                <label class="label">"Transcription Base URL"</label>
                <input
                    class="input"
                    type="text"
                    placeholder="http://192.168.86.29:8001/v1"
                    prop:value=move || transcription_base_url.get()
                    on:input=move |ev| set_transcription_base_url.set(event_target_value(&ev))
                />
            </div>

            <div class="field">
                <label class="label">"Transcription Model"</label>
                <input
                    class="input"
                    type="text"
                    placeholder="Qwen/Qwen3-ASR-0.6B"
                    prop:value=move || transcription_model.get()
                    on:input=move |ev| set_transcription_model.set(event_target_value(&ev))
                />
            </div>


            <div class="field">
                <label class="label">"Polish Base URL"</label>
                <input
                    class="input"
                    type="text"
                    placeholder="http://192.168.86.29:8000/v1"
                    prop:value=move || polish_base_url.get()
                    on:input=move |ev| set_polish_base_url.set(event_target_value(&ev))
                />
            </div>

            <div class="field">
                <label class="label">"Polish Model"</label>
                <input
                    class="input"
                    type="text"
                    placeholder="google/gemma-4-E4B-it"
                    prop:value=move || polish_model.get()
                    on:input=move |ev| set_polish_model.set(event_target_value(&ev))
                />
            </div>

            <div class="field">
                <label class="label">"Record / Transcribe Shortcut"</label>
                <ShortcutRecorder value=rec_shortcut set_value=set_rec_shortcut />
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

            <div class="field toggle-field">
                <label class="label">"Polish transcript"</label>
                <label class="toggle">
                    <input
                        type="checkbox"
                        prop:checked=move || polish.get()
                        on:change=move |ev| set_polish.set(event_target_checked(&ev))
                    />
                    <span class="toggle-track"></span>
                </label>
            </div>

            {move || error.get().map(|e| view! {
                <div class="error">{e}</div>
            })}

            <div class="actions">
                <div class="save-status">
                    {move || {
                        if saving.get() {
                            "Saving..."
                        } else if error.get().is_some() {
                            "Could not save"
                        } else {
                            "Saved"
                        }
                    }}
                </div>
                <button class="btn-save" on:click=close>"Done"</button>
            </div>
        </div>
    }
}
