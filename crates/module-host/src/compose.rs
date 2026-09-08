use sea_orm::DatabaseConnection;
use seaography::{
    async_graphql::dynamic::{Object, Schema},
    Builder, BuilderContext,
};

use crate::{error::ComposeError, module_def::ModuleDef, prefix_check};

/// Build one schema from the given Modules, registering each in order.
///
/// Every Module is first built alone into a standalone mini-schema and its
/// namespace checked there, so a violation names the Module that caused it.
/// A single-Module slice therefore yields that Module's mini-schema, which
/// is what per-module codegen validates operations against — custom
/// operations included.
///
/// A freshly scaffolded Module registers nothing yet, and nothing is what it
/// contributes: it gets no mini-schema build (a schema with an empty Query
/// root cannot build) and there is nothing for the namespace check to check,
/// so it composes cleanly before its first migration.
pub fn compose(
    context: &'static BuilderContext,
    database: DatabaseConnection,
    modules: &[ModuleDef],
) -> Result<Schema, ComposeError> {
    for module in modules {
        if contributes_nothing(context, database.clone(), module) {
            continue;
        }
        let mini = build(context, database.clone(), std::slice::from_ref(module))?;
        prefix_check::check(module.name, &mini.sdl())?;
    }
    build(context, database, modules)
}

fn contributes_nothing(
    context: &'static BuilderContext,
    database: DatabaseConnection,
    module: &ModuleDef,
) -> bool {
    if module.custom_ops().contributes_schema() {
        return false;
    }
    let builder = (module.register)(Builder::new(context, database));
    builder.queries.is_empty()
        && builder.mutations.is_empty()
        && builder.subscriptions.is_empty()
        && builder.outputs.is_empty()
        && builder.inputs.is_empty()
        && builder.enumerations.is_empty()
        && builder.unions.is_empty()
        && builder.scalars.is_empty()
}

fn build(
    context: &'static BuilderContext,
    database: DatabaseConnection,
    modules: &[ModuleDef],
) -> Result<Schema, ComposeError> {
    let mut builder = Builder::new(context, database.clone());

    // Seaography seeds its mutation root with a public `_ping` placeholder.
    // Real generated mutations keep the root valid, so omit that framework
    // implementation detail from the application contract.
    builder.mutation = Object::new("Mutation");

    let mut write_hooks = seaolim::ComposedWriteSetHooks::default();
    let mut schema_data = Vec::new();

    for module in modules {
        let ops = module.custom_ops();

        // A Module that selects its writes replaces the generated write
        // surface wholesale: registration is all-or-nothing per Model, so
        // the only way to publish three of four mutations is to drop what
        // registration just added and re-register the selection. Positional
        // truncation is what makes that exact — the mutations added since
        // this Module started registering are its own, and no others.
        let generated_writes_start = builder.mutations.len();
        builder = (module.register)(builder);
        if ops.replaces_generated_writes() {
            builder.mutations.truncate(generated_writes_start);
        }

        let applied = ops.apply(&mut builder);
        for hook in applied.hooks {
            write_hooks = hook(write_hooks);
        }
        schema_data.extend(applied.schema_data);
    }

    // Seaography 2.0.0-rc.9 declares a Subscription root even when no field
    // is registered, which produces invalid SDL. Declare the root only when
    // a Module actually registered a subscription.
    let subscription_root = (!builder.subscriptions.is_empty()).then_some("Subscription");
    builder.schema = Schema::build("Query", Some("Mutation"), subscription_root);

    let mut schema_builder = builder.schema_builder().data(database).data(write_hooks);
    for data in &schema_data {
        schema_builder = data(schema_builder);
    }
    schema_builder.finish().map_err(ComposeError::Schema)
}
