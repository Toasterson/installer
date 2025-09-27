use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::config::{
    AddressConfig, AddressType, InterfaceConfig, NetworkConfigV1, NetworkConfigV1Item,
    ProvisioningConfig, RouteConfig, SubnetConfig, UserConfig,
};
use crate::sources::utils;

/// OpenStack metadata source implementation
pub struct OpenStackSource {
    metadata_url: String,
    timeout_seconds: u64,
    config_drive_path: PathBuf,
}

impl OpenStackSource {
    /// Create a new OpenStack metadata source
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

    /// Load configuration from OpenStack metadata
    pub async fn load(&self) -> Result<ProvisioningConfig> {
        info!("Loading configuration from OpenStack metadata");

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
        debug!("Attempting to load from OpenStack metadata service");

        let mut config = ProvisioningConfig::new();

        // Fetch meta_data.json
        let metadata = self.fetch_metadata().await?;

        // Parse hostname
        if let Some(hostname) = metadata.get("hostname").and_then(|h| h.as_str()) {
            config.hostname = Some(hostname.to_string());
        } else if let Some(name) = metadata.get("name").and_then(|n| n.as_str()) {
            config.hostname = Some(name.to_string());
        }

        // Parse SSH keys
        if let Some(public_keys) = metadata.get("public_keys").and_then(|k| k.as_object()) {
            for (_key_name, key_value) in public_keys {
                if let Some(key_str) = key_value.as_str() {
                    config.ssh_authorized_keys.push(key_str.to_string());
                }
            }
        } else if let Some(keys_array) = metadata.get("keys").and_then(|k| k.as_array()) {
            for key_obj in keys_array {
                if let Some(data) = key_obj.get("data").and_then(|d| d.as_str()) {
                    config.ssh_authorized_keys.push(data.to_string());
                }
            }
        }

        // Fetch network configuration
        if let Ok(network_data) = self.fetch_network_data().await {
            if let Ok(interfaces) = self.parse_network_data(&network_data) {
                config.interfaces = interfaces;
            }
        }

        // Fetch user data
        if let Ok(user_data) = self.fetch_user_data().await {
            config.user_data = Some(user_data);
        }

        // Store metadata
        config
            .metadata
            .insert("openstack-instance".to_string(), metadata);

        Ok(config)
    }

    /// Load configuration from config drive
    async fn load_from_config_drive(&self) -> Result<ProvisioningConfig> {
        debug!("Attempting to load from OpenStack config drive");

        // Check if config drive is available
        let device = utils::find_device_by_label("config-2")
            .await
            .or_else(async || utils::find_device_by_label("CONFIG-2").await)
            .or_else(async || utils::find_device_by_label("cidata").await)
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

        // Look for OpenStack directory structure
        let openstack_dir = self.config_drive_path.join("openstack");
        let latest_dir = if openstack_dir.exists() {
            openstack_dir.join("latest")
        } else {
            // Fallback to root directory
            self.config_drive_path.clone()
        };

        // Read meta_data.json
        let meta_data_path = latest_dir.join("meta_data.json");
        if meta_data_path.exists() {
            let content = tokio::fs::read_to_string(&meta_data_path)
                .await
                .context("Failed to read meta_data.json")?;

            let metadata: serde_json::Value =
                serde_json::from_str(&content).context("Failed to parse meta_data.json")?;

            // Parse hostname
            if let Some(hostname) = metadata.get("hostname").and_then(|h| h.as_str()) {
                config.hostname = Some(hostname.to_string());
            } else if let Some(name) = metadata.get("name").and_then(|n| n.as_str()) {
                config.hostname = Some(name.to_string());
            }

            // Parse SSH keys
            if let Some(public_keys) = metadata.get("public_keys").and_then(|k| k.as_object()) {
                for (_key_name, key_value) in public_keys {
                    if let Some(key_str) = key_value.as_str() {
                        config.ssh_authorized_keys.push(key_str.to_string());
                    }
                }
            }

            // Store metadata
            config
                .metadata
                .insert("openstack-instance".to_string(), metadata);
        }

        // Read network_data.json
        let network_data_path = latest_dir.join("network_data.json");
        if network_data_path.exists() {
            let content = tokio::fs::read_to_string(&network_data_path)
                .await
                .context("Failed to read network_data.json")?;

            let network_data: serde_json::Value =
                serde_json::from_str(&content).context("Failed to parse network_data.json")?;

            if let Ok(interfaces) = self.parse_network_data(&network_data) {
                config.interfaces = interfaces;
            }
        }

        // Read user_data
        let user_data_path = latest_dir.join("user_data");
        if user_data_path.exists() {
            let user_data = tokio::fs::read_to_string(&user_data_path)
                .await
                .context("Failed to read user_data")?;
            config.user_data = Some(user_data);
        }

        Ok(config)
    }

    /// Fetch metadata from API
    async fn fetch_metadata(&self) -> Result<serde_json::Value> {
        let url = format!("{}/openstack/latest/meta_data.json", self.metadata_url);

        utils::fetch_metadata_json(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch metadata")
    }

    /// Fetch network data from API
    async fn fetch_network_data(&self) -> Result<serde_json::Value> {
        let url = format!("{}/openstack/latest/network_data.json", self.metadata_url);

        utils::fetch_metadata_json(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch network data")
    }

    /// Parse network data (supports both v1 and v2 formats)
    fn parse_network_data(
        &self,
        network_data: &serde_json::Value,
    ) -> Result<HashMap<String, InterfaceConfig>> {
        // Check version
        let version = network_data
            .get("version")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        match version {
            1 => self.parse_network_v1(network_data),
            2 => self.parse_network_v2(network_data),
            _ => {
                warn!("Unknown network data version: {}", version);
                self.parse_network_v1(network_data)
            }
        }
    }

    /// Parse network configuration version 1
    fn parse_network_v1(
        &self,
        network_data: &serde_json::Value,
    ) -> Result<HashMap<String, InterfaceConfig>> {
        let mut interfaces = HashMap::new();
        let mut nameservers = Vec::new();
        let mut search_domains = Vec::new();
        let mut routes = Vec::new();

        // Parse links (physical interfaces)
        let links = network_data
            .get("links")
            .and_then(|l| l.as_array())
            .unwrap_or(&Vec::new());

        let mut link_map = HashMap::new();
        for link in links {
            if let Some(id) = link.get("id").and_then(|i| i.as_str()) {
                let name = link
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or(id)
                    .to_string();

                let interface = InterfaceConfig {
                    mac_address: link
                        .get("ethernet_mac_address")
                        .and_then(|m| m.as_str())
                        .map(|s| utils::normalize_mac_address(s)),
                    mtu: link.get("mtu").and_then(|m| m.as_u64()).map(|m| m as u32),
                    addresses: Vec::new(),
                    enabled: true,
                    description: Some("OpenStack network interface".to_string()),
                    vlan_id: link
                        .get("vlan_id")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u16),
                    parent: link
                        .get("vlan_link")
                        .and_then(|p| p.as_str())
                        .map(|s| s.to_string()),
                };

                link_map.insert(id.to_string(), name.clone());
                interfaces.insert(name, interface);
            }
        }

        // Parse networks (IP configurations)
        let networks = network_data
            .get("networks")
            .and_then(|n| n.as_array())
            .unwrap_or(&Vec::new());

        for network in networks {
            let link_id = network.get("link").and_then(|l| l.as_str()).unwrap_or("");

            if let Some(interface_name) = link_map.get(link_id) {
                if let Some(interface) = interfaces.get_mut(interface_name) {
                    let network_type = network
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("ipv4");

                    if network_type == "ipv4_dhcp" || network_type == "dhcp4" {
                        interface.addresses.push(AddressConfig {
                            addr_type: AddressType::Dhcp4,
                            address: None,
                            gateway: None,
                            primary: interface.addresses.is_empty(),
                        });
                    } else if network_type == "ipv6_dhcp" || network_type == "dhcp6" {
                        interface.addresses.push(AddressConfig {
                            addr_type: AddressType::Dhcp6,
                            address: None,
                            gateway: None,
                            primary: false,
                        });
                    } else if network_type == "ipv6_slaac" {
                        interface.addresses.push(AddressConfig {
                            addr_type: AddressType::Slaac,
                            address: None,
                            gateway: None,
                            primary: false,
                        });
                    } else if network_type == "ipv4" || network_type == "ipv6" {
                        let ip_address = network
                            .get("ip_address")
                            .and_then(|ip| ip.as_str())
                            .unwrap_or("");

                        let netmask = network.get("netmask").and_then(|nm| nm.as_str());

                        let prefix_len = if let Some(nm) = netmask {
                            utils::netmask_to_cidr(nm).unwrap_or(24)
                        } else {
                            if network_type == "ipv6" {
                                64
                            } else {
                                24
                            }
                        };

                        if !ip_address.is_empty() {
                            interface.addresses.push(AddressConfig {
                                addr_type: AddressType::Static,
                                address: Some(format!("{}/{}", ip_address, prefix_len)),
                                gateway: network
                                    .get("gateway")
                                    .and_then(|g| g.as_str())
                                    .map(|s| s.to_string()),
                                primary: interface.addresses.is_empty(),
                            });
                        }

                        // Parse routes for this network
                        if let Some(routes_array) = network.get("routes").and_then(|r| r.as_array())
                        {
                            for route in routes_array {
                                if let (Some(dest), Some(gw)) = (
                                    route.get("network").and_then(|n| n.as_str()),
                                    route.get("gateway").and_then(|g| g.as_str()),
                                ) {
                                    routes.push(RouteConfig {
                                        destination: dest.to_string(),
                                        gateway: gw.to_string(),
                                        interface: Some(interface_name.clone()),
                                        metric: route
                                            .get("metric")
                                            .and_then(|m| m.as_u64())
                                            .map(|m| m as u32),
                                    });
                                }
                            }
                        }
                    }

                    // Parse DNS
                    if let Some(dns) = network.get("dns_nameservers").and_then(|d| d.as_array()) {
                        for ns in dns {
                            if let Some(ns_str) = ns.as_str() {
                                if !nameservers.contains(&ns_str.to_string()) {
                                    nameservers.push(ns_str.to_string());
                                }
                            }
                        }
                    }

                    if let Some(search) = network.get("dns_search").and_then(|s| s.as_array()) {
                        for domain in search {
                            if let Some(domain_str) = domain.as_str() {
                                if !search_domains.contains(&domain_str.to_string()) {
                                    search_domains.push(domain_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Parse services (additional DNS, NTP, etc.)
        if let Some(services) = network_data.get("services").and_then(|s| s.as_array()) {
            for service in services {
                let service_type = service.get("type").and_then(|t| t.as_str()).unwrap_or("");

                if service_type == "dns" {
                    if let Some(address) = service.get("address").and_then(|a| a.as_str()) {
                        if !nameservers.contains(&address.to_string()) {
                            nameservers.push(address.to_string());
                        }
                    }
                }
            }
        }

        Ok(interfaces)
    }

    /// Parse network configuration version 2 (Netplan format)
    fn parse_network_v2(
        &self,
        network_data: &serde_json::Value,
    ) -> Result<HashMap<String, InterfaceConfig>> {
        let mut interfaces = HashMap::new();

        // Parse ethernets
        if let Some(ethernets) = network_data.get("ethernets").and_then(|e| e.as_object()) {
            for (name, eth_config) in ethernets {
                let mut interface = InterfaceConfig {
                    mac_address: eth_config
                        .get("match")
                        .and_then(|m| m.get("macaddress"))
                        .and_then(|mac| mac.as_str())
                        .map(|s| utils::normalize_mac_address(s))
                        .or_else(|| {
                            eth_config
                                .get("set-name")
                                .and_then(|sn| sn.as_str())
                                .map(|s| s.to_string())
                        }),
                    mtu: eth_config
                        .get("mtu")
                        .and_then(|m| m.as_u64())
                        .map(|m| m as u32),
                    addresses: Vec::new(),
                    enabled: true,
                    description: Some("OpenStack network interface".to_string()),
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
                    // Apply gateway to the first IPv4 address
                    for addr in &mut interface.addresses {
                        if matches!(addr.addr_type, AddressType::Static) && addr.gateway.is_none() {
                            if let Some(ref address) = addr.address {
                                if !address.contains(':') {
                                    // IPv4
                                    addr.gateway = Some(gateway.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(gateway6) = eth_config.get("gateway6").and_then(|g| g.as_str()) {
                    // Apply gateway to the first IPv6 address
                    for addr in &mut interface.addresses {
                        if matches!(addr.addr_type, AddressType::Static) && addr.gateway.is_none() {
                            if let Some(ref address) = addr.address {
                                if address.contains(':') {
                                    // IPv6
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
        if let Some(vlans) = network_data.get("vlans").and_then(|v| v.as_object()) {
            for (name, vlan_config) in vlans {
                let interface = InterfaceConfig {
                    mac_address: None,
                    mtu: vlan_config
                        .get("mtu")
                        .and_then(|m| m.as_u64())
                        .map(|m| m as u32),
                    addresses: Vec::new(),
                    enabled: true,
                    description: Some("OpenStack VLAN interface".to_string()),
                    vlan_id: vlan_config
                        .get("id")
                        .and_then(|i| i.as_u64())
                        .map(|i| i as u16),
                    parent: vlan_config
                        .get("link")
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string()),
                };

                interfaces.insert(name.clone(), interface);
            }
        }

        Ok(interfaces)
    }

    /// Fetch user data
    async fn fetch_user_data(&self) -> Result<String> {
        let url = format!("{}/openstack/latest/user_data", self.metadata_url);

        utils::fetch_metadata(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch user data")
    }

    /// Fetch vendor data
    pub async fn fetch_vendor_data(&self) -> Result<serde_json::Value> {
        let url = format!("{}/openstack/latest/vendor_data.json", self.metadata_url);

        utils::fetch_metadata_json(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch vendor data")
    }

    /// Fetch vendor data 2 (newer OpenStack versions)
    pub async fn fetch_vendor_data2(&self) -> Result<serde_json::Value> {
        let url = format!("{}/openstack/latest/vendor_data2.json", self.metadata_url);

        utils::fetch_metadata_json(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch vendor data 2")
    }
}

impl Default for OpenStackSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openstack_source_creation() {
        let source = OpenStackSource::new();
        assert_eq!(source.metadata_url, "http://169.254.169.254");
        assert_eq!(source.timeout_seconds, 5);
        assert_eq!(source.config_drive_path, PathBuf::from("/mnt/config-2"));
    }

    #[test]
    fn test_set_timeout() {
        let mut source = OpenStackSource::new();
        source.set_timeout(10);
        assert_eq!(source.timeout_seconds, 10);
    }

    #[test]
    fn test_set_config_drive_path() {
        let mut source = OpenStackSource::new();
        let new_path = PathBuf::from("/tmp/config");
        source.set_config_drive_path(new_path.clone());
        assert_eq!(source.config_drive_path, new_path);
    }
}
