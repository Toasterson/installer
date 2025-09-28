use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::config::{
    AddressConfig, AddressType, InterfaceConfig, ProvisioningConfig,
};
use crate::sources::utils;

/// EC2 metadata source implementation
pub struct EC2Source {
    metadata_url: String,
    timeout_seconds: u64,
}

impl EC2Source {
    /// Create a new EC2 metadata source
    pub fn new() -> Self {
        Self {
            metadata_url: "http://169.254.169.254".to_string(),
            timeout_seconds: 5,
        }
    }

    /// Set the timeout for metadata requests
    pub fn set_timeout(&mut self, seconds: u64) {
        self.timeout_seconds = seconds;
    }

    /// Check if EC2 metadata service is available
    pub async fn is_available(&self) -> bool {
        // Check if we can reach the EC2 metadata service
        // Try to access a simple endpoint
        let url = format!("{}/latest/meta-data/", self.metadata_url);
        utils::check_metadata_service(&url, None, self.timeout_seconds).await
    }

    /// Load configuration from EC2 metadata service
    pub async fn load(&self) -> Result<ProvisioningConfig> {
        info!("Loading configuration from EC2 metadata service");

        let mut config = ProvisioningConfig::new();

        // Fetch hostname
        if let Ok(hostname) = self.fetch_hostname().await {
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

        // Fetch user data
        if let Ok(user_data) = self.fetch_user_data().await {
            config.user_data = Some(user_data);
        }

        // Fetch instance metadata
        if let Ok(metadata) = self.fetch_instance_metadata().await {
            config.metadata = metadata;
        }

        Ok(config)
    }

    /// Fetch hostname from metadata
    async fn fetch_hostname(&self) -> Result<String> {
        let url = format!("{}/latest/meta-data/hostname", self.metadata_url);
        let hostname = utils::fetch_metadata(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch hostname")?;

        // EC2 returns FQDN, we might want just the hostname part
        let hostname = hostname.trim().to_string();
        if let Some(dot_pos) = hostname.find('.') {
            Ok(hostname[..dot_pos].to_string())
        } else {
            Ok(hostname)
        }
    }

    /// Fetch network configuration
    async fn fetch_network_config(&self) -> Result<HashMap<String, InterfaceConfig>> {
        let mut interfaces = HashMap::new();

        // Get list of network interfaces
        let url = format!(
            "{}/latest/meta-data/network/interfaces/macs/",
            self.metadata_url
        );
        let macs_text = utils::fetch_metadata(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch network interfaces")?;

        for line in macs_text.lines() {
            let mac = line.trim_end_matches('/').to_string();
            if mac.is_empty() {
                continue;
            }

            debug!("Processing network interface with MAC: {}", mac);

            // Get interface details
            let interface_info = self.fetch_interface_info(&mac).await?;

            // Determine interface name (EC2 doesn't provide it directly, so we use MAC matching)
            let interface_name = format!("eth{}", interfaces.len());

            interfaces.insert(interface_name, interface_info);
        }

        Ok(interfaces)
    }

    /// Fetch information for a specific network interface
    async fn fetch_interface_info(&self, mac: &str) -> Result<InterfaceConfig> {
        let base_url = format!(
            "{}/latest/meta-data/network/interfaces/macs/{}/",
            self.metadata_url, mac
        );

        let mut interface = InterfaceConfig {
            mac_address: Some(utils::normalize_mac_address(mac)),
            mtu: None,
            addresses: Vec::new(),
            enabled: true,
            description: Some("EC2 network interface".to_string()),
            vlan_id: None,
            parent: None,
        };

        // Fetch local IPv4 addresses
        if let Ok(ipv4_addr) = utils::fetch_metadata(
            &format!("{}local-ipv4s", base_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            for ip in ipv4_addr.lines() {
                let ip = ip.trim();
                if ip.is_empty() {
                    continue;
                }

                // Get subnet CIDR block
                let subnet_cidr = utils::fetch_metadata(
                    &format!("{}subnet-ipv4-cidr-block", base_url),
                    None,
                    self.timeout_seconds,
                )
                .await
                .unwrap_or_else(|_| "".to_string());

                // Extract prefix length from CIDR
                let prefix_len = if let Some(pos) = subnet_cidr.find('/') {
                    subnet_cidr[pos + 1..].parse::<u8>().unwrap_or(24)
                } else {
                    24
                };

                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Static,
                    address: Some(format!("{}/{}", ip, prefix_len)),
                    gateway: None,
                    primary: interface.addresses.is_empty(),
                });
            }
        }

        // Fetch IPv6 addresses if available
        if let Ok(ipv6_addrs) =
            utils::fetch_metadata(&format!("{}ipv6s", base_url), None, self.timeout_seconds).await
        {
            for ip in ipv6_addrs.lines() {
                let ip = ip.trim();
                if ip.is_empty() {
                    continue;
                }

                interface.addresses.push(AddressConfig {
                    addr_type: AddressType::Static,
                    address: Some(ip.to_string()),
                    gateway: None,
                    primary: false,
                });
            }
        }

        // If no static addresses were found, use DHCP
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

    /// Fetch SSH public keys
    async fn fetch_ssh_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        // First get the list of available keys
        let url = format!("{}/latest/meta-data/public-keys/", self.metadata_url);
        match utils::fetch_metadata(&url, None, self.timeout_seconds).await {
            Ok(keys_list) => {
                for line in keys_list.lines() {
                    if line.is_empty() {
                        continue;
                    }

                    // Each line is in format: "0=my-key-name"
                    if let Some(equals_pos) = line.find('=') {
                        let key_index = &line[..equals_pos];

                        // Fetch the actual key
                        let key_url = format!(
                            "{}/latest/meta-data/public-keys/{}/openssh-key",
                            self.metadata_url, key_index
                        );

                        if let Ok(key) =
                            utils::fetch_metadata(&key_url, None, self.timeout_seconds).await
                        {
                            let key = key.trim().to_string();
                            if !key.is_empty() {
                                keys.push(key);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("No SSH keys found: {}", e);
            }
        }

        Ok(keys)
    }

    /// Fetch user data
    async fn fetch_user_data(&self) -> Result<String> {
        let url = format!("{}/latest/user-data", self.metadata_url);
        utils::fetch_metadata(&url, None, self.timeout_seconds)
            .await
            .context("Failed to fetch user data")
    }

    /// Fetch instance metadata
    async fn fetch_instance_metadata(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut metadata = HashMap::new();

        // Fetch instance ID
        if let Ok(instance_id) = utils::fetch_metadata(
            &format!("{}/latest/meta-data/instance-id", self.metadata_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "instance-id".to_string(),
                serde_json::Value::String(instance_id.trim().to_string()),
            );
        }

        // Fetch instance type
        if let Ok(instance_type) = utils::fetch_metadata(
            &format!("{}/latest/meta-data/instance-type", self.metadata_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "instance-type".to_string(),
                serde_json::Value::String(instance_type.trim().to_string()),
            );
        }

        // Fetch availability zone
        if let Ok(az) = utils::fetch_metadata(
            &format!(
                "{}/latest/meta-data/placement/availability-zone",
                self.metadata_url
            ),
            None,
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "availability-zone".to_string(),
                serde_json::Value::String(az.trim().to_string()),
            );
        }

        // Fetch region
        if let Ok(region) = utils::fetch_metadata(
            &format!("{}/latest/meta-data/placement/region", self.metadata_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "region".to_string(),
                serde_json::Value::String(region.trim().to_string()),
            );
        }

        // Fetch public hostname
        if let Ok(public_hostname) = utils::fetch_metadata(
            &format!("{}/latest/meta-data/public-hostname", self.metadata_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "public-hostname".to_string(),
                serde_json::Value::String(public_hostname.trim().to_string()),
            );
        }

        // Fetch public IP
        if let Ok(public_ip) = utils::fetch_metadata(
            &format!("{}/latest/meta-data/public-ipv4", self.metadata_url),
            None,
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "public-ipv4".to_string(),
                serde_json::Value::String(public_ip.trim().to_string()),
            );
        }

        // Fetch IAM role if present
        if let Ok(iam_role) = utils::fetch_metadata(
            &format!(
                "{}/latest/meta-data/iam/security-credentials/",
                self.metadata_url
            ),
            None,
            self.timeout_seconds,
        )
        .await
        {
            let role = iam_role.lines().next().unwrap_or("").trim();
            if !role.is_empty() {
                metadata.insert(
                    "iam-role".to_string(),
                    serde_json::Value::String(role.to_string()),
                );
            }
        }

        // Fetch tags if available (requires IMDSv2)
        if let Ok(tags) = self.fetch_instance_tags().await {
            metadata.insert("tags".to_string(), serde_json::Value::Object(tags));
        }

        Ok(metadata)
    }

    /// Fetch instance tags (requires IMDSv2 and proper IAM permissions)
    async fn fetch_instance_tags(&self) -> Result<serde_json::Map<String, serde_json::Value>> {
        let mut tags = serde_json::Map::new();

        // First, check if tags are enabled
        let url = format!("{}/latest/meta-data/tags/instance/", self.metadata_url);
        match utils::fetch_metadata(&url, None, self.timeout_seconds).await {
            Ok(tag_list) => {
                for tag_name in tag_list.lines() {
                    let tag_name = tag_name.trim();
                    if tag_name.is_empty() {
                        continue;
                    }

                    // Fetch tag value
                    let tag_url = format!(
                        "{}/latest/meta-data/tags/instance/{}",
                        self.metadata_url, tag_name
                    );

                    if let Ok(tag_value) =
                        utils::fetch_metadata(&tag_url, None, self.timeout_seconds).await
                    {
                        tags.insert(
                            tag_name.to_string(),
                            serde_json::Value::String(tag_value.trim().to_string()),
                        );
                    }
                }
            }
            Err(_) => {
                debug!(
                    "Instance tags not available (may require IMDSv2 or proper IAM permissions)"
                );
            }
        }

        Ok(tags)
    }

    /// Attempt to use IMDSv2 if available
    pub async fn try_imdsv2(&self) -> Result<String> {
        // Get session token
        let token_url = format!("{}/latest/api/token", self.metadata_url);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .put(&token_url)
            .header("X-aws-ec2-metadata-token-ttl-seconds", "21600")
            .send()
            .await
            .context("Failed to get IMDSv2 token")?;

        if response.status().is_success() {
            let token = response.text().await?;
            Ok(token)
        } else {
            Err(anyhow::anyhow!("Failed to get IMDSv2 token"))
        }
    }
}

impl Default for EC2Source {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ec2_source_creation() {
        let source = EC2Source::new();
        assert_eq!(source.metadata_url, "http://169.254.169.254");
        assert_eq!(source.timeout_seconds, 5);
    }

    #[test]
    fn test_set_timeout() {
        let mut source = EC2Source::new();
        source.set_timeout(10);
        assert_eq!(source.timeout_seconds, 10);
    }
}
