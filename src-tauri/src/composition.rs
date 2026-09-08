use std::path::PathBuf;

use app_schema::{database::file_database, schema::app_schema};
use tauri::Manager;
use tauri_graphql_transport::{GraphQlEndpoint, TransportApi, TransportApiImpl};

pub fn compose<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    compose_with_database_path_provider(builder, |app| {
        Ok(app.path().app_data_dir()?.join("application.sqlite3"))
    })
}

/// Compose the production application against an explicit database path.
///
/// This is primarily a test seam: integration tests must never resolve or
/// migrate the user's real application-data database.
pub fn compose_with_database_path<R: tauri::Runtime>(
    builder: tauri::Builder<R>,
    database_path: PathBuf,
) -> tauri::Builder<R> {
    compose_with_database_path_provider(builder, move |_| Ok(database_path))
}

fn compose_with_database_path_provider<R, F>(
    builder: tauri::Builder<R>,
    database_path: F,
) -> tauri::Builder<R>
where
    R: tauri::Runtime,
    F: FnOnce(&tauri::App<R>) -> Result<PathBuf, Box<dyn std::error::Error>> + Send + 'static,
{
    let api = TransportApiImpl::new();
    let setup_api = api.clone();

    let preferences: Box<dyn Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync> =
        Box::new(tauri::generate_handler![
            crate::lockin_settings::lockin_preferences,
            crate::lockin_settings::lockin_animation,
            crate::lockin_settings::lockin_shortcut
        ]);
    let graphql = taurpc::create_ipc_handler(api.into_handler());
    builder
        .setup(move |app| {
            let database_path = database_path(app)?;
            let endpoint = tauri::async_runtime::block_on(async move {
                let database = file_database(&database_path).await?;
                let schema = app_schema(database)?;
                Ok::<_, Box<dyn std::error::Error>>(GraphQlEndpoint::new(schema))
            })?;
            setup_api
                .install_endpoint(endpoint)
                .map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(move |invoke| match invoke.message.command() {
            "lockin_preferences" | "lockin_animation" | "lockin_shortcut" => preferences(invoke),
            _ => graphql(invoke),
        })
}
