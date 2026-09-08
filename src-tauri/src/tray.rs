//! Menu bar presence, Handy-style: no dock icon, a tray icon whose menu opens
//! the settings window. Closing settings hides it so the tray can reopen it.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime, WindowEvent};

pub const SETTINGS_WINDOW: &str = "settings";

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(menu) = app.menu() {
        if let Some(submenu) = menu.items()?.first().and_then(|item| item.as_submenu()) {
            submenu.insert(
                &MenuItem::with_id(app, "open-settings", "Settings…", true, Some("CmdOrCtrl+,"))?,
                1,
            )?;
        }
    }
    app.on_menu_event(|app, event| {
        if event.id().as_ref() == "open-settings" {
            show_settings(app);
        }
    });
    #[cfg(target_os = "macos")]
    // app.set_activation_policy(tauri::ActivationPolicy::Accessory)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit bage-rate", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &quit])?;
    let mut tray = TrayIconBuilder::new().menu(&menu).tooltip("bage-rate");
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone()).icon_as_template(true);
    }
    tray.on_menu_event(|app, event| match event.id().as_ref() {
        "settings" => show_settings(app),
        "quit" => app.exit(0),
        _ => {}
    })
    .build(app)?;

    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        let hide = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = hide.hide();
            }
        });
    }
    Ok(())
}

fn show_settings<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
