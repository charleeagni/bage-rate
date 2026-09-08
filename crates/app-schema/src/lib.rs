//! The App Schema: the one composed GraphQL schema both Targets execute
//! against, together with the Store access it is built from. Feature code
//! lives in Modules under `modules/`; this crate composes whatever the
//! generated module-registry hands it. It links no Tauri code, so it builds
//! headlessly without any desktop or webview prerequisite.

pub mod database;
pub mod schema;
