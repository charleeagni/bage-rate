//! Composition and namespace ownership: what the seam does with whatever a
//! Module registered, and what it refuses.

mod support;

use module_host::{compose, ComposeError};
use support::{context, database};

#[tokio::test]
async fn composing_the_projects_module_lands_its_types_and_root_fields() {
    let schema = compose(
        context(),
        database().await,
        &[projects_module::module_def()],
    )
    .expect("compose");
    let sdl = schema.sdl();

    assert!(sdl.contains("type Projects {"));
    assert!(sdl.contains("projects(filters: ProjectsFilterInput"));
    assert!(sdl.contains("projectsCreateOne(data: ProjectsInsertInput!): Projects!"));
    assert!(sdl.contains("projectsCreateBatch(data: [ProjectsInsertInput!]!): [Projects!]!"));
    assert!(sdl.contains("projectsUpdate(data: ProjectsUpdateInput!"));
    assert!(sdl.contains("projectsDelete(filter: ProjectsFilterInput): Int!"));
}

#[tokio::test]
async fn the_projects_mini_schema_preserves_the_schema_invariants() {
    let mini = compose(
        context(),
        database().await,
        &[projects_module::module_def()],
    )
    .expect("compose one module");
    let sdl = mini.sdl();

    assert!(!sdl.contains("ProjectsBasic"));
    assert!(!sdl.contains("_ping"));
    // No Module registered a subscription, so the root is absent rather than
    // declared and empty, which would be invalid SDL.
    assert!(!sdl.contains("type Subscription"));
}

// The shape scripts/new-module.sh stamps out: no migrations yet, no entities,
// and no custom registrations. Composition must accept it, or every
// scaffolded Module would break generate until its first migration.
mod scaffold {
    use module_host::migration::*;

    pub struct Migrator;

    #[async_trait::async_trait]
    impl MigratorTrait for Migrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            vec![]
        }
    }

    module_host::module_def! {
        name: "sample",
        migrations: Migrator,
    }
}

#[tokio::test]
async fn a_module_that_registers_nothing_composes_before_its_first_migration() {
    let schema = compose(
        context(),
        database().await,
        &[projects_module::module_def(), scaffold::module_def()],
    )
    .expect("compose with a freshly scaffolded module");
    assert!(schema.sdl().contains("type Projects {"));
}

mod unprefixed {
    use module_host::migration::*;

    pub mod widgets {
        use sea_orm::entity::prelude::*;

        #[sea_orm::model]
        #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "widgets")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub name: String,
        }

        impl ActiveModelBehavior for ActiveModel {}
    }

    seaography::register_entity_modules!([widgets,]);

    pub struct Migrator;

    #[async_trait::async_trait]
    impl MigratorTrait for Migrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            vec![]
        }
    }

    module_host::module_def! {
        name: "inventory",
        migrations: Migrator,
        entities: register_entity_modules,
    }
}

#[tokio::test]
async fn a_registration_outside_the_module_namespace_is_rejected() {
    let error = compose(context(), database().await, &[unprefixed::module_def()])
        .expect_err("unprefixed registration");
    assert!(
        matches!(
            &error,
            ComposeError::UnownedType { module, type_name }
                if *module == "inventory" && type_name.starts_with("Widgets")
        ) || matches!(
            &error,
            ComposeError::UnownedRootField { module, field, .. }
                if *module == "inventory" && field.starts_with("widgets")
        ),
        "unexpected error: {error}"
    );
    let message = error.to_string();
    assert!(message.contains("inventory"), "{message}");
}
