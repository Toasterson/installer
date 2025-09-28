use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;
use tracing::{debug, info};

use crate::config::{
    AddressConfig, AddressType, InterfaceConfig, ProvisioningConfig,
};
use crate::sources::utils;

/// SmartOS metadata source implementation
pub struct SmartOSSource {
    mdata_get_path: String,
    timeout_seconds: u64,
}

impl SmartOSSource {
    /// Create a new SmartOS metadata source
    pub fn new() -> Self {
        // Try to find mdata-get in common locations
        let mdata_get_path = if std::path::Path::new("/usr/sbin/mdata-get").exists() {
            "/usr/sbin/mdata-get".to_string()
        } else if std::path::Path::new("/native/usr/sbin/mdata-get").exists() {
            "/native/usr/sbin/mdata-get".to_string()
        } else {
            "mdata-get".to_string() // Fall back to PATH
        };

        Self {
            mdata_get_path,
            timeout_seconds: 5,
        }
    }

    /// Set the path to mdata-get command
    pub fn set_mdata_get_path(&mut self, path: String) {
        self.mdata_get_path = path;
    }

    /// Set the timeout for operations
    pub fn set_timeout(&mut self, seconds: u64) {
        self.timeout_seconds = seconds;
    }

    /// Check if SmartOS metadata is available
    pub async fn is_available() -> bool {
        // Check if mdata-get command exists
        // SmartOS metadata is only available on SmartOS zones
        std::process::Command::new("mdata-get")
            .arg("-l")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Load configuration from SmartOS metadata
    pub async fn load(&self) -> Result<ProvisioningConfig> {
        info!("Loading configuration from SmartOS metadata");

        let mut config = ProvisioningConfig::new();

        // Fetch hostname
        if let Ok(hostname) = self.get_metadata("hostname").await {
            config.hostname = Some(hostname);
        } else if let Ok(hostname) = self.get_metadata("sdc:hostname").await {
            config.hostname = Some(hostname);
        }

        // Fetch network configuration
        if let Ok(interfaces) = self.fetch_network_config().await {
            config.interfaces = interfaces;
        }

        // Fetch SSH keys
        if let Ok(keys) = self.fetch_ssh_keys().await {
            config.ssh_authorized_keys = keys;
        }

        // Fetch user script (SmartOS user-script)
        if let Ok(user_script) = self.get_metadata("user-script").await {
            config.user_data = Some(user_script);
        } else if let Ok(user_script) = self.get_metadata("sdc:user-script").await {
            config.user_data = Some(user_script);
        }

        // Fetch DNS configuration
        if let Ok(nameservers) = self.fetch_nameservers().await {
            config.nameservers = nameservers;
        }

        // Fetch NTP servers
        if let Ok(ntp_servers) = self.fetch_ntp_servers().await {
            config.ntp_servers = ntp_servers;
        }

        // Fetch additional metadata
        if let Ok(metadata) = self.fetch_all_metadata().await {
            config.metadata = metadata;
        }

        Ok(config)
    }

    /// Get a single metadata value using mdata-get
    async fn get_metadata(&self, key: &str) -> Result<String> {
        debug!("Fetching metadata key: {}", key);

        let output = Command::new(&self.mdata_get_path)
            .arg(key)
            .output()
            .with_context(|| format!("Failed to execute mdata-get for key: {}", key))?;

        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(value)
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!(
                "mdata-get failed for key '{}': {}",
                key,
                error
            ))
        }
    }

    /// List all available metadata keys
    async fn list_metadata_keys(&self) -> Result<Vec<String>> {
        debug!("Listing all metadata keys");

        let output = Command::new(&self.mdata_get_path)
            .arg("-l")
            .output()
            .context("Failed to list metadata keys")?;

        if output.status.success() {
            let keys: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(keys)
        } else {
            Err(anyhow::anyhow!("Failed to list metadata keys"))
        }
    }

    /// Fetch network configuration
    async fn fetch_network_config(&self) -> Result<HashMap<String, InterfaceConfig>> {
        let mut interfaces = HashMap::new();

        // Try to get network configuration from metadata
        // SmartOS typically uses sdc:nics for network configuration
        if let Ok(nics_json) = self.get_metadata("sdc:nics").await {
            if let Ok(nics) = serde_json::from_str::<Vec<serde_json::Value>>(&nics_json) {
                for (idx, nic) in nics.iter().enumerate() {
                    let interface_name = nic
                        .get("interface")
                        .and_then(|i| i.as_str())
                        .unwrap_or(&format!("net{}", idx))
                        .to_string();

                    let mut interface = InterfaceConfig {
                        mac_address: nic
                            .get("mac")
                            .and_then(|m| m.as_str())
                            .map(|s| utils::normalize_mac_address(s)),
                        mtu: nic.get("mtu").and_then(|m| m.as_u64()).map(|m| m as u32),
                        addresses: Vec::new(),
                        enabled: true,
                        description: Some("SmartOS network interface".to_string()),
                        vlan_id: nic
                            .get("vlan_id")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u16),
                        parent: None,
                    };

                    // Parse IP configuration
                    if let Some(ip) = nic.get("ip").and_then(|ip| ip.as_str()) {
                        let netmask = nic
                            .get("netmask")
                            .and_then(|nm| nm.as_str())
                            .unwrap_or("255.255.255.0");

                        let prefix_len = utils::netmask_to_cidr(netmask).unwrap_or(24);

                        interface.addresses.push(AddressConfig {
                            addr_type: AddressType::Static,
                            address: Some(format!("{}/{}", ip, prefix_len)),
                            gateway: nic
                                .get("gateway")
                                .and_then(|g| g.as_str())
                                .map(|s| s.to_string()),
                            primary: nic
                                .get("primary")
                                .and_then(|p| p.as_bool())
                                .unwrap_or(idx == 0),
                        });
                    }

                    // Parse IPv6 configuration if available
                    if let Some(ips) = nic.get("ips").and_then(|ips| ips.as_array()) {
                        for ip_obj in ips {
                            if let Some(ip_str) = ip_obj.as_str() {
                                // Check if it's IPv6
                                if ip_str.contains(':') {
                                    interface.addresses.push(AddressConfig {
                                        addr_type: AddressType::Static,
                                        address: Some(ip_str.to_string()),
                                        gateway: None,
                                        primary: false,
                                    });
                                }
                            } else if let Some(ip_obj) = ip_obj.as_object() {
                                if let Some(ip) = ip_obj.get("ip").and_then(|i| i.as_str()) {
                                    let prefix = ip_obj
                                        .get("prefix")
                                        .and_then(|p| p.as_u64())
                                        .unwrap_or(if ip.contains(':') { 64 } else { 24 })
                                        as u8;

                                    interface.addresses.push(AddressConfig {
                                        addr_type: AddressType::Static,
                                        address: Some(format!("{}/{}", ip, prefix)),
                                        gateway: ip_obj
                                            .get("gateway")
                                            .and_then(|g| g.as_str())
                                            .map(|s| s.to_string()),
                                        primary: false,
                                    });
                                }
                            }
                        }
                    }

                    // Check for DHCP
                    let dhcp = nic.get("dhcp").and_then(|d| d.as_bool()).unwrap_or(false);

                    if dhcp && interface.addresses.is_empty() {
                        interface.addresses.push(AddressConfig {
                            addr_type: AddressType::Dhcp4,
                            address: None,
                            gateway: None,
                            primary: true,
                        });
                    }

                    interfaces.insert(interface_name, interface);
                }
            }
        }

        // Fallback: try individual network metadata keys
        if interfaces.is_empty() {
            // Try to get primary network configuration
            let mut interface = InterfaceConfig {
                mac_address: self.get_metadata("sdc:network_primary_mac").await.ok(),
                mtu: None,
                addresses: Vec::new(),
                enabled: true,
                description: Some("SmartOS primary interface".to_string()),
                vlan_id: None,
                parent: None,
            };

            // Get primary IP
            if let Ok(primary_ip) = self.get_metadata("sdc:network_primary_ip").await {
                let netmask = self
                    .get_metadata("sdc:network_primary_netmask")
                    .await
                    .unwrap_or_else(|_| "255.255.255.0".to_string());

                let prefix_len = utils::netmask_to_cidr(&netmask).unwrap_or(24);

                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Static,
                    address: Some(format!("{}/{}", primary_ip, prefix_len)),
                    gateway: self.get_metadata("sdc:network_primary_gateway").await.ok(),
                    primary: true,
                });
            }

            // If we have any configuration, add it
            if !interface.addresses.is_empty() || interface.mac_address.is_some() {
                interfaces.insert("net0".to_string(), interface);
            }
        }

        Ok(interfaces)
    }

    /// Fetch SSH keys
    async fn fetch_ssh_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        // Try root_authorized_keys first
        if let Ok(root_keys) = self.get_metadata("root_authorized_keys").await {
            for line in root_keys.lines() {
                let key = line.trim();
                if !key.is_empty() && !key.starts_with('#') {
                    keys.push(key.to_string());
                }
            }
        }

        // Try sdc:ssh_keys
        if let Ok(sdc_keys) = self.get_metadata("sdc:ssh_keys").await {
            // This might be JSON
            if let Ok(keys_json) = serde_json::from_str::<serde_json::Value>(&sdc_keys) {
                if let Some(keys_obj) = keys_json.as_object() {
                    for (_key_name, key_value) in keys_obj {
                        if let Some(key_str) = key_value.as_str() {
                            if !keys.contains(&key_str.to_string()) {
                                keys.push(key_str.to_string());
                            }
                        }
                    }
                }
            } else {
                // Plain text format
                for line in sdc_keys.lines() {
                    let key = line.trim();
                    if !key.is_empty() && !key.starts_with('#') && !keys.contains(&key.to_string())
                    {
                        keys.push(key.to_string());
                    }
                }
            }
        }

        // Try authorized_keys
        if let Ok(auth_keys) = self.get_metadata("authorized_keys").await {
            for line in auth_keys.lines() {
                let key = line.trim();
                if !key.is_empty() && !key.starts_with('#') && !keys.contains(&key.to_string()) {
                    keys.push(key.to_string());
                }
            }
        }

        Ok(keys)
    }

    /// Fetch DNS nameservers
    async fn fetch_nameservers(&self) -> Result<Vec<String>> {
        let mut nameservers = Vec::new();

        // Try sdc:resolvers
        if let Ok(resolvers) = self.get_metadata("sdc:resolvers").await {
            // This might be JSON array
            if let Ok(resolvers_json) = serde_json::from_str::<Vec<String>>(&resolvers) {
                nameservers.extend(resolvers_json);
            } else {
                // Plain text, one per line
                for line in resolvers.lines() {
                    let ns = line.trim();
                    if !ns.is_empty() && !nameservers.contains(&ns.to_string()) {
                        nameservers.push(ns.to_string());
                    }
                }
            }
        }

        // Try individual resolver keys
        for i in 1..=4 {
            let key = format!("sdc:dns_resolver{}", i);
            if let Ok(resolver) = self.get_metadata(&key).await {
                let resolver = resolver.trim();
                if !resolver.is_empty() && !nameservers.contains(&resolver.to_string()) {
                    nameservers.push(resolver.to_string());
                }
            }
        }

        Ok(nameservers)
    }

    /// Fetch NTP servers
    async fn fetch_ntp_servers(&self) -> Result<Vec<String>> {
        let mut ntp_servers = Vec::new();

        // Try sdc:ntp_hosts
        if let Ok(ntp_hosts) = self.get_metadata("sdc:ntp_hosts").await {
            // This might be JSON array
            if let Ok(hosts_json) = serde_json::from_str::<Vec<String>>(&ntp_hosts) {
                ntp_servers.extend(hosts_json);
            } else {
                // Space or comma separated
                for host in ntp_hosts.split(&[' ', ',', '\n'][..]) {
                    let host = host.trim();
                    if !host.is_empty() && !ntp_servers.contains(&host.to_string()) {
                        ntp_servers.push(host.to_string());
                    }
                }
            }
        }

        Ok(ntp_servers)
    }

    /// Fetch all metadata into a HashMap
    async fn fetch_all_metadata(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut metadata = HashMap::new();

        // Try to list all keys
        if let Ok(keys) = self.list_metadata_keys().await {
            for key in keys {
                if let Ok(value) = self.get_metadata(&key).await {
                    // Try to parse as JSON first
                    if let Ok(json_value) = serde_json::from_str(&value) {
                        metadata.insert(key, json_value);
                    } else {
                        // Store as string
                        metadata.insert(key, serde_json::Value::String(value));
                    }
                }
            }
        } else {
            // Fallback: try common keys
            let common_keys = vec![
                "sdc:uuid",
                "sdc:server_uuid",
                "sdc:datacenter_name",
                "sdc:image_uuid",
                "sdc:package_name",
                "sdc:alias",
                "sdc:owner_uuid",
                "sdc:brand",
                "sdc:dataset_uuid",
                "sdc:dns_domain",
                "sdc:created_at",
                "sdc:ram",
                "sdc:max_physical_memory",
                "sdc:quota",
                "sdc:cpu_cap",
                "sdc:cpu_shares",
                "user-data",
                "vendor-data",
                "cloud-init",
            ];

            for key in common_keys {
                if let Ok(value) = self.get_metadata(key).await {
                    // Try to parse as JSON first
                    if let Ok(json_value) = serde_json::from_str(&value) {
                        metadata.insert(key.to_string(), json_value);
                    } else {
                        // Store as string
                        metadata.insert(key.to_string(), serde_json::Value::String(value));
                    }
                }
            }
        }

        Ok(metadata)
    }

    /// Delete a metadata key
    pub async fn delete_metadata(&self, key: &str) -> Result<()> {
        debug!("Deleting metadata key: {}", key);

        let output = Command::new(&self.mdata_get_path)
            .arg("-d")
            .arg(key)
            .output()
            .with_context(|| format!("Failed to delete metadata key: {}", key))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!(
                "Failed to delete metadata key '{}': {}",
                key,
                error
            ))
        }
    }

    /// Put a metadata value
    pub async fn put_metadata(&self, key: &str, value: &str) -> Result<()> {
        debug!("Setting metadata key: {}", key);

        let output = Command::new(&self.mdata_get_path)
            .arg("-p")
            .arg(format!("{}={}", key, value))
            .output()
            .with_context(|| format!("Failed to set metadata key: {}", key))?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!(
                "Failed to set metadata key '{}': {}",
                key,
                error
            ))
        }
    }
}

impl Default for SmartOSSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smartos_source_creation() {
        let source = SmartOSSource::new();
        assert_eq!(source.timeout_seconds, 5);
    }

    #[test]
    fn test_set_mdata_get_path() {
        let mut source = SmartOSSource::new();
        source.set_mdata_get_path("/custom/path/mdata-get".to_string());
        assert_eq!(source.mdata_get_path, "/custom/path/mdata-get");
    }

    #[test]
    fn test_set_timeout() {
        let mut source = SmartOSSource::new();
        source.set_timeout(10);
        assert_eq!(source.timeout_seconds, 10);
    }
}
