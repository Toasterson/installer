//! Storage-related management for base plugins.
//!
//! Currently focused on ZFS (pools, datasets, volumes). This module is a
//! placeholder scaffold; OS-specific implementations can be added later.

use serde_json::Value;

#[derive(Default)]
pub struct StorageZfs;

impl crate::TaskHandler for StorageZfs {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired ZFS state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply ZFS changes (create pools/datasets/volumes, set properties)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative ZFS actions (e.g. snapshot, rollback)
        Ok(String::new())
    }
}
