use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::proto::{self, *};
use crate::{Error, Result};

/// Plugin client for communicating with a plugin
#[derive(Clone)]
pub struct PluginClient {
    pub(crate) client: proto::plugin_service_client::PluginServiceClient<tonic::transport::Channel>,
}

impl PluginClient {
    /// Create a new plugin client
    pub async fn connect(socket_path: impl AsRef<Path> + 'static) -> Result<Self> {
        let socket_path = socket_path.as_ref().to_path_buf();
        let channel = tonic::transport::Endpoint::from_static("http://[::]:50051")
            .connect_with_connector(tower::service_fn(move |_| {
                let socket_path = socket_path.clone();
                async move {
                    let stream = tokio::net::UnixStream::connect(socket_path).await?;
                    Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(stream))
                }
            }))
            .await?;

        let client = proto::plugin_service_client::PluginServiceClient::new(channel);

        Ok(Self { client })
    }

    /// Initialize the plugin
    pub async fn initialize(&mut self, plugin_id: &str, service_socket_path: &str) -> Result<()> {
        let request = tonic::Request::new(InitializeRequest {
            plugin_id: plugin_id.to_string(),
            service_socket_path: service_socket_path.to_string(),
        });

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
    pub(crate) service: Arc<crate::service::SysConfigService>,
    pub(crate) plugins: Arc<Mutex<HashMap<String, PluginClient>>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(service: Arc<crate::service::SysConfigService>) -> Self {
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
