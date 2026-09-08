//! The documents Module: a registry of saved documents whose only write path
//! to a document's content is a compare-and-swap.
//!
//! It exists to exercise every authoring primitive against something real.
//! Reading it top to bottom shows what a Module says and where it says it:
//! the migration owns the shape, `custom::register` owns everything beyond
//! generated CRUD, and no file here names a crate below the seam.

pub mod entities;
pub mod migrations;

mod check;
mod custom;
mod rules;
mod save;

pub use check::DocumentsSaveCheck;
pub use save::{DocumentsSaveOutcome, DocumentsSavedEvent};

module_host::module_def! {
    name: "documents",
    migrations: migrations::Migrator,
    entities: entities::register_entity_modules,
    custom: custom::register,
}
