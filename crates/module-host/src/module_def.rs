use sea_orm_migration::MigrationTrait;
use seaography::Builder;

use crate::custom_ops::CustomOps;

/// One Module's contribution to the App Schema.
///
/// The type is opaque and there is exactly one way to build one — the
/// [`module_def!`](crate::module_def) macro — so a Module declares what it
/// contributes without naming anything underneath the seam.
#[derive(Clone, Copy)]
pub struct ModuleDef {
    /// The Module's snake_case name, which is also its namespace prefix:
    /// every table, GraphQL type, and root field the Module registers must
    /// carry it (compared case-insensitively per position's case
    /// convention).
    pub(crate) name: &'static str,
    pub(crate) migrations: fn() -> Vec<Box<dyn MigrationTrait>>,
    pub(crate) register: fn(Builder) -> Builder,
    pub(crate) custom: fn(&mut CustomOps),
}

impl ModuleDef {
    /// The Module's name, which is also its namespace prefix.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The Module's migrations in their declared order, so the host can
    /// compose one deterministic migration sequence.
    pub fn migrations(&self) -> Vec<Box<dyn MigrationTrait>> {
        (self.migrations)()
    }

    /// The escape-hatch operations this Module declared, in declaration
    /// order. Empty for every Module that fits the primitives, which is the
    /// point: the guard counts these and requires a written exception for
    /// each one.
    pub fn escape_hatches(&self) -> Vec<&'static str> {
        self.custom_ops().declared_escape_hatches().to_vec()
    }

    pub(crate) fn custom_ops(&self) -> CustomOps {
        let mut ops = CustomOps::default();
        (self.custom)(&mut ops);
        ops
    }
}

/// Declare a Module's contribution to the App Schema.
///
/// ```ignore
/// module_host::module_def! {
///     name: "documents",
///     migrations: migrations::Migrator,
///     entities: entities::register_entity_modules,
///     custom: custom::register,
/// }
/// ```
///
/// `entities` is absent until `generate` has produced the Module's entity
/// directory, and `custom` is absent until the Module needs something beyond
/// generated CRUD. The macro defines the `module_def()` function the
/// generated registry calls.
#[macro_export]
macro_rules! module_def {
    (
        name: $name:literal,
        migrations: $migrator:path
        $(, entities: $entities:path)?
        $(, custom: $custom:path)?
        $(,)?
    ) => {
        /// This Module's contribution to the App Schema, read by the
        /// generated composition registry.
        pub fn module_def() -> $crate::ModuleDef {
            let definition = $crate::__private::module_def(
                $name,
                <$migrator as $crate::__private::MigratorTrait>::migrations,
            );
            $(let definition = $crate::__private::with_entities(definition, $entities);)?
            $(let definition = $crate::__private::with_custom(definition, $custom);)?
            definition
        }
    };
}
