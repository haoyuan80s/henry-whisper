pub struct TrayTitleGuard<'a> {
    app: &'a tauri::AppHandle,
}

impl<'a> TrayTitleGuard<'a> {
    pub fn new(app: &'a tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl<'a> Drop for TrayTitleGuard<'a> {
    fn drop(&mut self) {
        println!("Dropping TrayTitleGuard, resetting tray title");
        set_tray_title(self.app, Some(""));
    }
}

pub fn set_tray_title(app: &tauri::AppHandle, title: Option<&str>) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(title);
    }
}
