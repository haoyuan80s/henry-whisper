use henry_whisper_shared::AppSettings;
use leptos::reactive::spawn_local;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

async fn log_to_backend(cmd: &str, message: &str) {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "message": message,
    }))
    .unwrap();
    let _ = invoke(cmd, args).await;
}

pub async fn debug_log(message: &str) {
    log_to_backend("frontend_debug", &message).await;
}

pub async fn info_log(message: &str) {
    log_to_backend("frontend_info", &message).await;
}
pub async fn warn_log(message: &str) {
    log_to_backend("frontend_warn", &message).await;
}

pub async fn error_log(message: &str) {
    log_to_backend("frontend_error", &message).await;
}

pub async fn sync_debug_log(message: String) {
    log_to_backend("frontend_debug", &message).await;
}

pub fn sync_info_log(message: String) {
    spawn_local(async move {
        log_to_backend("frontend_info", &message).await;
    });
}
pub fn sync_warn_log(message: String) {
    spawn_local(async move {
        log_to_backend("frontend_warn", &message).await;
    });
}
