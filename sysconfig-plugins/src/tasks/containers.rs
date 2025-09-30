//! Container management for base plugins.
//!
//! Provides platform-specific container implementations:
//! - illumos: Zones
//! - FreeBSD: Jails
//! - Linux: Docker/Podman containers

use serde_json::Value;

#[derive(Default)]
pub struct ContainerZones;

impl crate::TaskHandler for ContainerZones {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired Zone state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply Zone changes (create/configure zones, set properties)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative Zone actions (e.g. boot, halt, reboot)
        Ok(String::new())
    }
}

#[derive(Default)]
pub struct ContainerJails;

impl crate::TaskHandler for ContainerJails {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired Jail state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply Jail changes (create/configure jails, set properties)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative Jail actions (e.g. start, stop, restart)
        Ok(String::new())
    }
}

#[derive(Default)]
pub struct ContainerDocker;

impl crate::TaskHandler for ContainerDocker {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired Docker container state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply Docker container changes (create/run/configure containers)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative Docker actions (e.g. start, stop, restart, exec)
        Ok(String::new())
    }
}

#[derive(Default)]
pub struct ContainerPodman;

impl crate::TaskHandler for ContainerPodman {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired Podman container state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply Podman container changes (create/run/configure containers)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative Podman actions (e.g. start, stop, restart, exec)
        Ok(String::new())
    }
}
