//! Network link configuration management for base plugins.
//!
//! This task module is intended to handle interface/link-layer configuration
//! such as enabling/disabling interfaces, MTU, VLANs, bonding/aggregation,
//! and IP addressing primitives per-link (platform dependent).
//! For now, it is a placeholder scaffold; OS-specific implementations can
//! be filled in by the respective base plugin binaries.

use serde_json::Value;

#[derive(Default)]
pub struct NetworkLinks;

impl crate::TaskHandler for NetworkLinks {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired link config
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply link-level configuration (MTU, up/down, VLANs, etc.)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: perform imperative link actions if needed (e.g., bounce link)
        Ok(String::new())
    }
}
