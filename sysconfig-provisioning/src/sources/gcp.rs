use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::config::{
    AddressConfig, AddressType, InterfaceConfig, ProvisioningConfig, RouteConfig, UserConfig,
};
use crate::sources::utils;

/// GCP metadata source implementation
pub struct GCPSource {
    metadata_url: String,
    timeout_seconds: u64,
}

impl GCPSource {
    /// Create a new GCP metadata source
    pub fn new() -> Self {
        Self {
            metadata_url: "http://metadata.google.internal".to_string(),
            timeout_seconds: 5,
        }
    }

    /// Set the timeout for metadata requests
    pub fn set_timeout(&mut self, seconds: u64) {
        self.timeout_seconds = seconds;
    }

    /// Load configuration from GCP metadata service
    pub async fn load(&self) -> Result<ProvisioningConfig> {
        info!("Loading configuration from GCP metadata service");

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

        // Fetch startup script (user data equivalent)
        if let Ok(user_data) = self.fetch_startup_script().await {
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
        let url = format!("{}/computeMetadata/v1/instance/hostname", self.metadata_url);
        let headers = vec![("Metadata-Flavor", "Google")];

        let hostname = utils::fetch_metadata(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to fetch hostname")?;

        // GCP returns FQDN, extract just the hostname
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
            "{}/computeMetadata/v1/instance/network-interfaces/?recursive=true",
            self.metadata_url
        );
        let headers = vec![("Metadata-Flavor", "Google")];

        let interfaces_json = utils::fetch_metadata_json(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to fetch network interfaces")?;

        if let Some(interfaces_array) = interfaces_json.as_array() {
            for (idx, interface_data) in interfaces_array.iter().enumerate() {
                let interface_name = format!("eth{}", idx);

                let mut interface = InterfaceConfig {
                    mac_address: interface_data
                        .get("mac")
                        .and_then(|m| m.as_str())
                        .map(|s| utils::normalize_mac_address(s)),
                    mtu: interface_data
                        .get("mtu")
                        .and_then(|m| m.as_u64())
                        .map(|m| m as u32),
                    addresses: Vec::new(),
                    enabled: true,
                    description: Some("GCP network interface".to_string()),
                    vlan_id: None,
                    parent: None,
                };

                // Parse IP address
                if let Some(ip) = interface_data.get("ip").and_then(|ip| ip.as_str()) {
                    // Get network range to determine prefix
                    let prefix_len = interface_data
                        .get("subnetmask")
                        .and_then(|mask| mask.as_str())
                        .and_then(|mask| utils::netmask_to_cidr(mask).ok())
                        .unwrap_or(24);

                    interface.addresses.push(AddressConfig {
                        addr_type: AddressType::Static,
                        address: Some(format!("{}/{}", ip, prefix_len)),
                        gateway: interface_data
                            .get("gateway")
                            .and_then(|g| g.as_str())
                            .map(|s| s.to_string()),
                        primary: true,
                    });
                }

                // Parse access configs (external IPs)
                if let Some(access_configs) = interface_data
                    .get("accessConfigs")
                    .and_then(|ac| ac.as_array())
                {
                    for access_config in access_configs {
                        if let Some(external_ip) =
                            access_config.get("externalIp").and_then(|ip| ip.as_str())
                        {
                            debug!("Found external IP for {}: {}", interface_name, external_ip);
                        }
                    }
                }

                // Parse alias IP ranges
                if let Some(alias_ranges) = interface_data
                    .get("ipAliases")
                    .and_then(|aliases| aliases.as_array())
                {
                    for alias in alias_ranges {
                        if let Some(ip_range) = alias.as_str() {
                            interface.addresses.push(AddressConfig {
                                addr_type: AddressType::Static,
                                address: Some(ip_range.to_string()),
                                gateway: None,
                                primary: false,
                            });
                        }
                    }
                }

                // If no addresses were configured, use DHCP
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
        }

        Ok(interfaces)
    }

    /// Fetch SSH keys
    async fn fetch_ssh_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        // Try project-wide SSH keys
        let project_keys_url = format!(
            "{}/computeMetadata/v1/project/attributes/ssh-keys",
            self.metadata_url
        );
        let headers = vec![("Metadata-Flavor", "Google")];

        if let Ok(ssh_keys_text) = utils::fetch_metadata(
            &project_keys_url,
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            for line in ssh_keys_text.lines() {
                // GCP format is "username:ssh-rsa AAAA... comment"
                if let Some(colon_pos) = line.find(':') {
                    let key = line[colon_pos + 1..].trim();
                    if !key.is_empty()
                        && (key.starts_with("ssh-rsa")
                            || key.starts_with("ssh-ed25519")
                            || key.starts_with("ssh-ecdsa"))
                    {
                        keys.push(key.to_string());
                    }
                }
            }
        }

        // Try instance-specific SSH keys
        let instance_keys_url = format!(
            "{}/computeMetadata/v1/instance/attributes/ssh-keys",
            self.metadata_url
        );

        if let Ok(ssh_keys_text) = utils::fetch_metadata(
            &instance_keys_url,
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            for line in ssh_keys_text.lines() {
                // Same format as project keys
                if let Some(colon_pos) = line.find(':') {
                    let key = line[colon_pos + 1..].trim();
                    if !key.is_empty() && !keys.contains(&key.to_string()) {
                        if key.starts_with("ssh-rsa")
                            || key.starts_with("ssh-ed25519")
                            || key.starts_with("ssh-ecdsa")
                        {
                            keys.push(key.to_string());
                        }
                    }
                }
            }
        }

        // Also check for enable-oslogin
        let oslogin_url = format!(
            "{}/computeMetadata/v1/instance/attributes/enable-oslogin",
            self.metadata_url
        );

        if let Ok(oslogin) =
            utils::fetch_metadata(&oslogin_url, Some(headers), self.timeout_seconds).await
        {
            if oslogin.trim().to_lowercase() == "true" {
                debug!("OS Login is enabled, SSH keys may be managed externally");
            }
        }

        Ok(keys)
    }

    /// Fetch startup script (GCP's equivalent of user data)
    async fn fetch_startup_script(&self) -> Result<String> {
        let headers = vec![("Metadata-Flavor", "Google")];

        // Try instance startup script first
        let instance_script_url = format!(
            "{}/computeMetadata/v1/instance/attributes/startup-script",
            self.metadata_url
        );

        if let Ok(script) = utils::fetch_metadata(
            &instance_script_url,
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            return Ok(script);
        }

        // Try project startup script
        let project_script_url = format!(
            "{}/computeMetadata/v1/project/attributes/startup-script",
            self.metadata_url
        );

        if let Ok(script) = utils::fetch_metadata(
            &project_script_url,
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            return Ok(script);
        }

        // Try startup-script-url
        let script_url_url = format!(
            "{}/computeMetadata/v1/instance/attributes/startup-script-url",
            self.metadata_url
        );

        if let Ok(script_url) =
            utils::fetch_metadata(&script_url_url, Some(headers), self.timeout_seconds).await
        {
            let script_url = script_url.trim();
            debug!("Found startup script URL: {}", script_url);

            // Fetch the script from the URL
            if let Ok(script) = utils::fetch_metadata(script_url, None, self.timeout_seconds).await
            {
                return Ok(script);
            }
        }

        Err(anyhow::anyhow!("No startup script found"))
    }

    /// Fetch instance metadata
    async fn fetch_instance_metadata(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut metadata = HashMap::new();
        let headers = vec![("Metadata-Flavor", "Google")];

        // Fetch instance ID
        if let Ok(instance_id) = utils::fetch_metadata(
            &format!("{}/computeMetadata/v1/instance/id", self.metadata_url),
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "instance-id".to_string(),
                serde_json::Value::String(instance_id.trim().to_string()),
            );
        }

        // Fetch instance name
        if let Ok(instance_name) = utils::fetch_metadata(
            &format!("{}/computeMetadata/v1/instance/name", self.metadata_url),
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "instance-name".to_string(),
                serde_json::Value::String(instance_name.trim().to_string()),
            );
        }

        // Fetch machine type
        if let Ok(machine_type) = utils::fetch_metadata(
            &format!(
                "{}/computeMetadata/v1/instance/machine-type",
                self.metadata_url
            ),
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            // Extract just the machine type from the full path
            let machine_type = machine_type.trim();
            let machine_type = machine_type.split('/').last().unwrap_or(machine_type);
            metadata.insert(
                "machine-type".to_string(),
                serde_json::Value::String(machine_type.to_string()),
            );
        }

        // Fetch zone
        if let Ok(zone) = utils::fetch_metadata(
            &format!("{}/computeMetadata/v1/instance/zone", self.metadata_url),
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            let zone = zone.trim();
            let zone = zone.split('/').last().unwrap_or(zone);
            metadata.insert(
                "zone".to_string(),
                serde_json::Value::String(zone.to_string()),
            );

            // Extract region from zone (e.g., us-central1-a -> us-central1)
            if let Some(last_dash) = zone.rfind('-') {
                let region = &zone[..last_dash];
                metadata.insert(
                    "region".to_string(),
                    serde_json::Value::String(region.to_string()),
                );
            }
        }

        // Fetch project ID
        if let Ok(project_id) = utils::fetch_metadata(
            &format!(
                "{}/computeMetadata/v1/project/project-id",
                self.metadata_url
            ),
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            metadata.insert(
                "project-id".to_string(),
                serde_json::Value::String(project_id.trim().to_string()),
            );
        }

        // Fetch tags
        if let Ok(tags_text) = utils::fetch_metadata(
            &format!("{}/computeMetadata/v1/instance/tags", self.metadata_url),
            Some(headers.clone()),
            self.timeout_seconds,
        )
        .await
        {
            if let Ok(tags) = serde_json::from_str::<Vec<String>>(&tags_text) {
                metadata.insert(
                    "tags".to_string(),
                    serde_json::Value::Array(
                        tags.into_iter().map(serde_json::Value::String).collect(),
                    ),
                );
            }
        }

        // Fetch service accounts
        if let Ok(service_accounts) = self.fetch_service_accounts().await {
            metadata.insert("service-accounts".to_string(), service_accounts);
        }

        // Fetch custom metadata attributes
        if let Ok(attributes) = self.fetch_attributes().await {
            metadata.insert("attributes".to_string(), attributes);
        }

        Ok(metadata)
    }

    /// Fetch service accounts
    async fn fetch_service_accounts(&self) -> Result<serde_json::Value> {
        let url = format!(
            "{}/computeMetadata/v1/instance/service-accounts/?recursive=true",
            self.metadata_url
        );
        let headers = vec![("Metadata-Flavor", "Google")];

        utils::fetch_metadata_json(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to fetch service accounts")
    }

    /// Fetch custom attributes
    async fn fetch_attributes(&self) -> Result<serde_json::Value> {
        let url = format!(
            "{}/computeMetadata/v1/instance/attributes/?recursive=true",
            self.metadata_url
        );
        let headers = vec![("Metadata-Flavor", "Google")];

        utils::fetch_metadata_json(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to fetch attributes")
    }

    /// Get access token for service account
    pub async fn get_access_token(&self, service_account: &str) -> Result<serde_json::Value> {
        let url = format!(
            "{}/computeMetadata/v1/instance/service-accounts/{}/token",
            self.metadata_url, service_account
        );
        let headers = vec![("Metadata-Flavor", "Google")];

        utils::fetch_metadata_json(&url, Some(headers), self.timeout_seconds)
            .await
            .context("Failed to get access token")
    }
}

impl Default for GCPSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcp_source_creation() {
        let source = GCPSource::new();
        assert_eq!(source.metadata_url, "http://metadata.google.internal");
        assert_eq!(source.timeout_seconds, 5);
    }

    #[test]
    fn test_set_timeout() {
        let mut source = GCPSource::new();
        source.set_timeout(10);
        assert_eq!(source.timeout_seconds, 10);
    }
}
