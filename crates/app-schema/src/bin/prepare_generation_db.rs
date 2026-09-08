use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::{MigratorTrait, SchemaManager};

#[tokio::main]
async fn main() {
    let mut module: Option<String> = None;
    let mut output: Option<std::path::PathBuf> = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--module" {
            module = Some(arguments.next().expect("--module requires a module name"));
        } else {
            output = Some(std::path::PathBuf::from(argument));
        }
    }
    let output = output.unwrap_or_else(|| std::path::PathBuf::from("generation.sqlite"));
    let path = output.as_path();
    if path.exists() {
        std::fs::remove_file(path).expect("remove the prior generation database");
    }

    let database_path = path.to_owned();
    let mut options = ConnectOptions::new("sqlite:generation.sqlite?mode=rwc");
    options.map_sqlx_sqlite_opts(move |options| {
        options
            .filename(&database_path)
            .create_if_missing(true)
            .foreign_keys(true)
    });
    let database = Database::connect(options)
        .await
        .expect("create the generation database");
    match module {
        None => module_registry::Migrator::up(&database, None)
            .await
            .expect("apply migrations to the generation database"),
        Some(name) => migrate_one_module(&database, &name).await,
    }
    database
        .close()
        .await
        .expect("close the generation database");
    println!("{}", path.display());
}

/// Apply only the named Module's migrations, so the resulting Store contains
/// exactly that Module's tables for per-module entity generation.
async fn migrate_one_module(database: &DatabaseConnection, name: &str) {
    let module = module_registry::module(name)
        .unwrap_or_else(|| panic!("unknown module \"{name}\"; expected a modules/*/rust crate"));
    let manager = SchemaManager::new(database);
    for migration in module.migrations() {
        migration
            .up(&manager)
            .await
            .expect("apply a module migration to the generation database");
    }
}
