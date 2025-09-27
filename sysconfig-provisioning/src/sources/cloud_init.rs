use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::config::{
    AddressConfig, AddressType, InterfaceConfig, NetworkConfigV1, NetworkConfigV1Item,
    ProvisioningConfig, RouteConfig, SubnetConfig, UserConfig,
};
use crate::sources::utils;

/// CloudInit metadata source implementation
/// Supports multiple cloud-init datasources including NoCloud, ConfigDrive, and metadata services
pub struct CloudInitSource {
    metadata_url: String,
    timeout_seconds: u64,
    config_drive_paths: Vec<PathBuf>,
    nocloud_path: PathBuf,
}

impl CloudInitSource {
    /// Create a new CloudInit metadata source
    pub fn new() -> Self {
        Self {
            metadata_url: "http://169.254.169.254".to_string(),
            timeout_seconds: 5,
            config_drive_paths: vec![
                PathBuf::from("/mnt/config-2"),
                PathBuf::from("/mnt/config"),
                PathBuf::from("/media/config-2"),
                PathBuf::from("/media/config"),
            ],
            nocloud_path: PathBuf::from("/var/lib/cloud/seed/nocloud"),
        }
    }

    /// Set the timeout for metadata requests
    pub fn set_timeout(&mut self, seconds: u64) {
        self.timeout_seconds = seconds;
    }

    /// Add a config drive path to check
    pub fn add_config_drive_path(&mut self, path: PathBuf) {
        if !self.config_drive_paths.contains(&path) {
            self.config_drive_paths.push(path);
        }
    }

    /// Set the NoCloud seed directory path
    pub fn set_nocloud_path(&mut self, path: PathBuf) {
        self.nocloud_path = path;
    }

    /// Load configuration from cloud-init sources
    pub async fn load(&self) -> Result<ProvisioningConfig> {
        info!("Loading configuration from cloud-init sources");

        // Try NoCloud first (local seed directory)
        if let Ok(config) = self.load_from_nocloud().await {
            info!("Successfully loaded from NoCloud");
            return Ok(config);
        }

        // Try ConfigDrive
        if let Ok(config) = self.load_from_config_drive().await {
            info!("Successfully loaded from ConfigDrive");
            return Ok(config);
        }

        // Try EC2-style metadata service
        if let Ok(config) = self.load_from_metadata_service().await {
            info!("Successfully loaded from metadata service");
            return Ok(config);
        }

        // Try network datasource (newer cloud-init)
        if let Ok(config) = self.load_from_network_datasource().await {
            info!("Successfully loaded from network datasource");
            return Ok(config);
        }

        Err(anyhow::anyhow!(
            "Failed to load configuration from any cloud-init source"
        ))
    }

    /// Load configuration from NoCloud seed directory
    async fn load_from_nocloud(&self) -> Result<ProvisioningConfig> {
        debug!("Attempting to load from NoCloud at {:?}", self.nocloud_path);

        if !self.nocloud_path.exists() {
            return Err(anyhow::anyhow!("NoCloud seed directory not found"));
        }

        let mut config = ProvisioningConfig::new();

        // Read meta-data
        let meta_data_path = self.nocloud_path.join("meta-data");
        if meta_data_path.exists() {
            let content = tokio::fs::read_to_string(&meta_data_path)
                .await
                .context("Failed to read meta-data")?;

            // Parse YAML or JSON
            let metadata = if content.trim().starts_with('{') {
                serde_json::from_str(&content).context("Failed to parse meta-data as JSON")?
            } else {
                serde_yaml::from_str(&content).context("Failed to parse meta-data as YAML")?
            };

            self.parse_metadata(&metadata, &mut config)?;
        }

        // Read user-data
        let user_data_path = self.nocloud_path.join("user-data");
        if user_data_path.exists() {
            let user_data = tokio::fs::read_to_string(&user_data_path)
                .await
                .context("Failed to read user-data")?;

            if !user_data.trim().is_empty() {
                config.user_data = Some(user_data);
            }
        }

        // Read network-config
        let network_config_path = self.nocloud_path.join("network-config");
        if network_config_path.exists() {
            let content = tokio::fs::read_to_string(&network_config_path)
                .await
                .context("Failed to read network-config")?;

            let network_config: serde_json::Value = if content.trim().starts_with('{') {
                serde_json::from_str(&content).context("Failed to parse network-config as JSON")?
            } else {
                serde_yaml::from_str(&content).context("Failed to parse network-config as YAML")?
            };

            if let Ok(interfaces) = self.parse_network_config(&network_config) {
                config.interfaces = interfaces;
            }
        }

        Ok(config)
    }

    /// Load configuration from config drive
    async fn load_from_config_drive(&self) -> Result<ProvisioningConfig> {
        debug!("Attempting to load from ConfigDrive");

        // Find and mount config drive
        let device = utils::find_device_by_label("config-2")
            .await
            .or_else(async || utils::find_device_by_label("CONFIG-2").await)
            .or_else(async || utils::find_device_by_label("cidata").await)
            .or_else(async || utils::find_device_by_label("CIDATA").await)
            .context("Config drive not found")?;

        // Try each potential mount path
        for mount_path in &self.config_drive_paths {
            if let Ok(config) = self.try_mount_and_read(&device, mount_path).await {
                return Ok(config);
            }
        }

        Err(anyhow::anyhow!("Failed to mount and read config drive"))
    }

    /// Try to mount and read from a specific path
    async fn try_mount_and_read(
        &self,
        device: &Path,
        mount_path: &Path,
    ) -> Result<ProvisioningConfig> {
        // Create mount point if needed
        if !mount_path.exists() {
            tokio::fs::create_dir_all(mount_path)
                .await
                .context("Failed to create mount point")?;
        }

        // Mount filesystem
        utils::mount_filesystem(device, mount_path, Some("iso9660"))
            .await
            .or_else(async |_| utils::mount_filesystem(device, mount_path, Some("vfat")).await)
            .context("Failed to mount config drive")?;

        // Read configuration
        let result = self.read_config_drive(mount_path).await;

        // Unmount
        let _ = utils::unmount_filesystem(mount_path).await;

        result
    }

    /// Read configuration from mounted config drive
    async fn read_config_drive(&self, mount_path: &Path) -> Result<ProvisioningConfig> {
        let mut config = ProvisioningConfig::new();

        // Check for different directory structures
        let base_dirs = vec![
            mount_path.join("openstack/latest"),
            mount_path.join("openstack/2018-08-27"),
            mount_path.join("openstack/2017-08-29"),
            mount_path.join("openstack/2016-10-06"),
            mount_path.join("ec2/latest"),
            mount_path.to_path_buf(),
        ];

        let mut found = false;
        for base_dir in base_dirs {
            if !base_dir.exists() {
                continue;
            }

            // Read meta_data.json or meta-data
            for meta_file in &["meta_data.json", "meta-data.json", "meta-data"] {
                let meta_path = base_dir.join(meta_file);
                if meta_path.exists() {
                    let content = tokio::fs::read_to_string(&meta_path)
                        .await
                        .context("Failed to read metadata")?;

                    let metadata: serde_json::Value = if content.trim().starts_with('{') {
                        serde_json::from_str(&content)?
                    } else {
                        serde_yaml::from_str(&content)?
                    };

                    self.parse_metadata(&metadata, &mut config)?;
                    found = true;
                    break;
                }
            }

            // Read user-data
            let user_data_path = base_dir.join("user-data");
            if user_data_path.exists() {
                let user_data = tokio::fs::read_to_string(&user_data_path)
                    .await
                    .context("Failed to read user-data")?;

                if !user_data.trim().is_empty() {
                    config.user_data = Some(user_data);
                }
            }

            // Read network_data.json or network-config
            for net_file in &["network_data.json", "network-data.json", "network-config"] {
                let net_path = base_dir.join(net_file);
                if net_path.exists() {
                    let content = tokio::fs::read_to_string(&net_path)
                        .await
                        .context("Failed to read network config")?;

                    let network_config: serde_json::Value = if content.trim().starts_with('{') {
                        serde_json::from_str(&content)?
                    } else {
                        serde_yaml::from_str(&content)?
                    };

                    if let Ok(interfaces) = self.parse_network_config(&network_config) {
                        config.interfaces = interfaces;
                    }
                    break;
                }
            }

            if found {
                break;
            }
        }

        if !found && config.user_data.is_none() {
            return Err(anyhow::anyhow!(
                "No valid cloud-init data found on config drive"
            ));
        }

        Ok(config)
    }

    /// Load configuration from EC2-style metadata service
    async fn load_from_metadata_service(&self) -> Result<ProvisioningConfig> {
        debug!("Attempting to load from EC2-style metadata service");

        let mut config = ProvisioningConfig::new();

        // Try EC2 metadata endpoints
        let base_url = format!("{}/latest", self.metadata_url);

        // Fetch hostname
        if let Ok(hostname) = utils::fetch_metadata(
            &format!("{}/meta-data/hostname", base_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            config.hostname = Some(hostname.trim().to_string());
        }

        // Fetch SSH keys
        if let Ok(keys_text) = utils::fetch_metadata(
            &format!("{}/meta-data/public-keys", base_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            for line in keys_text.lines() {
                if let Some(equals_pos) = line.find('=') {
                    let key_index = &line[..equals_pos];
                    let key_url = format!(
                        "{}/meta-data/public-keys/{}/openssh-key",
                        base_url, key_index
                    );
                    if let Ok(key) =
                        utils::fetch_metadata(&key_url, None, self.timeout_seconds).await
                    {
                        config.ssh_authorized_keys.push(key.trim().to_string());
                    }
                }
            }
        }

        // Fetch user-data
        if let Ok(user_data) = utils::fetch_metadata(
            &format!("{}/user-data", base_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            if !user_data.trim().is_empty() && !user_data.contains("404") {
                config.user_data = Some(user_data);
            }
        }

        Ok(config)
    }

    /// Load configuration from network datasource (newer cloud-init)
    async fn load_from_network_datasource(&self) -> Result<ProvisioningConfig> {
        debug!("Attempting to load from network datasource");

        // Try cloud-init network datasource endpoint
        let url = format!("{}/metadata/v1/", self.metadata_url);

        let metadata = utils::fetch_metadata_json(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch network datasource metadata")?;

        let mut config = ProvisioningConfig::new();
        self.parse_metadata(&metadata, &mut config)?;

        Ok(config)
    }

    /// Parse metadata JSON into configuration
    fn parse_metadata(
        &self,
        metadata: &serde_json::Value,
        config: &mut ProvisioningConfig,
    ) -> Result<()> {
        // Parse hostname
        if let Some(hostname) = metadata.get("hostname").and_then(|h| h.as_str()) {
            config.hostname = Some(hostname.to_string());
        } else if let Some(id) = metadata.get("instance-id").and_then(|i| i.as_str()) {
            // Use instance ID as hostname if no hostname provided
            config.hostname = Some(id.to_string());
        } else if let Some(name) = metadata.get("name").and_then(|n| n.as_str()) {
            config.hostname = Some(name.to_string());
        }

        // Parse SSH keys
        if let Some(keys) = metadata.get("public-keys") {
            if let Some(keys_array) = keys.as_array() {
                for key in keys_array {
                    if let Some(key_str) = key.as_str() {
                        config.ssh_authorized_keys.push(key_str.to_string());
                    } else if let Some(key_obj) = key.as_object() {
                        if let Some(key_data) = key_obj.get("openssh-key").and_then(|k| k.as_str())
                        {
                            config.ssh_authorized_keys.push(key_data.to_string());
                        }
                    }
                }
            } else if let Some(keys_obj) = keys.as_object() {
                for (_key_name, key_value) in keys_obj {
                    if let Some(key_str) = key_value.as_str() {
                        config.ssh_authorized_keys.push(key_str.to_string());
                    }
                }
            } else if let Some(keys_str) = keys.as_str() {
                for line in keys_str.lines() {
                    let key = line.trim();
                    if !key.is_empty() && !key.starts_with('#') {
                        config.ssh_authorized_keys.push(key.to_string());
                    }
                }
            }
        }

        // Parse SSH authorized keys (alternative format)
        if let Some(ssh_keys) = metadata
            .get("ssh_authorized_keys")
            .and_then(|k| k.as_array())
        {
            for key in ssh_keys {
                if let Some(key_str) = key.as_str() {
                    if !config.ssh_authorized_keys.contains(&key_str.to_string()) {
                        config.ssh_authorized_keys.push(key_str.to_string());
                    }
                }
            }
        }

        // Store full metadata
        config
            .metadata
            .insert("cloud-init".to_string(), metadata.clone());

        Ok(())
    }

    /// Parse network configuration
    fn parse_network_config(
        &self,
        network_config: &serde_json::Value,
    ) -> Result<HashMap<String, InterfaceConfig>> {
        // Check version
        let version = network_config
            .get("version")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        match version {
            1 => self.parse_network_config_v1(network_config),
            2 => self.parse_network_config_v2(network_config),
            _ => {
                warn!("Unknown network config version: {}", version);
                Err(anyhow::anyhow!("Unsupported network config version"))
            }
        }
    }

    /// Parse network configuration version 1
    fn parse_network_config_v1(
        &self,
        network_config: &serde_json::Value,
    ) -> Result<HashMap<String, InterfaceConfig>> {
        let mut interfaces = HashMap::new();

        let config_array = network_config
            .get("config")
            .and_then(|c| c.as_array())
            .context("No config array in network configuration")?;

        for item in config_array {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match item_type {
                "physical" => {
                    let name = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .context("Physical interface missing name")?;

                    let mut interface = InterfaceConfig {
                        mac_address: item
                            .get("mac_address")
                            .and_then(|m| m.as_str())
                            .map(|s| utils::normalize_mac_address(s)),
                        mtu: item.get("mtu").and_then(|m| m.as_u64()).map(|m| m as u32),
                        addresses: Vec::new(),
                        enabled: true,
                        description: Some("Cloud-init network interface".to_string()),
                        vlan_id: None,
                        parent: None,
                    };

                    // Parse subnets
                    if let Some(subnets) = item.get("subnets").and_then(|s| s.as_array()) {
                        for subnet in subnets {
                            self.parse_subnet(subnet, &mut interface)?;
                        }
                    }

                    interfaces.insert(name.to_string(), interface);
                }
                "vlan" => {
                    let name = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .context("VLAN interface missing name")?;

                    let mut interface = InterfaceConfig {
                        mac_address: None,
                        mtu: item.get("mtu").and_then(|m| m.as_u64()).map(|m| m as u32),
                        addresses: Vec::new(),
                        enabled: true,
                        description: Some("Cloud-init VLAN interface".to_string()),
                        vlan_id: item
                            .get("vlan_id")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u16),
                        parent: item
                            .get("vlan_link")
                            .and_then(|p| p.as_str())
                            .map(|s| s.to_string()),
                    };

                    // Parse subnets
                    if let Some(subnets) = item.get("subnets").and_then(|s| s.as_array()) {
                        for subnet in subnets {
                            self.parse_subnet(subnet, &mut interface)?;
                        }
                    }

                    interfaces.insert(name.to_string(), interface);
                }
                _ => {
                    debug!("Ignoring network config item type: {}", item_type);
                }
            }
        }

        Ok(interfaces)
    }

    /// Parse a subnet configuration
    fn parse_subnet(
        &self,
        subnet: &serde_json::Value,
        interface: &mut InterfaceConfig,
    ) -> Result<()> {
        let subnet_type = subnet
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("static");

        match subnet_type {
            "static" | "static6" => {
                let address = subnet
                    .get("address")
                    .and_then(|a| a.as_str())
                    .context("Static subnet missing address")?;

                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Static,
                    address: Some(address.to_string()),
                    gateway: subnet
                        .get("gateway")
                        .and_then(|g| g.as_str())
                        .map(|s| s.to_string()),
                    primary: interface.addresses.is_empty(),
                });
            }
            "dhcp" | "dhcp4" => {
                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Dhcp4,
                    address: None,
                    gateway: None,
                    primary: interface.addresses.is_empty(),
                });
            }
            "dhcp6" => {
                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Dhcp6,
                    address: None,
                    gateway: None,
                    primary: false,
                });
            }
            _ => {
                debug!("Unknown subnet type: {}", subnet_type);
            }
        }

        Ok(())
    }

    /// Parse network configuration version 2 (Netplan format)
    fn parse_network_config_v2(
        &self,
        network_config: &serde_json::Value,
    ) -> Result<HashMap<String, InterfaceConfig>> {
        let mut interfaces = HashMap::new();

        // Parse ethernets
        if let Some(ethernets) = network_config.get("ethernets").and_then(|e| e.as_object()) {
            for (name, eth_config) in ethernets {
                let mut interface = InterfaceConfig {
                    mac_address: eth_config
                        .get("match")
                        .and_then(|m| m.get("macaddress"))
                        .and_then(|mac| mac.as_str())
                        .map(|s| utils::normalize_mac_address(s)),
                    mtu: eth_config
                        .get("mtu")
                        .and_then(|m| m.as_u64())
                        .map(|m| m as u32),
                    addresses: Vec::new(),
                    enabled: true,
                    description: Some("Cloud-init network interface".to_string()),
                    vlan_id: None,
                    parent: None,
                };

                // Parse addresses
                if let Some(addrs) = eth_config.get("addresses").and_then(|a| a.as_array()) {
                    for addr in addrs {
                        if let Some(addr_str) = addr.as_str() {
                            interface.addresses.push(AddressConfig {
                                addr_type: AddressType::Static,
                                address: Some(addr_str.to_string()),
                                gateway: None,
                                primary: interface.addresses.is_empty(),
                            });
                        }
                    }
                }

                // Check for DHCP
                if eth_config
                    .get("dhcp4")
                    .and_then(|d| d.as_bool())
                    .unwrap_or(false)
                {
                    interface.addresses.push(AddressConfig {
                        addr_type: AddressType::Dhcp4,
                        address: None,
                        gateway: None,
                        primary: interface.addresses.is_empty(),
                    });
                }

                if eth_config
                    .get("dhcp6")
                    .and_then(|d| d.as_bool())
                    .unwrap_or(false)
                {
                    interface.addresses.push(AddressConfig {
                        addr_type: AddressType::Dhcp6,
                        address: None,
                        gateway: None,
                        primary: false,
                    });
                }

                // Parse gateway
                if let Some(gateway) = eth_config.get("gateway4").and_then(|g| g.as_str()) {
                    for addr in &mut interface.addresses {
                        if matches!(addr.addr_type, AddressType::Static) && addr.gateway.is_none() {
                            if let Some(ref address) = addr.address {
                                if !address.contains(':') {
                                    addr.gateway = Some(gateway.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(gateway6) = eth_config.get("gateway6").and_then(|g| g.as_str()) {
                    for addr in &mut interface.addresses {
                        if matches!(addr.addr_type, AddressType::Static) && addr.gateway.is_none() {
                            if let Some(ref address) = addr.address {
                                if address.contains(':') {
                                    addr.gateway = Some(gateway6.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }

                interfaces.insert(name.clone(), interface);
            }
        }

        // Parse VLANs
        if let Some(vlans) = network_config.get("vlans").and_then(|v| v.as_object()) {
            for (name, vlan_config) in vlans {
                let mut interface = InterfaceConfig {
                    mac_address: None,
                    mtu: vlan_config
                        .get("mtu")
                        .and_then(|m| m.as_u64())
                        .map(|m| m as u32),
                    addresses: Vec::new(),
                    enabled: true,
                    description: Some("Cloud-init VLAN interface".to_string()),
                    vlan_id: vlan_config
                        .get("id")
                        .and_then(|i| i.as_u64())
                        .map(|i| i as u16),
                    parent: vlan_config
                        .get("link")
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string()),
                };

                // Parse addresses (same as ethernets)
                if let Some(addrs) = vlan_config.get("addresses").and_then(|a| a.as_array()) {
                    for addr in addrs {
                        if let Some(addr_str) = addr.as_str() {
                            interface.addresses.push(AddressConfig {
                                addr_type: AddressType::Static,
                                address: Some(addr_str.to_string()),
                                gateway: None,
                                primary: interface.addresses.is_empty(),
                            });
                        }
                    }
                }

                interfaces.insert(name.clone(), interface);
            }
        }

        Ok(interfaces)
    }
}

impl Default for CloudInitSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloudinit_source_creation() {
        let source = CloudInitSource::new();
        assert_eq!(source.metadata_url, "http://169.254.169.254");
        assert_eq!(source.timeout_seconds, 5);
        assert!(!source.config_drive_paths.is_empty());
    }

    #[test]
    fn test_set_timeout() {
        let mut source = CloudInitSource::new();
        source.set_timeout(10);
        assert_eq!(source.timeout_seconds, 10);
    }

    #[test]
    fn test_add_config_drive_path() {
        let mut source = CloudInitSource::new();
        let new_path = PathBuf::from("/custom/mount");
        source.add_config_drive_path(new_path.clone());
        assert!(source.config_drive_paths.contains(&new_path));
    }

    #[test]
    fn test_set_nocloud_path() {
        let mut source = CloudInitSource::new();
        let new_path = PathBuf::from("/custom/nocloud");
        source.set_nocloud_path(new_path.clone());
        assert_eq!(source.nocloud_path, new_path);
    }
}
