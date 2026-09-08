//! The command line the Server Process binary is started with, and the
//! `ServerConfig` it resolves to. This lives in the library rather than in
//! `main.rs` so a test can resolve the operator-facing defaults — above all
//! the loopback bind — through the same code the binary runs.

use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

use crate::config::{ServerConfig, DEFAULT_BIND, DEFAULT_STORE, DEFAULT_WEB_ROOT};

/// Serves the Web Target and its GraphQL endpoint from one origin. It ships no
/// authentication or authorization and is unsafe to expose publicly.
#[derive(Parser, Clone, Debug, PartialEq, Eq)]
#[command(name = "web-server", version)]
pub struct Arguments {
    /// Address to listen on. Loopback by default.
    #[arg(long, default_value_t = DEFAULT_BIND)]
    pub bind: SocketAddr,

    /// The Server Store. Created, and migrated forward, if absent.
    #[arg(long, default_value = DEFAULT_STORE)]
    pub store: PathBuf,

    /// Directory holding the built Web Target bundle.
    #[arg(long, default_value = DEFAULT_WEB_ROOT)]
    pub web_root: PathBuf,
}

impl Arguments {
    /// The configuration the Server Process starts with. This is the only
    /// place the command line becomes a `ServerConfig`, so no flag can be
    /// resolved one way in the binary and another way under test.
    pub fn into_config(self) -> ServerConfig {
        ServerConfig {
            bind: self.bind,
            store: self.store,
            web_root: self.web_root,
        }
    }
}
