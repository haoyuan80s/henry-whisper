use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = "listen")]
    async fn tauri_listen(event: &str, handler: &js_sys::Function) -> JsValue;
}

#[derive(Clone, PartialEq)]
enum Status {
    Idle,
    Recording,
    Transcribing,
}

impl Status {
    fn label(&self) -> &'static str {
        match self {
            Status::Idle => "Ready",
            Status::Recording => "Recording…",
            Status::Transcribing => "Transcribing…",
        }
    }
}

fn listen_menu(event: &'static str, cb: impl Fn() + 'static) {
    spawn_local(async move {
        let closure = Closure::wrap(Box::new(move |_: JsValue| {
            cb();
        }) as Box<dyn Fn(JsValue)>);
        tauri_listen(event, closure.as_ref().unchecked_ref()).await;
        closure.forget();
    });
}

#[component]
pub fn App() -> impl IntoView {
    let (status, set_status) = signal(Status::Idle);
    let (transcript, set_transcript) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (settings_open, set_settings_open) = signal(false);
    let (api_key_draft, set_api_key_draft) = signal(String::new());

    // Load saved API key
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(val) = invoke("get_api_key", JsValue::NULL).await {
                if let Some(s) = val.as_string() {
                    set_api_key_draft.set(s);
                }
            }
        });
    });

    // ── shared action helpers ──────────────────────────────────────────────
    let do_start = move || {
        if status.get_untracked() != Status::Idle {
            return;
        }
        set_error.set(None);
        set_status.set(Status::Recording);
        spawn_local(async move {
            if let Err(e) = invoke("start_recording", JsValue::NULL).await {
                set_status.set(Status::Idle);
                set_error.set(Some(js_err_string(e)));
            }
        });
    };

    let do_stop = move || {
        if status.get_untracked() != Status::Recording {
            return;
        }
        set_status.set(Status::Transcribing);
        spawn_local(async move {
            match invoke("stop_and_transcribe", JsValue::NULL).await {
                Ok(val) => {
                    set_status.set(Status::Idle);
                    set_transcript.set(val.as_string().unwrap_or_default());
                }
                Err(e) => {
                    set_status.set(Status::Idle);
                    set_error.set(Some(js_err_string(e)));
                }
            }
        });
    };

    let do_copy = move || {
        let text = transcript.get_untracked();
        if text.is_empty() {
            return;
        }
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "text": text })).unwrap();
            let _ = invoke("copy_to_clipboard", args).await;
        });
    };

    let do_clear = move || {
        set_transcript.set(String::new());
        set_error.set(None);
    };

    // ── menu event listeners ───────────────────────────────────────────────
    Effect::new(move |_| {
        listen_menu("record_start", do_start);
        listen_menu("record_stop", do_stop);
        listen_menu("copy_transcript", do_copy);
        listen_menu("clear_transcript", do_clear);
        listen_menu("settings", move || set_settings_open.set(true));
    });

    // ── record button handler ──────────────────────────────────────────────
    let on_record_click = move |_| match status.get_untracked() {
        Status::Idle => do_start(),
        Status::Recording => do_stop(),
        Status::Transcribing => {}
    };

    // ── save API key ───────────────────────────────────────────────────────
    let save_api_key = move |_| {
        let key = api_key_draft.get_untracked();
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "key": key })).unwrap();
            let _ = invoke("set_api_key", args).await;
        });
        set_settings_open.set(false);
    };

    view! {
        <div class="app">
            // ── header ────────────────────────────────────────────────────
            <header class="app-header">
                <span class="app-title">"Henry Whisper"</span>
                <button
                    class="icon-btn"
                    title="Settings"
                    on:click=move |_| set_settings_open.update(|v| *v = !*v)
                >
                    "⚙"
                </button>
            </header>

            // ── settings panel ────────────────────────────────────────────
            <div class=move || if settings_open.get() { "settings-panel open" } else { "settings-panel" }>
                <label class="settings-label">"OpenRouter API Key"</label>
                <div class="settings-row">
                    <input
                        type="password"
                        class="api-input"
                        placeholder="sk-or-..."
                        prop:value=move || api_key_draft.get()
                        on:input=move |ev| set_api_key_draft.set(event_target_value(&ev))
                    />
                    <button class="save-btn" on:click=save_api_key>"Save"</button>
                </div>
            </div>

            // ── main area ─────────────────────────────────────────────────
            <main class="main">
                // Record button
                <div class="record-area">
                    <button
                        class=move || {
                            let base = "record-btn";
                            match status.get() {
                                Status::Recording => format!("{base} recording"),
                                Status::Transcribing => format!("{base} busy"),
                                Status::Idle => base.to_string(),
                            }
                        }
                        on:click=on_record_click
                        disabled=move || status.get() == Status::Transcribing
                    >
                        {move || match status.get() {
                            Status::Recording => view! { <span class="btn-icon">"⏹"</span> }.into_any(),
                            Status::Transcribing => view! { <span class="btn-icon spinner">"◌"</span> }.into_any(),
                            Status::Idle => view! { <span class="btn-icon">"🎙"</span> }.into_any(),
                        }}
                    </button>
                    <p class=move || {
                        let base = "status-label";
                        if status.get() == Status::Recording {
                            format!("{base} recording")
                        } else {
                            base.to_string()
                        }
                    }>
                        {move || status.get().label()}
                    </p>
                </div>

                // Error
                {move || error.get().map(|e| view! {
                    <div class="error-banner">
                        <span>{e}</span>
                        <button class="dismiss-btn" on:click=move |_| set_error.set(None)>"✕"</button>
                    </div>
                })}

                // Transcript
                <div class="transcript-area">
                    <div class="transcript-header">
                        <span class="transcript-title">"Transcript"</span>
                        <div class="transcript-actions">
                            <button
                                class="action-btn"
                                disabled=move || transcript.get().is_empty()
                                on:click=move |_| do_copy()
                                title="Copy to clipboard"
                            >
                                "Copy"
                            </button>
                            <button
                                class="action-btn"
                                disabled=move || transcript.get().is_empty()
                                on:click=move |_| do_clear()
                                title="Clear transcript"
                            >
                                "Clear"
                            </button>
                        </div>
                    </div>
                    <div class="transcript-box">
                        {move || {
                            let t = transcript.get();
                            if t.is_empty() {
                                view! { <p class="transcript-empty">"Transcript will appear here…"</p> }.into_any()
                            } else {
                                view! { <p class="transcript-text">{t}</p> }.into_any()
                            }
                        }}
                    </div>
                </div>
            </main>
        </div>
    }
}

fn js_err_string(e: JsValue) -> String {
    if let Some(s) = e.as_string() {
        return s;
    }
    if let Ok(s) = js_sys::JSON::stringify(&e) {
        if let Some(s) = s.as_string() {
            return s;
        }
    }
    "Unknown error".to_string()
}
