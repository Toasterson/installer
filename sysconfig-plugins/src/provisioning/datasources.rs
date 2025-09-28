//! Data source implementations for the provisioning plugin
//!
//! This module provides implementations for collecting configuration data
//! from various cloud providers and local sources.

use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tokio::time::Duration;
use tracing::{debug, warn};

/// Data source for provisioning configuration
#[derive(Debug, Clone)]
pub enum DataSource {
    Local(String),                // File path
    CloudInit(CloudInitPaths),    // Cloud-init file paths
    Ec2,                          // EC2 metadata service
    Gcp,                          // GCP metadata service
    Azure,                        // Azure metadata service
}

#[derive(Debug, Clone)]
pub struct CloudInitPaths {
    pub meta_data: String,
    pub user_data: String,
    pub network_config: String,
}

/// Priority-ordered data source
#[derive(Debug, Clone)]
pub struct PrioritizedSource {
    pub source: DataSource,
    pub priority: u32,
}

/// Collect configuration from a data source
pub async fn collect_from_source(source: &DataSource) -> Result<Value, Box<dyn std::error::Error>> {
    match source {
        DataSource::Local(path) => collect_from_local_file(path).await,
        DataSource::CloudInit(paths) => collect_from_cloud_init(paths).await,
        DataSource::Ec2 => collect_from_ec2().await,
        DataSource::Gcp => collect_from_gcp().await,
        DataSource::Azure => collect_from_azure().await,
    }
}

async fn collect_from_local_file(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        return Ok(Value::Null);
    }

    debug!("Reading local configuration file: {}", path);
    let content = fs::read_to_string(path)?;

    // Try to parse as JSON first, then YAML
    match serde_json::from_str(&content) {
        Ok(json) => Ok(json),
        Err(_) => {
            // Try YAML parsing
            match serde_yaml::from_str(&content) {
                Ok(yaml) => Ok(yaml),
                Err(e) => {
                    warn!("Failed to parse configuration file as JSON or YAML: {}", e);
                    Err(Box::new(e))
                }
            }
        }
    }
}

async fn collect_from_cloud_init(paths: &CloudInitPaths) -> Result<Value, Box<dyn std::error::Error>> {
    let mut config = serde_json::json!({});

    // Read meta-data
    if Path::new(&paths.meta_data).exists() {
        debug!("Reading cloud-init meta-data: {}", paths.meta_data);
        let meta_content = fs::read_to_string(&paths.meta_data)?;
        if let Ok(meta_json) = serde_json::from_str::<Value>(&meta_content) {
            config["meta_data"] = meta_json;
        } else if let Ok(meta_yaml) = serde_yaml::from_str::<Value>(&meta_content) {
            config["meta_data"] = meta_yaml;
        }
    }

    // Read user-data
    if Path::new(&paths.user_data).exists() {
        debug!("Reading cloud-init user-data: {}", paths.user_data);
        let user_content = fs::read_to_string(&paths.user_data)?;

        // Handle different user-data formats
        if user_content.starts_with("#cloud-config") {
            // YAML format
            let yaml_content = user_content.lines().skip(1).collect::<Vec<_>>().join("\n");
            if let Ok(user_yaml) = serde_yaml::from_str::<Value>(&yaml_content) {
                config["user_data"] = user_yaml;
            }
        } else if user_content.starts_with("#!/") {
            // Script format
            config["user_data"] = serde_json::json!({
                "runcmd": [user_content]
            });
        } else {
            // Try parsing as JSON/YAML directly
            if let Ok(user_json) = serde_json::from_str::<Value>(&user_content) {
                config["user_data"] = user_json;
            } else if let Ok(user_yaml) = serde_yaml::from_str::<Value>(&user_content) {
                config["user_data"] = user_yaml;
            }
        }
    }

    // Read network-config
    if Path::new(&paths.network_config).exists() {
        debug!("Reading cloud-init network-config: {}", paths.network_config);
        let network_content = fs::read_to_string(&paths.network_config)?;
        if let Ok(network_json) = serde_json::from_str::<Value>(&network_content) {
            config["network_config"] = network_json;
        } else if let Ok(network_yaml) = serde_yaml::from_str::<Value>(&network_content) {
            config["network_config"] = network_yaml;
        }
    }

    Ok(config)
}

async fn collect_from_ec2() -> Result<Value, Box<dyn std::error::Error>> {
    debug!("Collecting configuration from EC2 metadata service");

    // Check if we're running on EC2 by trying to access the metadata service
    let client = Client::new();
    let token_response = client
        .put("http://169.254.169.254/latest/api/token")
        .header("X-aws-ec2-metadata-token-ttl-seconds", "21600")
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    let token = match token_response {
        Ok(response) => response.text().await.ok(),
        Err(_) => {
            debug!("Not running on EC2 or metadata service unavailable");
            return Ok(Value::Null);
        }
    };

    let mut config = serde_json::json!({});

    // Get instance metadata
    if let Some(token) = &token {
        let instance_id = get_ec2_metadata(&client, token, "instance-id").await.unwrap_or_default();
        let instance_type = get_ec2_metadata(&client, token, "instance-type").await.unwrap_or_default();
        let availability_zone = get_ec2_metadata(&client, token, "placement/availability-zone").await.unwrap_or_default();
        let local_hostname = get_ec2_metadata(&client, token, "local-hostname").await.unwrap_or_default();
        let public_hostname = get_ec2_metadata(&client, token, "public-hostname").await.unwrap_or_default();

        config["ec2"] = serde_json::json!({
            "instance_id": instance_id,
            "instance_type": instance_type,
            "availability_zone": availability_zone,
            "local_hostname": local_hostname,
            "public_hostname": public_hostname
        });

        // Try to get user-data
        if let Ok(user_data) = get_ec2_metadata(&client, token, "../user-data").await {
            if !user_data.trim().is_empty() {
                config["user_data_raw"] = Value::String(user_data);
            }
        }
    }

    Ok(config)
}

async fn get_ec2_metadata(
    client: &Client,
    token: &str,
    path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("http://169.254.169.254/latest/meta-data/{}", path);
    let response = client
        .get(&url)
        .header("X-aws-ec2-metadata-token", token)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(format!("Failed to get EC2 metadata from {}: {}", url, response.status()).into())
    }
}

async fn collect_from_gcp() -> Result<Value, Box<dyn std::error::Error>> {
    debug!("Collecting configuration from GCP metadata service");

    let client = Client::new();

    // Check if we're running on GCP
    let response = client
        .get("http://metadata.google.internal/computeMetadata/v1/instance/id")
        .header("Metadata-Flavor", "Google")
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    if response.is_err() {
        debug!("Not running on GCP or metadata service unavailable");
        return Ok(Value::Null);
    }

    let mut config = serde_json::json!({});

    // Get instance metadata
    let instance_id = get_gcp_metadata(&client, "instance/id").await.unwrap_or_default();
    let machine_type = get_gcp_metadata(&client, "instance/machine-type").await.unwrap_or_default();
    let zone = get_gcp_metadata(&client, "instance/zone").await.unwrap_or_default();
    let hostname = get_gcp_metadata(&client, "instance/hostname").await.unwrap_or_default();

    config["gcp"] = serde_json::json!({
        "instance_id": instance_id,
        "machine_type": machine_type,
        "zone": zone,
        "hostname": hostname
    });

    // Try to get startup script
    if let Ok(startup_script) = get_gcp_metadata(&client, "instance/attributes/startup-script").await {
        if !startup_script.trim().is_empty() {
            config["startup_script"] = Value::String(startup_script);
        }
    }

    Ok(config)
}

async fn get_gcp_metadata(
    client: &Client,
    path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("http://metadata.google.internal/computeMetadata/v1/{}", path);
    let response = client
        .get(&url)
        .header("Metadata-Flavor", "Google")
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(format!("Failed to get GCP metadata from {}: {}", url, response.status()).into())
    }
}

async fn collect_from_azure() -> Result<Value, Box<dyn std::error::Error>> {
    debug!("Collecting configuration from Azure metadata service");

    let client = Client::new();

    // Check if we're running on Azure
    let response = client
        .get("http://169.254.169.254/metadata/instance/compute/vmId?api-version=2021-02-01&format=text")
        .header("Metadata", "true")
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    if response.is_err() {
        debug!("Not running on Azure or metadata service unavailable");
        return Ok(Value::Null);
    }

    let mut config = serde_json::json!({});

    // Get instance metadata
    let vm_id = get_azure_metadata(&client, "instance/compute/vmId").await.unwrap_or_default();
    let vm_size = get_azure_metadata(&client, "instance/compute/vmSize").await.unwrap_or_default();
    let location = get_azure_metadata(&client, "instance/compute/location").await.unwrap_or_default();
    let resource_group = get_azure_metadata(&client, "instance/compute/resourceGroupName").await.unwrap_or_default();

    config["azure"] = serde_json::json!({
        "vm_id": vm_id,
        "vm_size": vm_size,
        "location": location,
        "resource_group": resource_group
    });

    // Try to get custom data
    if let Ok(custom_data) = get_azure_metadata(&client, "instance/compute/customData").await {
        if !custom_data.trim().is_empty() {
            // Custom data is base64 encoded
            if let Ok(decoded) = base64::decode(&custom_data) {
                if let Ok(decoded_str) = String::from_utf8(decoded) {
                    config["custom_data"] = Value::String(decoded_str);
                }
            }
        }
    }

    Ok(config)
}

async fn get_azure_metadata(
    client: &Client,
    path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "http://169.254.169.254/metadata/{}?api-version=2021-02-01&format=text",
        path
    );
    let response = client
        .get(&url)
        .header("Metadata", "true")
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(format!("Failed to get Azure metadata from {}: {}", url, response.status()).into())
    }
}
