//! Shared fixtures: one probe entity and schema assembly helpers.

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Schema};
use seaography::{async_graphql, Builder, BuilderContext};

pub mod probes {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "probes")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub value: String,
        pub stamp: Option<String>,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

#[allow(dead_code)]
pub mod write_rows {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "write_rows")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub parent_id: i64,
        pub active: bool,
        pub revision: i32,
        pub stamp: Option<String>,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub async fn database_with_probes() -> DatabaseConnection {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("open in-memory test database");
    let create =
        Schema::new(database.get_database_backend()).create_table_from_entity(probes::Entity);
    database
        .execute(&create)
        .await
        .expect("create probes table");
    database
}

#[allow(dead_code)]
pub async fn database_with_write_rows() -> DatabaseConnection {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("open in-memory test database");
    let create =
        Schema::new(database.get_database_backend()).create_table_from_entity(write_rows::Entity);
    database
        .execute(&create)
        .await
        .expect("create write rows table");
    database
}

/// Full generated bundle for `probes` under the given builder context.
/// Not every test binary uses every fixture.
#[allow(dead_code)]
pub async fn probes_schema(context: &'static BuilderContext) -> async_graphql::dynamic::Schema {
    let database = database_with_probes().await;
    let mut builder = Builder::new(context, database.clone());
    seaography::register_entity!(builder, probes);
    builder
        .schema_builder()
        .data(database)
        .finish()
        .expect("build probes schema")
}
