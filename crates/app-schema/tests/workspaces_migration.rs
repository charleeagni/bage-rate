use sea_orm::{ConnectionTrait, Database, Statement};
use sea_orm_migration::SchemaManager;

async fn table_exists(database: &sea_orm::DatabaseConnection) -> bool {
    !database
        .query_all_raw(Statement::from_string(
            database.get_database_backend(),
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'workspaces'",
        ))
        .await
        .expect("query sqlite_master")
        .is_empty()
}

#[tokio::test]
async fn workspace_migration_is_reversible() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("open database");
    let module = module_registry::module("workspaces").expect("workspaces Module");
    let manager = SchemaManager::new(&database);
    let migrations = module.migrations();

    for migration in &migrations {
        migration.up(&manager).await.expect("migrate up");
    }
    assert!(table_exists(&database).await);

    for migration in migrations.iter().rev() {
        migration.down(&manager).await.expect("migrate down");
    }
    assert!(!table_exists(&database).await);
}
