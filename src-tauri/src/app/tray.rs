pub fn set_tray_title(app: &tauri::AppHandle, title: Option<&str>) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(title);
    }
}
