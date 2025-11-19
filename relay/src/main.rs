//! Descord Relay Node
//!
//! A simple store-and-forward relay for encrypted blobs and CRDT operations.
//! Runs in local development mode on localhost:9000

use anyhow::Result;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting Descord relay node in development mode...");
    info!("Listening on localhost:9000");

    // TODO: Implement relay server
    // For now, just keep running
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down relay node");
    Ok(())
}
