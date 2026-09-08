//! The runnable Server Process: one binary an operator starts, with the bind
//! address, the Store, and the Web Target assets chosen on the command line.

use std::error::Error;

use clap::Parser;
use web_server::Arguments;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Arguments::parse().into_config();

    let server = web_server::start(&config).await?;
    println!(
        "Web Target on http://{} — store {}, assets {}",
        server.local_addr(),
        config.store.display(),
        config.web_root.display()
    );
    server.serve_forever().await
}
