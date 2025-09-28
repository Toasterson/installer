use crate::kdl_parser::{parse_kdl_file, parse_kdl_str, KdlSysConfig};
use crate::{Error, Result, SysConfigService, SystemState};
use serde_json::json;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

/// KDL Configuration Loader
///
/// This module provides functionality to load KDL configuration files
/// and apply them to the sysconfig service.
pub struct KdlConfigLoader {
    /// Path to the KDL configuration file
    config_path: Option<PathBuf>,

    /// Parsed KDL configuration
    config: Option<KdlSysConfig>,

    /// Whether to validate only (dry run mode)
    validate_only: bool,
}

impl KdlConfigLoader {
    /// Create a new KDL configuration loader
    pub fn new() -> Self {
        Self {
            config_path: None,
            config: None,
            validate_only: false,
        }
    }

    /// Create a loader with a specific configuration file path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            config_path: Some(path.as_ref().to_path_buf()),
            config: None,
            validate_only: false,
        }
    }

    /// Enable validation-only mode (dry run)
    pub fn validate_only(mut self, validate: bool) -> Self {
        self.validate_only = validate;
        self
    }

    /// Load a KDL configuration file
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        info!("Loading KDL configuration from: {}", path.display());

        let config = parse_kdl_file(path)
            .map_err(|e| Error::Plugin(format!("Failed to parse KDL file: {}", e)))?;

        self.config_path = Some(path.to_path_buf());
        self.config = Some(config);

        debug!("Successfully loaded KDL configuration");
        Ok(())
    }

    /// Load KDL configuration from a string
    pub fn load_string(&mut self, content: &str) -> Result<()> {
        info!("Loading KDL configuration from string");

        let config = parse_kdl_str(content)
            .map_err(|e| Error::Plugin(format!("Failed to parse KDL string: {}", e)))?;

        self.config = Some(config);

        debug!("Successfully parsed KDL configuration");
        Ok(())
    }

    /// Get the loaded configuration
    pub fn get_config(&self) -> Option<&KdlSysConfig> {
        self.config.as_ref()
    }

    /// Convert the KDL configuration to system state JSON
    pub fn to_system_state(&self) -> Result<SystemState> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| Error::State("No configuration loaded".to_string()))?;

        let mut state = SystemState::new();

        // Set hostname
        if let Some(hostname) = &config.hostname {
            state.set("hostname", json!(hostname))?;
        }

        // Set nameservers
        if !config.nameservers.is_empty() {
            state.set("nameservers", json!(config.nameservers))?;
        }

        // Set network interfaces
        let interfaces: Vec<serde_json::Value> = config
            .interfaces
            .iter()
            .map(|iface| {
                let addresses: Vec<serde_json::Value> = iface
                    .addresses
                    .iter()
                    .map(|addr| {
                        let mut addr_obj = json!({
                            "name": addr.name,
                            "kind": addr.kind,
                        });

                        if let Some(address) = &addr.address {
                            addr_obj["address"] = json!(address);
                        }

                        addr_obj
                    })
                    .collect();

                let mut iface_obj = json!({
                    "name": iface.name,
                    "addresses": addresses,
                });

                if let Some(selector) = &iface.selector {
                    iface_obj["selector"] = json!(selector);
                }

                iface_obj
            })
            .collect();

        state.set("interfaces", json!(interfaces))?;

        Ok(state)
    }

    /// Apply the loaded configuration to the system
    pub async fn apply(&self, service: &SysConfigService) -> Result<()> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| Error::State("No configuration loaded".to_string()))?;

        if self.validate_only {
            info!("Validation mode: configuration would be applied but not making changes");
            return self.validate_configuration(config);
        }

        info!("Applying KDL configuration to system");

        // Convert to system state
        let state = self.to_system_state()?;

        // Apply the state through the service
        // Convert state to JSON and apply with plugin_id "kdl-loader"
        let state_json = state.to_json()?;
        service.apply_state(&state_json, false, "kdl-loader").await?;

        info!("Successfully applied KDL configuration");
        Ok(())
    }

    /// Validate the configuration without applying it
    pub fn validate(&self) -> Result<()> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| Error::State("No configuration loaded".to_string()))?;

        self.validate_configuration(config)
    }

    /// Internal validation logic
    fn validate_configuration(&self, config: &KdlSysConfig) -> Result<()> {
        info!("Validating KDL configuration");

        // Validate hostname
        if let Some(hostname) = &config.hostname {
            if hostname.is_empty() {
                return Err(Error::State("Hostname cannot be empty".to_string()));
            }

            if hostname.len() > 255 {
                return Err(Error::State(
                    "Hostname too long (max 255 characters)".to_string(),
                ));
            }

            // Basic hostname validation (RFC 952/1123)
            if !hostname
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
            {
                warn!("Hostname contains non-standard characters: {}", hostname);
            }
        }

        // Validate nameservers
        for ns in &config.nameservers {
            if ns.is_empty() {
                return Err(Error::State(
                    "Nameserver address cannot be empty".to_string(),
                ));
            }
            // Basic IP address format check could be added here
            debug!("Nameserver: {}", ns);
        }

        // Validate interfaces
        for iface in &config.interfaces {
            if iface.name.is_empty() {
                return Err(Error::State("Interface name cannot be empty".to_string()));
            }

            debug!("Interface: {} (selector: {:?})", iface.name, iface.selector);

            // Validate addresses
            for addr in &iface.addresses {
                if addr.name.is_empty() {
                    return Err(Error::State(format!(
                        "Address name cannot be empty for interface {}",
                        iface.name
                    )));
                }

                // Validate static addresses have an address value
                if addr.kind == "static" && addr.address.is_none() {
                    return Err(Error::State(format!(
                        "Static address {} on interface {} requires an address value",
                        addr.name, iface.name
                    )));
                }

                debug!(
                    "  Address: {} (kind: {}, addr: {:?})",
                    addr.name, addr.kind, addr.address
                );
            }
        }

        info!("KDL configuration validation successful");
        Ok(())
    }

    /// Watch a KDL configuration file for changes
    pub async fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        use std::fs;
        use tokio::time::{interval, Duration};

        let path = path.as_ref().to_path_buf();
        let mut last_modified = fs::metadata(&path).and_then(|m| m.modified()).ok();

        let mut interval = interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if last_modified != Some(modified) {
                        info!("Configuration file changed, reloading: {}", path.display());

                        match self.load_file(&path) {
                            Ok(_) => {
                                info!("Successfully reloaded configuration");
                                last_modified = Some(modified);

                                // Emit a configuration change event
                                // This would trigger re-application if connected to a service
                            }
                            Err(e) => {
                                error!("Failed to reload configuration: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get a summary of the loaded configuration
    pub fn summary(&self) -> String {
        match &self.config {
            Some(config) => {
                let mut summary = String::new();

                if let Some(hostname) = &config.hostname {
                    summary.push_str(&format!("Hostname: {}\n", hostname));
                }

                if !config.nameservers.is_empty() {
                    summary.push_str(&format!("Nameservers: {}\n", config.nameservers.join(", ")));
                }

                if !config.interfaces.is_empty() {
                    summary.push_str(&format!(
                        "Interfaces: {}\n",
                        config
                            .interfaces
                            .iter()
                            .map(|i| i.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }

                summary
            }
            None => "No configuration loaded".to_string(),
        }
    }
}

/// Load and apply a KDL configuration file
pub async fn load_and_apply_kdl<P: AsRef<Path>>(
    path: P,
    service: &SysConfigService,
    dry_run: bool,
) -> Result<()> {
    let mut loader = KdlConfigLoader::new().validate_only(dry_run);

    loader.load_file(path)?;
    loader.validate()?;

    if !dry_run {
        // Apply using the async version since apply_state is now async
        let state = loader.to_system_state()?;
        let state_json = state.to_json()?;
        service.apply_state(&state_json, false, "kdl-loader").await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = KdlConfigLoader::new();
        assert!(loader.get_config().is_none());
    }

    #[test]
    fn test_load_string() {
        let mut loader = KdlConfigLoader::new();
        let kdl = r#"
            sysconfig {
                hostname "test-host"
                nameserver "8.8.8.8"
            }
        "#;

        loader.load_string(kdl).unwrap();
        let config = loader.get_config().unwrap();
        assert_eq!(config.hostname, Some("test-host".to_string()));
    }

    #[test]
    fn test_validation() {
        let mut loader = KdlConfigLoader::new();
        let kdl = r#"
            sysconfig {
                hostname "valid-hostname"
                nameserver "8.8.8.8"

                interface "eth0" {
                    address name="v4" kind="dhcp4"
                }
            }
        "#;

        loader.load_string(kdl).unwrap();
        assert!(loader.validate().is_ok());
    }

    #[test]
    fn test_validation_error_empty_hostname() {
        let mut loader = KdlConfigLoader::new();
        let kdl = r#"
            sysconfig {
                hostname ""
            }
        "#;

        loader.load_string(kdl).unwrap();
        assert!(loader.validate().is_err());
    }

    #[test]
    fn test_to_system_state() {
        let mut loader = KdlConfigLoader::new();
        let kdl = r#"
            sysconfig {
                hostname "test-host"
                nameserver "8.8.8.8"

                interface "eth0" {
                    address name="v4" kind="static" "192.168.1.100/24"
                }
            }
        "#;

        loader.load_string(kdl).unwrap();
        let state = loader.to_system_state().unwrap();

        assert_eq!(state.get("hostname").unwrap(), json!("test-host"));
        assert_eq!(state.get("nameservers").unwrap(), json!(["8.8.8.8"]));

        let interfaces = state.get("interfaces").unwrap();
        assert!(interfaces.is_array());
    }

    #[test]
    fn test_summary() {
        let mut loader = KdlConfigLoader::new();
        let kdl = r#"
            sysconfig {
                hostname "test-host"
                nameserver "8.8.8.8"
                interface "eth0" {
                    address name="v4" kind="dhcp4"
                }
            }
        "#;

        loader.load_string(kdl).unwrap();
        let summary = loader.summary();

        assert!(summary.contains("Hostname: test-host"));
        assert!(summary.contains("Nameservers: 8.8.8.8"));
        assert!(summary.contains("Interfaces: eth0"));
    }
}
