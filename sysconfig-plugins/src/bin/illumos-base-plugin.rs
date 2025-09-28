use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use sysconfig_plugins::tasks::files::Files;
use sysconfig_plugins::tasks::network_settings::NetworkSettings;
use sysconfig_plugins::TaskHandler;
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

// Local proto module generated from ../sysconfig/proto/sysconfig.proto
mod proto {
    tonic::include_proto!("sysconfig");
}
use proto::plugin_service_server::{PluginService, PluginServiceServer};
use proto::sys_config_service_client::SysConfigServiceClient;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Sysconfig base plugin: illumos", long_about = None)]
struct Args {
    /// Path to the Unix socket this plugin listens on
    #[clap(long)]
    socket: Option<String>,

    /// Path to the sysconfig service Unix socket to register with
    #[clap(long)]
    service_socket: Option<String>,

    /// Do not register with the sysconfig service automatically
    #[clap(long)]
    no_register: bool,
}

fn default_sysconfig_socket_path() -> String {
    if is_running_as_root() {
        "/var/run/sysconfig.sock".to_string()
    } else {
        // Use XDG_RUNTIME_DIR for non-root users
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/tmp/run-{}", unsafe { libc::geteuid() }));
        format!("{}/sysconfig.sock", runtime_dir)
    }
}

fn default_plugin_socket_path() -> String {
    if is_running_as_root() {
        "/var/run/sysconfig-illumos-base.sock".to_string()
    } else {
        // Use XDG_RUNTIME_DIR for non-root users
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/tmp/run-{}", unsafe { libc::geteuid() }));
        format!("{}/sysconfig-illumos-base.sock", runtime_dir)
    }
}

#[derive(Default)]
struct IllumosBasePlugin {
    inner: Arc<RwLock<PluginState>>,
}

#[derive(Default)]
struct PluginState {
    plugin_id: Option<String>,
    service_socket_path: Option<String>,
    auto_dry_run: bool,
}

/// Check if the process is running as root
fn is_running_as_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// Check if we should enable automatic dry-run mode
fn should_auto_enable_dry_run() -> bool {
    !is_running_as_root()
}

#[tonic::async_trait]
impl PluginService for IllumosBasePlugin {
    async fn initialize(
        &self,
        request: Request<proto::InitializeRequest>,
    ) -> Result<Response<proto::InitializeResponse>, Status> {
        let req = request.into_inner();
        let auto_dry_run = should_auto_enable_dry_run();
        {
            let mut st = self.inner.write().await;
            st.plugin_id = Some(req.plugin_id.clone());
            st.service_socket_path = Some(req.service_socket_path.clone());
            st.auto_dry_run = auto_dry_run;
        }

        if auto_dry_run {
            warn!(
                "Running as non-root user (UID: {}). Auto-enabling dry-run mode for testing.",
                unsafe { libc::geteuid() }
            );
            info!("All operations will be simulated and no actual changes will be made to the system.");
            info!(
                "Using XDG_RUNTIME_DIR for socket paths: {}",
                std::env::var("XDG_RUNTIME_DIR")
                    .unwrap_or_else(|_| format!("/tmp/run-{}", unsafe { libc::geteuid() }))
            );
        }

        info!(plugin_id = %req.plugin_id, service = %req.service_socket_path, auto_dry_run = %auto_dry_run, "illumos base plugin initialized");
        Ok(Response::new(proto::InitializeResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn get_config(
        &self,
        _request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::GetConfigResponse>, Status> {
        let auto_dry_run = self.inner.read().await.auto_dry_run;
        let config = serde_json::json!({
            "name": "illumos-base",
            "os": "illumos",
            "tasks": ["storage", "users", "packages", "services", "firewall", "files", "network.links", "network.settings"],
            "auto_dry_run": auto_dry_run,
            "running_as_root": is_running_as_root(),
        })
        .to_string();
        Ok(Response::new(proto::GetConfigResponse { config }))
    }

    async fn diff_state(
        &self,
        request: Request<proto::DiffStateRequest>,
    ) -> Result<Response<proto::DiffStateResponse>, Status> {
        let req = request.into_inner();
        let current: serde_json::Value =
            serde_json::from_str(&req.current_state).unwrap_or(serde_json::Value::Null);
        let desired: serde_json::Value = match serde_json::from_str(&req.desired_state) {
            Ok(v) => v,
            Err(_) => {
                return Ok(Response::new(proto::DiffStateResponse {
                    different: false,
                    changes: vec![],
                }));
            }
        };

        let mut task_changes: Vec<sysconfig_plugins::TaskChange> = Vec::new();

        if let Some(settings) = desired.get("network").and_then(|n| n.get("settings")) {
            let cur_settings = current
                .get("network")
                .and_then(|n| n.get("settings"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if let Ok(mut ch) = NetworkSettings::default().diff(&cur_settings, settings) {
                task_changes.append(&mut ch);
            }
        }
        if let Some(files) = desired.get("files") {
            if let Ok(mut ch) = Files::default().diff(&serde_json::Value::Null, files) {
                task_changes.append(&mut ch);
            }
        }

        let changes: Vec<proto::StateChange> = task_changes
            .into_iter()
            .map(|c| proto::StateChange {
                r#type: match c.change_type {
                    sysconfig_plugins::TaskChangeType::Create => proto::ChangeType::Create as i32,
                    sysconfig_plugins::TaskChangeType::Update => proto::ChangeType::Update as i32,
                    sysconfig_plugins::TaskChangeType::Delete => proto::ChangeType::Delete as i32,
                },
                path: c.path,
                old_value: c.old_value.map(|v| v.to_string()).unwrap_or_default(),
                new_value: c.new_value.map(|v| v.to_string()).unwrap_or_default(),
                verbose: c.verbose,
            })
            .collect();

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

        info!("DEBUG: apply_state called with state: {}", req.state);
        info!("DEBUG: apply_state dry_run flag: {}", req.dry_run);

        // Check if we should force dry-run mode
        let auto_dry_run = self.inner.read().await.auto_dry_run;
        let effective_dry_run = req.dry_run || auto_dry_run;

        info!("DEBUG: auto_dry_run: {}, effective_dry_run: {}", auto_dry_run, effective_dry_run);

        if auto_dry_run && !req.dry_run {
            info!("Auto-enabling dry-run mode since running as non-root user");
        }

        if effective_dry_run {
            info!("DRY-RUN MODE: Simulating state changes without applying them");
        }

        // Parse desired state and apply network.settings if present
        let desired: serde_json::Value = match serde_json::from_str(&req.state) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Response::new(proto::PluginApplyStateResponse {
                    success: false,
                    error: format!("invalid state JSON: {}", e),
                    changes: vec![],
                }));
            }
        };

        let mut task_changes: Vec<sysconfig_plugins::TaskChange> = Vec::new();
        if let Some(settings) = desired.get("network").and_then(|n| n.get("settings")) {
            if let Err(e) = NetworkSettings::validate_schema(settings) {
                return Ok(Response::new(proto::PluginApplyStateResponse {
                    success: false,
                    error: format!("invalid network.settings schema: {}", e),
                    changes: vec![],
                }));
            }

            if effective_dry_run {
                info!("DRY-RUN: Would apply network settings: {}", settings);
            }

            match NetworkSettings::default().apply(settings, effective_dry_run) {
                Ok(mut changes) => {
                    for change in &changes {
                        if effective_dry_run {
                            info!(
                                "DRY-RUN: Would {} {} - new value: {:?}",
                                change.change_type.as_str(),
                                change.path,
                                change.new_value
                            );
                        }
                    }
                    task_changes.append(&mut changes);
                }
                Err(e) => {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: e,
                        changes: vec![],
                    }));
                }
            }
        }

        // Apply files task if present at top-level: files: [ { ... } ]
        if let Some(files) = desired.get("files") {
            if effective_dry_run {
                info!("DRY-RUN: Would apply file changes: {}", files);
            }

            match Files::default().apply(files, effective_dry_run) {
                Ok(mut changes) => {
                    for change in &changes {
                        if effective_dry_run {
                            info!(
                                "DRY-RUN: Would {} {} - new value: {:?}",
                                change.change_type.as_str(),
                                change.path,
                                change.new_value
                            );
                        }
                    }
                    task_changes.append(&mut changes);
                }
                Err(e) => {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: e,
                        changes: vec![],
                    }));
                }
            }
        }

        let changes: Vec<proto::StateChange> = task_changes
            .into_iter()
            .map(|c| proto::StateChange {
                r#type: match c.change_type {
                    sysconfig_plugins::TaskChangeType::Create => proto::ChangeType::Create as i32,
                    sysconfig_plugins::TaskChangeType::Update => proto::ChangeType::Update as i32,
                    sysconfig_plugins::TaskChangeType::Delete => proto::ChangeType::Delete as i32,
                },
                path: c.path,
                old_value: c.old_value.map(|v| v.to_string()).unwrap_or_default(),
                new_value: c.new_value.map(|v| v.to_string()).unwrap_or_default(),
                verbose: c.verbose,
            })
            .collect();

        if effective_dry_run && !changes.is_empty() {
            info!(
                "DRY-RUN: Total of {} changes would be applied",
                changes.len()
            );
        }

        let resp = proto::PluginApplyStateResponse {
            success: true,
            error: String::new(),
            changes,
        };
        Ok(Response::new(resp))
    }

    async fn execute_action(
        &self,
        request: Request<proto::PluginExecuteActionRequest>,
    ) -> Result<Response<proto::PluginExecuteActionResponse>, Status> {
        let req = request.into_inner();
        let result = format!(
            "illumos-base executed action '{}' with params '{}'",
            req.action, req.parameters
        );
        Ok(Response::new(proto::PluginExecuteActionResponse {
            success: true,
            error: String::new(),
            result,
        }))
    }

    async fn notify_state_change(
        &self,
        _request: Request<proto::NotifyStateChangeRequest>,
    ) -> Result<Response<proto::NotifyStateChangeResponse>, Status> {
        Ok(Response::new(proto::NotifyStateChangeResponse {
            success: true,
            error: String::new(),
        }))
    }
}

async fn register_with_sysconfig(service_socket: String, plugin_socket: String) {
    use hyper_util::rt::TokioIo;
    use tokio::net::UnixStream;
    use tower::service_fn;

    let plugin_id = uuid::Uuid::new_v4().to_string();

    let endpoint = tonic::transport::Endpoint::from_static("http://[::]:50051");
    let sock_for_closure = service_socket.clone();
    let conn = endpoint
        .connect_with_connector(service_fn(move |_| {
            let path = sock_for_closure.clone();
            async move {
                let stream = UnixStream::connect(path).await?;
                Ok::<_, std::io::Error>(TokioIo::new(stream))
            }
        }))
        .await;

    let channel = match conn {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to connect to sysconfig service for registration");
            return;
        }
    };

    let mut client = SysConfigServiceClient::new(channel);
    let req = proto::RegisterPluginRequest {
        plugin_id: plugin_id.clone(),
        name: "illumos-base".to_string(),
        description: "Base plugin for illumos: storage, users, packages, services, firewall, files, network.links, network.settings".to_string(),
        socket_path: plugin_socket.to_string(),
        managed_paths: vec![
            "storage".to_string(),
            "users".to_string(),
            "packages".to_string(),
            "services".to_string(),
            "firewall".to_string(),
            "files".to_string(),
            "network.links".to_string(),
            "network.settings".to_string(),
        ],
    };
    match client.register_plugin(req).await {
        Ok(resp) => {
            let resp = resp.into_inner();
            if resp.success {
                info!(plugin_id = %plugin_id, "Registered illumos-base plugin with sysconfig service");
            } else {
                error!(error = %resp.error, "Plugin registration rejected by sysconfig service");
            }
        }
        Err(status) => {
            error!(code = ?status.code(), msg = %status.message(), "Plugin registration RPC failed");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let plugin_socket = args.socket.unwrap_or_else(default_plugin_socket_path);
    let service_socket = args
        .service_socket
        .unwrap_or_else(default_sysconfig_socket_path);

    // Ensure no stale socket exists
    let _ = std::fs::remove_file(&plugin_socket);
    if let Some(parent) = PathBuf::from(&plugin_socket).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = tokio::net::UnixListener::bind(&plugin_socket)?;
    let incoming = UnixListenerStream::new(listener);

    let plugin = IllumosBasePlugin::default();

    // Display startup information about dry-run mode
    if should_auto_enable_dry_run() {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/tmp/run-{}", unsafe { libc::geteuid() }));
        warn!("====================================================================");
        warn!(
            "TESTING MODE ACTIVATED - Running as non-root user (UID: {})",
            unsafe { libc::geteuid() }
        );
        warn!("All operations will be simulated (dry-run) and logged.");
        warn!("No actual system changes will be made.");
        warn!("Socket paths using: {}", runtime_dir);
        warn!("  - Plugin socket: {}", plugin_socket);
        warn!("  - Service socket: {}", service_socket);
        warn!("To run with actual changes, execute this plugin as root.");
        warn!("====================================================================");
    } else {
        info!("Running as root user - operations will make actual system changes");
    }

    info!(socket = %plugin_socket, "Starting illumos-base plugin server");

    // Optionally register with the sysconfig service
    if !args.no_register {
        let service_socket_clone = service_socket.clone();
        let plugin_socket_clone = plugin_socket.clone();
        tokio::spawn(async move {
            // small delay to ensure listener is up
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            register_with_sysconfig(service_socket_clone, plugin_socket_clone).await;
        });
    }

    tonic::transport::Server::builder()
        .add_service(PluginServiceServer::new(plugin))
        .serve_with_incoming(incoming)
        .await?;

    Ok(())
}
