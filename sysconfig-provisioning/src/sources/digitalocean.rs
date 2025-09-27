use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::config::{
    AddressConfig, AddressType, InterfaceConfig, ProvisioningConfig, RouteConfig, UserConfig,
};
use crate::sources::utils;

/// DigitalOcean metadata source implementation
pub struct DigitalOceanSource {
    metadata_url: String,
    timeout_seconds: u64,
    config_drive_path: PathBuf,
}

impl DigitalOceanSource {
    /// Create a new DigitalOcean metadata source
    pub fn new() -> Self {
        Self {
            metadata_url: "http://169.254.169.254".to_string(),
            timeout_seconds: 5,
            config_drive_path: PathBuf::from("/mnt/config-2"),
        }
    }

    /// Set the timeout for metadata requests
    pub fn set_timeout(&mut self, seconds: u64) {
        self.timeout_seconds = seconds;
    }

    /// Set the config drive mount path
    pub fn set_config_drive_path(&mut self, path: PathBuf) {
        self.config_drive_path = path;
    }

    /// Load configuration from DigitalOcean metadata
    pub async fn load(&self) -> Result<ProvisioningConfig> {
        info!("Loading configuration from DigitalOcean metadata");

        // Try metadata service first
        if let Ok(config) = self.load_from_metadata_service().await {
            return Ok(config);
        }

        // Fall back to config drive
        if let Ok(config) = self.load_from_config_drive().await {
            return Ok(config);
        }

        Err(anyhow::anyhow!(
            "Failed to load configuration from both metadata service and config drive"
        ))
    }

    /// Load configuration from metadata service
    async fn load_from_metadata_service(&self) -> Result<ProvisioningConfig> {
        debug!("Attempting to load from DigitalOcean metadata service");

        let mut config = ProvisioningConfig::new();

        // Fetch droplet metadata
        let metadata = self.fetch_droplet_metadata().await?;

        // Parse hostname
        if let Some(hostname) = metadata.get("hostname").and_then(|h| h.as_str()) {
            config.hostname = Some(hostname.to_string());
        }

        // Parse network configuration
        if let Ok(interfaces) = self.parse_network_config(&metadata) {
            config.interfaces = interfaces;
        }

        // Parse SSH keys
        if let Some(public_keys) = metadata.get("public_keys").and_then(|k| k.as_array()) {
            for key in public_keys {
                if let Some(key_str) = key.as_str() {
                    config.ssh_authorized_keys.push(key_str.to_string());
                }
            }
        }

        // Fetch user data
        if let Ok(user_data) = self.fetch_user_data().await {
            config.user_data = Some(user_data);
        }

        // Store metadata
        config
            .metadata
            .insert("digitalocean-droplet".to_string(), metadata.clone());

        Ok(config)
    }

    /// Load configuration from config drive
    async fn load_from_config_drive(&self) -> Result<ProvisioningConfig> {
        debug!("Attempting to load from DigitalOcean config drive");

        // Check if config drive is available
        let device = utils::find_device_by_label("config-2")
            .await
            .or_else(async || utils::find_device_by_label("CONFIG-2").await)
            .context("Config drive not found")?;

        // Mount the config drive if not already mounted
        if !self.config_drive_path.exists() {
            tokio::fs::create_dir_all(&self.config_drive_path)
                .await
                .context("Failed to create mount point")?;
        }

        // Mount as ISO9660 or FAT
        utils::mount_filesystem(&device, &self.config_drive_path, Some("iso9660"))
            .await
            .or_else(async |_| {
                utils::mount_filesystem(&device, &self.config_drive_path, Some("vfat")).await
            })
            .context("Failed to mount config drive")?;

        // Read the configuration
        let result = self.read_config_drive().await;

        // Unmount when done
        let _ = utils::unmount_filesystem(&self.config_drive_path).await;

        result
    }

    /// Read configuration from mounted config drive
    async fn read_config_drive(&self) -> Result<ProvisioningConfig> {
        let mut config = ProvisioningConfig::new();

        // Read meta_data.json
        let meta_data_path = self.config_drive_path.join("digitalocean_meta_data.json");
        if meta_data_path.exists() {
            let content = tokio::fs::read_to_string(&meta_data_path)
                .await
                .context("Failed to read meta_data.json")?;

            let metadata: serde_json::Value =
                serde_json::from_str(&content).context("Failed to parse meta_data.json")?;

            // Parse hostname
            if let Some(hostname) = metadata.get("hostname").and_then(|h| h.as_str()) {
                config.hostname = Some(hostname.to_string());
            }

            // Parse network configuration
            if let Ok(interfaces) = self.parse_network_config(&metadata) {
                config.interfaces = interfaces;
            }

            // Parse SSH keys
            if let Some(public_keys) = metadata.get("public_keys").and_then(|k| k.as_array()) {
                for key in public_keys {
                    if let Some(key_str) = key.as_str() {
                        config.ssh_authorized_keys.push(key_str.to_string());
                    }
                }
            }

            // Store metadata
            config
                .metadata
                .insert("digitalocean-droplet".to_string(), metadata);
        }

        // Read user_data
        let user_data_path = self.config_drive_path.join("user_data");
        if user_data_path.exists() {
            let user_data = tokio::fs::read_to_string(&user_data_path)
                .await
                .context("Failed to read user_data")?;
            config.user_data = Some(user_data);
        }

        Ok(config)
    }

    /// Fetch droplet metadata from API
    async fn fetch_droplet_metadata(&self) -> Result<serde_json::Value> {
        let url = format!("{}/metadata/v1.json", self.metadata_url);

        utils::fetch_metadata_json(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch droplet metadata")
    }

    /// Parse network configuration from metadata
    fn parse_network_config(
        &self,
        metadata: &serde_json::Value,
    ) -> Result<HashMap<String, InterfaceConfig>> {
        let mut interfaces = HashMap::new();

        // Parse interfaces
        if let Some(ifaces) = metadata.get("interfaces") {
            // Handle public interfaces
            if let Some(public_ifaces) = ifaces.get("public").and_then(|p| p.as_array()) {
                for (idx, iface) in public_ifaces.iter().enumerate() {
                    let interface_name = format!("eth{}", idx);
                    let mut interface = self.parse_interface(iface)?;
                    interface.description = Some("DigitalOcean public interface".to_string());
                    interfaces.insert(interface_name, interface);
                }
            }

            // Handle private interfaces
            if let Some(private_ifaces) = ifaces.get("private").and_then(|p| p.as_array()) {
                let offset = interfaces.len();
                for (idx, iface) in private_ifaces.iter().enumerate() {
                    let interface_name = format!("eth{}", offset + idx);
                    let mut interface = self.parse_interface(iface)?;
                    interface.description = Some("DigitalOcean private interface".to_string());
                    interfaces.insert(interface_name, interface);
                }
            }
        }

        // Legacy format support
        if interfaces.is_empty() {
            if let Some(ifaces_array) = metadata.get("interfaces").and_then(|i| i.as_array()) {
                for (idx, iface) in ifaces_array.iter().enumerate() {
                    let interface_name = format!("eth{}", idx);
                    let interface = self.parse_interface(iface)?;
                    interfaces.insert(interface_name, interface);
                }
            }
        }

        Ok(interfaces)
    }

    /// Parse a single interface configuration
    fn parse_interface(&self, iface: &serde_json::Value) -> Result<InterfaceConfig> {
        let mut interface = InterfaceConfig {
            mac_address: iface
                .get("mac")
                .and_then(|m| m.as_str())
                .map(|s| utils::normalize_mac_address(s)),
            mtu: None,
            addresses: Vec::new(),
            enabled: true,
            description: None,
            vlan_id: None,
            parent: None,
        };

        // Parse IPv4 configuration
        if let Some(ipv4) = iface.get("ipv4") {
            if let Some(ip_address) = ipv4.get("ip_address").and_then(|ip| ip.as_str()) {
                let netmask = ipv4
                    .get("netmask")
                    .and_then(|nm| nm.as_str())
                    .unwrap_or("255.255.255.0");

                let prefix_len = utils::netmask_to_cidr(netmask).unwrap_or(24);

                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Static,
                    address: Some(format!("{}/{}", ip_address, prefix_len)),
                    gateway: ipv4
                        .get("gateway")
                        .and_then(|g| g.as_str())
                        .map(|s| s.to_string()),
                    primary: true,
                });
            }
        }

        // Parse IPv6 configuration
        if let Some(ipv6) = iface.get("ipv6") {
            if let Some(ip_address) = ipv6.get("ip_address").and_then(|ip| ip.as_str()) {
                let prefix_len = ipv6.get("cidr").and_then(|c| c.as_u64()).unwrap_or(64) as u8;

                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Static,
                    address: Some(format!("{}/{}", ip_address, prefix_len)),
                    gateway: ipv6
                        .get("gateway")
                        .and_then(|g| g.as_str())
                        .map(|s| s.to_string()),
                    primary: false,
                });
            }
        }

        // Parse anchor IPs (floating IPs)
        if let Some(anchor_ipv4) = iface.get("anchor_ipv4") {
            if let Some(ip_address) = anchor_ipv4.get("ip_address").and_then(|ip| ip.as_str()) {
                let netmask = anchor_ipv4
                    .get("netmask")
                    .and_then(|nm| nm.as_str())
                    .unwrap_or("255.255.255.0");

                let prefix_len = utils::netmask_to_cidr(netmask).unwrap_or(24);

                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Static,
                    address: Some(format!("{}/{}", ip_address, prefix_len)),
                    gateway: None,
                    primary: false,
                });
            }
        }

        // If no addresses configured, use DHCP
        if interface.addresses.is_empty() {
            interface.addresses.push(AddressConfig {
                addr_type: AddressType::Dhcp4,
                address: None,
                gateway: None,
                primary: true,
            });
        }

        Ok(interface)
    }

    /// Fetch user data
    async fn fetch_user_data(&self) -> Result<String> {
        let url = format!("{}/metadata/v1/user-data", self.metadata_url);

        utils::fetch_metadata(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch user data")
    }

    /// Fetch vendor data (DigitalOcean specific scripts)
    pub async fn fetch_vendor_data(&self) -> Result<String> {
        let url = format!("{}/metadata/v1/vendor-data", self.metadata_url);

        utils::fetch_metadata(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch vendor data")
    }

    /// Get droplet tags
    pub async fn fetch_tags(&self) -> Result<Vec<String>> {
        let url = format!("{}/metadata/v1/tags", self.metadata_url);

        match utils::fetch_metadata(&url, None, self.timeout_seconds).await {
            Ok(tags_text) => Ok(tags_text
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()),
            Err(e) => {
                debug!("No tags found: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Get droplet features
    pub async fn fetch_features(&self) -> Result<serde_json::Value> {
        let url = format!("{}/metadata/v1/features", self.metadata_url);

        utils::fetch_metadata_json(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch features")
    }
}

impl Default for DigitalOceanSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digitalocean_source_creation() {
        let source = DigitalOceanSource::new();
        assert_eq!(source.metadata_url, "http://169.254.169.254");
        assert_eq!(source.timeout_seconds, 5);
        assert_eq!(source.config_drive_path, PathBuf::from("/mnt/config-2"));
    }

    #[test]
    fn test_set_timeout() {
        let mut source = DigitalOceanSource::new();
        source.set_timeout(10);
        assert_eq!(source.timeout_seconds, 10);
    }

    #[test]
    fn test_set_config_drive_path() {
        let mut source = DigitalOceanSource::new();
        let new_path = PathBuf::from("/tmp/config");
        source.set_config_drive_path(new_path.clone());
        assert_eq!(source.config_drive_path, new_path);
    }
}
