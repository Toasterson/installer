//! Provisioning Plugin for Sysconfig
//!
//! This plugin reads configuration from various cloud data sources (cloud-init,
//! EC2, GCP, Azure, local files) and converts them to the unified configuration
//! schema for processing by base plugins.

use clap::Parser;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use sysconfig::proto::{
    sys_config_service_client::SysConfigServiceClient, ApplyStateRequest,
};
use tonic::transport::Channel;

use sysconfig_plugins::provisioning::{
    collect_from_source, convert_to_unified_schema, merge_configurations,
    parse_data_sources, PrioritizedSource,
};

#[derive(Parser, Debug)]
#[command(name = "provisioning-plugin")]
#[command(about = "Cloud provisioning plugin for sysconfig")]
struct Args {
    /// Sysconfig daemon socket address
    #[arg(long, default_value = "http://[::1]:50051")]
    sysconfig_addr: String,

    /// Plugin name to register with sysconfig
    #[arg(long, default_value = "provisioning")]
    plugin_name: String,

    /// Configuration file path (for local source)
    #[arg(long)]
    config_file: Option<String>,

    /// Data source priorities (comma-separated list)
    #[arg(long, default_value = "local,cloud-init,ec2,gcp,azure")]
    sources: String,

    /// Cloud-init meta-data file path
    #[arg(long, default_value = "/var/lib/cloud/seed/nocloud-net/meta-data")]
    cloud_init_meta_data: String,

    /// Cloud-init user-data file path
    #[arg(long, default_value = "/var/lib/cloud/seed/nocloud-net/user-data")]
    cloud_init_user_data: String,

    /// Cloud-init network-config file path
    #[arg(long, default_value = "/var/lib/cloud/seed/nocloud-net/network-config")]
    cloud_init_network_config: String,

    /// Dry run mode - don't make actual changes
    #[arg(long)]
    dry_run: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Run once and exit (don't loop)
    #[arg(long)]
    run_once: bool,

    /// Interval between provisioning cycles in seconds
    #[arg(long, default_value = "300")]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!("Starting provisioning plugin");

    // Parse data sources
    let sources = parse_data_sources(
        &args.sources,
        args.config_file.as_deref(),
        &args.cloud_init_meta_data,
        &args.cloud_init_user_data,
        &args.cloud_init_network_config,
    )?;
    debug!("Configured data sources: {:?}", sources);

    // Connect to sysconfig daemon
    let channel = Channel::from_shared(args.sysconfig_addr.clone())?
        .connect()
        .await?;
    let mut client = SysConfigServiceClient::new(channel);

    info!("Connected to sysconfig daemon at {}", args.sysconfig_addr);

    // Run provisioning cycle(s)
    loop {
        match run_provisioning_cycle(&mut client, &args, &sources).await {
            Ok(_) => {
                info!("Provisioning cycle completed successfully");
            }
            Err(e) => {
                error!("Provisioning cycle failed: {}", e);
            }
        }

        if args.run_once {
            break;
        }

        // Wait before next cycle
        sleep(Duration::from_secs(args.interval)).await;
    }

    info!("Provisioning plugin shutting down");
    Ok(())
}

async fn run_provisioning_cycle(
    client: &mut SysConfigServiceClient<Channel>,
    args: &Args,
    sources: &[PrioritizedSource],
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting provisioning cycle");

    // Collect configuration from all sources
    let mut merged_config = serde_json::json!({});

    for source_config in sources {
        match collect_from_source(&source_config.source).await {
            Ok(config) => {
                if !config.is_null() {
                    info!(
                        "Collected configuration from source: {:?}",
                        source_config.source
                    );
                    debug!("Raw config from source: {}", serde_json::to_string_pretty(&config)?);
                    merge_configurations(&mut merged_config, config)?;
                } else {
                    debug!("No configuration found from source: {:?}", source_config.source);
                }
            }
            Err(e) => {
                warn!(
                    "Failed to collect from source {:?}: {}",
                    source_config.source, e
                );
                continue;
            }
        }
    }

    if merged_config.is_null() || merged_config.as_object().unwrap().is_empty() {
        info!("No configuration found from any source, skipping cycle");
        return Ok(());
    }

    debug!("Merged configuration: {}", serde_json::to_string_pretty(&merged_config)?);

    // Convert to unified schema and validate
    let unified_config = convert_to_unified_schema(merged_config)?;
    debug!("Unified configuration: {:#?}", unified_config);

    // Validate the configuration
    unified_config.validate().map_err(|e| format!("Configuration validation failed: {}", e))?;

    // Convert to JSON for sysconfig
    let unified_json = unified_config.to_json()?;
    debug!("Unified JSON: {}", unified_json);

    // Apply configuration via sysconfig
    apply_configuration(client, &args.plugin_name, unified_json, args.dry_run).await?;

    Ok(())
}

async fn apply_configuration(
    client: &mut SysConfigServiceClient<Channel>,
    _plugin_name: &str,
    config: String,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Applying configuration via sysconfig (dry_run: {})", dry_run);

    let request = ApplyStateRequest {
        state: config,
        dry_run,
    };

    let response = client.apply_state(request).await?;
    let result = response.into_inner();

    info!("Configuration applied successfully");

    if !result.changes.is_empty() {
        info!("Changes made:");
        for change in result.changes {
            let change_type = match change.r#type {
                0 => "CREATE",
                1 => "UPDATE",
                2 => "DELETE",
                _ => "UNKNOWN",
            };

            info!(
                "  {} {}: {} -> {}",
                change_type,
                change.path,
                change.old_value,
                change.new_value
            );
        }
    } else {
        info!("No changes required - system already in desired state");
    }

    Ok(())
}
