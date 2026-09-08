use std::sync::LazyLock;

use module_host::{ComposeError, ModuleDef};
use sea_orm::DatabaseConnection;
use seaography::{
    async_graphql::dynamic::{Schema, SchemaError},
    BuilderContext,
};

static CONTEXT: LazyLock<BuilderContext> = LazyLock::new(|| {
    let mut context = BuilderContext::default();
    // Seaography normally returns `<Entity>Basic` from mutations and
    // `<Entity>` from queries. One shared typename lets Apollo normalize a
    // mutation result into every cached query containing that entity.
    context.entity_object.basic_type_suffix = String::new();
    context
});

pub fn app_schema(database: DatabaseConnection) -> Result<Schema, SchemaError> {
    compose(database, &module_registry::modules())
}

/// One Module's standalone mini-schema, built from only its own
/// registrations. Per-module codegen validates a Module's operations against
/// it, so referencing another Module's types fails generation.
pub fn module_schema(database: DatabaseConnection, name: &str) -> Result<Schema, SchemaError> {
    let module = module_registry::module(name)
        .ok_or_else(|| SchemaError::from(format!("unknown module \"{name}\"")))?;
    compose(database, &[module])
}

fn compose(database: DatabaseConnection, modules: &[ModuleDef]) -> Result<Schema, SchemaError> {
    module_host::compose(&CONTEXT, database, modules).map_err(|error| match error {
        ComposeError::Schema(error) => error,
        violation => SchemaError::from(violation.to_string()),
    })
}
