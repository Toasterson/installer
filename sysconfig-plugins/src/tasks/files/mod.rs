//! Simple file state management with static content (no templating).
//!
//! NOTE: Templates must be pre-rendered outside of sysconfig. This module only
//! deals with explicit file paths, permissions, ownership, and static content.
//! ACLs are explicitly out of scope for now.

use serde::Deserialize;
use serde_json::Value;

#[derive(Default)]
pub struct Files;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ensure {
    Present,
    Absent,
}

impl Default for Ensure {
    fn default() -> Self { Ensure::Present }
}

impl<'de> Deserialize<'de> for Ensure {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "present" => Ok(Ensure::Present),
            "absent" => Ok(Ensure::Absent),
            other => Err(serde::de::Error::custom(format!(
                "invalid ensure value: {} (expected 'present' or 'absent')",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileSpec {
    /// Absolute path of the file to manage
    path: String,
    /// Ensure file is present or absent (default: present)
    #[serde(default)]
    ensure: Ensure,
    /// UTF-8 content to write. If omitted and file is absent, an empty file will be created.
    #[serde(default)]
    content: Option<String>,
    /// File mode (permissions) as a string like "0644" or "644". Only honored on Unix.
    #[serde(default)]
    mode: Option<String>,
    /// Numeric uid to set ownership. Only honored on Unix.
    #[serde(default)]
    uid: Option<u32>,
    /// Numeric gid to set group. Only honored on Unix.
    #[serde(default)]
    gid: Option<u32>,
}

fn parse_specs(desired: &Value) -> Result<Vec<FileSpec>, String> {
    // Accept either an array of file specs, or an object { files: [...] }
    if desired.is_array() {
        serde_json::from_value::<Vec<FileSpec>>(desired.clone()).map_err(|e| e.to_string())
    } else if let Some(arr) = desired.get("files").cloned() {
        serde_json::from_value::<Vec<FileSpec>>(arr).map_err(|e| e.to_string())
    } else {
        Err("files must be an array of file specs".to_string())
    }
}

// OS-specific implementations live in submodules
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

// Re-export active implementation as os_impl
#[cfg(unix)]
use unix as os_impl;
#[cfg(windows)]
use windows as os_impl;

impl crate::TaskHandler for Files {
    fn diff(&self, _current: &Value, desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        let specs = parse_specs(desired)?;
        os_impl::diff(&specs).map_err(|e| e.to_string())
    }

    fn apply(&self, desired: &Value, dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        let specs = parse_specs(desired).map_err(|e| format!("invalid files schema: {}", e))?;
        os_impl::apply(&specs, dry_run).map_err(|e| e.to_string())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        Ok(String::new())
    }
}
