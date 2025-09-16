use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};
use tracing::{error, info};
use sysconfig_plugins::tasks::network_settings::NetworkSettings;
use sysconfig_plugins::tasks::files::Files;
use sysconfig_plugins::TaskHandler;

// Local proto module generated from ../sysconfig/proto/sysconfig.proto
mod proto {
    tonic::include_proto!("sysconfig");
}
use proto::plugin_service_server::{PluginService, PluginServiceServer};
use proto::sys_config_service_client::SysConfigServiceClient;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Sysconfig base plugin: Linux", long_about = None)]
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

fn default_plugin_socket_path() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
            return format!("{}/sysconfig-linux-base.sock", dir);
        }
        let euid = unsafe { libc::geteuid() as u32 };
        if euid == 0 {
            "/var/run/sysconfig-linux-base.sock".to_string()
        } else {
            format!("/run/user/{}/sysconfig-linux-base.sock", euid)
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        "/var/run/sysconfig-linux-base.sock".to_string()
    }
}

#[derive(Default)]
struct LinuxBasePlugin {
    inner: Arc<RwLock<PluginState>>, 
}

#[derive(Default)]
struct PluginState {
    plugin_id: Option<String>,
    service_socket_path: Option<String>,
}

#[tonic::async_trait]
impl PluginService for LinuxBasePlugin {
    async fn initialize(
        &self,
        request: Request<proto::InitializeRequest>,
    ) -> Result<Response<proto::InitializeResponse>, Status> {
        let req = request.into_inner();
        {
            let mut st = self.inner.write().await;
            st.plugin_id = Some(req.plugin_id.clone());
            st.service_socket_path = Some(req.service_socket_path.clone());
        }
        info!(plugin_id = %req.plugin_id, service = %req.service_socket_path, "Linux base plugin initialized");
        Ok(Response::new(proto::InitializeResponse { success: true, error: String::new() }))
    }

    async fn get_config(
        &self,
        _request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::GetConfigResponse>, Status> {
        let config = serde_json::json!({
            "name": "linux-base",
            "os": "linux",
            "tasks": ["storage", "users", "packages", "services", "firewall", "files", "network.links", "network.settings"],
        })
        .to_string();
        Ok(Response::new(proto::GetConfigResponse { config }))
    }

    async fn diff_state(
        &self,
        request: Request<proto::DiffStateRequest>,
    ) -> Result<Response<proto::DiffStateResponse>, Status> {
        let req = request.into_inner();
        let current: serde_json::Value = serde_json::from_str(&req.current_state).unwrap_or(serde_json::Value::Null);
        let desired: serde_json::Value = match serde_json::from_str(&req.desired_state) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Response::new(proto::DiffStateResponse {
                    different: false,
                    changes: vec![],
                }));
            }
        };

        let mut task_changes: Vec<sysconfig_plugins::TaskChange> = Vec::new();

        if let Some(settings) = desired.get("network").and_then(|n| n.get("settings")) {
            let cur_settings = current.get("network").and_then(|n| n.get("settings")).cloned().unwrap_or(serde_json::Value::Null);
            match NetworkSettings::default().diff(&cur_settings, settings) {
                Ok(mut ch) => task_changes.append(&mut ch),
                Err(_) => {}
            }
        }

        if let Some(files) = desired.get("files") {
            match Files::default().diff(&serde_json::Value::Null, files) {
                Ok(mut ch) => task_changes.append(&mut ch),
                Err(_) => {}
            }
        }

        let changes: Vec<proto::StateChange> = task_changes.into_iter().map(|c| proto::StateChange {
            r#type: match c.change_type {
                sysconfig_plugins::TaskChangeType::Create => proto::ChangeType::Create as i32,
                sysconfig_plugins::TaskChangeType::Update => proto::ChangeType::Update as i32,
                sysconfig_plugins::TaskChangeType::Delete => proto::ChangeType::Delete as i32,
            },
            path: c.path,
            old_value: c.old_value.map(|v| v.to_string()).unwrap_or_default(),
            new_value: c.new_value.map(|v| v.to_string()).unwrap_or_default(),
            verbose: c.verbose,
        }).collect();

        let resp = proto::DiffStateResponse { different: !changes.is_empty(), changes };
        Ok(Response::new(resp))
    }

    async fn apply_state(
        &self,
        request: Request<proto::PluginApplyStateRequest>,
    ) -> Result<Response<proto::PluginApplyStateResponse>, Status> {
        let req = request.into_inner();

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
            match NetworkSettings::default().apply(settings, req.dry_run) {
                Ok(mut changes) => {
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
            match Files::default().apply(files, req.dry_run) {
                Ok(mut changes) => {
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

        // Translate TaskChange -> proto::StateChange
        let changes: Vec<proto::StateChange> = task_changes.into_iter().map(|c| proto::StateChange {
            r#type: match c.change_type {
                sysconfig_plugins::TaskChangeType::Create => proto::ChangeType::Create as i32,
                sysconfig_plugins::TaskChangeType::Update => proto::ChangeType::Update as i32,
                sysconfig_plugins::TaskChangeType::Delete => proto::ChangeType::Delete as i32,
            },
            path: c.path,
            old_value: c.old_value.map(|v| v.to_string()).unwrap_or_default(),
            new_value: c.new_value.map(|v| v.to_string()).unwrap_or_default(),
            verbose: c.verbose,
        }).collect();

        let resp = proto::PluginApplyStateResponse { success: true, error: String::new(), changes };
        Ok(Response::new(resp))
    }

    async fn execute_action(
        &self,
        request: Request<proto::PluginExecuteActionRequest>,
    ) -> Result<Response<proto::PluginExecuteActionResponse>, Status> {
        let req = request.into_inner();
        let result = format!("linux-base executed action '{}' with params '{}'", req.action, req.parameters);
        Ok(Response::new(proto::PluginExecuteActionResponse { success: true, error: String::new(), result }))
    }

    async fn notify_state_change(
        &self,
        _request: Request<proto::NotifyStateChangeRequest>,
    ) -> Result<Response<proto::NotifyStateChangeResponse>, Status> {
        Ok(Response::new(proto::NotifyStateChangeResponse { success: true, error: String::new() }))
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
        name: "linux-base".to_string(),
        description: "Base plugin for Linux: storage, users, packages, services, firewall, files, network.links, network.settings".to_string(),
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
                info!(plugin_id = %plugin_id, "Registered linux-base plugin with sysconfig service");
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
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

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

    let plugin = LinuxBasePlugin::default();

    info!(socket = %plugin_socket, "Starting linux-base plugin server");

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
