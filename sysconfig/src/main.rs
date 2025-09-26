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

    /// Path to a KDL configuration file to load on startup
    #[clap(short = 'c', long = "config")]
    config_file: Option<String>,

    /// Validate configuration file only (dry run)
    #[clap(short = 'n', long = "dry-run")]
    dry_run: bool,

    /// Watch configuration file for changes and reload automatically
    #[clap(short = 'w', long = "watch")]
    watch: bool,

    /// Print configuration summary and exit
    #[clap(long = "summary")]
    summary: bool,
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

    // Handle configuration file operations if specified
    if let Some(config_path) = &args.config_file {
        use std::path::Path;
        use sysconfig::kdl_loader::KdlConfigLoader;

        let mut loader = KdlConfigLoader::new().validate_only(args.dry_run);

        // Load the configuration file
        loader.load_file(Path::new(config_path))?;

        // Validate the configuration
        loader.validate()?;

        if args.summary {
            println!("Configuration Summary:");
            println!("{}", loader.summary());
            return Ok(());
        }

        if args.dry_run {
            println!("Configuration validation successful");
            println!("\nConfiguration Summary:");
            println!("{}", loader.summary());
            return Ok(());
        }

        // If not dry-run and not just summary, we'll apply the configuration after starting the service
        if !args.watch {
            // Create the sysconfig service
            let service = SysConfigService::new()?;
            let service = Arc::new(service);

            // Apply the configuration
            tracing::info!("Applying KDL configuration from: {}", config_path);
            loader.apply(&service).await?;
            tracing::info!("Configuration applied successfully");

            // Continue to start the service if socket is specified
            if args.socket.is_some() || args.socket.is_none() {
                let socket = args.socket.unwrap_or_else(default_socket_path);
                tracing::info!("Starting sysconfig service on socket: {}", socket);
                service.start(&socket).await?;
            }
        } else {
            // Watch mode
            tracing::info!(
                "Starting in watch mode for configuration file: {}",
                config_path
            );

            // Create the sysconfig service
            let service = SysConfigService::new()?;
            let service = Arc::new(service.clone());

            // Apply initial configuration
            loader.apply(&service).await?;

            // Start the service in a background task
            let socket = args.socket.unwrap_or_else(default_socket_path);
            let service_clone = service.clone();
            let socket_clone = socket.clone();

            let _service_task = tokio::spawn(async move {
                tracing::info!("Starting sysconfig service on socket: {}", socket_clone);
                if let Err(e) = service_clone.start(&socket_clone).await {
                    tracing::error!("Service error: {}", e);
                }
            });

            // Watch for configuration changes
            use std::fs;
            use tokio::time::{interval, Duration};

            let path = Path::new(config_path).to_path_buf();
            let mut last_modified = fs::metadata(&path).and_then(|m| m.modified()).ok();

            let mut interval = interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        if last_modified != Some(modified) {
                            tracing::info!(
                                "Configuration file changed, reloading: {}",
                                path.display()
                            );

                            let mut new_loader = KdlConfigLoader::new();
                            match new_loader.load_file(&path) {
                                Ok(_) => {
                                    if let Err(e) = new_loader.validate() {
                                        tracing::error!("Configuration validation failed: {}", e);
                                        continue;
                                    }

                                    match new_loader.apply(&service).await {
                                        Ok(_) => {
                                            tracing::info!(
                                                "Successfully reloaded and applied configuration"
                                            );
                                            last_modified = Some(modified);
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to apply configuration: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to reload configuration: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // No configuration file specified, just start the service
        let socket = args.socket.unwrap_or_else(default_socket_path);

        // Create the sysconfig service
        let service = SysConfigService::new()?;
        let service = Arc::new(service);

        // Start the service
        tracing::info!("Starting sysconfig service on socket: {}", socket);
        service.start(&socket).await?;
    }

    Ok(())
}
