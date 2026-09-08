//! The projects Module: the demo Model's migrations and generated
//! registrations, carried as one folder per ADR-0006.

pub mod entities;
pub mod migrations;

module_host::module_def! {
    name: "projects",
    migrations: migrations::Migrator,
    entities: entities::register_entity_modules,
}
