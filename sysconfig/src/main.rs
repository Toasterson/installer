use clap::Parser;
use std::sync::Arc;
use sysconfig::{Result, SysConfigService};
use tracing_subscriber::EnvFilter;

/// Command line arguments for the sysconfig service
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the Unix socket to listen on
    #[clap(short, long)]
    socket: Option<String>,
}

fn default_socket_path() -> String {
    #[cfg(target_os = "linux")]
    {
        // Prefer XDG_RUNTIME_DIR if set (usually /run/user/$UID)
        if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
            return format!("{}/sysconfig.sock", dir);
        }
        // Fallback to /run/user/$EUID
        let euid = unsafe { libc::geteuid() as u32 };
        if euid == 0 {
            "/var/run/sysconfig.sock".to_string()
        } else {
            format!("/run/user/{}/sysconfig.sock", euid)
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        "/var/run/sysconfig.sock".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Parse command line arguments
    let args = Args::parse();

    let socket = args.socket.unwrap_or_else(default_socket_path);

    // Create the sysconfig service
    let service = SysConfigService::new()?;
    let service = Arc::new(service);

    // Start the service
    tracing::info!("Starting sysconfig service on socket: {}", socket);
    service.start(&socket).await?;

    Ok(())
}
