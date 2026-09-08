//! The composed Server Process: one origin carrying the App Schema and the Web
//! Target bundle, started from an explicit bind address, Store, and asset root
//! so that no run — test or deployment — inherits a location it did not name.

use std::{error::Error, net::SocketAddr, path::Path};

use app_schema::{database::file_database, schema::app_schema};
use async_graphql::dynamic::Schema;
use axum::Router;
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::{
    asset_route, config::ServerConfig, graphql_route, response_headers, subscription_route,
};

/// A Server Process that is listening. Dropping it leaves the server running
/// until the process exits; [`RunningServer::shutdown`] stops it and waits.
pub struct RunningServer {
    local_addr: SocketAddr,
    stop: oneshot::Sender<()>,
    serving: JoinHandle<std::io::Result<()>>,
}

impl RunningServer {
    /// The address actually bound, which is what a caller needs when the
    /// requested port was zero.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn shutdown(self) -> Result<(), Box<dyn Error>> {
        // A closed receiver means the server already stopped, which is the
        // outcome this call asks for.
        let _ = self.stop.send(());
        self.serving.await??;
        Ok(())
    }

    /// Waits until the server stops on its own.
    pub async fn serve_forever(self) -> Result<(), Box<dyn Error>> {
        self.serving.await??;
        Ok(())
    }
}

/// Opens the configured Store, migrates it, composes the App Schema, and
/// serves it beside the configured asset root.
pub async fn start(config: &ServerConfig) -> Result<RunningServer, Box<dyn Error>> {
    let database = file_database(&config.store).await?;
    let schema = app_schema(database)?;
    start_with_schema(schema, config.bind, &config.web_root).await
}

/// The seam tests drive: any schema, an ephemeral port, and a temporary asset
/// root, with no Store location implied.
pub async fn start_with_schema(
    schema: Schema,
    bind: SocketAddr,
    web_root: &Path,
) -> Result<RunningServer, Box<dyn Error>> {
    let router = response_headers::harden(
        Router::new()
            .merge(graphql_route::router(schema.clone()))
            .merge(subscription_route::router(schema))
            .merge(asset_route::router(web_root)),
    );

    let listener = TcpListener::bind(bind).await?;
    let local_addr = listener.local_addr()?;

    let (stop, stopped) = oneshot::channel();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
    });

    Ok(RunningServer {
        local_addr,
        stop,
        serving,
    })
}
