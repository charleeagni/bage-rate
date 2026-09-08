//! The Web Target bundle, read from a directory at runtime rather than
//! embedded in the binary, so the Rust and bundler build graphs stay
//! independent and a stale embedded bundle cannot happen.
//!
//! Any path the bundle does not name is answered with the entry document, so a
//! refresh or a deep link on a client-side route loads the application instead
//! of a not-found page.

use std::path::Path;

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

pub const ENTRY_DOCUMENT: &str = "index.html";

pub fn router(web_root: &Path) -> Router {
    let entry_document = ServeFile::new(web_root.join(ENTRY_DOCUMENT));
    Router::new().fallback_service(ServeDir::new(web_root).fallback(entry_document))
}
