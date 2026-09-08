use app_schema::database::file_database;
use projects_module::migrations::Migrator;
use sea_orm::{ConnectionTrait, Database, Statement, TryGetable};
use sea_orm_migration::MigratorTrait;

async fn project_table_exists(database: &sea_orm::DatabaseConnection) -> bool {
    !database
        .query_all_raw(Statement::from_string(
            database.get_database_backend(),
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
        ))
        .await
        .expect("query sqlite_master")
        .is_empty()
}

#[tokio::test]
async fn migration_is_reversible() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("open database");
    Migrator::up(&database, None).await.expect("migrate up");
    assert!(project_table_exists(&database).await);
    Migrator::down(&database, None).await.expect("migrate down");
    assert!(!project_table_exists(&database).await);
}

#[tokio::test]
async fn migration_rejects_an_incompatible_preexisting_table() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("open database");
    database
        .execute_unprepared("CREATE TABLE projects (id INTEGER PRIMARY KEY)")
        .await
        .expect("create an incompatible projects table");

    assert!(
        Migrator::up(&database, None).await.is_err(),
        "a migration must not accept a table whose schema it did not create"
    );
}

#[tokio::test]
async fn file_database_handles_spaces_and_persists_data() {
    let directory = tempfile::Builder::new()
        .prefix("tauri graphql ")
        .tempdir()
        .expect("create temporary app-data directory");
    let path = directory.path().join("application data.sqlite3");

    let database = file_database(&path).await.expect("create file database");
    assert!(path.is_file());
    assert!(project_table_exists(&database).await);
    database
        .execute_unprepared(
            "INSERT INTO projects (name, workspace_root) VALUES ('Persisted', '/tmp/persisted')",
        )
        .await
        .expect("insert persistent data");
    database.close().await.expect("close database");

    let reopened = file_database(&path).await.expect("reopen file database");
    assert!(project_table_exists(&reopened).await);
    let row = reopened
        .query_one_raw(Statement::from_string(
            reopened.get_database_backend(),
            "SELECT name FROM projects WHERE workspace_root = '/tmp/persisted'",
        ))
        .await
        .expect("query reopened database")
        .expect("persisted project exists");
    assert_eq!(
        String::try_get_by(&row, "name").expect("read persisted project name"),
        "Persisted"
    );
}
