//! The hardening headers the Server Process sends with every response.
//!
//! The Desktop Target constrains what its document may load through the Tauri
//! Content Security Policy in `src-tauri/tauri.conf.json`. The Web Target
//! presents the same application, so it states the same intent the only way an
//! HTTP origin can: as response headers on the composed router. Without this
//! the two Targets would ship the same UI under opposite rules, and a reader
//! could not tell whether the difference was decided or overlooked.
//!
//! This is not the CORS question ADR-0004 settles. That decision is about
//! cross-origin requests this design never makes; these headers constrain what
//! the served document itself is allowed to load. Neither is what makes the
//! Server Process safe to expose — it is unauthenticated, and the loopback
//! default is what keeps it private.

use axum::{
    http::{header, HeaderValue},
    Router,
};
use tower_http::set_header::SetResponseHeaderLayer;

/// The Desktop Target's policy translated to one HTTP origin: the App Schema
/// and the bundle share that origin, so `connect-src 'self'` covers both the
/// GraphQL endpoint and the same-origin graphql-ws socket, where the Desktop
/// Target names the IPC origins instead.
pub const CONTENT_SECURITY_POLICY: &str =
    "default-src 'self'; connect-src 'self'; img-src 'self'; style-src 'self'";

/// Bundle files are served from a directory, so a wrong guess about a file's
/// type must not become an execution decision.
pub const CONTENT_TYPE_OPTIONS: &str = "nosniff";

/// A loopback URL carrying a client-side route is not something a navigation
/// away from the application should disclose.
pub const REFERRER_POLICY: &str = "no-referrer";

/// Applies the headers to a composed router, covering the bundle and both
/// GraphQL endpoints rather than only the assets, so no route added later
/// silently opts out.
pub fn harden(router: Router) -> Router {
    router
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CONTENT_SECURITY_POLICY),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static(CONTENT_TYPE_OPTIONS),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static(REFERRER_POLICY),
        ))
}
