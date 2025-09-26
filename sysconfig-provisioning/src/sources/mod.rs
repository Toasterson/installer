use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::config::{CloudVendor, ProvisioningConfig};

mod cloud_init;
mod ec2;
mod azure;
mod gcp;
mod digitalocean;
mod local;
mod openstack;
mod smartos;
mod utils;

pub use cloud_init::CloudInitSource;
pub use ec2::EC2Source;
pub use azure::AzureSource;
pub use gcp::GCPSource;
pub use digitalocean::DigitalOceanSource;
pub use local::LocalSource;
pub use openstack::OpenStackSource;
pub use smartos::SmartOSSource;

/// Manages all configuration sources
pub struct SourceManager {
    disabled_sources: Vec<String>,
    timeout_seconds: u64,
}

impl SourceManager {
    /// Create a new source manager
    pub fn new(disabled_sources: Vec<String>) -> Self {
        Self {
            disabled_sources,
            timeout_seconds: 5,
        }
    }

    /// Set the timeout for metadata service requests
    pub fn set_timeout(&mut self, seconds: u64) {
        self.timeout_seconds = seconds;
    }

    /// Check if a source is disabled
    fn is_source_disabled(&self, source_name: &str) -> bool {
        self.disabled_sources.iter().any(|s| s == source_name)
    }

    /// Load configuration from local KDL file
    pub async fn load_local_kdl(&self, path: &Path) -> Result<ProvisioningConfig> {
        if self.is_source_disabled("local") {
            return Err(anyhow!("Local source is disabled"));
        }

        let source = LocalSource::new();
        source.load_kdl(path).await
    }

    /// Load configuration from cloud-init sources
    pub async fn load_cloud_init(&self) -> Result<ProvisioningConfig> {
        if self.is_source_disabled("cloud-init") {
            return Err(anyhow!("Cloud-init source is disabled"));
        }

        let mut source = CloudInitSource::new();
        source.set_timeout(self.timeout_seconds);
        source.load().await
    }

    /// Detect cloud vendor and load configuration
    pub async fn detect_and_load_cloud_vendor(&self) -> Result<(String, ProvisioningConfig)> {
        // First, try to detect the cloud vendor
        let vendor = self.detect_cloud_vendor().await?;

        info!("Detected cloud vendor: {}", vendor);

        // Load configuration based on detected vendor
        let config = match vendor {
            CloudVendor::EC2 => {
                if self.is_source_disabled("ec2") {
                    return Err(anyhow!("EC2 source is disabled"));
                }
                let mut source = EC2Source::new();
                source.set_timeout(self.timeout_seconds);
                source.load().await?
            }
            CloudVendor::Azure => {
                if self.is_source_disabled("azure") {
                    return Err(anyhow!("Azure source is disabled"));
                }
                let mut source = AzureSource::new();
                source.set_timeout(self.timeout_seconds);
                source.load().await?
            }
            CloudVendor::GCP => {
                if self.is_source_disabled("gcp") {
                    return Err(anyhow!("GCP source is disabled"));
                }
                let mut source = GCPSource::new();
                source.set_timeout(self.timeout_seconds);
                source.load().await?
            }
            CloudVendor::DigitalOcean => {
                if self.is_source_disabled("digitalocean") {
                    return Err(anyhow!("DigitalOcean source is disabled"));
                }
                let source = DigitalOceanSource::new();
                source.load().await?
            }
            CloudVendor::OpenStack => {
                if self.is_source_disabled("openstack") {
                    return Err(anyhow!("OpenStack source is disabled"));
                }
                let mut source = OpenStackSource::new();
                source.set_timeout(self.timeout_seconds);
                source.load().await?
            }
            CloudVendor::SmartOS => {
                if self.is_source_disabled("smartos") {
                    return Err(anyhow!("SmartOS source is disabled"));
                }
                let source = SmartOSSource::new();
                source.load().await?
            }
            CloudVendor::VMware => {
                return Err(anyhow!("VMware source not yet implemented"));
            }
            CloudVendor::Oracle => {
                return Err(anyhow!("Oracle Cloud source not yet implemented"));
            }
            CloudVendor::Unknown => {
                return Err(anyhow!("Could not detect cloud vendor"));
            }
        };

        Ok((vendor.to_string(), config))
    }

    /// Detect which cloud vendor we're running on
    pub async fn detect_cloud_vendor(&self) -> Result<CloudVendor> {
        debug!("Attempting to detect cloud vendor...");

        // Check DMI/SMBIOS information first
        if let Ok(vendor) = self.detect_vendor_from_dmi().await {
            return Ok(vendor);
        }

        // Check for specific cloud vendor markers
        if self.is_ec2().await {
            return Ok(CloudVendor::EC2);
        }

        if self.is_azure().await {
            return Ok(CloudVendor::Azure);
        }

        if self.is_gcp().await {
            return Ok(CloudVendor::GCP);
        }

        if self.is_digitalocean().await {
            return Ok(CloudVendor::DigitalOcean);
        }

        if self.is_openstack().await {
            return Ok(CloudVendor::OpenStack);
        }

        if self.is_smartos().await {
            return Ok(CloudVendor::SmartOS);
        }

        Ok(CloudVendor::Unknown)
    }

    /// Detect vendor from DMI/SMBIOS information
    async fn detect_vendor_from_dmi(&self) -> Result<CloudVendor> {
        // Linux DMI path
        #[cfg(target_os = "linux")]
        {
            if let Ok(vendor) = tokio::fs::read_to_string("/sys/class/dmi/id/sys_vendor").await {
                let vendor = vendor.trim().to_lowercase();

                if vendor.contains("amazon") || vendor.contains("ec2") {
                    return Ok(CloudVendor::EC2);
                }
                if vendor.contains("microsoft") {
                    return Ok(CloudVendor::Azure);
                }
                if vendor.contains("google") {
                    return Ok(CloudVendor::GCP);
                }
                if vendor.contains("digitalocean") {
                    return Ok(CloudVendor::DigitalOcean);
                }
                if vendor.contains("openstack") {
                    return Ok(CloudVendor::OpenStack);
                }
            }

            if let Ok(product) = tokio::fs::read_to_string("/sys/class/dmi/id/product_name").await {
                let product = product.trim().to_lowercase();

                if product.contains("openstack") {
                    return Ok(CloudVendor::OpenStack);
                }
                if product.contains("droplet") {
                    return Ok(CloudVendor::DigitalOcean);
                }
            }
        }

        // illumos/SmartOS smbios
        #[cfg(target_os = "illumos")]
        {
            use std::process::Command;

            if let Ok(output) = Command::new("/usr/sbin/smbios")
                .args(&["-t", "1"])
                .output()
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();

                    if text.contains("joyent") || text.contains("smartdc") {
                        return Ok(CloudVendor::SmartOS);
                    }
                    if text.contains("amazon") || text.contains("ec2") {
                        return Ok(CloudVendor::EC2);
                    }
                    if text.contains("digitalocean") {
                        return Ok(CloudVendor::DigitalOcean);
                    }
                    if text.contains("google") {
                        return Ok(CloudVendor::GCP);
                    }
                }
            }
        }

        Err(anyhow!("Could not detect vendor from DMI/SMBIOS"))
    }

    /// Check if running on EC2
    async fn is_ec2(&self) -> bool {
        // Check for EC2 metadata service
        if utils::check_metadata_service("http://169.254.169.254/latest/meta-data/", None, self.timeout_seconds).await {
            return true;
        }

        // Check for EC2 specific files
        Path::new("/sys/hypervisor/uuid").exists() &&
            tokio::fs::read_to_string("/sys/hypervisor/uuid")
                .await
                .map(|u| u.starts_with("ec2"))
                .unwrap_or(false)
    }

    /// Check if running on Azure
    async fn is_azure(&self) -> bool {
        // Check for Azure metadata service with required header
        let headers = vec![("Metadata", "true")];
        utils::check_metadata_service(
            "http://169.254.169.254/metadata/instance?api-version=2021-01-01",
            Some(headers),
            self.timeout_seconds
        ).await || Path::new("/sys/class/dmi/id/chassis_asset_tag").exists() &&
            tokio::fs::read_to_string("/sys/class/dmi/id/chassis_asset_tag")
                .await
                .map(|t| t.trim() == "7783-7084-3265-9085-8269-3286-77")
                .unwrap_or(false)
    }

    /// Check if running on GCP
    async fn is_gcp(&self) -> bool {
        // Check for GCP metadata service with required header
        let headers = vec![("Metadata-Flavor", "Google")];
        utils::check_metadata_service(
            "http://metadata.google.internal/computeMetadata/v1/",
            Some(headers),
            self.timeout_seconds
        ).await
    }

    /// Check if running on DigitalOcean
    async fn is_digitalocean(&self) -> bool {
        // Check for DigitalOcean metadata ISO
        Path::new("/dev/disk/by-label/config-2").exists() ||
        // Or check metadata service
        utils::check_metadata_service(
            "http://169.254.169.254/metadata/v1/",
            None,
            self.timeout_seconds
        ).await
    }

    /// Check if running on OpenStack
    async fn is_openstack(&self) -> bool {
        // Check for OpenStack metadata service
        utils::check_metadata_service(
            "http://169.254.169.254/openstack/latest/meta_data.json",
            None,
            self.timeout_seconds
        ).await ||
        // Check for config drive
        Path::new("/dev/disk/by-label/config-2").exists() ||
        Path::new("/dev/disk/by-label/CONFIG-2").exists()
    }

    /// Check if running on SmartOS
    async fn is_smartos(&self) -> bool {
        // Check for mdata-get command
        Path::new("/usr/sbin/mdata-get").exists() ||
        Path::new("/native/usr/sbin/mdata-get").exists()
    }

    /// Load all available sources (for testing/debugging)
    pub async fn load_all_available(&self) -> Vec<(String, ProvisioningConfig)> {
        let mut configs = Vec::new();

        // Try local
        if let Ok(config) = self.load_local_kdl(&PathBuf::from("/etc/sysconfig.kdl")).await {
            configs.push(("local".to_string(), config));
        }

        // Try cloud-init
        if let Ok(config) = self.load_cloud_init().await {
            configs.push(("cloud-init".to_string(), config));
        }

        // Try cloud vendor
        if let Ok((vendor, config)) = self.detect_and_load_cloud_vendor().await {
            configs.push((vendor, config));
        }

        configs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_source_manager_creation() {
        let manager = SourceManager::new(vec!["ec2".to_string()]);
        assert!(manager.is_source_disabled("ec2"));
        assert!(!manager.is_source_disabled("azure"));
    }

    #[tokio::test]
    async fn test_disabled_source() {
        let manager = SourceManager::new(vec!["local".to_string()]);
        let result = manager.load_local_kdl(&PathBuf::from("/etc/sysconfig.kdl")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disabled"));
    }
}
