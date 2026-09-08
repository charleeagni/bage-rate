#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;

const OVERLAY_WINDOW: &str = "main";

fn main() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .register_uri_scheme_protocol("animation", tauri_graphql_app::lockin_settings::protocol);
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_overlay::init());
    let context = tauri::generate_context!();
    let app = tauri_graphql_app::composition::compose(builder)
        .build(context)
        .expect("the Tauri application failed to build");
    app.run(|app, event| {
        if let tauri::RunEvent::Ready = event {
            if let Some(window) = app.get_webview_window(OVERLAY_WINDOW) {
                tauri_overlay::cover_primary_monitor(&window)
                    .expect("the overlay window failed to configure");
            }
            tauri_graphql_app::lockin::install(app);
            tauri_graphql_app::lockin_settings::install(app)
                .expect("lock-in settings failed to load");
            tauri_graphql_app::open_url::install(app);
            tauri_graphql_app::tray::install(app).expect("the tray failed to install");
        }
    });
}
