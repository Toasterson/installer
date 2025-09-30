# Unified Provisioning System for illumos/FreeBSD/Linux

A modern, type-safe replacement for cloud-init that provides consistent, orthogonal configuration management across Unix-like operating systems.

## Overview

This unified provisioning system addresses the complexity and inconsistencies of existing cloud provisioning tools by providing:

- **Single Source of Truth**: One configuration schema that works across all supported platforms
- **Type Safety**: Rust-based schema with compile-time validation
- **Orthogonal Design**: One concept, one configuration path - no overlapping or conflicting options
- **Multi-Platform Support**: Native support for illumos IPS, FreeBSD pkg, and Linux package managers
- **Extensible Architecture**: Plugin-based system for adding new platforms and data sources

## Architecture

The system consists of four main components:

### 1. Configuration Schema (`sysconfig-config-schema/`)

A shared Rust crate defining the unified configuration structure:

```rust
struct UnifiedConfig {
    system: Option<SystemConfig>,           // Hostname, timezone, locale
    storage: Option<StorageConfig>,         // Advanced ZFS management, pools, datasets, replication
    networking: Option<NetworkingConfig>,   // Interfaces, DNS, routes
    software: Option<SoftwareConfig>,       // Packages, repositories
    users: Vec<UserConfig>,                 // User accounts and authentication
    scripts: Option<ScriptConfig>,          // Boot-time scripts
    integrations: Option<IntegrationConfig>, // Ansible, Puppet, Chef
    containers: Option<ContainerConfig>,    // Zones, jails, containers with nested config
    power_state: Option<PowerStateConfig>,  // Final power state
}
```

### 2. Provisioning Plugin (`sysconfig-plugins/src/bin/provisioning-plugin.rs`)

Orchestrates the provisioning process by:
- Collecting configuration from multiple data sources (priority-ordered)
- Converting legacy formats (cloud-init, etc.) to unified schema
- Validating and applying configuration via sysconfig daemon

### 3. Enhanced Base Plugins

Platform-specific plugins that understand the unified schema:
- **illumos Base Plugin**: Advanced ZFS, IPS, SMF, zones with nested sysconfig
- **FreeBSD Base Plugin**: Advanced ZFS, pkg, rc.d, jails with nested sysconfig
- **Linux Base Plugin**: systemd, APT/YUM/APK, containers with nested sysconfig

### 4. Provisioning CLI (`sysconfig-cli`)

Command-line interface for manual provisioning and testing:
```bash
sysconfig provision --config-file unified-config.json --dry-run
sysconfig provision --sources "cloud-init,ec2,gcp" --run-once
```

## Key Features

### Orthogonal User Management

Eliminates cloud-init's confusing overlapping user configuration methods:

```json
{
  "users": [
    {
      "name": "admin",
      "sudo": "unrestricted",
      "authentication": {
        "password": {
          "hash": "$6$rounds=4096$...",
          "expire_on_first_login": true
        },
        "ssh_keys": ["ssh-rsa AAAAB3..."],
        "ssh_import_ids": ["gh:username"]
      }
    }
  ]
}
```

### Multi-Platform Package Management

Native support for all major Unix package managers:

```json
{
  "software": {
    "repositories": {
      "apt": {
        "sources": [{"name": "docker", "uri": "https://download.docker.com/..."}]
      },
      "ips": {
        "publishers": [{"name": "solaris", "origin": "https://pkg.oracle.com/..."}]
      },
      "pkg": {
        "repositories": [{"name": "FreeBSD", "url": "pkg+http://pkg.FreeBSD.org/..."}]
      }
    }
  }
}
```

### Advanced ZFS Storage Management

Comprehensive ZFS management with enterprise features:

```json
{
  "storage": {
    "pools": [
      {
        "name": "rpool",
        "pool_type": "zpool",
        "topology": {
          "data": [{"vdev_type": "raidz2", "devices": ["/dev/disk1", "/dev/disk2", "/dev/disk3", "/dev/disk4"]}],
          "log": [{"vdev_type": "mirror", "devices": ["/dev/nvme0", "/dev/nvme1"]}],
          "cache": [{"vdev_type": "stripe", "devices": ["/dev/ssd0"]}],
          "spare": ["/dev/spare0"]
        }
      }
    ],
    "zfs_datasets": [
      {
        "name": "rpool/data",
        "dataset_type": "filesystem",
        "quota": "500G",
        "reservation": "50G",
        "children": [
          {"name": "rpool/data/databases", "dataset_type": "filesystem", "quota": "200G"}
        ]
      }
    ],
    "zfs_snapshots": [
      {
        "dataset": "rpool/data",
        "name": "daily-backup",
        "recursive": true,
        "properties": {"com.example:retention": "30d"}
      }
    ],
    "zfs_replication": [
      {
        "source_dataset": "rpool/data",
        "target": "backup-server:backup/data",
        "replication_type": "incremental",
        "ssh_config": {"user": "backup", "host": "backup-server.example.com"}
      }
    ]
  }
}
```

### Container Management with Nested Sysconfig

Cross-platform container orchestration with complete nested configuration:

```json
{
  "containers": {
    "zones": [
      {
        "name": "web-zone",
        "brand": "sparse",
        "state": "running",
        "zonepath": "/zones/web-zone",
        "resources": {"cpu_cap": 2.0, "physical_memory_cap": "2G"},
        "sysconfig": {
          "system": {"hostname": "web-server"},
          "software": {"packages_to_install": ["web/server/apache-24"]},
          "users": [{"name": "webadmin", "sudo": "deny", "authentication": {"ssh_keys": ["ssh-rsa AAAAB3..."]}}]
        }
      }
    ],
    "jails": [
      {
        "name": "db-jail",
        "hostname": "db.example.com",
        "ip_addresses": ["192.168.1.52"],
        "sysconfig": {
          "storage": {"zfs_datasets": [{"name": "storage/jails/db-jail/data", "quota": "100G"}]},
          "software": {"packages_to_install": ["mysql80-server"]}
        }
      }
    ],
    "containers": [
      {
        "name": "web-server",
        "image": "nginx:1.21-alpine",
        "runtime": "docker",
        "sysconfig": {
          "scripts": {
            "main_scripts": [{"id": "setup", "content": "#!/bin/sh\necho 'Container configured'"}]
          }
        }
      }
    ]
  }
}
```

## Data Source Support

The provisioning system can collect configuration from multiple sources in priority order:

### Supported Data Sources
- **Local files**: JSON, YAML, KDL formats
- **Cloud-init compatibility**: Automatic conversion from existing cloud-init configs
- **EC2 metadata service**: Instance metadata and user-data
- **GCP metadata service**: Instance attributes and startup scripts  
- **Azure metadata service**: VM metadata and custom data
- **Extensible**: Plugin architecture for custom data sources

### Data Source Priority
Sources are processed in configurable priority order, with later sources overriding earlier ones:

```bash
# Default priority: local (highest) -> cloud-init -> ec2 -> gcp -> azure (lowest)
sysconfig provision --sources "local,cloud-init,ec2,gcp,azure"
```

## Cloud-Init Migration

Existing cloud-init configurations are automatically converted to the unified schema:

**Before (cloud-init)**:
```yaml
#cloud-config
hostname: myhost
users:
  - name: ubuntu
    sudo: ALL=(ALL) NOPASSWD:ALL
    ssh_authorized_keys:
      - ssh-rsa AAAAB3...
packages:
  - vim
  - git
```

**After (unified)**:
```json
{
  "system": {"hostname": "myhost"},
  "users": [{
    "name": "ubuntu",
    "sudo": "unrestricted", 
    "authentication": {"ssh_keys": ["ssh-rsa AAAAB3..."]}
  }],
  "software": {"packages_to_install": ["vim", "git"]}
}
```

## Platform-Specific Features

### illumos/Solaris
- **IPS Publishers**: Configure package publishers with SSL certificates
- **Advanced ZFS Management**: Complex pool topologies, hierarchical datasets, snapshots, replication
- **SMF Services**: Enable/disable and configure system services
- **Zone Management**: Full zone lifecycle management with nested sysconfig using `oxide/zone` crate
- **Resource Controls**: CPU caps, memory limits, network isolation

### FreeBSD  
- **PKG Repositories**: Configure custom repositories with signature verification
- **Advanced ZFS Management**: Complex pool topologies, hierarchical datasets, snapshots, replication
- **RC Scripts**: Enable/disable system services
- **Jail Management**: Complete jail provisioning with nested sysconfig
- **Resource Controls**: Jail parameters, networking, and isolation

### Linux
- **Multi-Package Manager**: APT, YUM/DNF, APK support
- **Container Runtime**: Docker, Podman integration with nested sysconfig
- **systemd Integration**: Service management and configuration
- **Volume Management**: Named volumes, bind mounts, and network configuration
- **Resource Constraints**: CPU and memory limits

## Usage Examples

### Basic Server Provisioning

```bash
# Create configuration
cat > server-config.json << 'EOF'
{
  "system": {
    "hostname": "web-server-01",
    "timezone": "UTC"
  },
  "users": [{
    "name": "admin",
    "sudo": "unrestricted",
    "authentication": {
      "ssh_keys": ["ssh-rsa AAAAB3NzaC1yc2E... admin@workstation"]
    }
  }],
  "software": {
    "packages_to_install": ["nginx", "git", "htop"]
  }
}
EOF

# Apply configuration
sysconfig provision --config-file server-config.json --dry-run
sysconfig provision --config-file server-config.json
```

### Cloud Environment Provisioning

```bash
# Automatically detect and use available cloud data sources
sysconfig provision --sources "cloud-init,ec2,gcp,azure" --run-once

# Run continuously with 5-minute intervals
sysconfig provision --sources "cloud-init,ec2" --interval 300
```

### Migration from Cloud-Init

```bash
# Convert existing cloud-init config
sysconfig provision \
  --cloud-init-user-data /var/lib/cloud/seed/nocloud/user-data \
  --cloud-init-meta-data /var/lib/cloud/seed/nocloud/meta-data \
  --dry-run
```

## Configuration Examples

### Comprehensive Examples

- [`test-unified-provisioning.json`](sysconfig-plugins/test-unified-provisioning.json) - Basic unified provisioning example
- [`test-cloud-init-example.yaml`](sysconfig-plugins/test-cloud-init-example.yaml) - Cloud-init format for conversion testing
- [`test-provisioning-simple.kdl`](sysconfig-plugins/test-provisioning-simple.kdl) - Simple KDL format example

### Advanced Examples

- [`examples/advanced-illumos-config.json`](sysconfig-plugins/examples/advanced-illumos-config.json) - Complete illumos server with zones and advanced ZFS
- [`examples/advanced-freebsd-config.json`](sysconfig-plugins/examples/advanced-freebsd-config.json) - FreeBSD infrastructure with jails and storage
- [`examples/advanced-linux-config.json`](sysconfig-plugins/examples/advanced-linux-config.json) - Linux container orchestration with Docker
- [`examples/README.md`](sysconfig-plugins/examples/README.md) - Comprehensive documentation and usage examples

## Installation

### Prerequisites
- Rust 1.70+ 
- Protobuf compiler (`protoc`)
- Platform-specific tools:
  - illumos: `pkg`, `zfs`, `svccfg`
  - FreeBSD: `pkg`, `zfs`, `service`
  - Linux: `apt`/`yum`/`apk`, `systemctl`

### Building

```bash
# Build the entire system
cd installer/
cargo build --release

# Build specific components
cd sysconfig-config-schema/
cargo build --release

cd ../sysconfig-plugins/
cargo build --release

cd ../sysconfig-cli/
cargo build --release
```

### Platform-Specific Dependencies

**illumos/Solaris:**
```bash
# Add zone crate for illumos zone management
# Automatically included for target_os = "illumos"
```

**FreeBSD:**
```bash
# Native jail management using system commands
# No additional dependencies required
```

**Linux:**
```bash
# Docker/Podman for container management
sudo apt-get install docker.io podman  # Ubuntu/Debian
sudo yum install docker podman         # RHEL/CentOS
```

### Installation

```bash
# Install binaries
sudo cp target/release/sysconfig-cli /usr/local/bin/
sudo cp target/release/illumos-base-plugin /usr/local/bin/
sudo cp target/release/provisioning-plugin /usr/local/bin/

# Create service directories
sudo mkdir -p /var/run/sysconfig
sudo mkdir -p /etc/sysconfig/plugins
```

## Testing

### Unit Tests
```bash
cd sysconfig-config-schema/
cargo test

cd ../sysconfig-plugins/
cargo test
```

### Integration Tests
```bash
# Test provisioning with dry-run
./sysconfig-plugins/test_provisioning_e2e.sh

# Test configuration conversion
sysconfig provision --config-file test-unified-provisioning.json --dry-run --run-once

# Test advanced features
sysconfig provision --config-file examples/advanced-illumos-config.json --dry-run
sysconfig provision --config-file examples/advanced-freebsd-config.json --dry-run
sysconfig provision --config-file examples/advanced-linux-config.json --dry-run
```

### Manual Testing
```bash
# Start sysconfig daemon (in terminal 1)
cd ../sysconfig/
cargo run

# Start base plugin (in terminal 2) 
cd ../sysconfig-plugins/
cargo run --bin illumos-base-plugin  # or freebsd-base-plugin

# Test provisioning (in terminal 3)
cd ../sysconfig-cli/
cargo run -- provision --config-file ../sysconfig-plugins/test-unified-provisioning.json --dry-run --run-once
```

## Design Principles

1. **Orthogonality**: Each configuration concept has exactly one representation
2. **Type Safety**: Compile-time validation prevents invalid configurations  
3. **Platform Awareness**: Native support for platform-specific features
4. **Migration Friendly**: Seamless conversion from existing cloud-init configs
5. **Extensibility**: Plugin architecture supports new platforms and data sources
6. **Debugging**: Comprehensive logging and dry-run capabilities

## Comparison with Cloud-Init

| Feature | Cloud-Init | Unified Provisioning |
|---------|------------|---------------------|
| User Password Config | 3 overlapping methods | 1 unified method |
| Package Managers | Linux-centric | Native multi-platform |
| Configuration Format | YAML with magic strings | JSON with semantic types |
| Type Safety | Runtime validation | Compile-time validation |
| Platform Support | Linux primary | Equal illumos/FreeBSD/Linux |
| Extensibility | Monolithic | Plugin-based |
| Error Handling | Inconsistent | Structured error types |

## Contributing

1. **Schema Changes**: Update `sysconfig-config-schema/src/lib.rs`
2. **New Platforms**: Add base plugin in `sysconfig-plugins/src/bin/`
3. **Data Sources**: Extend `provisioning/datasources.rs`
4. **Converters**: Update `provisioning/converter.rs`

### Guidelines
- Maintain orthogonality - avoid overlapping configuration paths
- Use semantic types (enums) instead of magic strings
- Provide comprehensive validation and error messages
- Include cross-platform considerations in design
- Add tests for new features

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

## Status

**Current State**: Advanced features implementation complete
- ✅ Unified configuration schema with container support
- ✅ Multi-platform base plugins (illumos, FreeBSD, Linux) 
- ✅ Cloud-init conversion
- ✅ Multi-source data collection (local, cloud-init, EC2, GCP, Azure)
- ✅ CLI integration
- ✅ Advanced storage management (ZFS) - Complex topologies, datasets, snapshots, replication
- ✅ Container integration - Zones, jails, Docker with nested sysconfig
- ✅ Network configuration implementation
- ✅ Comprehensive example configurations
- ✅ Documentation and usage examples

**New Advanced Features:**
- ✅ **Complex ZFS Pool Topologies**: RAIDZ, mirrors, log devices, cache devices, spares
- ✅ **Hierarchical ZFS Datasets**: Nested structures with quotas, reservations, properties
- ✅ **ZFS Snapshots & Replication**: Automated snapshots with SSH-based replication
- ✅ **Cross-Platform Container Management**: Native zone/jail/container support
- ✅ **Nested Sysconfig**: Complete configuration management within containers
- ✅ **Resource Management**: CPU, memory, and network resource controls
- ✅ **Service Orchestration**: Multi-tier application deployment automation

**Future Enhancements:**
- 📋 Service discovery and load balancing
- 📋 High availability and clustering
- 📋 Monitoring and alerting integration
- 📋 Cloud provider native integration