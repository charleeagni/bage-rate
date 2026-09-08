use tauri::Manager;
use tauri_graphql_app::composition::compose_with_database_path;

#[test]
fn tauri_composition_builds_with_an_isolated_database() {
    let directory = tempfile::tempdir().expect("create isolated app-data directory");
    let database_path = directory.path().join("application.sqlite3");
    let app = compose_with_database_path(tauri::test::mock_builder(), database_path.clone())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("the composed Tauri app must build under the mock runtime");
    tauri::WebviewWindowBuilder::new(&app, "test", tauri::WebviewUrl::default())
        .build()
        .expect("build the mock webview window");
    let exit_code = app.run_return(|app, event| {
        if matches!(event, tauri::RunEvent::Ready) {
            app.get_webview_window("test")
                .expect("test window exists")
                .close()
                .expect("close the mock main window");
        }
    });
    assert_eq!(exit_code, 0);
    assert!(database_path.is_file());
}
