use async_trait::async_trait;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::net::UnixListener;
use tokio::sync::broadcast;
use tokio_stream::Stream;
use uuid::Uuid;

// Include the generated proto code
pub mod proto {
    tonic::include_proto!("sysconfig");
}

// Re-export the proto types for convenience
pub use proto::*;

// Separate module for knus parsing to avoid conflicts with our custom Result type
pub mod config {
    use knus;
    use std::fmt::Debug;

    // Define types for knus parsing
    #[derive(Debug, Default, knus::Decode)]
    pub struct SysConfig {
        #[knus(child, unwrap(argument))]
        pub hostname: String,

        #[knus(children(name = "nameserver"), unwrap(argument))]
        pub nameservers: Vec<String>,

        #[knus(children(name = "interface"))]
        pub interfaces: Vec<Interface>,
    }

    #[derive(Debug, Default, knus::Decode)]
    pub struct Interface {
        #[knus(argument)]
        pub name: Option<String>,

        #[knus(property)]
        pub selector: Option<String>,

        #[knus(children(name = "address"))]
        pub addresses: Vec<AddressObject>,
    }

    #[derive(Debug, Default, knus::Decode)]
    pub struct AddressObject {
        #[knus(property)]
        pub name: String,

        #[knus(property)]
        pub kind: AddressKind,

        #[knus(argument)]
        pub address: Option<String>,
    }

    #[derive(knus::DecodeScalar, Debug, Default, strum::Display)]
    pub enum AddressKind {
        #[default]
        Dhcp4,
        Dhcp6,
        Addrconf,
        Static,
    }

    // Parse config using knus
    pub fn parse_config(path: &str, content: &str) -> Result<SysConfig, knus::Error> {
        knus::parse(path, content)
    }
}

// Re-export the config types for convenience
pub use config::SysConfig;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    Knus(#[from] knus::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("Status error: {0}")]
    Status(#[from] tonic::Status),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("Decode error: {0}")]
    Decode(String),

    #[error("Broadcast receive error: {0}")]
    BroadcastRecv(String),
}

impl<S: std::fmt::Debug + Clone + Send + Sync + Into<knus::span::ErrorSpan> + 'static>
    From<knus::errors::DecodeError<S>> for Error
{
    fn from(err: knus::errors::DecodeError<S>) -> Self {
        Error::Decode(format!("{:?}", err))
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for Error {
    fn from(err: tokio::sync::broadcast::error::RecvError) -> Self {
        Error::BroadcastRecv(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// Original config parsing functionality
pub fn parse_config(path: &str, content: &str) -> Result<SysConfig> {
    Ok(config::parse_config(path, &content)?)
}

// New types for the sysconfig service

/// Represents a plugin registered with the sysconfig service
#[derive(Debug, Clone)]
pub struct Plugin {
    /// Unique identifier for the plugin
    pub id: String,

    /// Name of the plugin
    pub name: String,

    /// Description of the plugin
    pub description: String,

    /// Socket path where the plugin is listening
    pub socket_path: String,

    /// State paths that this plugin manages
    pub managed_paths: Vec<String>,
}

/// Represents a lock on a part of the system state
#[derive(Debug, Clone)]
pub struct StateLock {
    /// The path that is locked
    pub path: String,

    /// The plugin that holds the lock
    pub plugin_id: String,
}

/// The system state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemState {
    /// The state data as a nested structure
    pub data: serde_json::Value,
}

impl SystemState {
    /// Create a new empty system state
    pub fn new() -> Self {
        Self {
            data: serde_json::json!({}),
        }
    }

    /// Get a value from the state at the specified path
    pub fn get(&self, path: &str) -> Option<serde_json::Value> {
        if path.is_empty() {
            return Some(self.data.clone());
        }

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.data;

        for part in parts {
            if let Some(obj) = current.as_object() {
                if let Some(value) = obj.get(part) {
                    current = value;
                } else {
                    return None;
                }
            } else if let Some(arr) = current.as_array() {
                if let Ok(index) = part.parse::<usize>() {
                    if index < arr.len() {
                        current = &arr[index];
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }

        Some(current.clone())
    }

    /// Set a value in the state at the specified path
    pub fn set(&mut self, path: &str, value: serde_json::Value) -> Result<()> {
        if path.is_empty() {
            self.data = value;
            return Ok(());
        }

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &mut self.data;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part, set the value
                match current {
                    serde_json::Value::Object(obj) => {
                        obj.insert(part.to_string(), value);
                        return Ok(());
                    }
                    serde_json::Value::Array(arr) => {
                        if let Ok(index) = part.parse::<usize>() {
                            if index < arr.len() {
                                arr[index] = value;
                                return Ok(());
                            } else {
                                return Err(Error::State(format!(
                                    "Index out of bounds: {}",
                                    index
                                )));
                            }
                        } else {
                            return Err(Error::State(format!("Invalid array index: {}", part)));
                        }
                    }
                    _ => return Err(Error::State(format!("Cannot set value at path: {}", path))),
                }
            } else {
                // Not the last part, navigate to the next level
                match current {
                    serde_json::Value::Object(obj) => {
                        if !obj.contains_key(*part) {
                            obj.insert(part.to_string(), serde_json::json!({}));
                        }
                        current = obj.get_mut(*part).unwrap();
                    }
                    serde_json::Value::Array(arr) => {
                        if let Ok(index) = part.parse::<usize>() {
                            if index < arr.len() {
                                current = &mut arr[index];
                            } else {
                                return Err(Error::State(format!(
                                    "Index out of bounds: {}",
                                    index
                                )));
                            }
                        } else {
                            return Err(Error::State(format!("Invalid array index: {}", part)));
                        }
                    }
                    _ => return Err(Error::State(format!("Cannot navigate to path: {}", path))),
                }
            }
        }

        Ok(())
    }

    /// Remove a value from the state at the specified path
    pub fn remove(&mut self, path: &str) -> Result<()> {
        if path.is_empty() {
            return Err(Error::State("Cannot remove root state".to_string()));
        }

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &mut self.data;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part, remove the value
                match current {
                    serde_json::Value::Object(obj) => {
                        obj.remove(*part);
                        return Ok(());
                    }
                    serde_json::Value::Array(arr) => {
                        if let Ok(index) = part.parse::<usize>() {
                            if index < arr.len() {
                                arr.remove(index);
                                return Ok(());
                            } else {
                                return Err(Error::State(format!(
                                    "Index out of bounds: {}",
                                    index
                                )));
                            }
                        } else {
                            return Err(Error::State(format!("Invalid array index: {}", part)));
                        }
                    }
                    _ => {
                        return Err(Error::State(format!(
                            "Cannot remove value at path: {}",
                            path
                        )))
                    }
                }
            } else {
                // Not the last part, navigate to the next level
                match current {
                    serde_json::Value::Object(obj) => {
                        if let Some(next) = obj.get_mut(*part) {
                            current = next;
                        } else {
                            return Ok(()); // Path doesn't exist, nothing to remove
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        if let Ok(index) = part.parse::<usize>() {
                            if index < arr.len() {
                                current = &mut arr[index];
                            } else {
                                return Ok(()); // Path doesn't exist, nothing to remove
                            }
                        } else {
                            return Err(Error::State(format!("Invalid array index: {}", part)));
                        }
                    }
                    _ => return Ok(()), // Path doesn't exist, nothing to remove
                }
            }
        }

        Ok(())
    }

    /// Convert the state to a JSON string
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self.data)?)
    }

    /// Create a state from a JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let data = serde_json::from_str(json)?;
        Ok(Self { data })
    }
}

// Define a custom stream type that maps BroadcastStreamRecvError to tonic::Status
pub struct StateChangeStream {
    inner: tokio_stream::wrappers::BroadcastStream<StateChangeEvent>,
}

impl Stream for StateChangeStream {
    type Item = std::result::Result<StateChangeEvent, tonic::Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => Poll::Ready(Some(Ok(event))),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(tonic::Status::internal(
                format!("Broadcast receive error: {}", err),
            )))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// The sysconfig service
#[derive(Debug)]
pub struct SysConfigService {
    /// The system state
    state: Arc<Mutex<SystemState>>,

    /// Registered plugins
    plugins: Arc<Mutex<HashMap<String, Plugin>>>,

    /// State locks
    locks: Arc<Mutex<Vec<StateLock>>>,

    /// State change broadcaster
    state_change_tx: broadcast::Sender<StateChangeEvent>,
}

impl SysConfigService {
    /// Create a new sysconfig service
    pub fn new() -> Self {
        let (state_change_tx, _) = broadcast::channel(100);

        Self {
            state: Arc::new(Mutex::new(SystemState::new())),
            plugins: Arc::new(Mutex::new(HashMap::new())),
            locks: Arc::new(Mutex::new(Vec::new())),
            state_change_tx,
        }
    }

    /// Start the service
    pub async fn start(&self, socket_path: impl AsRef<Path>) -> Result<()> {
        // Remove the socket file if it already exists
        let socket_path = socket_path.as_ref();
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        // Create the Unix socket listener
        let listener = UnixListener::bind(socket_path)?;
        tracing::info!("Listening on Unix socket: {:?}", socket_path);

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    tracing::info!("Accepted connection");

                    // Clone the service for the connection handler
                    let service = self.clone();

                    // Spawn a task to handle the connection
                    tokio::spawn(async move {
                        let svc = sys_config_service_server::SysConfigServiceServer::new(service);

                        match tonic::transport::Server::builder()
                            .add_service(svc)
                            .serve_with_incoming(futures::stream::iter(vec![
                                Ok::<_, std::io::Error>(stream),
                            ]))
                            .await
                        {
                            Ok(_) => tracing::info!("Connection handled successfully"),
                            Err(e) => tracing::error!("Error handling connection: {:?}", e),
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Error accepting connection: {:?}", e);
                }
            }
        }
    }

    /// Register a plugin
    pub fn register_plugin(&self, plugin: Plugin) -> Result<()> {
        let mut plugins = self.plugins.lock().unwrap();
        plugins.insert(plugin.id.clone(), plugin);
        Ok(())
    }

    /// Get a plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Option<Plugin> {
        let plugins = self.plugins.lock().unwrap();
        plugins.get(plugin_id).cloned()
    }

    /// Get all registered plugins
    pub fn get_plugins(&self) -> Vec<Plugin> {
        let plugins = self.plugins.lock().unwrap();
        plugins.values().cloned().collect()
    }

    /// Lock a part of the state
    pub fn lock_state(&self, path: &str, plugin_id: &str) -> Result<bool> {
        let mut locks = self.locks.lock().unwrap();

        // Check if the path is already locked by another plugin
        for lock in locks.iter() {
            if lock.path == path && lock.plugin_id != plugin_id {
                return Ok(false);
            }
        }

        // Add the lock
        locks.push(StateLock {
            path: path.to_string(),
            plugin_id: plugin_id.to_string(),
        });

        Ok(true)
    }

    /// Unlock a part of the state
    pub fn unlock_state(&self, path: &str, plugin_id: &str) -> Result<bool> {
        let mut locks = self.locks.lock().unwrap();

        // Find the lock
        let index = locks
            .iter()
            .position(|lock| lock.path == path && lock.plugin_id == plugin_id);

        if let Some(index) = index {
            // Remove the lock
            locks.remove(index);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if a path is locked by a plugin
    pub fn is_locked_by(&self, path: &str, plugin_id: &str) -> bool {
        let locks = self.locks.lock().unwrap();

        for lock in locks.iter() {
            if lock.path == path && lock.plugin_id == plugin_id {
                return true;
            }
        }

        false
    }

    /// Check if a path is locked by another plugin
    pub fn is_locked_by_other(&self, path: &str, plugin_id: &str) -> bool {
        let locks = self.locks.lock().unwrap();

        for lock in locks.iter() {
            if lock.path == path && lock.plugin_id != plugin_id {
                return true;
            }
        }

        false
    }

    /// Get the current system state
    pub fn get_state(&self, path: &str) -> Result<serde_json::Value> {
        let state = self.state.lock().unwrap();

        if let Some(value) = state.get(path) {
            Ok(value)
        } else {
            Err(Error::State(format!("Path not found: {}", path)))
        }
    }

    /// Apply a new state to the system
    pub fn apply_state(
        &self,
        state_json: &str,
        dry_run: bool,
        plugin_id: &str,
    ) -> Result<Vec<StateChange>> {
        let new_state = SystemState::from_json(state_json)?;
        let changes = Vec::new();

        // Check if any of the paths are locked by another plugin
        {
            let locks = self.locks.lock().unwrap();

            for lock in locks.iter() {
                if lock.plugin_id != plugin_id {
                    if let Some(_) = new_state.get(&lock.path) {
                        return Err(Error::Lock(format!(
                            "Path is locked by another plugin: {}",
                            lock.path
                        )));
                    }
                }
            }
        }

        // Apply the state changes
        if !dry_run {
            let mut state = self.state.lock().unwrap();

            // For simplicity, we'll just replace the entire state
            // In a real implementation, you would want to merge the states
            *state = new_state;

            // Broadcast the state change
            let event = StateChangeEvent {
                path: "".to_string(),
                value: state.to_json()?,
                plugin_id: plugin_id.to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            };

            let _ = self.state_change_tx.send(event);
        }

        Ok(changes)
    }

    /// Execute an action
    pub fn execute_action(
        &self,
        action: &str,
        parameters: &str,
        plugin_id: Option<&str>,
    ) -> Result<String> {
        // Find the plugin to execute the action
        let _plugin = if let Some(plugin_id) = plugin_id {
            self.get_plugin(plugin_id)
                .ok_or_else(|| Error::Plugin(format!("Plugin not found: {}", plugin_id)))?
        } else {
            // Find a plugin that can handle the action
            // For simplicity, we'll just use the first plugin
            let plugins = self.get_plugins();
            if plugins.is_empty() {
                return Err(Error::Plugin("No plugins registered".to_string()));
            }
            plugins[0].clone()
        };

        // Execute the action using the plugin
        // In a real implementation, you would use the plugin's RPC interface
        // For now, we'll just return a dummy result
        Ok(format!(
            "Action executed: {} with parameters: {}",
            action, parameters
        ))
    }

    /// Subscribe to state changes
    pub fn subscribe_to_state_changes(&self) -> broadcast::Receiver<StateChangeEvent> {
        self.state_change_tx.subscribe()
    }
}

impl Clone for SysConfigService {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            plugins: Arc::clone(&self.plugins),
            locks: Arc::clone(&self.locks),
            state_change_tx: self.state_change_tx.clone(),
        }
    }
}

#[async_trait]
impl sys_config_service_server::SysConfigService for SysConfigService {
    async fn register_plugin(
        &self,
        request: tonic::Request<RegisterPluginRequest>,
    ) -> std::result::Result<tonic::Response<RegisterPluginResponse>, tonic::Status> {
        let req = request.into_inner();

        let plugin = Plugin {
            id: req.plugin_id.clone(),
            name: req.name,
            description: req.description,
            socket_path: req.socket_path,
            managed_paths: req.managed_paths,
        };

        match self.register_plugin(plugin) {
            Ok(_) => {
                let response = RegisterPluginResponse {
                    success: true,
                    error: "".to_string(),
                };
                Ok(tonic::Response::new(response))
            }
            Err(e) => {
                let response = RegisterPluginResponse {
                    success: false,
                    error: e.to_string(),
                };
                Ok(tonic::Response::new(response))
            }
        }
    }

    async fn get_state(
        &self,
        request: tonic::Request<GetStateRequest>,
    ) -> std::result::Result<tonic::Response<GetStateResponse>, tonic::Status> {
        let req = request.into_inner();

        match self.get_state(&req.path) {
            Ok(value) => {
                let state_json = serde_json::to_string(&value)
                    .map_err(|e| tonic::Status::internal(e.to_string()))?;

                let response = GetStateResponse { state: state_json };
                Ok(tonic::Response::new(response))
            }
            Err(e) => Err(tonic::Status::internal(e.to_string())),
        }
    }

    async fn apply_state(
        &self,
        request: tonic::Request<ApplyStateRequest>,
    ) -> std::result::Result<tonic::Response<ApplyStateResponse>, tonic::Status> {
        let plugin_id = request
            .metadata()
            .get("plugin-id")
            .map(|v| v.to_str().unwrap_or("unknown"))
            .unwrap_or("unknown")
            .to_string();
        let req = request.into_inner();

        match self.apply_state(&req.state, req.dry_run, &plugin_id) {
            Ok(changes) => {
                let proto_changes = changes
                    .into_iter()
                    .map(|c| proto::StateChange {
                        r#type: c.r#type as i32,
                        path: c.path,
                        old_value: c.old_value,
                        new_value: c.new_value,
                    })
                    .collect();

                let response = ApplyStateResponse {
                    success: true,
                    error: "".to_string(),
                    changes: proto_changes,
                };
                Ok(tonic::Response::new(response))
            }
            Err(e) => {
                let response = ApplyStateResponse {
                    success: false,
                    error: e.to_string(),
                    changes: Vec::new(),
                };
                Ok(tonic::Response::new(response))
            }
        }
    }

    async fn execute_action(
        &self,
        request: tonic::Request<ExecuteActionRequest>,
    ) -> std::result::Result<tonic::Response<ExecuteActionResponse>, tonic::Status> {
        let req = request.into_inner();

        match self.execute_action(
            &req.action,
            &req.parameters,
            if req.plugin_id.is_empty() {
                None
            } else {
                Some(&req.plugin_id)
            },
        ) {
            Ok(result) => {
                let response = ExecuteActionResponse {
                    success: true,
                    error: "".to_string(),
                    result,
                };
                Ok(tonic::Response::new(response))
            }
            Err(e) => {
                let response = ExecuteActionResponse {
                    success: false,
                    error: e.to_string(),
                    result: "".to_string(),
                };
                Ok(tonic::Response::new(response))
            }
        }
    }

    type WatchStateStream = StateChangeStream;

    async fn watch_state(
        &self,
        request: tonic::Request<WatchStateRequest>,
    ) -> std::result::Result<tonic::Response<Self::WatchStateStream>, tonic::Status> {
        let req = request.into_inner();
        let _path = req.path;

        let rx = self.subscribe_to_state_changes();
        let inner = tokio_stream::wrappers::BroadcastStream::new(rx);
        let stream = StateChangeStream { inner };

        Ok(tonic::Response::new(stream))
    }

    async fn lock_state(
        &self,
        request: tonic::Request<LockStateRequest>,
    ) -> std::result::Result<tonic::Response<LockStateResponse>, tonic::Status> {
        let req = request.into_inner();

        match self.lock_state(&req.path, &req.plugin_id) {
            Ok(success) => {
                let response = LockStateResponse {
                    success,
                    error: if success {
                        "".to_string()
                    } else {
                        "Path is already locked by another plugin".to_string()
                    },
                };
                Ok(tonic::Response::new(response))
            }
            Err(e) => {
                let response = LockStateResponse {
                    success: false,
                    error: e.to_string(),
                };
                Ok(tonic::Response::new(response))
            }
        }
    }

    async fn unlock_state(
        &self,
        request: tonic::Request<UnlockStateRequest>,
    ) -> std::result::Result<tonic::Response<UnlockStateResponse>, tonic::Status> {
        let req = request.into_inner();

        match self.unlock_state(&req.path, &req.plugin_id) {
            Ok(success) => {
                let response = UnlockStateResponse {
                    success,
                    error: if success {
                        "".to_string()
                    } else {
                        "Path is not locked by this plugin".to_string()
                    },
                };
                Ok(tonic::Response::new(response))
            }
            Err(e) => {
                let response = UnlockStateResponse {
                    success: false,
                    error: e.to_string(),
                };
                Ok(tonic::Response::new(response))
            }
        }
    }
}

/// Plugin client for communicating with a plugin
#[derive(Clone)]
pub struct PluginClient {
    client: proto::plugin_service_client::PluginServiceClient<tonic::transport::Channel>,
}

impl PluginClient {
    /// Create a new plugin client
    pub async fn connect(socket_path: impl AsRef<Path> + 'static) -> Result<Self> {
        let socket_path = socket_path.as_ref().to_path_buf();
        let channel = tonic::transport::Endpoint::from_static("http://[::]:50051")
            .connect_with_connector(tower::service_fn(move |_| {
                let socket_path = socket_path.clone();
                async move {
                    use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncWriteCompatExt};
                    let stream = tokio::net::UnixStream::connect(socket_path).await?;
                    Ok::<_, std::io::Error>(stream.compat_write().compat())
                }
            }))
            .await?;

        let client = proto::plugin_service_client::PluginServiceClient::new(channel);

        Ok(Self { client })
    }

    /// Initialize the plugin
    pub async fn initialize(&mut self, plugin_id: &str, service_socket_path: &str) -> Result<()> {
        let request = InitializeRequest {
            plugin_id: plugin_id.to_string(),
            service_socket_path: service_socket_path.to_string(),
        };

        let response = self.client.initialize(request).await?;
        let response = response.into_inner();

        if response.success {
            Ok(())
        } else {
            Err(Error::Plugin(response.error))
        }
    }

    /// Get the plugin's configuration
    pub async fn get_config(&mut self) -> Result<String> {
        let request = GetConfigRequest {};

        let response = self.client.get_config(request).await?;
        let response = response.into_inner();

        Ok(response.config)
    }

    /// Diff the current state with the desired state
    pub async fn diff_state(
        &mut self,
        current_state: &str,
        desired_state: &str,
    ) -> Result<Vec<StateChange>> {
        let request = DiffStateRequest {
            current_state: current_state.to_string(),
            desired_state: desired_state.to_string(),
        };

        let response = self.client.diff_state(request).await?;
        let response = response.into_inner();

        let changes = response
            .changes
            .into_iter()
            .map(|c| StateChange {
                r#type: c.r#type,
                path: c.path,
                old_value: c.old_value,
                new_value: c.new_value,
            })
            .collect();

        Ok(changes)
    }

    /// Apply a new state
    pub async fn apply_state(&mut self, state: &str, dry_run: bool) -> Result<Vec<StateChange>> {
        let request = PluginApplyStateRequest {
            state: state.to_string(),
            dry_run,
        };

        let response = self.client.apply_state(request).await?;
        let response = response.into_inner();

        if response.success {
            let changes = response
                .changes
                .into_iter()
                .map(|c| StateChange {
                    r#type: c.r#type,
                    path: c.path,
                    old_value: c.old_value,
                    new_value: c.new_value,
                })
                .collect();

            Ok(changes)
        } else {
            Err(Error::Plugin(response.error))
        }
    }

    /// Execute an action
    pub async fn execute_action(&mut self, action: &str, parameters: &str) -> Result<String> {
        let request = PluginExecuteActionRequest {
            action: action.to_string(),
            parameters: parameters.to_string(),
        };

        let response = self.client.execute_action(request).await?;
        let response = response.into_inner();

        if response.success {
            Ok(response.result)
        } else {
            Err(Error::Plugin(response.error))
        }
    }

    /// Notify the plugin of a state change
    pub async fn notify_state_change(&mut self, event: StateChangeEvent) -> Result<()> {
        let request = NotifyStateChangeRequest { event: Some(event) };

        let response = self.client.notify_state_change(request).await?;
        let response = response.into_inner();

        if response.success {
            Ok(())
        } else {
            Err(Error::Plugin(response.error))
        }
    }
}

/// Plugin manager for managing plugins
pub struct PluginManager {
    service: Arc<SysConfigService>,
    plugins: Arc<Mutex<HashMap<String, PluginClient>>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(service: Arc<SysConfigService>) -> Self {
        Self {
            service,
            plugins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start a plugin
    pub async fn start_plugin(&self, path: impl AsRef<Path>, args: &[&str]) -> Result<String> {
        let plugin_id = Uuid::new_v4().to_string();

        // Start the plugin process
        let mut command = Command::new(path.as_ref());
        command.args(args);

        match command.spawn() {
            Ok(_child) => {
                // Wait for the plugin to start and register
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                // Return the plugin ID
                Ok(plugin_id)
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Connect to a plugin
    pub async fn connect_to_plugin(&self, plugin_id: &str) -> Result<()> {
        let plugin = self
            .service
            .get_plugin(plugin_id)
            .ok_or_else(|| Error::Plugin(format!("Plugin not found: {}", plugin_id)))?;

        // Clone the socket path to satisfy the 'static lifetime requirement
        let socket_path = plugin.socket_path.clone();
        let mut client = PluginClient::connect(socket_path).await?;

        // Initialize the plugin
        client.initialize(plugin_id, "/tmp/sysconfig.sock").await?;

        // Store the client
        let mut plugins = self.plugins.lock().unwrap();
        plugins.insert(plugin_id.to_string(), client);

        Ok(())
    }

    /// Get a plugin client
    pub fn get_plugin_client(&self, plugin_id: &str) -> Option<PluginClient> {
        let plugins = self.plugins.lock().unwrap();
        plugins.get(plugin_id).cloned()
    }
}

/// Plugin trait for implementing plugins
#[async_trait]
pub trait PluginTrait: Send + Sync {
    /// Initialize the plugin
    async fn initialize(&self, plugin_id: &str, service_socket_path: &str) -> Result<()>;

    /// Get the plugin's configuration
    async fn get_config(&self) -> Result<String>;

    /// Diff the current state with the desired state
    async fn diff_state(
        &self,
        current_state: &str,
        desired_state: &str,
    ) -> Result<Vec<StateChange>>;

    /// Apply a new state
    async fn apply_state(&self, state: &str, dry_run: bool) -> Result<Vec<StateChange>>;

    /// Execute an action
    async fn execute_action(&self, action: &str, parameters: &str) -> Result<String>;

    /// Notify the plugin of a state change
    async fn notify_state_change(&self, event: StateChangeEvent) -> Result<()>;
}
