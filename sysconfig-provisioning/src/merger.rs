use crate::config::ProvisioningConfig;
use std::collections::BTreeMap;
use tracing::{debug, trace};

/// Configuration merger that handles multiple configurations with priorities
pub struct ConfigMerger {
    /// Configurations stored by priority (lower number = higher priority)
    configs: BTreeMap<u32, ProvisioningConfig>,
}

impl ConfigMerger {
    /// Create a new configuration merger
    pub fn new() -> Self {
        Self {
            configs: BTreeMap::new(),
        }
    }

    /// Add a configuration with a specific priority
    /// Lower priority numbers take precedence over higher numbers
    pub fn add_config(&mut self, config: ProvisioningConfig, priority: u32) {
        debug!("Adding configuration with priority {}", priority);
        self.configs.insert(priority, config);
    }

    /// Merge all configurations based on priority
    /// Returns the merged configuration
    pub fn merge(&self) -> ProvisioningConfig {
        let mut result = ProvisioningConfig::default();

        // Process in reverse order (highest priority number first)
        // so that lower priority numbers can override
        for (priority, config) in self.configs.iter().rev() {
            trace!("Merging configuration with priority {}", priority);

            // Hostname - take from highest priority source
            if config.hostname.is_some() && result.hostname.is_none() {
                result.hostname = config.hostname.clone();
            }

            // Nameservers - merge unique values
            for ns in &config.nameservers {
                if !result.nameservers.contains(ns) {
                    result.nameservers.push(ns.clone());
                }
            }

            // Search domains - merge unique values
            for domain in &config.search_domains {
                if !result.search_domains.contains(domain) {
                    result.search_domains.push(domain.clone());
                }
            }

            // Interfaces - merge with override based on priority
            for (name, iface) in &config.interfaces {
                result.interfaces.insert(name.clone(), iface.clone());
            }

            // SSH keys - merge unique values
            for key in &config.ssh_authorized_keys {
                if !result.ssh_authorized_keys.contains(key) {
                    result.ssh_authorized_keys.push(key.clone());
                }
            }

            // Users - merge unique users by name
            for user in &config.users {
                if !result.users.iter().any(|u| u.name == user.name) {
                    result.users.push(user.clone());
                }
            }

            // User data - take from highest priority source
            if config.user_data.is_some() && result.user_data.is_none() {
                result.user_data = config.user_data.clone();
            }

            // User data base64 - take from highest priority source
            if config.user_data_base64.is_some() && result.user_data_base64.is_none() {
                result.user_data_base64 = config.user_data_base64.clone();
            }

            // Metadata - merge with override
            for (key, value) in &config.metadata {
                result.metadata.insert(key.clone(), value.clone());
            }

            // Routes - merge unique routes
            for route in &config.routes {
                if !result
                    .routes
                    .iter()
                    .any(|r| r.destination == route.destination && r.gateway == route.gateway)
                {
                    result.routes.push(route.clone());
                }
            }

            // NTP servers - merge unique values
            for ntp in &config.ntp_servers {
                if !result.ntp_servers.contains(ntp) {
                    result.ntp_servers.push(ntp.clone());
                }
            }

            // Timezone - take from highest priority source
            if config.timezone.is_some() && result.timezone.is_none() {
                result.timezone = config.timezone.clone();
            }
        }

        // Now apply overrides from higher priority configs (lower numbers)
        for (priority, config) in self.configs.iter() {
            trace!("Applying overrides from priority {}", priority);

            // Override hostname from higher priority
            if config.hostname.is_some() {
                result.hostname = config.hostname.clone();
            }

            // Override user data from higher priority
            if config.user_data.is_some() {
                result.user_data = config.user_data.clone();
            }

            // Override user data base64 from higher priority
            if config.user_data_base64.is_some() {
                result.user_data_base64 = config.user_data_base64.clone();
            }

            // Override timezone from higher priority
            if config.timezone.is_some() {
                result.timezone = config.timezone.clone();
            }

            // Override interfaces from higher priority
            for (name, iface) in &config.interfaces {
                result.interfaces.insert(name.clone(), iface.clone());
            }

            // Override metadata values from higher priority
            for (key, value) in &config.metadata {
                result.metadata.insert(key.clone(), value.clone());
            }
        }

        result
    }

    /// Get the number of configurations stored
    pub fn len(&self) -> usize {
        self.configs.len()
    }

    /// Check if the merger has any configurations
    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    /// Clear all stored configurations
    pub fn clear(&mut self) {
        self.configs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AddressConfig, AddressType, InterfaceConfig};

    #[test]
    fn test_priority_override() {
        let mut merger = ConfigMerger::new();

        // Lower priority config
        let mut config1 = ProvisioningConfig::default();
        config1.hostname = Some("low-priority-host".to_string());
        config1.nameservers = vec!["8.8.8.8".to_string()];
        merger.add_config(config1, 10);

        // Higher priority config (lower number)
        let mut config2 = ProvisioningConfig::default();
        config2.hostname = Some("high-priority-host".to_string());
        config2.nameservers = vec!["1.1.1.1".to_string()];
        merger.add_config(config2, 1);

        let merged = merger.merge();

        // Hostname should come from higher priority (lower number)
        assert_eq!(merged.hostname, Some("high-priority-host".to_string()));

        // Nameservers should be merged
        assert!(merged.nameservers.contains(&"8.8.8.8".to_string()));
        assert!(merged.nameservers.contains(&"1.1.1.1".to_string()));
    }

    #[test]
    fn test_interface_merge() {
        let mut merger = ConfigMerger::new();

        // Config with eth0
        let mut config1 = ProvisioningConfig::default();
        let mut eth0 = InterfaceConfig {
            mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
            mtu: Some(1500),
            addresses: vec![AddressConfig {
                addr_type: AddressType::Static,
                address: Some("192.168.1.10/24".to_string()),
                gateway: Some("192.168.1.1".to_string()),
                primary: true,
            }],
            enabled: true,
            description: None,
            vlan_id: None,
            parent: None,
        };
        config1.interfaces.insert("eth0".to_string(), eth0);
        merger.add_config(config1, 10);

        // Config with eth1 and different eth0
        let mut config2 = ProvisioningConfig::default();
        let eth0_override = InterfaceConfig {
            mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
            mtu: Some(9000), // Different MTU
            addresses: vec![AddressConfig {
                addr_type: AddressType::Dhcp4,
                address: None,
                gateway: None,
                primary: true,
            }],
            enabled: true,
            description: None,
            vlan_id: None,
            parent: None,
        };
        let eth1 = InterfaceConfig {
            mac_address: Some("11:22:33:44:55:66".to_string()),
            mtu: Some(1500),
            addresses: vec![AddressConfig {
                addr_type: AddressType::Static,
                address: Some("10.0.0.10/24".to_string()),
                gateway: None,
                primary: false,
            }],
            enabled: true,
            description: None,
            vlan_id: None,
            parent: None,
        };
        config2.interfaces.insert("eth0".to_string(), eth0_override);
        config2.interfaces.insert("eth1".to_string(), eth1.clone());
        merger.add_config(config2, 1);

        let merged = merger.merge();

        // Should have both interfaces
        assert_eq!(merged.interfaces.len(), 2);

        // eth0 should be from higher priority config (DHCP with MTU 9000)
        let eth0_merged = &merged.interfaces["eth0"];
        assert_eq!(eth0_merged.mtu, Some(9000));
        assert_eq!(eth0_merged.addresses[0].addr_type, AddressType::Dhcp4);

        // eth1 should be present
        assert!(merged.interfaces.contains_key("eth1"));
    }

    #[test]
    fn test_ssh_key_merge() {
        let mut merger = ConfigMerger::new();

        let mut config1 = ProvisioningConfig::default();
        config1.ssh_authorized_keys = vec![
            "ssh-rsa KEY1 user1@host".to_string(),
            "ssh-rsa KEY2 user2@host".to_string(),
        ];
        merger.add_config(config1, 10);

        let mut config2 = ProvisioningConfig::default();
        config2.ssh_authorized_keys = vec![
            "ssh-rsa KEY2 user2@host".to_string(), // Duplicate
            "ssh-rsa KEY3 user3@host".to_string(), // New
        ];
        merger.add_config(config2, 5);

        let merged = merger.merge();

        // Should have all unique keys
        assert_eq!(merged.ssh_authorized_keys.len(), 3);
        assert!(merged
            .ssh_authorized_keys
            .contains(&"ssh-rsa KEY1 user1@host".to_string()));
        assert!(merged
            .ssh_authorized_keys
            .contains(&"ssh-rsa KEY2 user2@host".to_string()));
        assert!(merged
            .ssh_authorized_keys
            .contains(&"ssh-rsa KEY3 user3@host".to_string()));
    }

    #[test]
    fn test_metadata_merge() {
        let mut merger = ConfigMerger::new();

        let mut config1 = ProvisioningConfig::default();
        config1
            .metadata
            .insert("key1".to_string(), serde_json::json!("value1"));
        config1
            .metadata
            .insert("key2".to_string(), serde_json::json!("value2"));
        merger.add_config(config1, 10);

        let mut config2 = ProvisioningConfig::default();
        config2
            .metadata
            .insert("key2".to_string(), serde_json::json!("overridden"));
        config2
            .metadata
            .insert("key3".to_string(), serde_json::json!("value3"));
        merger.add_config(config2, 1);

        let merged = merger.merge();

        // Should have all keys with overrides from higher priority
        assert_eq!(merged.metadata.len(), 3);
        assert_eq!(merged.metadata["key1"], serde_json::json!("value1"));
        assert_eq!(merged.metadata["key2"], serde_json::json!("overridden"));
        assert_eq!(merged.metadata["key3"], serde_json::json!("value3"));
    }
}
