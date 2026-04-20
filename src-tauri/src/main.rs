// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // Work around WebKitGTK blank windows on systems where GBM/KMS buffer
        // creation is denied, e.g. "DRM_IOCTL_MODE_CREATE_DUMB failed".
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    henry_whisper_lib::app::run()
}
