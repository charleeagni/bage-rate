//! Where the Server Process listens, which Store it owns, and which directory
//! it serves the Web Target from.

use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

/// Loopback by default, so starting the Server Process never silently exposes
/// an unauthenticated Store to the network. The Web Target's development proxy
/// forwards to this address as `DEVELOPMENT_SERVER_PROCESS_ORIGIN` in
/// `src/graphql/transport/webEndpoints.ts`, and
/// `scripts/cross-language-constants.test.mjs` fails if the two disagree.
pub const DEFAULT_BIND: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), 1421);

/// The Server Store lives in the working directory unless the operator points
/// it at the volume they intend to back up.
pub const DEFAULT_STORE: &str = "server-store.db";

/// The Web Target bundle as written by the bundler, so the binary the Web
/// Target build produces serves that build's assets with no flag at all. The
/// Desktop Target's asset bundle is a different directory and is never served.
/// The bundler declares the same directory as `build.outDir` in
/// `vite.config.ts` and `scripts/check-target-bundles.sh` inspects it as
/// `web_bundle`; `scripts/cross-language-constants.test.mjs` fails if the three
/// disagree.
pub const DEFAULT_WEB_ROOT: &str = "dist-web";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub bind: SocketAddr,
    pub store: PathBuf,
    pub web_root: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: DEFAULT_BIND,
            store: PathBuf::from(DEFAULT_STORE),
            web_root: PathBuf::from(DEFAULT_WEB_ROOT),
        }
    }
}
