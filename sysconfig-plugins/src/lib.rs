//! Shared library for sysconfig base plugins.
//!
//! This crate provides a thin, shared structure for OS base plugins. Each
//! base plugin (one per OS) groups task-specific modules that actually manage
//! parts of the machine. Tasks are intentionally separated so they can be
//! filled in incrementally.
//!
//! NOTE: Template rendering is out of scope for sysconfig. Templates must be
//! pre-rendered by higher layers before they are submitted into the desired
//! state handled by sysconfig.

pub mod tasks {
    //! Task modules grouped by responsibility. These are intentionally thin
    //! placeholders to establish structure. Concrete implementations can be
    //! added per OS as needed.

    pub mod storage; // ZFS pools, datasets, volumes
    pub mod users;   // user and group management
    pub mod packages; // package/publisher/image management
    pub mod services; // service (e.g. systemd/SMF) management
    pub mod firewall; // firewall rule management
    pub mod files;    // simple file state with static content
    pub mod network_links;    // network link/interface configuration
    pub mod network_settings; // other network settings (hostname, DNS, etc.)
}

pub mod provisioning {
    //! Provisioning plugin functionality for collecting configuration from
    //! various cloud data sources and converting to unified schema.

    pub mod datasources;
    pub mod converter;

    pub use datasources::{DataSource, CloudInitPaths, PrioritizedSource, collect_from_source};
    pub use converter::convert_to_unified_schema;

    // Import the module implementation to access its functions
    use serde_json::Value;
    use std::error::Error;

    /// Merge two JSON configurations, with the overlay taking precedence
    pub fn merge_configurations(
        base: &mut Value,
        overlay: Value,
    ) -> Result<(), Box<dyn Error>> {
        match (base, overlay) {
            (Value::Object(base_map), Value::Object(overlay_map)) => {
                for (key, value) in overlay_map {
                    if base_map.contains_key(&key) {
                        merge_configurations(base_map.get_mut(&key).unwrap(), value)?;
                    } else {
                        base_map.insert(key, value);
                    }
                }
            }
            (base_val, overlay_val) => {
                *base_val = overlay_val;
            }
        }
        Ok(())
    }

    /// Parse data source priority string into a list of prioritized sources
    pub fn parse_data_sources(
        sources_str: &str,
        config_file: Option<&str>,
        cloud_init_meta_data: &str,
        cloud_init_user_data: &str,
        cloud_init_network_config: &str,
    ) -> Result<Vec<PrioritizedSource>, Box<dyn Error>> {
        let mut sources = Vec::new();
        let mut priority = 0;

        for source_name in sources_str.split(',') {
            priority += 10;
            let source = match source_name.trim() {
                "local" => {
                    if let Some(config_file) = config_file {
                        DataSource::Local(config_file.to_string())
                    } else {
                        continue; // Skip if no config file specified
                    }
                }
                "cloud-init" => DataSource::CloudInit(datasources::CloudInitPaths {
                    meta_data: cloud_init_meta_data.to_string(),
                    user_data: cloud_init_user_data.to_string(),
                    network_config: cloud_init_network_config.to_string(),
                }),
                "ec2" => DataSource::Ec2,
                "gcp" => DataSource::Gcp,
                "azure" => DataSource::Azure,
                _ => {
                    continue; // Skip unknown sources
                }
            };

            sources.push(PrioritizedSource { source, priority });
        }

        // Sort by priority (lower number = higher priority)
        sources.sort_by_key(|s| s.priority);

        Ok(sources)
    }
}

/// A structured change produced by task handlers. This is mapped to the
/// sysconfig.proto StateChange in the plugin binaries.
#[derive(Debug, Clone)]
pub struct TaskChange {
    pub change_type: TaskChangeType,
    pub path: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    /// Whether this change may contain verbose/large data that clients can omit
    pub verbose: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskChangeType {
    Create,
    Update,
    Delete,
}

impl TaskChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskChangeType::Create => "create",
            TaskChangeType::Update => "update",
            TaskChangeType::Delete => "delete",
        }
    }
}

/// A minimal interface that task modules can implement. This is a deliberately
/// simple, JSON-in/JSON-out trait to avoid committing to specific schema details
/// until they are finalized.
pub trait TaskHandler: Send + Sync {
    /// Calculate a diff between current and desired JSON state and return a list
    /// of structured changes. Returns Err(...) if the desired schema is invalid
    /// or diffing fails.
    fn diff(
        &self,
        _current: &serde_json::Value,
        _desired: &serde_json::Value,
    ) -> Result<Vec<TaskChange>, String> {
        Ok(Vec::new())
    }

    /// Apply the desired JSON state. Returning a list of applied changes is
    /// optional but useful for logging. Returns Err(...) to propagate failures
    /// back to the plugin application and ultimately to sysconfig.
    fn apply(
        &self,
        _desired: &serde_json::Value,
        _dry_run: bool,
    ) -> Result<Vec<TaskChange>, String> {
        Ok(Vec::new())
    }

    /// Execute an imperative action with parameters; return a string result or an
    /// error string on failure.
    fn exec(&self, _action: &str, _params: &serde_json::Value) -> Result<String, String> {
        Ok(String::new())
    }
}
