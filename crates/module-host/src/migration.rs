//! The migration vocabulary a Module may name.
//!
//! A re-export of SeaORM's migration prelude, so a Module's migrations —
//! the one hand-authored source of truth for its Models — are written
//! against the seam like everything else it authors.

pub use sea_orm_migration::prelude::*;
