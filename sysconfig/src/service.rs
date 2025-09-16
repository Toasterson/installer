use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use async_trait::async_trait;
use tokio::net::UnixListener;
use tokio::sync::broadcast;
use tokio_stream::Stream;

use crate::plugin::PluginClient;
use crate::proto::{self, *};
use crate::{Error, Result};

/// Define a custom stream type that maps BroadcastStreamRecvError to tonic::Status
pub struct StateChangeStream {
    pub(crate) inner: tokio_stream::wrappers::BroadcastStream<StateChangeEvent>,
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
    pub(crate) state: Arc<Mutex<crate::SystemState>>,

    /// Registered plugins
    pub(crate) plugins: Arc<Mutex<HashMap<String, crate::Plugin>>>,

    /// State locks
    pub(crate) locks: Arc<Mutex<Vec<crate::StateLock>>>,

    /// State change broadcaster
    pub(crate) state_change_tx: broadcast::Sender<StateChangeEvent>,
}

impl SysConfigService {
    /// Create a new sysconfig service
    pub fn new() -> Result<Self> {
        let (state_change_tx, _) = broadcast::channel(100);
        // Load latest persisted state revision if available. If a state file exists but
        // cannot be read or parsed, bail out instead of using an empty state.
        let initial_state = match Self::load_latest_state_revision()? {
            Some(state) => state,
            None => crate::SystemState::new(),
        };

        Ok(Self {
            state: Arc::new(Mutex::new(initial_state)),
            plugins: Arc::new(Mutex::new(HashMap::new())),
            locks: Arc::new(Mutex::new(Vec::new())),
            state_change_tx,
        })
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
                        let svc = proto::sys_config_service_server::SysConfigServiceServer::new(service);

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
    pub fn register_plugin(&self, plugin: crate::Plugin) -> Result<()> {
        let mut plugins = self.plugins.lock().unwrap();
        plugins.insert(plugin.id.clone(), plugin);
        Ok(())
    }

    /// Get a plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Option<crate::Plugin> {
        let plugins = self.plugins.lock().unwrap();
        plugins.get(plugin_id).cloned()
    }

    /// Get all registered plugins
    pub fn get_plugins(&self) -> Vec<crate::Plugin> {
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
        locks.push(crate::StateLock {
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
        let new_state = crate::SystemState::from_json(state_json)?;
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
            // Replace the entire state and persist a timestamped revision to disk
            let serialized = {
                let mut state = self.state.lock().unwrap();

                // Overwrite existing state with new_state (apply is an overwrite, not a merge)
                *state = new_state;

                // Broadcast the state change
                let event = StateChangeEvent {
                    path: "".to_string(),
                    value: state.to_json()?,
                    plugin_id: plugin_id.to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                };
                let _ = self.state_change_tx.send(event);

                // Serialize to JSON for persistent snapshot
                state.to_json()?
            };

            // Persist the new state snapshot with a timestamped filename
            self.persist_state_revision(&serialized)?;
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
        // Determine which plugin should handle the action
        let plugin = if let Some(plugin_id) = plugin_id {
            self.get_plugin(plugin_id)
                .ok_or_else(|| Error::Plugin(format!("Plugin not found: {}", plugin_id)))?
        } else {
            // Fallback: use the first registered plugin
            let plugins = self.get_plugins();
            if plugins.is_empty() {
                return Err(Error::Plugin("No plugins registered".to_string()));
            }
            plugins[0].clone()
        };

        // Bridge to the async PluginClient API and invoke the action on the plugin.
        let socket_path = plugin.socket_path.clone();
        let action_s = action.to_string();
        let params_s = parameters.to_string();

        // If we're inside a Tokio runtime, use block_in_place + Handle::block_on.
        // Otherwise, spin up a temporary runtime just for this call.
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(move || {
                handle.block_on(async move {
                    let mut client = PluginClient::connect(socket_path).await?;
                    client.execute_action(&action_s, &params_s).await
                })
            }),
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async move {
                    let mut client = PluginClient::connect(socket_path).await?;
                    client.execute_action(&action_s, &params_s).await
                })
            }
        }
    }

    /// Subscribe to state changes
    pub fn subscribe_to_state_changes(&self) -> broadcast::Receiver<StateChangeEvent> {
        self.state_change_tx.subscribe()
    }

    fn state_revision_dir() -> std::path::PathBuf {
        // Determine where to store persistent state revisions
        let euid = unsafe { libc::geteuid() as u32 };
        if euid == 0 {
            // Root: store under /var
            std::path::PathBuf::from("/var/lib/sysconfig")
        } else {
            // Non-root: prefer XDG_STATE_HOME, then XDG_DATA_HOME, then ~/.local/state
            if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
                return std::path::PathBuf::from(dir).join("sysconfig");
            }
            if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
                return std::path::PathBuf::from(dir).join("sysconfig");
            }
            if let Ok(home) = std::env::var("HOME") {
                return std::path::PathBuf::from(home).join(".local/state/sysconfig");
            }
            // Last resort
            std::path::PathBuf::from("/tmp/sysconfig-state")
        }
    }

    fn persist_state_revision(&self, json: &str) -> Result<()> {
        let dir = Self::state_revision_dir();
        std::fs::create_dir_all(&dir)?;

        // Use a UTC timestamp for the filename, ensure uniqueness with millis
        let now = chrono::Utc::now();
        let base = now.format("%Y%m%dT%H%M%S").to_string();
        let mut path = dir.join(format!("{}.json", base));
        if path.exists() {
            // Very unlikely, but add milliseconds to avoid collision
            let millis = now.timestamp_millis() % 1000;
            path = dir.join(format!("{}.{:03}Z.json", base, millis));
        }

        // Write file atomically where possible: write to temp then rename
        let tmp_path = dir.join(format!("{}.json.tmp", base));
        {
            let mut f = std::fs::File::create(&tmp_path)?;
            use std::io::Write as _;
            f.write_all(json.as_bytes())?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp_path, &path)?;

        Ok(())
    }

    fn load_latest_state_revision() -> Result<Option<crate::SystemState>> {
        let dir = Self::state_revision_dir();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Ok(None), // No directory or unreadable; treat as no prior state
        };

        let mut latest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
        for ent in entries {
            let ent = match ent { Ok(e) => e, Err(_) => continue };
            let path = ent.path();
            if !path.is_file() { continue; }
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext != "json" { continue; }
            } else {
                continue;
            }
            let md = match ent.metadata() { Ok(m) => m, Err(_) => continue };
            if !md.is_file() { continue; }
            let modified = match md.modified() { Ok(m) => m, Err(_) => continue };
            match &latest {
                Some((cur, _)) if modified <= *cur => {}
                _ => latest = Some((modified, path)),
            }
        }

        if let Some((_, path)) = latest {
            match std::fs::read_to_string(&path) {
                Ok(content) => match crate::SystemState::from_json(&content) {
                    Ok(state) => {
                        tracing::info!("Loaded initial state from {:?}", path);
                        Ok(Some(state))
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse state file {:?}: {}", path, e);
                        Err(e)
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read state file {:?}: {}", path, e);
                    Err(e.into())
                }
            }
        } else {
            Ok(None)
        }
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
impl proto::sys_config_service_server::SysConfigService for SysConfigService {
    async fn register_plugin(
        &self,
        request: tonic::Request<RegisterPluginRequest>,
    ) -> std::result::Result<tonic::Response<RegisterPluginResponse>, tonic::Status> {
        let req = request.into_inner();

        let plugin = crate::Plugin {
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
