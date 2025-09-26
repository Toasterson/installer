use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

mod config;
mod merger;
mod sources;
mod proto {
    tonic::include_proto!("sysconfig");
}

use proto::plugin_service_server::{PluginService, PluginServiceServer};
use proto::sys_config_service_client::SysConfigServiceClient;
use proto::{ChangeType, RegisterPluginRequest, StateChange};

use crate::config::ProvisioningConfig;
use crate::merger::ConfigMerger;
use crate::sources::SourceManager;

#[derive(Parser, Debug)]
#[clap(
    name = "provisioning-plugin",
    version = env!("CARGO_PKG_VERSION"),
    author = "illumos installer team",
    about = "Multi-source provisioning plugin for sysconfig"
)]
struct Args {
    /// Path to the Unix socket this plugin listens on
    #[clap(long, default_value = "/var/run/sysconfig-provisioning.sock")]
    socket: String,

    /// Path to the sysconfig service Unix socket
    #[clap(long, default_value = "/var/run/sysconfig.sock")]
    service_socket: String,

    /// Do not register with the sysconfig service automatically
    #[clap(long)]
    no_register: bool,

    /// Path to the local sysconfig.kdl file
    #[clap(long, default_value = "/etc/sysconfig.kdl")]
    config_file: PathBuf,

    /// Enable debug logging
    #[clap(long)]
    debug: bool,

    /// Disable specific sources (comma-separated)
    #[clap(long)]
    disable_sources: Option<String>,

    /// Force reload configuration from all sources
    #[clap(long)]
    force_reload: bool,
}

struct ProvisioningPlugin {
    state: Arc<RwLock<PluginState>>,
}

struct PluginState {
    config: ProvisioningConfig,
    source_manager: SourceManager,
    config_file: PathBuf,
    sources_loaded: Vec<String>,
}

impl ProvisioningPlugin {
    pub fn new(config_file: PathBuf, disabled_sources: Vec<String>) -> Self {
        let source_manager = SourceManager::new(disabled_sources);

        Self {
            state: Arc::new(RwLock::new(PluginState {
                config: ProvisioningConfig::default(),
                source_manager,
                config_file,
                sources_loaded: Vec::new(),
            })),
        }
    }

    pub async fn load_configuration(&self, force: bool) -> Result<()> {
        let mut state = self.state.write().await;

        // Skip if already loaded and not forcing
        if !state.sources_loaded.is_empty() && !force {
            info!("Configuration already loaded, skipping reload");
            return Ok(());
        }

        info!("Loading configuration from all available sources...");
        state.sources_loaded.clear();

        let mut merger = ConfigMerger::new();

        // Priority 1: Local configuration file
        if state.config_file.exists() {
            match state
                .source_manager
                .load_local_kdl(&state.config_file)
                .await
            {
                Ok(config) => {
                    info!("Loaded configuration from {:?}", state.config_file);
                    merger.add_config(config, 1);
                    state.sources_loaded.push("local".to_string());
                }
                Err(e) => {
                    warn!("Failed to load local configuration: {}", e);
                }
            }
        }

        // Priority 2: Cloud-init sources
        match state.source_manager.load_cloud_init().await {
            Ok(config) => {
                info!("Loaded cloud-init configuration");
                merger.add_config(config, 2);
                state.sources_loaded.push("cloud-init".to_string());
            }
            Err(e) => {
                debug!("No cloud-init configuration found: {}", e);
            }
        }

        // Priority 3: Cloud vendor metadata
        match state.source_manager.detect_and_load_cloud_vendor().await {
            Ok((vendor, config)) => {
                info!("Loaded configuration from cloud vendor: {}", vendor);
                merger.add_config(config, 3);
                state.sources_loaded.push(vendor);
            }
            Err(e) => {
                debug!("No cloud vendor metadata found: {}", e);
            }
        }

        // Merge all configurations
        state.config = merger.merge();

        info!(
            "Configuration loaded from {} source(s): {:?}",
            state.sources_loaded.len(),
            state.sources_loaded
        );

        Ok(())
    }

    async fn generate_state_json(&self) -> Result<serde_json::Value> {
        let state = self.state.read().await;
        let config = &state.config;

        let mut json = serde_json::json!({});

        // Add hostname
        if let Some(hostname) = &config.hostname {
            json["hostname"] = serde_json::json!(hostname);
        }

        // Add nameservers
        if !config.nameservers.is_empty() {
            json["nameservers"] = serde_json::json!(config.nameservers);
        }

        // Add search domains
        if !config.search_domains.is_empty() {
            json["search_domains"] = serde_json::json!(config.search_domains);
        }

        // Add interfaces
        if !config.interfaces.is_empty() {
            json["interfaces"] = serde_json::json!(config.interfaces);
        }

        // Add SSH keys
        if !config.ssh_authorized_keys.is_empty() {
            json["ssh_authorized_keys"] = serde_json::json!(config.ssh_authorized_keys);
        }

        // Add metadata
        if !config.metadata.is_empty() {
            json["metadata"] = serde_json::json!(config.metadata);
        }

        Ok(json)
    }

    fn diff_json_values(
        &self,
        path: String,
        old: &serde_json::Value,
        new: &serde_json::Value,
        changes: &mut Vec<StateChange>,
    ) {
        if old == new {
            return;
        }

        match (old, new) {
            (serde_json::Value::Object(old_map), serde_json::Value::Object(new_map)) => {
                // Check for added/modified keys
                for (key, new_value) in new_map {
                    let sub_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if let Some(old_value) = old_map.get(key) {
                        self.diff_json_values(sub_path, old_value, new_value, changes);
                    } else {
                        changes.push(StateChange {
                            r#type: ChangeType::Create as i32,
                            path: sub_path,
                            old_value: "".to_string(),
                            new_value: serde_json::to_string(new_value).unwrap_or_default(),
                            verbose: false,
                        });
                    }
                }

                // Check for removed keys
                for (key, old_value) in old_map {
                    if !new_map.contains_key(key) {
                        let sub_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };

                        changes.push(StateChange {
                            r#type: ChangeType::Delete as i32,
                            path: sub_path,
                            old_value: serde_json::to_string(old_value).unwrap_or_default(),
                            new_value: "".to_string(),
                            verbose: false,
                        });
                    }
                }
            }
            _ => {
                let change_type = if old == &serde_json::Value::Null {
                    ChangeType::Create
                } else if new == &serde_json::Value::Null {
                    ChangeType::Delete
                } else {
                    ChangeType::Update
                };

                changes.push(StateChange {
                    r#type: change_type as i32,
                    path,
                    old_value: serde_json::to_string(old).unwrap_or_default(),
                    new_value: serde_json::to_string(new).unwrap_or_default(),
                    verbose: false,
                });
            }
        }
    }
}

#[tonic::async_trait]
impl PluginService for ProvisioningPlugin {
    async fn initialize(
        &self,
        request: Request<proto::InitializeRequest>,
    ) -> Result<Response<proto::InitializeResponse>, Status> {
        let req = request.into_inner();
        info!("Provisioning plugin initialized with ID: {}", req.plugin_id);

        // Load configuration on initialization
        if let Err(e) = self.load_configuration(false).await {
            error!("Failed to load configuration: {}", e);
        }

        Ok(Response::new(proto::InitializeResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn get_config(
        &self,
        _request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::GetConfigResponse>, Status> {
        let state_json = self
            .generate_state_json()
            .await
            .map_err(|e| Status::internal(format!("Failed to generate state: {}", e)))?;

        let config = serde_json::to_string(&state_json)
            .map_err(|e| Status::internal(format!("Failed to serialize config: {}", e)))?;

        Ok(Response::new(proto::GetConfigResponse { config }))
    }

    async fn diff_state(
        &self,
        request: Request<proto::DiffStateRequest>,
    ) -> Result<Response<proto::DiffStateResponse>, Status> {
        let req = request.into_inner();

        let current: serde_json::Value = serde_json::from_str(&req.current_state)
            .map_err(|e| Status::invalid_argument(format!("Invalid current state: {}", e)))?;
        let desired: serde_json::Value = serde_json::from_str(&req.desired_state)
            .map_err(|e| Status::invalid_argument(format!("Invalid desired state: {}", e)))?;

        let mut changes = Vec::new();
        self.diff_json_values(String::new(), &current, &desired, &mut changes);

        Ok(Response::new(proto::DiffStateResponse {
            different: !changes.is_empty(),
            changes,
        }))
    }

    async fn apply_state(
        &self,
        request: Request<proto::PluginApplyStateRequest>,
    ) -> Result<Response<proto::PluginApplyStateResponse>, Status> {
        let req = request.into_inner();

        if req.dry_run {
            // Just validate the state
            let _: serde_json::Value = serde_json::from_str(&req.state)
                .map_err(|e| Status::invalid_argument(format!("Invalid state: {}", e)))?;

            Ok(Response::new(proto::PluginApplyStateResponse {
                success: true,
                error: String::new(),
                changes: Vec::new(),
            }))
        } else {
            // In a real implementation, we would apply the state here
            // For now, we just acknowledge it
            Ok(Response::new(proto::PluginApplyStateResponse {
                success: true,
                error: String::new(),
                changes: Vec::new(),
            }))
        }
    }

    async fn execute_action(
        &self,
        request: Request<proto::PluginExecuteActionRequest>,
    ) -> Result<Response<proto::PluginExecuteActionResponse>, Status> {
        let req = request.into_inner();

        match req.action.as_str() {
            "reload" => {
                info!("Reloading configuration from all sources");

                match self.load_configuration(true).await {
                    Ok(_) => {
                        let state = self.state.read().await;
                        let result = format!(
                            "Configuration reloaded from {} sources: {:?}",
                            state.sources_loaded.len(),
                            state.sources_loaded
                        );

                        Ok(Response::new(proto::PluginExecuteActionResponse {
                            success: true,
                            error: String::new(),
                            result,
                        }))
                    }
                    Err(e) => {
                        let error = format!("Failed to reload configuration: {}", e);
                        Ok(Response::new(proto::PluginExecuteActionResponse {
                            success: false,
                            error,
                            result: String::new(),
                        }))
                    }
                }
            }
            "status" => {
                let state = self.state.read().await;
                let result = serde_json::json!({
                    "sources_loaded": state.sources_loaded,
                    "config_file": state.config_file,
                    "has_hostname": state.config.hostname.is_some(),
                    "nameserver_count": state.config.nameservers.len(),
                    "interface_count": state.config.interfaces.len(),
                    "ssh_key_count": state.config.ssh_authorized_keys.len(),
                });

                Ok(Response::new(proto::PluginExecuteActionResponse {
                    success: true,
                    error: String::new(),
                    result: serde_json::to_string(&result).unwrap_or_default(),
                }))
            }
            _ => Err(Status::unimplemented(format!(
                "Unknown action: {}",
                req.action
            ))),
        }
    }

    async fn notify_state_change(
        &self,
        _request: Request<proto::NotifyStateChangeRequest>,
    ) -> Result<Response<proto::NotifyStateChangeResponse>, Status> {
        // We don't need to react to state changes for provisioning
        Ok(Response::new(proto::NotifyStateChangeResponse {
            success: true,
            error: String::new(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.debug {
        "provisioning_plugin=debug,info"
    } else {
        "provisioning_plugin=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .init();

    info!(
        "Starting sysconfig provisioning plugin v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Parse disabled sources
    let disabled_sources = args
        .disable_sources
        .map(|s| s.split(',').map(String::from).collect())
        .unwrap_or_default();

    // Create the plugin
    let plugin = ProvisioningPlugin::new(args.config_file.clone(), disabled_sources);

    // Force reload if requested
    if args.force_reload {
        plugin.load_configuration(true).await?;
    }

    // Register with sysconfig service if not disabled
    if !args.no_register {
        let endpoint = format!("unix://{}", args.service_socket);
        match SysConfigServiceClient::connect(endpoint).await {
            Ok(mut client) => {
                let request = RegisterPluginRequest {
                    plugin_id: "provisioning".to_string(),
                    name: "Provisioning Plugin".to_string(),
                    description: "Multi-source configuration provisioning plugin".to_string(),
                    socket_path: args.socket.clone(),
                    managed_paths: vec![
                        "hostname".to_string(),
                        "nameservers".to_string(),
                        "search_domains".to_string(),
                        "interfaces".to_string(),
                        "ssh_authorized_keys".to_string(),
                        "metadata".to_string(),
                    ],
                };

                match client.register_plugin(request).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        if resp.success {
                            info!("Successfully registered with sysconfig service");
                        } else {
                            error!("Failed to register with sysconfig: {}", resp.error);
                        }
                    }
                    Err(e) => {
                        error!("Failed to call register_plugin: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to connect to sysconfig service: {}", e);
                if !args.no_register {
                    warn!("Continuing without registration. Use --no-register to suppress this warning.");
                }
            }
        }
    }

    // Clean up any existing socket
    let _ = std::fs::remove_file(&args.socket);

    // Create parent directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&args.socket).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Bind to Unix socket
    let uds = tokio::net::UnixListener::bind(&args.socket)?;
    let uds_stream = UnixListenerStream::new(uds);

    info!("Listening on socket: {}", args.socket);

    // Create the service
    let service = PluginServiceServer::new(plugin);

    // Start serving
    tonic::transport::Server::builder()
        .add_service(service)
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}
