//! Fixtures shared by the seam's tests: the application's builder context
//! and a Store to compose against.

use std::sync::LazyLock;

use sea_orm::{Database, DatabaseConnection};
use seaography::BuilderContext;

static CONTEXT: LazyLock<BuilderContext> = LazyLock::new(|| {
    let mut context = BuilderContext::default();
    // Match the application context: query and mutation results share one
    // typename.
    context.entity_object.basic_type_suffix = String::new();
    context
});

pub fn context() -> &'static BuilderContext {
    &CONTEXT
}

/// A Store with no tables. Schema composition never reads one, so the tests
/// that only inspect SDL need nothing migrated.
pub async fn database() -> DatabaseConnection {
    Database::connect("sqlite::memory:")
        .await
        .expect("open database")
}
