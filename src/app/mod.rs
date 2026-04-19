mod settings;
mod shortcut_recorder;

use henry_whisper_shared::AppSettings;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::time::Duration;

use crate::app::settings::{current_settings, persist_settings};
use crate::app::shortcut_recorder::ShortcutRecorder;
use crate::ipc::{
    frontend_debug, frontend_info, frontend_warn, get_settings, hide_settings_window,
};

#[component]
pub fn App() -> impl IntoView {
    let (model_base_url, set_model_base_url) = signal(String::new());
    let (model_name, set_model_name) = signal(String::new());
    let (rec_shortcut, set_rec_shortcut) = signal(String::new());
    let (cancel_shortcut, set_cancel_shortcut) = signal(String::new());
    let (play_sound, set_play_sound) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (loaded, set_loaded) = signal(false);
    let (last_saved, set_last_saved) = signal(None::<AppSettings>);
    let (save_request_id, set_save_request_id) = signal(0_u64);

    Effect::new(move |_| {
        spawn_local(async move {
            let _ = frontend_debug("requesting settings").await;
            match get_settings().await {
                Ok(s) => {
                    let _ = frontend_info("settings loaded").await;
                    set_last_saved.set(Some(s.clone()));
                    set_model_base_url.set(s.transcription_model.base_url);
                    set_model_name.set(s.transcription_model.model);
                    set_rec_shortcut.set(s.shortcut.recording);
                    set_cancel_shortcut.set(s.shortcut.cancel);
                    set_play_sound.set(s.play_sound);
                    set_loaded.set(true);
                }
                Err(_) => {
                    let _ = frontend_warn("get_settings invoke failed").await;
                }
            }
        });
    });

    let mut schedule_save = leptos::leptos_dom::helpers::debounce(
        Duration::from_millis(550),
        move |settings: AppSettings| {
            set_error.set(None);
            let request_id = save_request_id.get_untracked() + 1;
            set_save_request_id.set(request_id);

            persist_settings(
                settings,
                request_id,
                save_request_id,
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
            model_base_url,
            model_name,
            rec_shortcut,
            cancel_shortcut,
            play_sound,
        );

        if last_saved.get().as_ref() == Some(&settings) {
            return;
        }

        schedule_save(settings);
    });

    let close = move |_| {
        let settings = current_settings(
            model_base_url,
            model_name,
            rec_shortcut,
            cancel_shortcut,
            play_sound,
        );

        if loaded.get_untracked() && last_saved.get_untracked().as_ref() != Some(&settings) {
            set_error.set(None);
            let request_id = save_request_id.get_untracked() + 1;
            set_save_request_id.set(request_id);
            persist_settings(
                settings,
                request_id,
                save_request_id,
                set_error,
                set_last_saved,
                || {
                    spawn_local(async move {
                        let _ = hide_settings_window().await;
                    });
                },
            );
        } else {
            spawn_local(async move {
                let _ = hide_settings_window().await;
            });
        }
    };

    view! {
        <div class="settings">
            <h1 class="settings-title">"Henry Whisper"</h1>

            <div class="field">
                <label class="label">"Model Base URL"</label>
                <input
                    class="input"
                    type="text"
                    placeholder="https://gemini.gooseread.com/v1"
                    prop:value=move || model_base_url.get()
                    on:input=move |ev| set_model_base_url.set(event_target_value(&ev))
                />
            </div>

            <div class="field">
                <label class="label">"Model"</label>
                <input
                    class="input"
                    type="text"
                    placeholder="google/gemma-4-E4B-it"
                    prop:value=move || model_name.get()
                    on:input=move |ev| set_model_name.set(event_target_value(&ev))
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

            {move || error.get().map(|e| view! {
                <div class="error">{e}</div>
            })}

            <div class="actions">
                <button class="btn-save" on:click=close>"Done"</button>
            </div>
        </div>
    }
}
