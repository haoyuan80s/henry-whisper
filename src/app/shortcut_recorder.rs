use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::ipc::frontend_debug;

#[component]
pub fn ShortcutRecorder(
    value: ReadSignal<String>,
    set_value: WriteSignal<String>,
) -> impl IntoView {
    let (recording, set_recording) = signal(false);

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        ev.prevent_default();
        ev.stop_propagation();

        let key = ev.key();

        let key2 = key.clone();
        spawn_local(async move {
            let _ = frontend_debug(&format!("key: {:#?}", key2)).await;
        });

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
        if meta {
            parts.push("Super".to_string());
        }
        if ctrl {
            parts.push("Ctrl".to_string());
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
            k => k.to_string(),
        };
        parts.push(main_key);

        set_value.set(parts.join("+"));
        set_recording.set(false);
        if let Some(target) = ev.target() {
            if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                let _ = input.blur();
            }
        }
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
