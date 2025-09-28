//! Provisioning plugin module
//!
//! This module provides the core functionality for the provisioning plugin,
//! including data source collection, configuration conversion, and integration
//! with the sysconfig system.

pub mod datasources;
pub mod converter;

pub use datasources::{DataSource, CloudInitPaths, PrioritizedSource, collect_from_source};
pub use converter::convert_to_unified_schema;

// Re-export functions from this module
pub use self::{merge_configurations, parse_data_sources};

use serde_json::Value;
use std::error::Error;

/// Merge two JSON configurations, with the overlay taking precedence
pub fn merge_configurations(
    base: &mut Value,
    overlay: Value,
) -> Result<(), Box<dyn Error>> {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if base_map.contains_key(&key) {
                    merge_configurations(base_map.get_mut(&key).unwrap(), value)?;
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base_val, overlay_val) => {
            *base_val = overlay_val;
        }
    }
    Ok(())
}

/// Parse data source priority string into a list of prioritized sources
pub fn parse_data_sources(
    sources_str: &str,
    config_file: Option<&str>,
    cloud_init_meta_data: &str,
    cloud_init_user_data: &str,
    cloud_init_network_config: &str,
) -> Result<Vec<PrioritizedSource>, Box<dyn Error>> {
    let mut sources = Vec::new();
    let mut priority = 0;

    for source_name in sources_str.split(',') {
        priority += 10;
        let source = match source_name.trim() {
            "local" => {
                if let Some(config_file) = config_file {
                    DataSource::Local(config_file.to_string())
                } else {
                    continue; // Skip if no config file specified
                }
            }
            "cloud-init" => DataSource::CloudInit(CloudInitPaths {
                meta_data: cloud_init_meta_data.to_string(),
                user_data: cloud_init_user_data.to_string(),
                network_config: cloud_init_network_config.to_string(),
            }),
            "ec2" => DataSource::Ec2,
            "gcp" => DataSource::Gcp,
            "azure" => DataSource::Azure,
            _ => {
                continue; // Skip unknown sources
            }
        };

        sources.push(PrioritizedSource { source, priority });
    }

    // Sort by priority (lower number = higher priority)
    sources.sort_by_key(|s| s.priority);

    Ok(sources)
}

/// Parse data source priority string into a list of prioritized sources
pub fn parse_data_sources(
    sources_str: &str,
    config_file: Option<&str>,
    cloud_init_meta_data: &str,
    cloud_init_user_data: &str,
    cloud_init_network_config: &str,
) -> Result<Vec<PrioritizedSource>, Box<dyn std::error::Error>> {
    let mut sources = Vec::new();
    let mut priority = 0;

    for source_name in sources_str.split(',') {
        priority += 10;
        let source = match source_name.trim() {
            "local" => {
                if let Some(config_file) = config_file {
                    DataSource::Local(config_file.to_string())
                } else {
                    continue; // Skip if no config file specified
                }
            }
            "cloud-init" => DataSource::CloudInit(CloudInitPaths {
                meta_data: cloud_init_meta_data.to_string(),
                user_data: cloud_init_user_data.to_string(),
                network_config: cloud_init_network_config.to_string(),
            }),
            "ec2" => DataSource::Ec2,
            "gcp" => DataSource::Gcp,
            "azure" => DataSource::Azure,
            _ => {
                continue; // Skip unknown sources
            }
        };

        sources.push(PrioritizedSource { source, priority });
    }

    // Sort by priority (lower number = higher priority)
    sources.sort_by_key(|s| s.priority);

    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_merge_configurations() {
        let mut base = json!({
            "system": {
                "hostname": "old-host"
            },
            "users": ["user1"]
        });

        let overlay = json!({
            "system": {
                "timezone": "UTC"
            },
            "users": ["user2"],
            "networking": {
                "interfaces": []
            }
        });

        merge_configurations(&mut base, overlay).unwrap();

        assert_eq!(base["system"]["hostname"], "old-host");
        assert_eq!(base["system"]["timezone"], "UTC");
        assert_eq!(base["users"], json!(["user2"]));
        assert_eq!(base["networking"]["interfaces"], json!([]));
    }

    #[test]
    fn test_parse_data_sources() {
        let sources = parse_data_sources(
            "local,cloud-init,ec2",
            Some("/tmp/config.json"),
            "/tmp/meta-data",
            "/tmp/user-data",
            "/tmp/network-config",
        ).unwrap();

        assert_eq!(sources.len(), 3);
        assert_eq!(sources[0].priority, 10);
        assert_eq!(sources[1].priority, 20);
        assert_eq!(sources[2].priority, 30);

        // Test without local config file
        let sources_no_local = parse_data_sources(
            "local,cloud-init",
            None,
            "/tmp/meta-data",
            "/tmp/user-data",
            "/tmp/network-config",
        ).unwrap();

        assert_eq!(sources_no_local.len(), 1); // Only cloud-init should be included
    }
}
