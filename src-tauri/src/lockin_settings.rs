//! Desktop preferences and isolated animation documents. No network listener.
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+L";
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Animation {
    pub name: String,
    pub html: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub animation: Option<Animation>,
    pub shortcut: String,
    #[serde(skip_deserializing)]
    pub shortcut_error: Option<String>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            animation: None,
            shortcut: DEFAULT_SHORTCUT.into(),
            shortcut_error: None,
        }
    }
}
pub struct Settings {
    path: PathBuf,
    value: Mutex<Preferences>,
}
fn save(path: &std::path::Path, value: &Preferences) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())
}
fn register<R: Runtime>(app: &AppHandle<R>, shortcut: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if event.state == ShortcutState::Pressed
                && !app
                    .get_webview_window("settings")
                    .is_some_and(|w| w.is_focused().unwrap_or(false))
            {
                let _ = app.emit_to("main", "lockin:toggle", ());
            }
        })
        .map_err(|e| format!("Could not register {shortcut}: {e}. Choose another shortcut."))
}
pub fn install<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join("lockin-settings.json");
    let mut value: Preferences = match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Preferences::default(),
        Err(e) => return Err(e.into()),
    };
    value.shortcut_error = register(app, &value.shortcut).err();
    app.manage(Settings {
        path,
        value: Mutex::new(value),
    });
    Ok(())
}
#[tauri::command]
pub fn lockin_preferences(settings: State<'_, Settings>) -> Result<Preferences, String> {
    settings
        .value
        .lock()
        .map(|v| v.clone())
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn lockin_animation<R: Runtime>(
    app: AppHandle<R>,
    settings: State<'_, Settings>,
    animation: Option<Animation>,
) -> Result<Preferences, String> {
    if let Some(a) = &animation {
        if a.html.len() > 5 * 1024 * 1024 {
            return Err("Animation must be smaller than 5 MB.".into());
        }
        if a.html.trim().is_empty() {
            return Err("The HTML file is empty.".into());
        }
    }
    let mut value = settings.value.lock().map_err(|e| e.to_string())?;
    let mut next = value.clone();
    next.animation = animation;
    save(&settings.path, &next)?;
    *value = next;
    let _ = app.emit("lockin:preferences", &*value);
    Ok(value.clone())
}
#[tauri::command]
pub fn lockin_shortcut<R: Runtime>(
    app: AppHandle<R>,
    settings: State<'_, Settings>,
    shortcut: String,
) -> Result<Preferences, String> {
    let parsed: Shortcut = shortcut
        .parse()
        .map_err(|e| format!("Invalid shortcut: {e}"))?;
    let mut value = settings.value.lock().map_err(|e| e.to_string())?;
    if value.shortcut.parse::<Shortcut>().ok() == Some(parsed) && value.shortcut_error.is_none() {
        return Ok(value.clone());
    }
    register(&app, &shortcut)?;
    let mut next = value.clone();
    next.shortcut = shortcut.clone();
    next.shortcut_error = None;
    if let Err(e) = save(&settings.path, &next) {
        let _ = app.global_shortcut().unregister(shortcut.as_str());
        return Err(e);
    }
    if value.shortcut_error.is_none() {
        if let Err(e) = app.global_shortcut().unregister(value.shortcut.as_str()) {
            let _ = app.global_shortcut().unregister(shortcut.as_str());
            save(&settings.path, &value)?;
            return Err(e.to_string());
        }
    }
    *value = next;
    let _ = app.emit("lockin:preferences", &*value);
    Ok(value.clone())
}

const BRIDGE: &str = r#"<!doctype html><script>
(() => {
const send = (event, detail) => parent.postMessage({source:'cuteAnimation',event,detail}, '*');
addEventListener('error', e => send('error', e.message || 'An animation resource failed to load.'), true);
addEventListener('unhandledrejection', e => send('error', String(e.reason)));
addEventListener('securitypolicyviolation', e => send('error', 'Blocked '+e.violatedDirective+': '+e.blockedURI));
addEventListener('DOMContentLoaded', () => send('ready'));
})();
</script>"#;
const POLICY: &str = "default-src 'none'; script-src 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'unsafe-inline'; img-src data: blob:; font-src data:; media-src data: blob:; connect-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'";
fn document(html: &str) -> String {
    format!("{BRIDGE}{html}")
}
pub fn protocol<R: Runtime>(
    ctx: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let body = (request.method() == tauri::http::Method::GET && request.uri().path() == "/current")
        .then(|| ctx.app_handle().try_state::<Settings>())
        .flatten()
        .and_then(|s| {
            s.value
                .lock()
                .ok()?
                .animation
                .as_ref()
                .map(|a| document(&a.html))
        });
    tauri::http::Response::builder()
        .status(if body.is_some() { 200 } else { 404 })
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Content-Security-Policy", POLICY)
        .header("Cache-Control", "no-store")
        .body(body.unwrap_or_default().into_bytes())
        .expect("static response headers")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let value = Preferences {
            animation: Some(Animation {
                name: "test.html".into(),
                html: "<script>run()</script>".into(),
            }),
            shortcut: "Control+Shift+K".into(),
            shortcut_error: Some("runtime only".into()),
        };
        save(&path, &value).unwrap();
        let loaded: Preferences = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(loaded.animation.unwrap().html, "<script>run()</script>");
        assert_eq!(loaded.shortcut, value.shortcut);
        assert!(loaded.shortcut_error.is_none());
    }
    #[test]
    fn diagnostics_precede_user_script() {
        let html = document("<html><script>throw Error('broken')</script></html>");
        assert!(html.find("addEventListener('error'").unwrap() < html.find("throw Error").unwrap());
        assert!(POLICY.contains("connect-src 'none'"));
        assert!(!POLICY.contains("sha256"));
    }
}
