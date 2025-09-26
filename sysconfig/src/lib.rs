use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Include the generated proto code
pub mod proto;

// Re-export the proto types for convenience
pub use proto::*;

// Separate module for knus parsing to avoid conflicts with our custom Result type
pub mod config;

// KDL parser module for handling KDL configuration files
pub mod kdl_parser;

// KDL loader module for loading and applying KDL configurations
pub mod kdl_loader;

// Re-export the config types for convenience
pub use config::SysConfig;

// Split out service and plugin modules
pub mod service;
pub use service::SysConfigService;

pub mod plugin;
pub use plugin::{PluginClient, PluginManager, PluginTrait};

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
