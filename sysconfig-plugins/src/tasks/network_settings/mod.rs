//! Network settings management for base plugins (hostname and DNS).
//!
//! Implements per-OS logic in submodules, selected via cfg(target_os).
//! Supported OS modules: linux, illumos, freebsd. Windows is stubbed.

use serde::Deserialize;
use serde_json::Value;

// Submodules per OS live under tasks/network_settings/
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "illumos")]
mod illumos;
#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "windows")]
mod windows;

// Re-export the active OS implementation as os_impl
#[cfg(target_os = "linux")]
use linux as os_impl;
#[cfg(target_os = "illumos")]
use illumos as os_impl;
#[cfg(target_os = "freebsd")]
use freebsd as os_impl;
#[cfg(target_os = "windows")]
use windows as os_impl;

#[derive(Default)]
pub struct NetworkSettings;

// Simple desired settings schema we support initially
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DesiredDns {
    #[serde(default)]
    nameservers: Vec<String>,
    #[serde(default)]
    search: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DesiredSettings {
    #[serde(default)]
    hostname: Option<String>,
    #[serde(default)]
    dns: Option<DesiredDns>,
}

fn parse_desired(desired: &Value) -> Result<DesiredSettings, String> {
    let mut ds: DesiredSettings = serde_json::from_value(desired.clone())
        .map_err(|e| format!("{}", e))?;

    // Trim and sanitize
    if let Some(h) = ds.hostname.take() {
        let t = h.trim().to_string();
        ds.hostname = if t.is_empty() { None } else { Some(t) };
    }
    if let Some(ref mut d) = ds.dns {
        let ns: Vec<String> = d
            .nameservers
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        d.nameservers = ns;
        let search: Vec<String> = d
            .search
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        d.search = search;
        if d.nameservers.is_empty() && d.search.is_empty() {
            ds.dns = None;
        }
    }

    Ok(ds)
}

impl crate::TaskHandler for NetworkSettings {
    fn diff(&self, _current: &Value, desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        let ds = parse_desired(desired)?;
        let mut changes = Vec::new();
        if let Some(h) = ds.hostname {
            changes.push(crate::TaskChange {
                change_type: crate::TaskChangeType::Update,
                path: "network.settings.hostname".to_string(),
                old_value: None,
                new_value: Some(serde_json::json!(h)),
                verbose: false,
            });
        }
        if let Some(dns) = ds.dns {
            changes.push(crate::TaskChange {
                change_type: crate::TaskChangeType::Update,
                path: "network.settings.dns".to_string(),
                old_value: None,
                new_value: Some(serde_json::json!({
                    "nameservers": dns.nameservers,
                    "search": dns.search,
                })),
                verbose: false,
            });
        }
        Ok(changes)
    }

    fn apply(&self, desired: &Value, dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        let ds = parse_desired(desired).map_err(|e| format!("invalid network.settings schema: {}", e))?;
        let mut changes = Vec::new();

        if let Some(h) = ds.hostname {
            match os_impl::apply_hostname(&h, dry_run) {
                Ok(applied) => {
                    if applied {
                        changes.push(crate::TaskChange {
                            change_type: crate::TaskChangeType::Update,
                            path: "network.settings.hostname".to_string(),
                            old_value: None,
                            new_value: Some(serde_json::json!(h)),
                            verbose: false,
                        });
                    }
                }
                Err(e) => {
                    return Err(format!("error setting hostname: {}", e));
                }
            }
        }
        if let Some(dns) = ds.dns {
            match os_impl::apply_dns(&dns.nameservers, &dns.search, dry_run) {
                Ok(applied) => {
                    if applied {
                        changes.push(crate::TaskChange {
                            change_type: crate::TaskChangeType::Update,
                            path: "network.settings.dns".to_string(),
                            old_value: None,
                            new_value: Some(serde_json::json!({
                                "nameservers": dns.nameservers,
                                "search": dns.search,
                            })),
                            verbose: false,
                        });
                    }
                }
                Err(e) => {
                    return Err(format!("error updating DNS: {}", e));
                }
            }
        }
        Ok(changes)
    }

    fn exec(&self, action: &str, _params: &Value) -> Result<String, String> {
        match action {
            "refresh-dns" => {
                // Best-effort stub; many systems pick up resolv.conf instantly.
                // On systemd-resolved one could call `resolvectl flush-caches`.
                #[cfg(target_os = "linux")]
                {
                    if let Err(e) = os_impl::flush_dns_cache() {
                        return Err(format!("failed to refresh DNS cache: {}", e));
                    }
                }
                Ok(String::from("dns refresh attempted"))
            }
            _ => Ok(String::new()),
        }
    }
}

impl NetworkSettings {
    /// Validate the desired settings against our schema. Returns Err if invalid.
    pub fn validate_schema(desired: &Value) -> Result<(), String> {
        parse_desired(desired).map(|_| ())
    }
}
