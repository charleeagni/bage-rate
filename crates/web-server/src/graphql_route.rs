//! `POST /graphql`: the Web Target's Transport into the App Schema.

use async_graphql::{dynamic::Schema, Request, Response as GraphQLResponse, ServerError};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};

pub const PATH: &str = "/graphql";

/// One MiB, matching the desktop Transport's request ceiling. This crate stays
/// free of `tauri-graphql-transport` so the Server Process never links Tauri,
/// so the constant is declared twice and the two are held in agreement by
/// `scripts/cross-language-constants.test.mjs`.
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;

pub fn router(schema: Schema) -> Router {
    Router::new()
        .route(PATH, post(execute))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(schema)
}

async fn execute(State(schema): State<Schema>, body: String) -> Response {
    let request = match serde_json::from_str::<Request>(&body) {
        Ok(request) => request,
        // async-graphql's own error response, not a shape of this template's
        // invention: client-visible GraphQL error mapping is an unproven
        // extension point (docs/stability-boundary.md), so the Server Process
        // must not model a contract a copier could read across Transports.
        Err(error) => {
            let rejected =
                GraphQLResponse::from_errors(vec![ServerError::new(error.to_string(), None)]);
            return (StatusCode::BAD_REQUEST, Json(rejected)).into_response();
        }
    };

    Json(schema.execute(request).await).into_response()
}
