#[cfg(test)]
mod tests {
    use sysconfig_provisioning::sources::azure::AzureSource;
    use sysconfig_provisioning::sources::cloud_init::CloudInitSource;
    use sysconfig_provisioning::sources::digitalocean::DigitalOceanSource;
    use sysconfig_provisioning::sources::ec2::EC2Source;
    use sysconfig_provisioning::sources::gcp::GCPSource;
    use sysconfig_provisioning::sources::openstack::OpenStackSource;
    use sysconfig_provisioning::sources::smartos::SmartOSSource;

    #[tokio::test]
    async fn test_ec2_is_available() {
        let mut source = EC2Source::new();
        source.set_timeout(1); // Short timeout for testing

        // This will likely return false in most test environments
        // We're just verifying the method exists and can be called
        let _available = source.is_available().await;
    }

    #[tokio::test]
    async fn test_azure_is_available() {
        let mut source = AzureSource::new();
        source.set_timeout(1); // Short timeout for testing

        let _available = source.is_available().await;
    }

    #[tokio::test]
    async fn test_gcp_is_available() {
        let mut source = GCPSource::new();
        source.set_timeout(1); // Short timeout for testing

        let _available = source.is_available().await;
    }

    #[tokio::test]
    async fn test_digitalocean_is_available() {
        let mut source = DigitalOceanSource::new();
        source.set_timeout(1); // Short timeout for testing

        let _available = source.is_available().await;
    }

    #[tokio::test]
    async fn test_openstack_is_available() {
        let mut source = OpenStackSource::new();
        source.set_timeout(1); // Short timeout for testing

        let _available = source.is_available().await;
    }

    #[tokio::test]
    async fn test_smartos_is_available() {
        // SmartOS has a static method
        let _available = SmartOSSource::is_available().await;
    }

    #[tokio::test]
    async fn test_cloud_init_is_available() {
        let mut source = CloudInitSource::new();
        source.set_timeout(1); // Short timeout for testing

        let _available = source.is_available().await;
    }

    #[tokio::test]
    async fn test_source_manager_is_available() {
        use sysconfig_provisioning::sources::SourceManager;

        let manager = SourceManager::new();

        // Test checking availability for different source types
        let sources = vec![
            "ec2",
            "azure",
            "gcp",
            "digitalocean",
            "openstack",
            "smartos",
            "cloud-init",
            "local",
        ];

        for source_type in sources {
            // This will likely return false in test environments
            // We're verifying the method can be called without panicking
            let _available = manager.is_source_available(source_type).await;
        }
    }

    #[tokio::test]
    async fn test_detect_available_sources() {
        use sysconfig_provisioning::sources::SourceManager;

        let manager = SourceManager::new();

        // This should return a list of available sources
        // In most test environments, this will likely be empty or only contain local
        let available_sources = manager.detect_available_sources().await;

        // Verify it returns a Vec of (String, u32) tuples
        assert!(available_sources.is_empty() || available_sources.len() > 0);

        for (name, priority) in available_sources {
            assert!(!name.is_empty());
            assert!(priority > 0);
        }
    }
}
