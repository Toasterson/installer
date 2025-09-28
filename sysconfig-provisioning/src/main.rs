use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use hyper_util::rt::TokioIo;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;
use tracing::{debug, error, info, warn};

mod config;
mod merger;
mod sources;

use config::ProvisioningConfig;
use merger::ConfigMerger;
use sources::{LocalSource, SourceManager, SourcePriority};

// Remove the kdl crate dependency since we're using LocalSource

// Include the generated proto code
pub mod proto {
    tonic::include_proto!("sysconfig");
}

use proto::{
    sys_config_service_client::SysConfigServiceClient, ApplyStateRequest, GetStateRequest,
};

/// Get the default sysconfig socket path
fn default_sysconfig_socket() -> String {
    if let Ok(socket) = std::env::var("SYSCONFIG_SOCKET") {
        return socket;
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
            return format!("{}/sysconfig.sock", dir);
        }
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

/// CLI tool for system provisioning that writes to sysconfig
#[derive(Parser)]
#[command(author, version, about = "System provisioning CLI for sysconfig", long_about = None)]
struct Cli {
    /// Path to the sysconfig Unix socket
    #[arg(long, default_value_t = default_sysconfig_socket())]
    socket: String,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply provisioning configuration to sysconfig
    Apply {
        /// Path to KDL configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Enable specific sources (comma-separated: local,ec2,azure,gcp,digitalocean,openstack,smartos,cloud-init)
        #[arg(long)]
        sources: Option<String>,

        /// Disable specific sources (comma-separated)
        #[arg(long)]
        disable_sources: Option<String>,

        /// Perform a dry run (show what would be applied)
        #[arg(short, long)]
        dry_run: bool,

        /// Force apply even if no changes detected
        #[arg(long)]
        force: bool,
    },

    /// Auto-detect and apply provisioning based on environment
    Autodetect {
        /// Check if network setup is required first
        #[arg(long)]
        check_network: bool,

        /// Perform a dry run (show what would be applied)
        #[arg(short, long)]
        dry_run: bool,

        /// Maximum time to wait for network sources (seconds)
        #[arg(long, default_value = "30")]
        network_timeout: u64,
    },

    /// Parse a KDL config file and show the resulting state
    Parse {
        /// Path to KDL configuration file
        #[arg(short, long, required = true)]
        config: PathBuf,

        /// Output format: json or pretty
        #[arg(short, long, default_value = "pretty")]
        format: String,
    },

    /// Detect available provisioning sources
    Detect {
        /// Check network sources (requires network)
        #[arg(long)]
        network: bool,

        /// Output format: json or pretty
        #[arg(short, long, default_value = "pretty")]
        format: String,
    },

    /// Show current provisioning status
    Status {
        /// Output format: json or pretty
        #[arg(short, long, default_value = "pretty")]
        format: String,
    },
}

/// Connect to sysconfig service via Unix socket
async fn connect_to_sysconfig(socket_path: &str) -> Result<SysConfigServiceClient<Channel>> {
    let path = socket_path.to_string();
    let channel = Endpoint::from_static("http://[::]:50051")
        .connect_with_connector(service_fn(move |_| {
            let path = path.clone();
            async move {
                let stream = UnixStream::connect(path).await?;
                Ok::<_, std::io::Error>(TokioIo::new(stream))
            }
        }))
        .await
        .context("Failed to connect to sysconfig service")?;

    Ok(SysConfigServiceClient::new(channel))
}

/// Convert ProvisioningConfig to sysconfig state JSON
fn config_to_state(config: &ProvisioningConfig) -> serde_json::Value {
    let mut state = serde_json::Map::new();

    // Create network.settings object for network-related configuration
    let mut network_settings = serde_json::Map::new();
    let mut has_network_settings = false;

    // Hostname
    if let Some(hostname) = &config.hostname {
        network_settings.insert("hostname".to_string(), json!(hostname));
        has_network_settings = true;
    }

    // DNS settings (nameservers and search domains)
    if !config.nameservers.is_empty() || !config.search_domains.is_empty() {
        let mut dns = serde_json::Map::new();
        if !config.nameservers.is_empty() {
            dns.insert("nameservers".to_string(), json!(config.nameservers));
        }
        if !config.search_domains.is_empty() {
            dns.insert("search".to_string(), json!(config.search_domains));
        }
        network_settings.insert("dns".to_string(), json!(dns));
        has_network_settings = true;
    }

    // Network interfaces go under network.links (separate from network.settings)
    let mut has_network_links = false;
    let mut network_links = serde_json::Map::new();
    if !config.interfaces.is_empty() {
        for (name, iface) in &config.interfaces {
            network_links.insert(
                name.clone(),
                serde_json::to_value(iface).unwrap_or(json!({})),
            );
        }
        has_network_links = true;
    }

    // Routes could go with network settings or links - putting with links for now
    if !config.routes.is_empty() {
        network_links.insert(
            "routes".to_string(),
            serde_json::to_value(&config.routes).unwrap_or(json!([])),
        );
        has_network_links = true;
    }

    // Add network configuration to state if any network config exists
    if has_network_settings || has_network_links {
        let mut network = serde_json::Map::new();
        if has_network_settings {
            network.insert("settings".to_string(), json!(network_settings));
        }
        if has_network_links {
            network.insert("links".to_string(), json!(network_links));
        }
        state.insert("network".to_string(), json!(network));
    }

    // SSH authorized keys
    if !config.ssh_authorized_keys.is_empty() {
        state.insert(
            "ssh_authorized_keys".to_string(),
            json!(config.ssh_authorized_keys),
        );
    }

    // Users
    if !config.users.is_empty() {
        state.insert(
            "users".to_string(),
            serde_json::to_value(&config.users).unwrap_or(json!([])),
        );
    }

    // NTP servers
    if !config.ntp_servers.is_empty() {
        state.insert("ntp_servers".to_string(), json!(config.ntp_servers));
    }

    // Timezone
    if let Some(timezone) = &config.timezone {
        state.insert("timezone".to_string(), json!(timezone));
    }

    // Metadata
    if !config.metadata.is_empty() {
        state.insert("metadata".to_string(), json!(config.metadata));
    }

    json!(state)
}

/// Check if network sources are available
async fn check_network_sources(source_manager: &SourceManager) -> Vec<String> {
    let mut available = Vec::new();

    // Check each network source
    for source_type in &[
        "ec2",
        "azure",
        "gcp",
        "digitalocean",
        "openstack",
        "cloud-init",
    ] {
        if source_manager.is_source_available(source_type).await {
            available.push(source_type.to_string());
        }
    }

    available
}

/// Check if network is required for provisioning
async fn needs_network_for_provisioning() -> Result<bool> {
    // Check for local configuration sources first
    let local_sources = vec![
        PathBuf::from("/etc/sysconfig.kdl"),
        PathBuf::from("/etc/provisioning.json"),
        PathBuf::from("/mnt/provisioning/config.kdl"),
    ];

    for path in local_sources {
        if path.exists() {
            info!(
                "Found local config at {}, network not required",
                path.display()
            );
            return Ok(false);
        }
    }

    // Check if we're in a cloud environment that needs network
    let cloud_markers = vec![
        (
            "/sys/class/dmi/id/product_name",
            vec!["EC2", "Google", "Microsoft Corporation"],
        ),
        (
            "/sys/class/dmi/id/sys_vendor",
            vec!["Amazon EC2", "Google", "Microsoft Corporation"],
        ),
        (
            "/sys/class/dmi/id/bios_vendor",
            vec!["Amazon EC2", "Google", "Microsoft Corporation"],
        ),
    ];

    for (file, patterns) in cloud_markers {
        if let Ok(content) = tokio::fs::read_to_string(file).await {
            for pattern in patterns {
                if content.contains(pattern) {
                    info!("Detected cloud environment ({}), network required", pattern);
                    return Ok(true);
                }
            }
        }
    }

    // Check for SmartOS
    if PathBuf::from("/usr/sbin/mdata-get").exists() {
        info!("Detected SmartOS environment, checking for local metadata");
        // SmartOS may have local metadata, check if network is needed
        let output = tokio::process::Command::new("/usr/sbin/mdata-get")
            .arg("sdc:nics")
            .output()
            .await;

        if output.is_ok() {
            info!("SmartOS metadata available locally, network not required");
            return Ok(false);
        } else {
            info!("SmartOS metadata not available locally, network may be required");
            return Ok(true);
        }
    }

    info!("No cloud environment detected, assuming network not required");
    Ok(false)
}

/// Apply configuration to sysconfig
async fn apply_config(
    client: &mut SysConfigServiceClient<Channel>,
    config: &ProvisioningConfig,
    dry_run: bool,
    _force: bool,
) -> Result<()> {
    let state = config_to_state(config);
    let state_str = serde_json::to_string(&state)?;

    if dry_run {
        println!("Would apply the following state to sysconfig:");
        println!("{}", serde_json::to_string_pretty(&state)?);
        return Ok(());
    }

    let request = ApplyStateRequest {
        state: state_str,
        dry_run,
    };

    let response = client.apply_state(request).await?;
    let resp = response.into_inner();

    if resp.success {
        info!("Successfully applied configuration to sysconfig");
        if !resp.changes.is_empty() {
            println!("Applied changes:");
            for change in resp.changes {
                println!("  - {:?}", change);
            }
        } else {
            println!("No changes were necessary");
        }
    } else {
        error!("Failed to apply configuration: {}", resp.error);
        return Err(anyhow::anyhow!(
            "Failed to apply configuration: {}",
            resp.error
        ));
    }

    Ok(())
}

/// Execute the apply command
async fn cmd_apply(
    socket: &str,
    config_path: Option<PathBuf>,
    sources: Option<String>,
    disable_sources: Option<String>,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let mut source_manager = SourceManager::new();

    // Parse disabled sources
    let disabled: Vec<String> = disable_sources
        .map(|s| s.split(',').map(String::from).collect())
        .unwrap_or_default();

    for source in &disabled {
        source_manager.disable_source(source);
    }

    // Parse enabled sources (if specified, only these are used)
    let enabled: Option<Vec<String>> = sources.map(|s| s.split(',').map(String::from).collect());

    let mut merger = ConfigMerger::new();
    let mut sources_loaded = Vec::new();

    // Load KDL config file if specified
    if let Some(ref path) = config_path {
        if path.exists() {
            info!("Loading config from: {}", path.display());
            // Use LocalSource to parse config (auto-detects format)
            let local_source = LocalSource::new();
            let config = local_source.load_any(&path).await?;
            merger.add_config(config, SourcePriority::LocalFile as u32);
            sources_loaded.push(format!(
                "local-{}",
                path.extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
            ));
        } else {
            return Err(anyhow::anyhow!("Config file not found: {}", path.display()));
        }
    }

    // Load from sources
    if let Some(enabled_sources) = enabled {
        // Only load from explicitly enabled sources
        for source in enabled_sources {
            if disabled.contains(&source) {
                warn!("Source {} is both enabled and disabled, skipping", source);
                continue;
            }

            info!("Loading from source: {}", source);
            match source_manager.load_from_source(&source).await {
                Ok(Some((config, priority))) => {
                    merger.add_config(config, priority);
                    sources_loaded.push(source);
                }
                Ok(None) => {
                    debug!("Source {} not available", source);
                }
                Err(e) => {
                    warn!("Failed to load from source {}: {}", source, e);
                }
            }
        }
    } else if config_path.is_none() {
        // No config file and no specific sources, try to auto-detect
        info!("Auto-detecting available sources");
        let available = source_manager.detect_available_sources().await;

        for (source, priority) in available {
            if disabled.contains(&source) {
                debug!("Skipping disabled source: {}", source);
                continue;
            }

            info!("Loading from detected source: {}", source);
            match source_manager.load_from_source(&source).await {
                Ok(Some((config, _))) => {
                    merger.add_config(config, priority);
                    sources_loaded.push(source);
                }
                Ok(None) => {
                    debug!("Source {} not available", source);
                }
                Err(e) => {
                    warn!("Failed to load from source {}: {}", source, e);
                }
            }
        }
    }

    if merger.is_empty() {
        return Err(anyhow::anyhow!("No configuration sources found or loaded"));
    }

    info!("Loaded configuration from sources: {:?}", sources_loaded);

    let merged_config = merger.merge();

    // Connect to sysconfig and apply
    let mut client = connect_to_sysconfig(socket).await?;
    apply_config(&mut client, &merged_config, dry_run, force).await?;

    Ok(())
}

/// Execute the autodetect command
async fn cmd_autodetect(
    socket: &str,
    check_network: bool,
    dry_run: bool,
    network_timeout: u64,
) -> Result<()> {
    info!("Starting auto-detection of provisioning sources");

    // Check if network is required
    if check_network {
        let needs_net = needs_network_for_provisioning().await?;
        if needs_net {
            println!("Network setup required for provisioning");
            println!("Please ensure network is configured before cloud provisioning");

            // We could optionally apply a minimal network config here
            // to enable DHCP on the primary interface
            if !dry_run {
                info!("Setting up minimal network configuration for provisioning");
                let minimal_config = ProvisioningConfig {
                    interfaces: {
                        let mut interfaces = HashMap::new();
                        interfaces.insert(
                            "net0".to_string(),
                            config::InterfaceConfig {
                                addresses: vec![config::AddressConfig {
                                    addr_type: config::AddressType::Dhcp4,
                                    address: None,
                                    gateway: None,
                                    primary: true,
                                }],
                                enabled: true,
                                mac_address: None,
                                mtu: None,
                                description: None,
                                vlan_id: None,
                                parent: None,
                            },
                        );
                        interfaces
                    },
                    ..Default::default()
                };

                let mut client = connect_to_sysconfig(socket).await?;
                apply_config(&mut client, &minimal_config, false, true).await?;

                // Wait a bit for network to come up
                println!("Waiting for network to initialize...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        } else {
            println!("No network setup required, using local sources");
        }
    }

    let mut source_manager = SourceManager::new();

    // Set network timeout
    source_manager.set_network_timeout(network_timeout);

    // Detect and load from all available sources
    info!("Detecting available provisioning sources");
    let available = source_manager.detect_available_sources().await;

    if available.is_empty() {
        return Err(anyhow::anyhow!("No provisioning sources detected"));
    }

    println!("Detected provisioning sources:");
    for (source, priority) in &available {
        println!("  - {} (priority: {})", source, priority);
    }

    let mut merger = ConfigMerger::new();
    let mut sources_loaded = Vec::new();

    for (source, priority) in available {
        info!("Loading from source: {}", source);
        match source_manager.load_from_source(&source).await {
            Ok(Some((config, _))) => {
                merger.add_config(config, priority);
                sources_loaded.push(source);
            }
            Ok(None) => {
                debug!("Source {} not available", source);
            }
            Err(e) => {
                warn!("Failed to load from source {}: {}", source, e);
            }
        }
    }

    if merger.is_empty() {
        return Err(anyhow::anyhow!("Failed to load any configuration"));
    }

    info!(
        "Successfully loaded configuration from: {:?}",
        sources_loaded
    );

    let merged_config = merger.merge();

    // Connect to sysconfig and apply
    let mut client = connect_to_sysconfig(socket).await?;
    apply_config(&mut client, &merged_config, dry_run, false).await?;

    Ok(())
}

/// Execute the parse command
async fn cmd_parse(config_path: PathBuf, format: String) -> Result<()> {
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Config file not found: {}",
            config_path.display()
        ));
    }

    // Use LocalSource to parse the config file (auto-detects format)
    let local_source = LocalSource::new();
    let config = local_source.load_any(&config_path).await?;
    let state = config_to_state(&config);

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string(&state)?);
        }
        "pretty" | _ => {
            println!("Parsed configuration from {}:", config_path.display());
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
    }

    Ok(())
}

/// Execute the detect command
async fn cmd_detect(check_network: bool, format: String) -> Result<()> {
    let source_manager = SourceManager::new();
    let mut sources = Vec::new();

    // Check local sources
    let local_checks = vec![
        ("/etc/sysconfig.kdl", "local-kdl"),
        ("/etc/provisioning.json", "local-json"),
        ("/mnt/provisioning/config.kdl", "mounted-kdl"),
    ];

    for (path, name) in local_checks {
        if PathBuf::from(path).exists() {
            sources.push((name.to_string(), "local".to_string()));
        }
    }

    // Check SmartOS
    if PathBuf::from("/usr/sbin/mdata-get").exists() {
        sources.push(("smartos".to_string(), "metadata-api".to_string()));
    }

    // Check cloud sources if network flag is set
    if check_network {
        let network_sources = check_network_sources(&source_manager).await;
        for source in network_sources {
            sources.push((source.clone(), "network".to_string()));
        }
    }

    match format.as_str() {
        "json" => {
            let output = json!({
                "sources": sources.iter().map(|(name, type_)| {
                    json!({
                        "name": name,
                        "type": type_
                    })
                }).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string(&output)?);
        }
        "pretty" | _ => {
            if sources.is_empty() {
                println!("No provisioning sources detected");
            } else {
                println!("Detected provisioning sources:");
                for (name, type_) in sources {
                    println!("  - {} ({})", name, type_);
                }
            }
        }
    }

    Ok(())
}

/// Execute the status command
async fn cmd_status(socket: &str, format: String) -> Result<()> {
    let mut client = connect_to_sysconfig(socket).await?;

    let request = GetStateRequest {
        path: String::new(),
    };

    let response = client.get_state(request).await?;
    let resp = response.into_inner();

    if !resp.state.is_empty() {
        let state: serde_json::Value = serde_json::from_str(&resp.state)?;

        // Extract provisioning-related state
        let provisioning_state = json!({
            "hostname": state.get("hostname"),
            "nameservers": state.get("nameservers"),
            "interfaces": state.get("interfaces"),
            "ssh_authorized_keys": state.get("ssh_authorized_keys"),
            "metadata": state.get("metadata"),
        });

        match format.as_str() {
            "json" => {
                println!("{}", serde_json::to_string(&provisioning_state)?);
            }
            "pretty" | _ => {
                println!("Current provisioning state in sysconfig:");
                println!("{}", serde_json::to_string_pretty(&provisioning_state)?);
            }
        }
    } else {
        println!("No state found in sysconfig");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.debug {
        "provisioning=debug,info"
    } else {
        "provisioning=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .init();

    match cli.command {
        Commands::Apply {
            config,
            sources,
            disable_sources,
            dry_run,
            force,
        } => {
            cmd_apply(
                &cli.socket,
                config,
                sources,
                disable_sources,
                dry_run,
                force,
            )
            .await
        }
        Commands::Autodetect {
            check_network,
            dry_run,
            network_timeout,
        } => cmd_autodetect(&cli.socket, check_network, dry_run, network_timeout).await,
        Commands::Parse { config, format } => cmd_parse(config, format).await,
        Commands::Detect { network, format } => cmd_detect(network, format).await,
        Commands::Status { format } => cmd_status(&cli.socket, format).await,
    }
}
