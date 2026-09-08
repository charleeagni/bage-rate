//! What the seam's macros expand to.
//!
//! Nothing here is part of the authoring surface: these are the only
//! signatures that still name a layer beneath the seam, and they exist so
//! that `module_def!`, `#[custom_fields]`, and `#[output]` can mention them
//! instead of a Module's own source. `scripts/check-module-imports.mjs`
//! rejects `__private` anywhere under `modules/`.

use sea_orm_migration::MigrationTrait;
use seaography::{async_graphql::Context, Builder};

pub use sea_orm_migration::MigratorTrait;

use crate::{custom_ops::CustomOps, error::Error, module_ctx::ModuleCtx, module_def::ModuleDef};

pub const fn module_def(
    name: &'static str,
    migrations: fn() -> Vec<Box<dyn MigrationTrait>>,
) -> ModuleDef {
    ModuleDef {
        name,
        migrations,
        register: pass_through,
        custom: registers_nothing,
    }
}

pub const fn with_entities(
    mut definition: ModuleDef,
    register: fn(Builder) -> Builder,
) -> ModuleDef {
    definition.register = register;
    definition
}

pub const fn with_custom(mut definition: ModuleDef, custom: fn(&mut CustomOps)) -> ModuleDef {
    definition.custom = custom;
    definition
}

pub fn module_ctx<'a>(context: &'a Context<'a>) -> ModuleCtx<'a> {
    ModuleCtx::new(context)
}

pub fn argument_error(
    field: &str,
    argument: &str,
    error: impl std::fmt::Display,
) -> seaography::SeaographyError {
    seaography::SeaographyError::AsyncGraphQLError(Error::new(format!(
        "{field}: could not read argument {argument}: {error}"
    )))
}

/// A Module before its first migration registers nothing, and nothing is
/// what it contributes.
fn pass_through(builder: Builder) -> Builder {
    builder
}

fn registers_nothing(_: &mut CustomOps) {}
