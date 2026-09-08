//! `WS /graphql/ws`: the Web Target's subscription Transport into the App
//! Schema.
//!
//! The wire format is the standard `graphql-ws` protocol, served by the
//! async-graphql axum integration. No envelope is invented here: the bespoke
//! accept/next/complete envelope belongs to the Desktop Transport, which
//! carries subscriptions over a Tauri channel instead of a socket.

use async_graphql::dynamic::Schema;
use async_graphql_axum::GraphQLSubscription;
use axum::Router;

pub const PATH: &str = "/graphql/ws";

pub fn router(schema: Schema) -> Router {
    Router::new().route_service(PATH, GraphQLSubscription::new(schema))
}
