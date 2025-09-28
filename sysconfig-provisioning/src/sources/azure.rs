use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::config::{
    AddressConfig, AddressType, InterfaceConfig, ProvisioningConfig,
};
use crate::sources::utils;

/// Azure metadata source implementation
pub struct AzureSource {
    metadata_url: String,
    api_version: String,
    timeout_seconds: u64,
}

impl AzureSource {
    /// Create a new Azure metadata source
    pub fn new() -> Self {
        Self {
            metadata_url: "http://169.254.169.254".to_string(),
            api_version: "2021-01-01".to_string(),
            timeout_seconds: 5,
        }
    }

    /// Set the timeout for metadata requests
    pub fn set_timeout(&mut self, seconds: u64) {
        self.timeout_seconds = seconds;
    }

    /// Set the API version
    pub fn set_api_version(&mut self, version: String) {
        self.api_version = version;
    }

    /// Check if Azure metadata service is available
    pub async fn is_available(&self) -> bool {
        // Check if we can reach the Azure metadata service
        // Azure requires the Metadata header
        let url = format!(
            "{}/metadata/instance?api-version={}",
            self.metadata_url, self.api_version
        );
        let headers = vec![("Metadata", "true")];
        utils::check_metadata_service(&url, Some(headers), self.timeout_seconds).await
    }

    /// Load configuration from Azure metadata service
    pub async fn load(&self) -> Result<ProvisioningConfig> {
        info!("Loading configuration from Azure metadata service");

        let mut config = ProvisioningConfig::new();

        // Fetch instance metadata
        let instance_metadata = self.fetch_instance_metadata().await?;

        // Parse hostname
        if let Some(hostname) = self.extract_hostname(&instance_metadata) {
            config.hostname = Some(hostname);
        }

        // Parse network configuration
        if let Ok(interfaces) = self.parse_network_config(&instance_metadata) {
            config.interfaces = interfaces;
        }

        // Fetch SSH keys from metadata
        if let Ok(keys) = self.fetch_ssh_keys(&instance_metadata).await {
            config.ssh_authorized_keys = keys;
        }

        // Fetch user data
        if let Ok(user_data) = self.fetch_user_data().await {
            config.user_data = Some(user_data);
        }

        // Store metadata
        config
            .metadata
            .insert("azure-instance".to_string(), instance_metadata.clone());

        Ok(config)
    }

    /// Fetch instance metadata from Azure
    async fn fetch_instance_metadata(&self) -> Result<serde_json::Value> {
        let url = format!(
            "{}/metadata/instance?api-version={}",
            self.metadata_url, self.api_version
        );

        let headers = vec![("Metadata", "true")];

        utils::fetch_metadata_json(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to fetch Azure instance metadata")
    }

    /// Extract hostname from metadata
    fn extract_hostname(&self, metadata: &serde_json::Value) -> Option<String> {
        metadata
            .get("compute")
            .and_then(|compute| compute.get("name"))
            .and_then(|name| name.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                metadata
                    .get("compute")
                    .and_then(|compute| compute.get("computerName"))
                    .and_then(|name| name.as_str())
                    .map(|s| s.to_string())
            })
    }

    /// Parse network configuration from metadata
    fn parse_network_config(
        &self,
        metadata: &serde_json::Value,
    ) -> Result<HashMap<String, InterfaceConfig>> {
        let mut interfaces = HashMap::new();

        let network = metadata
            .get("network")
            .context("No network information in metadata")?;

        let interfaces_array = network
            .get("interface")
            .and_then(|i| i.as_array())
            .context("No interface array in network metadata")?;

        for (idx, interface_json) in interfaces_array.iter().enumerate() {
            let interface_name = format!("eth{}", idx);
            let mut interface = InterfaceConfig {
                mac_address: interface_json
                    .get("macAddress")
                    .and_then(|m| m.as_str())
                    .map(|s| utils::normalize_mac_address(s)),
                mtu: None,
                addresses: Vec::new(),
                enabled: true,
                description: Some("Azure network interface".to_string()),
                vlan_id: None,
                parent: None,
            };

            // Parse IPv4 addresses
            if let Some(ipv4) = interface_json.get("ipv4") {
                if let Some(addresses) = ipv4.get("ipAddress").and_then(|a| a.as_array()) {
                    for addr_obj in addresses {
                        let private_ip = addr_obj
                            .get("privateIpAddress")
                            .and_then(|ip| ip.as_str())
                            .unwrap_or("");

                        let public_ip = addr_obj.get("publicIpAddress").and_then(|ip| ip.as_str());

                        if !private_ip.is_empty() {
                            // Try to get subnet information
                            let prefix_len = ipv4
                                .get("subnet")
                                .and_then(|s| s.as_array())
                                .and_then(|subnets| subnets.first())
                                .and_then(|subnet| subnet.get("prefix"))
                                .and_then(|p| p.as_str())
                                .and_then(|prefix_str| {
                                    prefix_str.trim_start_matches('/').parse::<u8>().ok()
                                })
                                .unwrap_or(24);

                            interface.addresses.push(AddressConfig {
                                addr_type: AddressType::Static,
                                address: Some(format!("{}/{}", private_ip, prefix_len)),
                                gateway: None,
                                primary: interface.addresses.is_empty(),
                            });
                        }

                        // Store public IP in metadata
                        if let Some(pub_ip) = public_ip {
                            debug!(
                                "Found public IP for interface {}: {}",
                                interface_name, pub_ip
                            );
                        }
                    }
                }
            }

            // Parse IPv6 addresses
            if let Some(ipv6) = interface_json.get("ipv6") {
                if let Some(addresses) = ipv6.get("ipAddress").and_then(|a| a.as_array()) {
                    for addr_obj in addresses {
                        if let Some(ip) =
                            addr_obj.get("privateIpAddress").and_then(|ip| ip.as_str())
                        {
                            interface.addresses.push(AddressConfig {
                                addr_type: AddressType::Static,
                                address: Some(ip.to_string()),
                                gateway: None,
                                primary: false,
                            });
                        }
                    }
                }
            }

            // If no static addresses were configured, use DHCP
            if interface.addresses.is_empty() {
                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Dhcp4,
                    address: None,
                    gateway: None,
                    primary: true,
                });
            }

            interfaces.insert(interface_name, interface);
        }

        Ok(interfaces)
    }

    /// Fetch SSH keys from metadata or custom data
    async fn fetch_ssh_keys(&self, metadata: &serde_json::Value) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        // Check compute metadata for SSH public keys
        if let Some(compute) = metadata.get("compute") {
            // Azure can provide SSH keys in the publicKeys array
            if let Some(public_keys) = compute.get("publicKeys").and_then(|k| k.as_array()) {
                for key_obj in public_keys {
                    if let Some(key_data) = key_obj.get("keyData").and_then(|k| k.as_str()) {
                        keys.push(key_data.to_string());
                    }
                }
            }

            // Also check for SSH key in osProfile
            if let Some(os_profile) = compute.get("osProfile") {
                if let Some(linux_config) = os_profile.get("linuxConfiguration") {
                    if let Some(ssh) = linux_config.get("ssh") {
                        if let Some(public_keys) = ssh.get("publicKeys").and_then(|k| k.as_array())
                        {
                            for key_obj in public_keys {
                                if let Some(key_data) =
                                    key_obj.get("keyData").and_then(|k| k.as_str())
                                {
                                    if !keys.contains(&key_data.to_string()) {
                                        keys.push(key_data.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Try to fetch from custom data as well
        if let Ok(custom_data) = self.fetch_custom_data().await {
            // Custom data might be base64 encoded
            if let Ok(decoded) = utils::decode_base64(&custom_data) {
                let text = String::from_utf8_lossy(&decoded);
                // Look for SSH keys in the decoded data
                for line in text.lines() {
                    if line.starts_with("ssh-rsa")
                        || line.starts_with("ssh-ed25519")
                        || line.starts_with("ssh-ecdsa")
                    {
                        let key = line.trim().to_string();
                        if !keys.contains(&key) {
                            keys.push(key);
                        }
                    }
                }
            }
        }

        Ok(keys)
    }

    /// Fetch user data from Azure
    async fn fetch_user_data(&self) -> Result<String> {
        // Try to fetch custom data first (Azure's equivalent of user data)
        if let Ok(custom_data) = self.fetch_custom_data().await {
            // Custom data is usually base64 encoded
            if let Ok(decoded) = utils::decode_base64(&custom_data) {
                return Ok(String::from_utf8_lossy(&decoded).to_string());
            }
            return Ok(custom_data);
        }

        // Fall back to user data endpoint if available
        let url = format!(
            "{}/metadata/instance/compute/userData?api-version={}&format=text",
            self.metadata_url, self.api_version
        );

        let headers = vec![("Metadata", "true")];

        match utils::fetch_metadata(&url, Some(headers), self.timeout_seconds).await {
            Ok(user_data) => {
                // User data might be base64 encoded
                if let Ok(decoded) = utils::decode_base64(&user_data) {
                    Ok(String::from_utf8_lossy(&decoded).to_string())
                } else {
                    Ok(user_data)
                }
            }
            Err(e) => {
                debug!("No user data found: {}", e);
                Err(e)
            }
        }
    }

    /// Fetch custom data from Azure
    async fn fetch_custom_data(&self) -> Result<String> {
        let url = format!(
            "{}/metadata/instance/compute/customData?api-version={}&format=text",
            self.metadata_url, self.api_version
        );

        let headers = vec![("Metadata", "true")];

        utils::fetch_metadata(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to fetch custom data")
    }

    /// Fetch attested document for secure metadata
    pub async fn fetch_attested_document(&self) -> Result<serde_json::Value> {
        let url = format!(
            "{}/metadata/attested/document?api-version={}",
            self.metadata_url, self.api_version
        );

        let headers = vec![("Metadata", "true")];

        utils::fetch_metadata_json(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to fetch attested document")
    }

    /// Get managed identity token
    pub async fn get_identity_token(&self, resource: &str) -> Result<serde_json::Value> {
        let url = format!(
            "{}/metadata/identity/oauth2/token?api-version={}&resource={}",
            self.metadata_url, self.api_version, resource
        );

        let headers = vec![("Metadata", "true")];

        utils::fetch_metadata_json(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to get identity token")
    }
}

impl Default for AzureSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_source_creation() {
        let source = AzureSource::new();
        assert_eq!(source.metadata_url, "http://169.254.169.254");
        assert_eq!(source.api_version, "2021-01-01");
        assert_eq!(source.timeout_seconds, 5);
    }

    #[test]
    fn test_set_timeout() {
        let mut source = AzureSource::new();
        source.set_timeout(10);
        assert_eq!(source.timeout_seconds, 10);
    }

    #[test]
    fn test_set_api_version() {
        let mut source = AzureSource::new();
        source.set_api_version("2022-01-01".to_string());
        assert_eq!(source.api_version, "2022-01-01");
    }

    #[test]
    fn test_extract_hostname() {
        let source = AzureSource::new();

        let metadata = serde_json::json!({
            "compute": {
                "name": "my-vm",
                "computerName": "my-computer"
            }
        });

        let hostname = source.extract_hostname(&metadata);
        assert_eq!(hostname, Some("my-vm".to_string()));
    }
}
