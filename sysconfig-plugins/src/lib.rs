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
