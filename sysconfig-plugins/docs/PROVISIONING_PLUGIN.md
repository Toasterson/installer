# SysConfig Provisioning Plugin

## Overview

The SysConfig Provisioning Plugin is a comprehensive metadata reader that aggregates system configuration from multiple sources during system boot. It reads configuration data from various cloud providers and local sources, then applies them through the SysConfig service to configure the system.

## Architecture

The plugin operates as a gRPC service that registers with the main SysConfig service. It follows a priority-based approach to reading configuration, where local configurations take precedence over cloud-provided metadata.

### Priority Order

1. **Local Configuration** (`/etc/sysconfig.kdl`) - Highest priority
2. **Cloud-Init Sources** (NoCloud, ConfigDrive)
3. **Cloud Vendor Metadata Services** - Lowest priority

Configuration from higher priority sources overrides configuration from lower priority sources for the same settings.

## Implemented Features

### Core Functionality

- **Plugin Framework**: Full gRPC-based plugin implementation that registers with SysConfig service
- **Multi-Source Support**: Aggregates configuration from multiple sources with proper priority handling
- **Configuration Merging**: Intelligent merging of configurations from different sources

### Configuration Sources

#### 1. Local KDL File (`/etc/sysconfig.kdl`)
- **Status**: ✅ Implemented
- **Features**:
  - Parses KDL format configuration files
  - Supports hostname, nameservers, and network interface configuration
  - Uses the same KDL schema as the main SysConfig service

Example configuration:
```kdl
hostname "my-server"
nameserver "8.8.8.8"
nameserver "8.8.4.4"

interface "eth0" {
    address name="ipv4" kind="static" "192.168.1.100/24"
    address name="gateway" kind="static" "192.168.1.1"
}
```

#### 2. Cloud-Init Sources
- **Status**: ✅ Partially Implemented
- **Supported Datasources**:
  - **NoCloud**: Reads from devices labeled "cidata"
  - **ConfigDrive**: Reads from `/media/configdrive`
- **Features**:
  - Automatic detection of cloud-init media
  - Support for meta-data, user-data, and network-config files
  - Network Config v1 parsing (v2 planned)

#### 3. EC2/AWS Metadata Service
- **Status**: ✅ Implemented
- **Endpoint**: `http://169.254.169.254/latest/`
- **Features**:
  - Hostname retrieval
  - SSH public keys
  - User data scripts
  - Instance metadata

#### 4. DigitalOcean Metadata
- **Status**: ✅ Implemented
- **Source**: Metadata ISO (label: config-2)
- **Features**:
  - Droplet configuration
  - Network interface setup
  - DNS configuration
  - SSH keys

#### 5. SmartOS Metadata
- **Status**: ✅ Basic Implementation
- **Method**: `mdata-get` command
- **Features**:
  - Hostname
  - SSH authorized keys
  - Basic network configuration

#### 6. Azure Metadata Service
- **Status**: ✅ Basic Implementation
- **Endpoint**: `http://169.254.169.254/metadata/`
- **Features**:
  - Instance name
  - Basic metadata retrieval

#### 7. Google Cloud Platform (GCP)
- **Status**: ✅ Basic Implementation
- **Endpoint**: `http://metadata.google.internal/`
- **Features**:
  - Hostname
  - SSH keys

### Data Types Supported

- **Hostname**: System hostname configuration
- **Network Configuration**:
  - Static IP addresses with CIDR notation
  - DHCP (v4 and v6)
  - Gateway configuration
  - MTU settings
  - MAC address mapping
- **DNS Configuration**:
  - Nameservers
  - Search domains
- **SSH Configuration**:
  - Authorized keys for root user
- **User Data**:
  - Custom scripts and cloud-init configurations
- **Metadata Storage**:
  - Generic key-value metadata storage

## Planned Features

### High Priority (Next Release)

1. **OpenStack Metadata Service**
   - Support for OpenStack metadata API
   - Network configuration from OpenStack
   - Vendor data support

2. **VMware GuestInfo**
   - Reading configuration from VMware guestinfo variables
   - OVF environment support

3. **Cloud-Init Network Config v2**
   - Full Netplan-style network configuration
   - Advanced networking features (bonds, VLANs, bridges)

4. **Oracle Cloud Infrastructure (OCI)**
   - Support for Oracle Cloud metadata service
   - iSCSI boot volume configuration

### Medium Priority

1. **Enhanced User Management**
   - User creation and configuration
   - Group management
   - SSH keys for non-root users

2. **Package Management Integration**
   - Package installation directives
   - Repository configuration

3. **Filesystem Configuration**
   - Mount point configuration
   - Disk partitioning directives

4. **Service Management**
   - Enable/disable services
   - Service configuration files

### Low Priority / Future Enhancements

1. **Alibaba Cloud Support**
2. **IBM Cloud Support**
3. **Vultr Metadata Support**
4. **Hetzner Cloud Support**
5. **Custom Metadata Sources**
   - Plugin API for custom metadata providers
   - Webhook-based configuration

## Configuration

### Command-Line Arguments

```bash
provisioning-plugin [OPTIONS]

OPTIONS:
    --socket <PATH>           Unix socket path for plugin service
    --service-socket <PATH>   SysConfig service socket path
    --no-register            Don't auto-register with SysConfig
    --config-file <PATH>     Path to local config file (default: /etc/sysconfig.kdl)
```

### Environment Variables

- `PROVISIONING_DEBUG`: Enable debug logging
- `PROVISIONING_SOURCES`: Comma-separated list of sources to check
- `PROVISIONING_TIMEOUT`: Timeout for metadata service requests (seconds)

## Building

```bash
cd installer/sysconfig-plugins
cargo build --release --bin provisioning-plugin
```

## Installation

1. Build the plugin
2. Copy to system path: `/usr/lib/sysconfig/plugins/provisioning-plugin`
3. Create systemd/SMF service for the plugin
4. Start the service

### Example SMF Manifest (illumos)

```xml
<?xml version="1.0"?>
<!DOCTYPE service_bundle SYSTEM "/usr/share/lib/xml/dtd/service_bundle.dtd.1">
<service_bundle type='manifest' name='provisioning-plugin'>
  <service name='system/sysconfig/provisioning' type='service' version='1'>
    <dependency name='sysconfig' type='service' grouping='require_all' restart_on='none'>
      <service_fmri value='svc:/system/sysconfig:default' />
    </dependency>
    <exec_method type='method' name='start' 
                 exec='/usr/lib/sysconfig/plugins/provisioning-plugin --socket /var/run/sysconfig-provisioning.sock'
                 timeout_seconds='60' />
    <exec_method type='method' name='stop' exec=':kill' timeout_seconds='60' />
    <instance name='default' enabled='true' />
  </service>
</service_bundle>
```

## Integration with Machined

The provisioning plugin works in conjunction with the machined installer:

1. **Installation Phase**: Machined writes initial `/etc/sysconfig.kdl` based on machine configuration
2. **First Boot**: Provisioning plugin reads all available sources and applies configuration
3. **Runtime**: Configuration can be updated through SysConfig APIs

## Security Considerations

- The plugin runs with root privileges to access system configuration
- Metadata services are accessed over HTTP (not HTTPS) as per cloud provider standards
- SSH keys and sensitive data are handled securely
- Local configuration files should have appropriate permissions (600 or 640)

## Troubleshooting

### Debug Logging

Enable debug logging:
```bash
export PROVISIONING_DEBUG=1
provisioning-plugin --socket /var/run/test.sock
```

### Common Issues

1. **Plugin fails to register**: Check that SysConfig service is running
2. **Metadata not detected**: Verify cloud detection works with DMI/SMBIOS data
3. **Network configuration not applied**: Check plugin has sufficient privileges
4. **Configuration conflicts**: Review priority order and source precedence

### Testing

Test with local configuration only:
```bash
provisioning-plugin --config-file /tmp/test.kdl --no-register
```

## Contributing

When adding new metadata sources:

1. Implement detection method in `detect_cloud_vendor()`
2. Add loading function `load_<vendor>_metadata()`
3. Add parsing logic for vendor-specific formats
4. Update merge logic if needed
5. Add documentation and examples
6. Add tests for the new source

## License

This plugin is part of the illumos installer project and follows the same licensing terms.

## References

- [Cloud-Init Documentation](https://cloud-init.io/)
- [EC2 Instance Metadata](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-metadata.html)
- [DigitalOcean Metadata](https://docs.digitalocean.com/products/droplets/how-to/retrieve-droplet-metadata/)
- [Azure Instance Metadata Service](https://docs.microsoft.com/en-us/azure/virtual-machines/linux/instance-metadata-service)
- [GCP Instance Metadata](https://cloud.google.com/compute/docs/storing-retrieving-metadata)