use std::sync::Arc;
use clap::Parser;
use sysconfig::{SysConfigService, Result};
use tracing_subscriber::EnvFilter;

/// Command line arguments for the sysconfig service
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the Unix socket to listen on
    #[clap(short, long, default_value = "/var/run/sysconfig.sock")]
    socket: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Parse command line arguments
    let args = Args::parse();

    // Create the sysconfig service
    let service = SysConfigService::new();
    let service = Arc::new(service);

    // Start the service
    tracing::info!("Starting sysconfig service on socket: {}", args.socket);
    service.start(&args.socket).await?;

    Ok(())
}