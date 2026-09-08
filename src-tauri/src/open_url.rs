//! Opens an `https://` link in the default browser when JS emits `open-url`
//! with the URL as its payload. Only https, so a pet document cannot launch
//! arbitrary schemes through the settings window.

use std::process::Command;

use tauri::{AppHandle, Listener, Runtime};

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    app.listen("open-url", |event| {
        let Ok(url) = serde_json::from_str::<String>(event.payload()) else {
            return;
        };
        if url.starts_with("https://") {
            // ponytail: macOS `open`; swap for tauri-plugin-opener when another OS matters.
            let _ = Command::new("open").arg(&url).spawn();
        }
    });
}
