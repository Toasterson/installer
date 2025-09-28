# Unified Configuration Schema for Cloud Instance Provisioning

This crate provides a type-safe, unified configuration schema that replaces the complex and redundant structure of cloud-init with a clean, hierarchical, and domain-driven approach.

## Overview

The unified configuration schema addresses several problems with existing cloud provisioning systems:

- **Orthogonality**: One concept, one configuration path - eliminates functional overlaps and ambiguities
- **Clarity and Explicitness**: Self-documenting field names and structures
- **Type Safety**: Maps directly to strongly-typed Rust language constructs  
- **Composition over Proliferation**: Structured by functional domain rather than flat lists

## Architecture

The schema replaces cloud-init's 60+ top-level keys with a small, predictable set of logical domains:

```rust
struct UnifiedConfig {
    system: Option<SystemConfig>,           // Identity and environment
    storage: Option<StorageConfig>,         // Disks, pools, filesystems
    networking: Option<NetworkingConfig>,   // Interfaces, DNS, routes
    software: Option<SoftwareConfig>,       // Packages and repositories
    users: Vec<UserConfig>,                 // User accounts and auth
    scripts: Option<ScriptConfig>,          // Boot-time scripts
    integrations: Option<IntegrationConfig>, // Third-party tools
    power_state: Option<PowerStateConfig>,  // Final power state
}
```

## Key Features

### Unified User Management

Eliminates cloud-init's three overlapping methods for user configuration:

```rust
struct UserConfig {
    name: String,
    sudo: Option<SudoConfig>,              // Enum: Deny | Unrestricted | Custom(Vec<String>)
    authentication: AuthenticationConfig,  // Unified password/SSH handling
    // ... other user properties
}
```

### Multi-Platform Package Management

Supports all major package managers with distribution-specific configurations:

```rust
struct RepositoryConfig {
    apt: Option<AptRepositoryConfig>,     // Debian/Ubuntu
    yum: Option<YumRepositoryConfig>,     // RedHat/CentOS/Fedora
    apk: Option<ApkRepositoryConfig>,     // Alpine Linux
    ips: Option<IpsRepositoryConfig>,     // illumos/Solaris IPS
    pkg: Option<PkgRepositoryConfig>,     // FreeBSD pkg
}
```

### Semantic Network Configuration

Type-safe network interface and addressing configuration:

```rust
enum AddressKind {
    Static(String),    // CIDR notation
    Dhcp4,
    Dhcp6,
    Addrconf,          // IPv6 autoconfiguration
}
```

## Usage

### Creating Configuration

```rust
use sysconfig_config_schema::*;

let config = UnifiedConfig {
    system: Some(SystemConfig {
        hostname: Some("web-server-01".to_string()),
        timezone: Some("UTC".to_string()),
        locale: Some("en_US.UTF-8".to_string()),
        ..Default::default()
    }),
    users: vec![
        UserConfig {
            name: "admin".to_string(),
            sudo: Some(SudoConfig::Unrestricted),
            authentication: AuthenticationConfig {
                ssh_keys: vec![
                    "ssh-rsa AAAAB3NzaC1yc2E... admin@workstation".to_string()
                ],
                ..Default::default()
            },
            ..Default::default()
        }
    ],
    ..Default::default()
};
```

### Serialization

```rust
// To JSON
let json = config.to_json()?;

// From JSON
let config = UnifiedConfig::from_json(&json)?;

// Validation
config.validate()?;
```

## Data Source Integration

The provisioning system can collect configuration from multiple sources:

- **Local files**: JSON, YAML, or KDL format
- **Cloud-init**: Automatic conversion from existing cloud-init configurations
- **Cloud metadata services**: EC2, GCP, Azure
- **Custom sources**: Extensible data source framework

### Cloud-Init Migration

Existing cloud-init configurations are automatically converted:

```yaml
# cloud-init format
users:
  - name: ubuntu
    sudo: ALL=(ALL) NOPASSWD:ALL
    ssh_authorized_keys:
      - ssh-rsa AAAAB3...
```

Becomes:

```rust
UserConfig {
    name: "ubuntu".to_string(),
    sudo: Some(SudoConfig::Unrestricted),
    authentication: AuthenticationConfig {
        ssh_keys: vec!["ssh-rsa AAAAB3...".to_string()],
        ..Default::default()
    },
    ..Default::default()
}
```

## Platform Support

The schema supports platform-specific configurations while maintaining portability:

### illumos/Solaris
- IPS (Image Packaging System) repositories
- ZFS storage pools and datasets
- SMF service management
- Zones and resource management

### FreeBSD
- pkg repositories and signature verification  
- ZFS storage pools and datasets
- rc.d service management
- Jails and resource management

### Linux
- APT, YUM, APK package managers
- Various filesystems (ext4, xfs, btrfs)
- systemd/OpenRC service management
- Container runtimes

## Integration

The unified schema integrates with:

- **sysconfig-plugins**: Base plugins for each operating system
- **sysconfig daemon**: Central state management
- **Provisioning plugin**: Data source collection and conversion
- **CLI tools**: Human-friendly configuration management

## Examples

See the `examples/` directory for complete configuration examples:

- `basic-server.json`: Simple web server setup
- `database-cluster.json`: Multi-node database configuration  
- `development-workstation.json`: Developer environment setup
- `cloud-init-migration.yaml`: Cloud-init to unified conversion example

## Error Handling

The schema includes comprehensive error handling:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    ValidationError(String),
}
```

## Contributing

When adding new configuration options:

1. Follow the orthogonality principle - one concept, one path
2. Use semantic enums instead of magic strings where possible
3. Provide comprehensive validation
4. Include cross-platform considerations
5. Add integration tests for new features

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.