use leptos::reactive::spawn_local;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

include!("ipc_generated.rs");

pub fn sync_info_log(message: String) {
    spawn_local(async move {
        let _ = frontend_info(&message).await;
    });
}

pub fn sync_warn_log(message: String) {
    spawn_local(async move {
        let _ = frontend_warn(&message).await;
    });
}
