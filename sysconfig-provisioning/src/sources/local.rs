use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info};

use crate::config::{AddressConfig, AddressType, InterfaceConfig, ProvisioningConfig};

/// Source for reading local configuration files
pub struct LocalSource;

impl LocalSource {
    /// Create a new local source
    pub fn new() -> Self {
        Self
    }

    /// Load configuration from a KDL file
    pub async fn load_kdl(&self, path: &Path) -> Result<ProvisioningConfig> {
        info!("Loading local KDL configuration from {:?}", path);

        // Read the file
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read KDL file from {:?}", path))?;

        // Parse the KDL content
        self.parse_kdl_content(&content)
            .with_context(|| format!("Failed to parse KDL file from {:?}", path))
    }

    /// Parse KDL content into ProvisioningConfig
    fn parse_kdl_content(&self, content: &str) -> Result<ProvisioningConfig> {
        // Use the sysconfig parser to parse the KDL
        let kdl_config = sysconfig::config::parse_config("sysconfig.kdl", content)?;

        let mut config = ProvisioningConfig::default();

        // Set hostname
        config.hostname = Some(kdl_config.hostname);

        // Set nameservers
        config.nameservers = kdl_config.nameservers;

        // Convert interfaces
        for iface in kdl_config.interfaces {
            if let Some(name) = iface.name {
                let mut interface_config = InterfaceConfig {
                    mac_address: None,
                    mtu: None,
                    addresses: Vec::new(),
                    enabled: true,
                    description: None,
                    vlan_id: None,
                    parent: None,
                };

                // If selector is present, it might be a MAC address
                if let Some(selector) = iface.selector {
                    if selector.contains(':') {
                        interface_config.mac_address = Some(selector);
                    }
                }

                // Convert addresses
                for addr in iface.addresses {
                    let addr_type = match addr.kind.to_string().to_lowercase().as_str() {
                        "dhcp4" => AddressType::Dhcp4,
                        "dhcp6" => AddressType::Dhcp6,
                        "dhcp" => AddressType::Dhcp,
                        "static" => AddressType::Static,
                        "slaac" => AddressType::Slaac,
                        "addrconf" => AddressType::Addrconf,
                        _ => AddressType::Static,
                    };

                    let address_config = AddressConfig {
                        addr_type,
                        address: addr.address,
                        gateway: None, // KDL format doesn't directly support gateway in address
                        primary: false,
                    };

                    interface_config.addresses.push(address_config);
                }

                config.interfaces.insert(name, interface_config);
            }
        }

        debug!("Parsed KDL configuration: {:?}", config);
        Ok(config)
    }

    /// Load configuration from a YAML file (for compatibility)
    pub async fn load_yaml(&self, path: &Path) -> Result<ProvisioningConfig> {
        info!("Loading local YAML configuration from {:?}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read YAML file from {:?}", path))?;

        let config: ProvisioningConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file from {:?}", path))?;

        Ok(config)
    }

    /// Load configuration from a JSON file
    pub async fn load_json(&self, path: &Path) -> Result<ProvisioningConfig> {
        info!("Loading local JSON configuration from {:?}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read JSON file from {:?}", path))?;

        let config: ProvisioningConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON file from {:?}", path))?;

        Ok(config)
    }

    /// Load configuration from a TOML file
    pub async fn load_toml(&self, path: &Path) -> Result<ProvisioningConfig> {
        info!("Loading local TOML configuration from {:?}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read TOML file from {:?}", path))?;

        let config: ProvisioningConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML file from {:?}", path))?;

        Ok(config)
    }

    /// Try to load configuration from any supported format
    pub async fn load_any(&self, base_path: &Path) -> Result<ProvisioningConfig> {
        // Try different file extensions in order of preference
        let extensions = vec![
            ("kdl", Self::load_kdl),
            ("yaml", Self::load_yaml),
            ("yml", Self::load_yaml),
            ("json", Self::load_json),
            ("toml", Self::load_toml),
        ];

        for (ext, loader) in extensions {
            let path = base_path.with_extension(ext);
            if path.exists() {
                debug!("Found configuration file: {:?}", path);
                return loader(self, &path).await;
            }
        }

        Err(anyhow::anyhow!(
            "No configuration file found at {:?} with any supported extension",
            base_path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_parse_kdl_content() {
        let source = LocalSource::new();
        let kdl_content = r#"
            hostname "test-host"
            nameserver "8.8.8.8"
            nameserver "8.8.4.4"

            interface "eth0" {
                address name="ipv4" kind="static" "192.168.1.10/24"
            }

            interface "eth1" {
                address name="dhcp" kind="dhcp4"
            }
        "#;

        let config = source.parse_kdl_content(kdl_content).unwrap();

        assert_eq!(config.hostname, Some("test-host".to_string()));
        assert_eq!(config.nameservers.len(), 2);
        assert_eq!(config.nameservers[0], "8.8.8.8");
        assert_eq!(config.nameservers[1], "8.8.4.4");

        assert_eq!(config.interfaces.len(), 2);
        assert!(config.interfaces.contains_key("eth0"));
        assert!(config.interfaces.contains_key("eth1"));

        let eth0 = &config.interfaces["eth0"];
        assert_eq!(eth0.addresses.len(), 1);
        assert_eq!(eth0.addresses[0].addr_type, AddressType::Static);
        assert_eq!(
            eth0.addresses[0].address,
            Some("192.168.1.10/24".to_string())
        );

        let eth1 = &config.interfaces["eth1"];
        assert_eq!(eth1.addresses.len(), 1);
        assert_eq!(eth1.addresses[0].addr_type, AddressType::Dhcp4);
    }

    #[tokio::test]
    async fn test_load_json() {
        let source = LocalSource::new();
        let mut temp_file = NamedTempFile::new().unwrap();

        let json_content = r#"{
            "hostname": "json-host",
            "nameservers": ["1.1.1.1"],
            "interfaces": {
                "eth0": {
                    "mtu": 1500,
                    "addresses": [
                        {
                            "type": "static",
                            "address": "10.0.0.10/24",
                            "gateway": "10.0.0.1"
                        }
                    ]
                }
            }
        }"#;

        temp_file.write_all(json_content.as_bytes()).unwrap();

        let config = source.load_json(temp_file.path()).await.unwrap();

        assert_eq!(config.hostname, Some("json-host".to_string()));
        assert_eq!(config.nameservers.len(), 1);
        assert_eq!(config.nameservers[0], "1.1.1.1");

        let eth0 = &config.interfaces["eth0"];
        assert_eq!(eth0.mtu, Some(1500));
        assert_eq!(eth0.addresses[0].address, Some("10.0.0.10/24".to_string()));
        assert_eq!(eth0.addresses[0].gateway, Some("10.0.0.1".to_string()));
    }

    #[tokio::test]
    async fn test_load_yaml() {
        let source = LocalSource::new();
        let mut temp_file = NamedTempFile::new().unwrap();

        let yaml_content = r#"
hostname: yaml-host
nameservers:
  - 8.8.8.8
  - 8.8.4.4
interfaces:
  eth0:
    mtu: 9000
    addresses:
      - type: dhcp4
"#;

        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let config = source.load_yaml(temp_file.path()).await.unwrap();

        assert_eq!(config.hostname, Some("yaml-host".to_string()));
        assert_eq!(config.nameservers.len(), 2);

        let eth0 = &config.interfaces["eth0"];
        assert_eq!(eth0.mtu, Some(9000));
        assert_eq!(eth0.addresses[0].addr_type, AddressType::Dhcp4);
    }
}
