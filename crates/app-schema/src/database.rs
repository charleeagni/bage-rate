use std::path::Path;

use module_registry::Migrator;
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

pub async fn in_memory_database() -> Result<DatabaseConnection, DbErr> {
    let database = Database::connect("sqlite::memory:").await?;
    Migrator::up(&database, None).await?;
    Ok(database)
}

pub async fn file_database(path: &Path) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // The placeholder URL supplies SQLite's create mode. The low-level option
    // mapping supplies the real Path without lossy URL construction, so spaces
    // and non-ASCII application-data paths work on every desktop platform.
    let database_path = path.to_owned();
    let mut options = ConnectOptions::new("sqlite:app.db?mode=rwc");
    options
        .max_connections(4)
        .min_connections(1)
        .sqlx_logging(cfg!(debug_assertions))
        .map_sqlx_sqlite_opts(move |options| {
            options
                .filename(&database_path)
                .create_if_missing(true)
                .foreign_keys(true)
        });

    let database = Database::connect(options).await?;
    Migrator::up(&database, None).await?;
    Ok(database)
}
