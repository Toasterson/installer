use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;

use serde_json::{json, Value};
use std::io::{self, Read};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

// Include the generated proto code
pub mod proto {
    tonic::include_proto!("sysconfig");
}

use proto::{
    sys_config_service_client::SysConfigServiceClient, ApplyStateRequest, GetStateRequest,
    StateChange, WatchStateRequest,
};

// Import provisioning functions
use sysconfig_plugins::provisioning::{
    collect_from_source, convert_to_unified_schema, merge_configurations,
    parse_data_sources, PrioritizedSource,
};

/// Get the default socket path based on user permissions
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

/// CLI tool for interacting with the Sysconfig service
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the Unix socket for the Sysconfig service
    #[arg(short, long)]
    socket: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show configuration information including detected socket path
    Info,

    /// Get the current system state
    Get {
        /// Optional path to get a specific part of the state
        #[arg(short, long)]
        path: Option<String>,

        /// Output format: json (default) or pretty
        #[arg(short, long, default_value = "pretty")]
        format: String,
    },

    /// Apply a state diff to the system
    Apply {
        /// JSON file containing the state to apply
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Read state from stdin
        #[arg(short, long, conflicts_with = "file")]
        stdin: bool,

        /// Perform a dry run (validate but don't apply)
        #[arg(short = 'd', long)]
        dry_run: bool,

        /// Show verbose output for changes
        #[arg(short = 'v', long)]
        verbose: bool,
    },

    /// Set a specific value in the state using JSONPath
    Set {
        /// JSONPath expression (e.g., $.network.hostname)
        path: String,

        /// Value to set (as JSON)
        value: String,

        /// Perform a dry run (validate but don't apply)
        #[arg(short = 'd', long)]
        dry_run: bool,
    },

    /// Watch for state changes
    Watch {
        /// Optional path to watch for changes
        #[arg(short, long)]
        path: Option<String>,

        /// Output format: json or pretty
        #[arg(short, long, default_value = "pretty")]
        format: String,
    },

    /// Show the diff between current state and a new state
    Diff {
        /// JSON file containing the desired state
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Read state from stdin
        #[arg(short, long, conflicts_with = "file")]
        stdin: bool,
    },

    /// Cloud provisioning - collect config from data sources and apply
    Provision {
        /// Configuration file path (for local source)
        #[arg(short, long)]
        config_file: Option<String>,

        /// Data source priorities (comma-separated list: local,cloud-init,ec2,gcp,azure)
        #[arg(short, long, default_value = "local,cloud-init,ec2,gcp,azure")]
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
        #[arg(short = 'd', long)]
        dry_run: bool,

        /// Run once and exit (don't loop)
        #[arg(long)]
        run_once: bool,

        /// Interval between provisioning cycles in seconds
        #[arg(long, default_value = "300")]
        interval: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env()
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Use provided socket or compute default at runtime
    let socket_path = cli.socket.unwrap_or_else(default_socket_path);

    // Handle info command before connecting (it doesn't need a connection)
    if matches!(cli.command, Commands::Info) {
        return cmd_info(&socket_path);
    }

    // Connect to the Sysconfig service for other commands
    let mut client = connect_to_service(&socket_path).await?;

    match cli.command {
        Commands::Info => unreachable!(), // Already handled above
        Commands::Get { path, format } => cmd_get(&mut client, path.as_deref(), &format).await?,
        Commands::Apply {
            file,
            stdin,
            dry_run,
            verbose,
        } => {
            let state = read_state_input(file, stdin).await?;
            cmd_apply(&mut client, &state, dry_run, verbose).await?;
        }
        Commands::Set {
            path,
            value,
            dry_run,
        } => cmd_set(&mut client, &path, &value, dry_run).await?,
        Commands::Watch { path, format } => {
            cmd_watch(&mut client, path.as_deref(), &format).await?
        }
        Commands::Diff { file, stdin } => {
            let desired_state = read_state_input(file, stdin).await?;
            cmd_diff(&mut client, &desired_state).await?;
        }
        Commands::Provision {
            config_file,
            sources,
            cloud_init_meta_data,
            cloud_init_user_data,
            cloud_init_network_config,
            dry_run,
            run_once,
            interval,
        } => {
            cmd_provision(
                &mut client,
                config_file,
                &sources,
                &cloud_init_meta_data,
                &cloud_init_user_data,
                &cloud_init_network_config,
                dry_run,
                run_once,
                interval,
            )
            .await?;
        }
    }

    Ok(())
}

async fn connect_to_service(socket_path: &str) -> Result<SysConfigServiceClient<Channel>> {
    debug!("Connecting to Sysconfig service at {}", socket_path);

    let socket_path = socket_path.to_string();
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            let socket_path = socket_path.clone();
            async move {
                let stream = UnixStream::connect(&socket_path).await?;
                Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(stream))
            }
        }))
        .await
        .context("Failed to connect to Sysconfig service")?;

    Ok(SysConfigServiceClient::new(channel))
}

fn cmd_info(socket_path: &str) -> Result<()> {
    println!("{}", "Sysconfig CLI Configuration".cyan().bold());
    println!("{}", "===========================".cyan());
    println!();

    // Show detected socket path
    println!("{}: {}", "Socket Path".yellow(), socket_path.green());

    // Show how it was determined
    let euid = unsafe { libc::geteuid() as u32 };
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        println!(
            "{}: {} (from XDG_RUNTIME_DIR)",
            "Detection Method".yellow(),
            "XDG Runtime Directory".cyan()
        );
        println!("{}: {}", "XDG_RUNTIME_DIR".yellow(), xdg);
    } else if euid == 0 {
        println!("{}: {}", "Detection Method".yellow(), "Root User".cyan());
    } else {
        println!(
            "{}: {}",
            "Detection Method".yellow(),
            "User Runtime Directory".cyan()
        );
        println!("{}: {}", "User ID".yellow(), euid);
    }

    println!();
    println!(
        "{}: {}",
        "Current User".yellow(),
        std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
    );
    println!("{}: {}", "Effective UID".yellow(), euid);

    // Platform info
    println!();
    println!("{}: {}", "Platform".yellow(), std::env::consts::OS);
    println!("{}: {}", "Architecture".yellow(), std::env::consts::ARCH);

    println!();
    println!("{}", "Tips:".cyan().bold());
    println!("• The socket path is automatically detected based on your user permissions");
    println!("• Root users use: /var/run/sysconfig.sock");
    println!(
        "• Regular users use: $XDG_RUNTIME_DIR/sysconfig.sock or /run/user/$UID/sysconfig.sock"
    );
    println!("• You can override with: --socket /custom/path.sock");
    println!();
    println!(
        "To test the connection, run: {}",
        "sysconfig-cli get".green()
    );

    Ok(())
}

async fn cmd_get(
    client: &mut SysConfigServiceClient<Channel>,
    path: Option<&str>,
    format: &str,
) -> Result<()> {
    let request = GetStateRequest {
        path: path.unwrap_or("").to_string(),
    };

    let response = client
        .get_state(request)
        .await
        .context("Failed to get state")?;

    let state: Value =
        serde_json::from_str(&response.into_inner().state).context("Failed to parse state JSON")?;

    match format {
        "json" => println!("{}", serde_json::to_string(&state)?),
        "pretty" => println!("{}", serde_json::to_string_pretty(&state)?),
        _ => anyhow::bail!("Invalid format: {}", format),
    }

    Ok(())
}

async fn cmd_apply(
    client: &mut SysConfigServiceClient<Channel>,
    state: &str,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    // Validate JSON
    let _: Value = serde_json::from_str(state).context("Invalid JSON state")?;

    let request = ApplyStateRequest {
        state: state.to_string(),
        dry_run,
    };

    let response = client
        .apply_state(request)
        .await
        .context("Failed to apply state")?;
    let response = response.into_inner();

    if !response.success {
        error!("Failed to apply state: {}", response.error);
        return Err(anyhow::anyhow!(response.error));
    }

    if response.changes.is_empty() {
        println!("{}", "No changes required".green());
    } else {
        println!("{}", format!("{} changes:", response.changes.len()).cyan());
        for change in response.changes {
            print_change(&change, verbose);
        }
    }

    if dry_run {
        println!(
            "\n{}",
            "DRY RUN - No changes were actually applied".yellow()
        );
    } else {
        println!("\n{}", "Changes applied successfully".green());
    }

    Ok(())
}

async fn cmd_set(
    client: &mut SysConfigServiceClient<Channel>,
    path: &str,
    value: &str,
    dry_run: bool,
) -> Result<()> {
    // First, get the current state
    let get_request = GetStateRequest {
        path: String::new(),
    };

    let response = client
        .get_state(get_request)
        .await
        .context("Failed to get current state")?;

    let mut current_state: Value = serde_json::from_str(&response.into_inner().state)
        .context("Failed to parse current state")?;

    // Parse the value as JSON
    let new_value: Value = serde_json::from_str(value)
        .or_else(|_| Ok::<Value, serde_json::Error>(Value::String(value.to_string())))
        .context("Failed to parse value")?;

    // Apply the JSONPath update
    let modified_state = apply_jsonpath_set(&mut current_state, path, new_value.clone())?;

    // Show what will change
    println!("Setting {} to {}", path.cyan(), value.yellow());

    // Apply the modified state
    let apply_request = ApplyStateRequest {
        state: serde_json::to_string(&modified_state)?,
        dry_run,
    };

    let response = client
        .apply_state(apply_request)
        .await
        .context("Failed to apply state")?;
    let response = response.into_inner();

    if !response.success {
        error!("Failed to set value: {}", response.error);
        return Err(anyhow::anyhow!(response.error));
    }

    if response.changes.is_empty() {
        println!("{}", "No changes required (value already set)".green());
    } else {
        println!("{}", "Changes:".cyan());
        for change in response.changes {
            print_change(&change, true);
        }
    }

    if dry_run {
        println!(
            "\n{}",
            "DRY RUN - No changes were actually applied".yellow()
        );
    } else {
        println!("\n{}", "Value set successfully".green());
    }

    Ok(())
}

async fn cmd_watch(
    client: &mut SysConfigServiceClient<Channel>,
    path: Option<&str>,
    format: &str,
) -> Result<()> {
    let request = WatchStateRequest {
        path: path.unwrap_or("").to_string(),
    };

    println!("{}", "Watching for state changes...".cyan());
    if let Some(p) = path {
        println!("Path filter: {}", p.yellow());
    }
    println!("{}", "Press Ctrl+C to stop".dimmed());
    println!();

    let mut stream = client
        .watch_state(request)
        .await
        .context("Failed to watch state")?
        .into_inner();

    while let Some(event) = stream
        .message()
        .await
        .context("Error receiving state change event")?
    {
        let timestamp =
            chrono::DateTime::from_timestamp(event.timestamp, 0).unwrap_or_else(chrono::Utc::now);

        match format {
            "json" => {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "timestamp": timestamp.to_rfc3339(),
                        "path": event.path,
                        "value": serde_json::from_str::<Value>(&event.value).unwrap_or(Value::String(event.value.clone())),
                        "plugin_id": event.plugin_id,
                    }))?
                );
            }
            "pretty" => {
                println!(
                    "{} {} changed by {}",
                    format!("[{}]", timestamp.format("%H:%M:%S")).dimmed(),
                    event.path.cyan(),
                    event.plugin_id.yellow()
                );
                if !event.value.is_empty() {
                    if let Ok(value) = serde_json::from_str::<Value>(&event.value) {
                        println!("  New value: {}", serde_json::to_string_pretty(&value)?);
                    } else {
                        println!("  New value: {}", event.value);
                    }
                }
                println!();
            }
            _ => anyhow::bail!("Invalid format: {}", format),
        }
    }

    Ok(())
}

async fn cmd_diff(client: &mut SysConfigServiceClient<Channel>, desired_state: &str) -> Result<()> {
    // Get current state
    let get_request = GetStateRequest {
        path: String::new(),
    };

    let response = client
        .get_state(get_request)
        .await
        .context("Failed to get current state")?;

    let current_state: Value = serde_json::from_str(&response.into_inner().state)
        .context("Failed to parse current state")?;

    let desired: Value =
        serde_json::from_str(desired_state).context("Failed to parse desired state")?;

    // Show a simple diff (in a real implementation, you might want to use a proper diff library)
    println!("{}", "Current State:".cyan());
    println!("{}", serde_json::to_string_pretty(&current_state)?);
    println!();
    println!("{}", "Desired State:".cyan());
    println!("{}", serde_json::to_string_pretty(&desired)?);
    println!();

    // Apply as dry-run to see what would change
    let apply_request = ApplyStateRequest {
        state: desired_state.to_string(),
        dry_run: true,
    };

    let response = client
        .apply_state(apply_request)
        .await
        .context("Failed to compute diff")?;
    let response = response.into_inner();

    if response.changes.is_empty() {
        println!("{}", "States are identical".green());
    } else {
        println!(
            "{}",
            format!("{} changes would be made:", response.changes.len()).cyan()
        );
        for change in response.changes {
            print_change(&change, true);
        }
    }

    Ok(())
}

fn print_change(change: &StateChange, verbose: bool) {
    let change_type = match change.r#type {
        0 => "CREATE".green(),
        1 => "UPDATE".yellow(),
        2 => "DELETE".red(),
        _ => "UNKNOWN".bright_black(),
    };

    print!("  {} {}", change_type, change.path.cyan());

    if verbose || !change.verbose {
        match change.r#type {
            0 => {
                // CREATE
                if !change.new_value.is_empty() {
                    if let Ok(value) = serde_json::from_str::<Value>(&change.new_value) {
                        print!(" => {}", format_json_compact(&value));
                    } else {
                        print!(" => {}", change.new_value);
                    }
                }
            }
            1 => {
                // UPDATE
                if !change.old_value.is_empty() && !change.new_value.is_empty() {
                    let old = serde_json::from_str::<Value>(&change.old_value)
                        .map(|v| format_json_compact(&v))
                        .unwrap_or_else(|_| change.old_value.clone());
                    let new = serde_json::from_str::<Value>(&change.new_value)
                        .map(|v| format_json_compact(&v))
                        .unwrap_or_else(|_| change.new_value.clone());
                    print!(" {} => {}", old.red(), new.green());
                }
            }
            2 => {
                // DELETE
                if !change.old_value.is_empty() {
                    if let Ok(value) = serde_json::from_str::<Value>(&change.old_value) {
                        print!(" (was: {})", format_json_compact(&value).red());
                    } else {
                        print!(" (was: {})", change.old_value.red());
                    }
                }
            }
            _ => {}
        }
    } else if change.verbose {
        print!(" {}", "(verbose content hidden, use -v to show)".dimmed());
    }

    println!();
}

fn format_json_compact(value: &Value) -> String {
    match value {
        Value::String(s) if s.len() <= 50 => format!("\"{}\"", s),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(arr) if arr.len() <= 3 => {
            serde_json::to_string(value).unwrap_or_else(|_| "[...]".to_string())
        }
        Value::Object(obj) if obj.len() <= 2 => {
            serde_json::to_string(value).unwrap_or_else(|_| "{...}".to_string())
        }
        _ => {
            let s = serde_json::to_string(value).unwrap_or_else(|_| "...".to_string());
            if s.len() > 80 {
                format!("{}...", &s[..77])
            } else {
                s
            }
        }
    }
}

async fn read_state_input(file: Option<PathBuf>, stdin: bool) -> Result<String> {
    if stdin {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        Ok(buffer)
    } else if let Some(file) = file {
        tokio::fs::read_to_string(&file)
            .await
            .context(format!("Failed to read file: {:?}", file))
    } else {
        Err(anyhow::anyhow!(
            "Must specify either --file or --stdin for input"
        ))
    }
}

fn apply_jsonpath_set(current_state: &Value, path: &str, new_value: Value) -> Result<Value> {
    // Clone the current state to work with
    let mut state = current_state.clone();

    // Convert JSONPath to a simple dot notation for basic paths
    // Handle both $.path.to.value and path.to.value formats
    let path = path.trim_start_matches("$.");
    let path = path.trim_start_matches('$');

    // Split the path into parts
    let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();

    if parts.is_empty() {
        return Err(anyhow::anyhow!("Invalid JSONPath: empty path"));
    }

    // Ensure the state is an object at the root level
    if !state.is_object() {
        state = json!({});
    }

    // Navigate to the parent of the target
    let mut current = &mut state;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - set the value
            if let Value::Object(map) = current {
                map.insert(part.to_string(), new_value.clone());
            } else {
                // This shouldn't happen if we've been creating objects along the way
                return Err(anyhow::anyhow!("Cannot set value on non-object"));
            }
        } else {
            // Navigate deeper, creating objects as needed
            if !current.is_object() {
                *current = json!({});
            }

            if let Value::Object(map) = current {
                // Create the next level if it doesn't exist
                if !map.contains_key(*part) {
                    map.insert(part.to_string(), json!({}));
                }
                current = map.get_mut(*part).unwrap();
            }
        }
    }

    Ok(state)
}

async fn cmd_provision(
    client: &mut SysConfigServiceClient<Channel>,
    config_file: Option<String>,
    sources: &str,
    cloud_init_meta_data: &str,
    cloud_init_user_data: &str,
    cloud_init_network_config: &str,
    dry_run: bool,
    run_once: bool,
    interval: u64,
) -> Result<()> {
    use tokio::time::{sleep, Duration};

    println!("{}", "Starting provisioning...".cyan());

    // Parse data sources
    let sources = parse_data_sources(
        sources,
        config_file.as_deref(),
        cloud_init_meta_data,
        cloud_init_user_data,
        cloud_init_network_config,
    )
    .map_err(|e| anyhow::anyhow!("Failed to parse data sources: {}", e))?;

    debug!("Configured data sources: {:?}", sources);

    // Main provisioning loop
    loop {
        match run_provisioning_cycle(client, &sources, dry_run).await {
            Ok(_) => {
                println!("{}", "Provisioning cycle completed successfully".green());
            }
            Err(e) => {
                eprintln!("{}: {}", "Provisioning cycle failed".red(), e);
            }
        }

        if run_once {
            break;
        }

        // Wait before next cycle
        println!(
            "{}",
            format!("Waiting {} seconds before next cycle...", interval).yellow()
        );
        sleep(Duration::from_secs(interval)).await;
    }

    Ok(())
}

async fn run_provisioning_cycle(
    client: &mut SysConfigServiceClient<Channel>,
    sources: &[PrioritizedSource],
    dry_run: bool,
) -> Result<()> {

    println!("{}", "Starting provisioning cycle".blue());

    // Collect configuration from all sources
    let mut merged_config = serde_json::json!({});

    for source_config in sources {
        match collect_from_source(&source_config.source).await {
            Ok(config) => {
                if !config.is_null() {
                    println!(
                        "{}",
                        format!("Collected configuration from source: {:?}", source_config.source).green()
                    );
                    debug!("Raw config from source: {}", serde_json::to_string_pretty(&config)?);
                    merge_configurations(&mut merged_config, config)
                        .map_err(|e| anyhow::anyhow!("Failed to merge configurations: {}", e))?;
                } else {
                    debug!("No configuration found from source: {:?}", source_config.source);
                }
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    format!("Failed to collect from source {:?}: {}", source_config.source, e).yellow()
                );
                continue;
            }
        }
    }

    if merged_config.is_null() || merged_config.as_object().unwrap().is_empty() {
        println!("{}", "No configuration found from any source, skipping cycle".yellow());
        return Ok(());
    }

    debug!("Merged configuration: {}", serde_json::to_string_pretty(&merged_config)?);

    // Convert to unified schema and validate
    let unified_config = convert_to_unified_schema(merged_config)
        .map_err(|e| anyhow::anyhow!("Failed to convert to unified schema: {}", e))?;

    debug!("Unified configuration: {:#?}", unified_config);

    // Validate the configuration
    unified_config.validate()
        .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;

    // Convert to JSON for sysconfig
    let unified_json = unified_config.to_json()
        .context("Failed to serialize unified config")?;

    debug!("Unified JSON: {}", unified_json);

    // Apply configuration via sysconfig
    let request = proto::ApplyStateRequest {
        state: unified_json,
        dry_run,
    };

    let response = client
        .apply_state(request)
        .await
        .context("Failed to apply configuration")?;

    let result = response.into_inner();

    if result.error.is_empty() {
        println!("{}", "Configuration applied successfully".green());

        if !result.changes.is_empty() {
            println!("{}", "Changes made:".cyan());
            for change in result.changes {
                let change_type = match change.r#type {
                    0 => "CREATE",
                    1 => "UPDATE",
                    2 => "DELETE",
                    _ => "UNKNOWN",
                };

                println!(
                    "  {} {}: {} -> {}",
                    change_type.bold(),
                    change.path,
                    if change.old_value.is_empty() { "none".dimmed().to_string() } else { change.old_value },
                    if change.new_value.is_empty() { "none".dimmed().to_string() } else { change.new_value }
                );
            }
        } else {
            println!("{}", "No changes required - system already in desired state".green());
        }
    } else {
        return Err(anyhow::anyhow!("Failed to apply configuration: {}", result.error));
    }

    Ok(())
}
