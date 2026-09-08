//! Lock-in mode: the frontmost app when it starts is the anchor. Any other
//! app held for `dwell_ms` is a distraction; returning to the anchor ends it.
//! Driven over Tauri events so the TauRPC handler stays untouched:
//!   JS emits `lockin:start` `{ "dwellMs": n }` and `lockin:stop`;
//!   Rust emits `lockin` `{ state: "locked" | "focused" | "distracted", app }`:
//!   `locked` once with the anchor, then one per transition.
//!   JS emits `lockin:refocus` `{ "app": name }` when the animation finishes.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Listener, Runtime};

const POLL: Duration = Duration::from_millis(500);
/// OS chrome that neither starts nor resets the dwell clock.
const NEUTRAL: &[&str] = &["bage-rate", "Finder", "Spotlight", "System Settings"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Start {
    dwell_ms: u64,
}

#[derive(Deserialize)]
struct Refocus {
    app: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Transition {
    state: &'static str,
    app: String,
}

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    if cfg!(debug_assertions) {
        app.listen("lockin:log", |event| {
            eprintln!("[lockin] {}", event.payload())
        });
    }
    // Each start bumps the generation; the running thread exits when it sees a newer one.
    let generation = Arc::new(AtomicU64::new(0));
    let stop = generation.clone();
    app.listen("lockin:stop", move |_| {
        stop.fetch_add(1, Ordering::SeqCst);
    });
    app.listen("lockin:refocus", |event| {
        if let Ok(refocus) = serde_json::from_str::<Refocus>(event.payload()) {
            activate_app(&refocus.app);
        }
    });
    let handle = app.clone();
    app.listen("lockin:start", move |event| {
        let Ok(start) = serde_json::from_str::<Start>(event.payload()) else {
            return;
        };
        let mine = generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = generation.clone();
        let handle = handle.clone();
        std::thread::spawn(move || watch(handle, start.dwell_ms, mine, generation));
    });
}

fn watch<R: Runtime>(app: AppHandle<R>, dwell_ms: u64, mine: u64, generation: Arc<AtomicU64>) {
    let anchor = frontmost_app();
    let _ = app.emit(
        "lockin",
        Transition {
            state: "locked",
            app: anchor.clone(),
        },
    );
    let dwell = Duration::from_millis(dwell_ms);
    let mut away_since: Option<Instant> = None;
    let mut distracted = false;
    while generation.load(Ordering::SeqCst) == mine {
        std::thread::sleep(POLL);
        let app_name = frontmost_app();
        if app_name == anchor {
            away_since = None;
            if distracted {
                distracted = false;
                emit(&app, "focused", app_name);
            }
        } else if !NEUTRAL.contains(&app_name.as_str()) {
            let since = *away_since.get_or_insert_with(Instant::now);
            if !distracted && since.elapsed() >= dwell {
                distracted = true;
                emit(&app, "distracted", app_name);
            }
        }
    }
}

fn emit<R: Runtime>(app: &AppHandle<R>, state: &'static str, app_name: String) {
    let _ = app.emit(
        "lockin",
        Transition {
            state,
            app: app_name,
        },
    );
}

fn osascript(script: &str) -> Option<String> {
    let out = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ponytail: osascript spawn per poll; swap for objc2 NSWorkspace if 500ms of it shows up.
fn frontmost_app() -> String {
    osascript("path to frontmost application as text")
        .map(|path| app_name_from_hfs_path(&path))
        .unwrap_or_default()
}

fn activate_app(name: &str) {
    if name.is_empty() || name.contains('"') {
        return;
    }
    osascript(&format!(r#"tell application "{name}" to activate"#));
}

/// `Macintosh HD:Applications:Google Chrome.app:` -> `Google Chrome`
fn app_name_from_hfs_path(path: &str) -> String {
    path.trim_end_matches(':')
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .trim_end_matches(".app")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::app_name_from_hfs_path;

    #[test]
    fn hfs_path_to_app_name() {
        assert_eq!(
            app_name_from_hfs_path("Macintosh HD:Applications:Google Chrome.app:"),
            "Google Chrome"
        );
        assert_eq!(
            app_name_from_hfs_path("Macintosh HD:Applications:Ghostty.app:"),
            "Ghostty"
        );
        assert_eq!(app_name_from_hfs_path(""), "");
    }
}
