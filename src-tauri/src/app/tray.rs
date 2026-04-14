pub const DEFAULT_TRAY_TITLE: &str = "Henry Whisper";

pub fn set_tray_title(app: &tauri::AppHandle, title: Option<&str>) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(Some(title.unwrap_or(DEFAULT_TRAY_TITLE)));
    }
}
