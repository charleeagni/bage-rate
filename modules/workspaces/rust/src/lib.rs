//! Rustry's durable recent-workspaces Model, carried as one Module.

pub mod custom;
pub mod entities;
pub mod migrations;
pub mod rules;

module_host::module_def! {
    name: "workspaces",
    migrations: migrations::Migrator,
    entities: entities::register_entity_modules,
    custom: custom::register,
}
