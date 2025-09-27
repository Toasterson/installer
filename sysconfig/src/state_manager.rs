use crate::{Error, Result, SystemState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Maximum number of state revisions to keep in memory
const MAX_MEMORY_REVISIONS: usize = 100;

/// Maximum number of state revisions to keep on disk
const MAX_DISK_REVISIONS: usize = 1000;

/// State revision with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRevision {
    /// Unique identifier for this revision
    pub id: String,

    /// Timestamp when this revision was created
    pub timestamp: DateTime<Utc>,

    /// The actual state data
    pub state: SystemState,

    /// Optional description of what changed
    pub description: Option<String>,

    /// The entity that made this change (plugin ID, user, etc.)
    pub changed_by: String,

    /// Previous revision ID for tracking history
    pub previous_revision_id: Option<String>,

    /// Hash of the state for integrity checking
    pub state_hash: String,
}

impl StateRevision {
    /// Create a new state revision
    pub fn new(state: SystemState, changed_by: String, description: Option<String>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let state_json = state.to_json().unwrap_or_default();
        let state_hash = Self::calculate_hash(&state_json);

        Self {
            id,
            timestamp,
            state,
            description,
            changed_by,
            previous_revision_id: None,
            state_hash,
        }
    }

    /// Calculate SHA256 hash of state JSON
    fn calculate_hash(data: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify the integrity of this revision
    pub fn verify_integrity(&self) -> bool {
        match self.state.to_json() {
            Ok(json) => {
                let calculated_hash = Self::calculate_hash(&json);
                calculated_hash == self.state_hash
            }
            Err(_) => false,
        }
    }
}

/// State comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// Paths that were added
    pub added: Vec<String>,

    /// Paths that were removed
    pub removed: Vec<String>,

    /// Paths that were modified
    pub modified: Vec<(String, serde_json::Value, serde_json::Value)>,
}

impl StateDiff {
    /// Check if there are any differences
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Get a human-readable summary of the differences
    pub fn summary(&self) -> String {
        let mut summary = String::new();

        if !self.added.is_empty() {
            summary.push_str(&format!("Added {} paths:\n", self.added.len()));
            for path in &self.added {
                summary.push_str(&format!("  + {}\n", path));
            }
        }

        if !self.removed.is_empty() {
            summary.push_str(&format!("Removed {} paths:\n", self.removed.len()));
            for path in &self.removed {
                summary.push_str(&format!("  - {}\n", path));
            }
        }

        if !self.modified.is_empty() {
            summary.push_str(&format!("Modified {} paths:\n", self.modified.len()));
            for (path, _old, _new) in &self.modified {
                summary.push_str(&format!("  ~ {}\n", path));
            }
        }

        if summary.is_empty() {
            summary = "No differences".to_string();
        }

        summary
    }
}

/// State manager for handling state persistence and history
pub struct StateManager {
    /// Current state
    current_state: Arc<Mutex<SystemState>>,

    /// Directory for storing state revisions
    state_dir: PathBuf,

    /// In-memory cache of recent revisions
    revision_cache: Arc<Mutex<VecDeque<StateRevision>>>,

    /// Current revision ID
    current_revision_id: Arc<Mutex<Option<String>>>,

    /// Whether to enable automatic persistence
    auto_persist: bool,
}

impl StateManager {
    /// Create a new state manager
    pub fn new<P: AsRef<Path>>(state_dir: P) -> Result<Self> {
        let state_dir = state_dir.as_ref().to_path_buf();

        // Create state directory if it doesn't exist
        if !state_dir.exists() {
            fs::create_dir_all(&state_dir)?;
        }

        // Initialize with empty state
        let current_state = Arc::new(Mutex::new(SystemState::new()));
        let revision_cache = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_MEMORY_REVISIONS)));
        let current_revision_id = Arc::new(Mutex::new(None));

        let mut manager = Self {
            current_state,
            state_dir,
            revision_cache,
            current_revision_id,
            auto_persist: true,
        };

        // Load the latest state from disk if available
        manager.load_latest_state()?;

        Ok(manager)
    }

    /// Set whether to automatically persist state changes
    pub fn set_auto_persist(&mut self, enabled: bool) {
        self.auto_persist = enabled;
    }

    /// Get the current state
    pub fn get_current_state(&self) -> SystemState {
        self.current_state.lock().unwrap().clone()
    }

    /// Update the current state
    pub fn update_state(
        &mut self,
        new_state: SystemState,
        changed_by: String,
        description: Option<String>,
    ) -> Result<String> {
        // Create a new revision
        let mut revision = StateRevision::new(new_state.clone(), changed_by, description);

        // Link to previous revision
        {
            let current_id = self.current_revision_id.lock().unwrap();
            revision.previous_revision_id = current_id.clone();
        }

        let revision_id = revision.id.clone();

        // Update current state
        {
            let mut state = self.current_state.lock().unwrap();
            *state = new_state;
        }

        // Update current revision ID
        {
            let mut current_id = self.current_revision_id.lock().unwrap();
            *current_id = Some(revision_id.clone());
        }

        // Add to cache
        {
            let mut cache = self.revision_cache.lock().unwrap();
            if cache.len() >= MAX_MEMORY_REVISIONS {
                cache.pop_front();
            }
            cache.push_back(revision.clone());
        }

        // Persist if enabled
        if self.auto_persist {
            self.persist_revision(&revision)?;
            self.cleanup_old_revisions()?;
        }

        info!("State updated with revision {}", revision_id);
        Ok(revision_id)
    }

    /// Compare two states and return the differences
    pub fn diff_states(state1: &SystemState, state2: &SystemState) -> StateDiff {
        let mut diff = StateDiff {
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
        };

        // Convert states to JSON for comparison
        let json1 = state1.data.clone();
        let json2 = state2.data.clone();

        // Compare the JSON objects
        Self::diff_json_recursive(&json1, &json2, "", &mut diff);

        diff
    }

    /// Recursive JSON comparison
    fn diff_json_recursive(
        json1: &serde_json::Value,
        json2: &serde_json::Value,
        path: &str,
        diff: &mut StateDiff,
    ) {
        use serde_json::Value;

        match (json1, json2) {
            (Value::Object(map1), Value::Object(map2)) => {
                // Check for removed keys
                for (key, value1) in map1 {
                    let full_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if !map2.contains_key(key) {
                        diff.removed.push(full_path);
                    } else {
                        let value2 = &map2[key];
                        Self::diff_json_recursive(value1, value2, &full_path, diff);
                    }
                }

                // Check for added keys
                for key in map2.keys() {
                    if !map1.contains_key(key) {
                        let full_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        diff.added.push(full_path);
                    }
                }
            }
            (Value::Array(arr1), Value::Array(arr2)) => {
                if arr1.len() != arr2.len() || arr1 != arr2 {
                    diff.modified
                        .push((path.to_string(), json1.clone(), json2.clone()));
                }
            }
            _ => {
                if json1 != json2 {
                    diff.modified
                        .push((path.to_string(), json1.clone(), json2.clone()));
                }
            }
        }
    }

    /// Get a specific revision by ID
    pub fn get_revision(&self, revision_id: &str) -> Result<StateRevision> {
        // Check cache first
        {
            let cache = self.revision_cache.lock().unwrap();
            if let Some(revision) = cache.iter().find(|r| r.id == revision_id) {
                return Ok(revision.clone());
            }
        }

        // Load from disk
        self.load_revision_from_disk(revision_id)
    }

    /// Rollback to a specific revision
    pub fn rollback_to_revision(&mut self, revision_id: &str) -> Result<()> {
        let revision = self.get_revision(revision_id)?;

        // Verify integrity
        if !revision.verify_integrity() {
            return Err(Error::State(format!(
                "Revision {} failed integrity check",
                revision_id
            )));
        }

        // Create a rollback revision
        let description = Some(format!("Rollback to revision {}", revision_id));
        self.update_state(revision.state, "system".to_string(), description)?;

        info!("Rolled back to revision {}", revision_id);
        Ok(())
    }

    /// Get revision history
    pub fn get_history(&self, limit: usize) -> Vec<StateRevision> {
        let cache = self.revision_cache.lock().unwrap();
        cache.iter().rev().take(limit).cloned().collect()
    }

    /// Persist a revision to disk
    fn persist_revision(&self, revision: &StateRevision) -> Result<()> {
        let filename = format!("{}.json", revision.id);
        let filepath = self.state_dir.join(&filename);

        let json = serde_json::to_string_pretty(revision)?;
        fs::write(&filepath, json)?;

        debug!("Persisted revision {} to disk", revision.id);
        Ok(())
    }

    /// Load a revision from disk
    fn load_revision_from_disk(&self, revision_id: &str) -> Result<StateRevision> {
        let filename = format!("{}.json", revision_id);
        let filepath = self.state_dir.join(&filename);

        if !filepath.exists() {
            return Err(Error::State(format!("Revision {} not found", revision_id)));
        }

        let json = fs::read_to_string(&filepath)?;
        let revision: StateRevision = serde_json::from_str(&json)?;

        Ok(revision)
    }

    /// Load the latest state from disk
    fn load_latest_state(&mut self) -> Result<()> {
        let latest_file = self.state_dir.join("latest.json");

        if !latest_file.exists() {
            debug!("No latest state file found, using default state");
            return Ok(());
        }

        let json = fs::read_to_string(&latest_file)?;
        let revision: StateRevision = serde_json::from_str(&json)?;

        // Verify integrity
        if !revision.verify_integrity() {
            warn!("Latest state failed integrity check, using default state");
            return Ok(());
        }

        // Update current state
        {
            let mut state = self.current_state.lock().unwrap();
            *state = revision.state.clone();
        }

        // Update current revision ID
        {
            let mut current_id = self.current_revision_id.lock().unwrap();
            *current_id = Some(revision.id.clone());
        }

        // Add to cache
        {
            let mut cache = self.revision_cache.lock().unwrap();
            cache.push_back(revision);
        }

        info!("Loaded latest state from disk");
        Ok(())
    }

    /// Save the current state as the latest
    pub fn save_latest(&self) -> Result<()> {
        let state = self.get_current_state();
        let revision = StateRevision::new(
            state,
            "system".to_string(),
            Some("Latest state".to_string()),
        );

        let latest_file = self.state_dir.join("latest.json");
        let json = serde_json::to_string_pretty(&revision)?;
        fs::write(&latest_file, json)?;

        info!("Saved latest state to disk");
        Ok(())
    }

    /// Clean up old revisions beyond the maximum limit
    fn cleanup_old_revisions(&self) -> Result<()> {
        let mut entries: Vec<_> = fs::read_dir(&self.state_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.ends_with(".json") && name != "latest.json")
                    .unwrap_or(false)
            })
            .collect();

        if entries.len() <= MAX_DISK_REVISIONS {
            return Ok(());
        }

        // Sort by modification time (oldest first)
        entries.sort_by_key(|entry| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        // Remove oldest revisions
        let to_remove = entries.len() - MAX_DISK_REVISIONS;
        for entry in entries.iter().take(to_remove) {
            if let Err(e) = fs::remove_file(entry.path()) {
                warn!("Failed to remove old revision: {}", e);
            }
        }

        debug!("Cleaned up {} old revisions", to_remove);
        Ok(())
    }

    /// Export state history to a file
    pub fn export_history<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let history = self.get_history(MAX_MEMORY_REVISIONS);
        let json = serde_json::to_string_pretty(&history)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Import state history from a file
    pub fn import_history<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let json = fs::read_to_string(path)?;
        let history: Vec<StateRevision> = serde_json::from_str(&json)?;

        // Verify integrity of all revisions
        for revision in &history {
            if !revision.verify_integrity() {
                return Err(Error::State(format!(
                    "Revision {} failed integrity check during import",
                    revision.id
                )));
            }
        }

        // Clear cache and add imported history
        {
            let mut cache = self.revision_cache.lock().unwrap();
            cache.clear();
            for revision in history.into_iter().take(MAX_MEMORY_REVISIONS) {
                cache.push_back(revision);
            }
        }

        info!("Imported state history");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_state_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path()).unwrap();
        let state = manager.get_current_state();
        assert_eq!(state.data, serde_json::json!({}));
    }

    #[test]
    fn test_state_update() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = StateManager::new(temp_dir.path()).unwrap();

        let mut new_state = SystemState::new();
        new_state.set("test", serde_json::json!("value")).unwrap();

        let revision_id = manager
            .update_state(
                new_state,
                "test".to_string(),
                Some("Test update".to_string()),
            )
            .unwrap();

        assert!(!revision_id.is_empty());

        let current = manager.get_current_state();
        assert_eq!(current.get("test").unwrap(), serde_json::json!("value"));
    }

    #[test]
    fn test_state_diff() {
        let mut state1 = SystemState::new();
        state1.set("key1", serde_json::json!("value1")).unwrap();
        state1.set("key2", serde_json::json!("value2")).unwrap();

        let mut state2 = SystemState::new();
        state2.set("key1", serde_json::json!("modified")).unwrap();
        state2.set("key3", serde_json::json!("value3")).unwrap();

        let diff = StateManager::diff_states(&state1, &state2);

        assert_eq!(diff.removed, vec!["key2"]);
        assert_eq!(diff.added, vec!["key3"]);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].0, "key1");
    }

    #[test]
    fn test_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = StateManager::new(temp_dir.path()).unwrap();

        // First update
        let mut state1 = SystemState::new();
        state1.set("version", serde_json::json!(1)).unwrap();
        let revision1 = manager
            .update_state(state1, "test".to_string(), Some("Version 1".to_string()))
            .unwrap();

        // Second update
        let mut state2 = SystemState::new();
        state2.set("version", serde_json::json!(2)).unwrap();
        manager
            .update_state(state2, "test".to_string(), Some("Version 2".to_string()))
            .unwrap();

        // Verify current state is version 2
        let current = manager.get_current_state();
        assert_eq!(current.get("version").unwrap(), serde_json::json!(2));

        // Rollback to version 1
        manager.rollback_to_revision(&revision1).unwrap();

        // Verify state is back to version 1
        let current = manager.get_current_state();
        assert_eq!(current.get("version").unwrap(), serde_json::json!(1));
    }

    #[test]
    fn test_history() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = StateManager::new(temp_dir.path()).unwrap();

        // Create multiple revisions
        for i in 1..=5 {
            let mut state = SystemState::new();
            state.set("count", serde_json::json!(i)).unwrap();
            manager
                .update_state(state, "test".to_string(), Some(format!("Update {}", i)))
                .unwrap();
        }

        let history = manager.get_history(10);
        assert_eq!(history.len(), 5);

        // Verify history is in reverse order (newest first)
        assert_eq!(history[0].description, Some("Update 5".to_string()));
        assert_eq!(history[4].description, Some("Update 1".to_string()));
    }
}
