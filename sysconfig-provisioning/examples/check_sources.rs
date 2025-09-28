use sysconfig_provisioning::sources::azure::AzureSource;
use sysconfig_provisioning::sources::cloud_init::CloudInitSource;
use sysconfig_provisioning::sources::digitalocean::DigitalOceanSource;
use sysconfig_provisioning::sources::ec2::EC2Source;
use sysconfig_provisioning::sources::gcp::GCPSource;
use sysconfig_provisioning::sources::openstack::OpenStackSource;
use sysconfig_provisioning::sources::smartos::SmartOSSource;
use sysconfig_provisioning::sources::SourceManager;

#[tokio::main]
async fn main() {
    println!("Checking metadata source availability...\n");

    // Check EC2
    let mut ec2 = EC2Source::new();
    ec2.set_timeout(2);
    println!(
        "EC2 metadata service available: {}",
        ec2.is_available().await
    );

    // Check Azure
    let mut azure = AzureSource::new();
    azure.set_timeout(2);
    println!(
        "Azure metadata service available: {}",
        azure.is_available().await
    );

    // Check GCP
    let mut gcp = GCPSource::new();
    gcp.set_timeout(2);
    println!(
        "GCP metadata service available: {}",
        gcp.is_available().await
    );

    // Check DigitalOcean
    let mut digitalocean = DigitalOceanSource::new();
    digitalocean.set_timeout(2);
    println!(
        "DigitalOcean metadata service available: {}",
        digitalocean.is_available().await
    );

    // Check OpenStack
    let mut openstack = OpenStackSource::new();
    openstack.set_timeout(2);
    println!(
        "OpenStack metadata service available: {}",
        openstack.is_available().await
    );

    // Check SmartOS
    println!(
        "SmartOS metadata available: {}",
        SmartOSSource::is_available().await
    );

    // Check CloudInit
    let mut cloud_init = CloudInitSource::new();
    cloud_init.set_timeout(2);
    println!(
        "CloudInit sources available: {}",
        cloud_init.is_available().await
    );

    println!("\nUsing SourceManager to detect available sources:");
    let manager = SourceManager::new();
    let available = manager.detect_available_sources().await;

    if available.is_empty() {
        println!("No metadata sources detected");
    } else {
        println!("Detected sources:");
        for (name, priority) in available {
            println!("  - {} (priority: {})", name, priority);
        }
    }
}
