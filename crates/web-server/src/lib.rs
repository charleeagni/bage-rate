//! The Server Process: the headless binary that hosts the App Schema over the
//! network for the Web Target. It links no Tauri code, so it builds and
//! deploys without any desktop or webview prerequisite.
//!
//! The Web Target bundle, the GraphQL endpoint, and the graphql-ws
//! subscription endpoint share one origin, so the browser reaches its data
//! through a relative URL. There is no CORS
//! configuration and no API base URL, and there is no authentication or
//! authorization: this process is unsafe to expose publicly.

pub mod arguments;
pub mod asset_route;
pub mod config;
pub mod graphql_route;
pub mod response_headers;
pub mod server;
pub mod subscription_route;

pub use arguments::Arguments;
pub use config::{ServerConfig, DEFAULT_BIND, DEFAULT_STORE, DEFAULT_WEB_ROOT};
pub use server::{start, start_with_schema, RunningServer};
