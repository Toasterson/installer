# Advanced Unified Provisioning Examples

This directory contains comprehensive examples demonstrating the advanced storage management and container orchestration capabilities of the unified provisioning system.

## Overview

The unified provisioning system now supports:

- **Advanced ZFS Storage Management**: Complex pool topologies, datasets, snapshots, and replication
- **Container Management with Nested Configuration**: Solaris zones, FreeBSD jails, and Linux containers with full nested sysconfig support
- **Cross-Platform Consistency**: Same configuration schema works across illumos, FreeBSD, and Linux

## Example Configurations

### 1. `advanced-illumos-config.json`

Demonstrates comprehensive illumos/Solaris provisioning with:

#### Storage Features
- **Complex ZFS Pool Topologies**: RAIDZ2 data vdevs, mirrored log devices, cache devices, and spare drives
- **Hierarchical ZFS Datasets**: Nested filesystem structures with quotas, reservations, and custom properties
- **Volume Management**: ZFS volumes for VM storage with optimized block sizes
- **Automated Snapshots**: Scheduled snapshots with retention policies and custom metadata
- **ZFS Replication**: Incremental replication to remote backup systems over SSH

#### Zone Management
- **Multi-Zone Architecture**: Web, database, and monitoring zones with resource controls
- **Nested Sysconfig**: Each zone has its own complete sysconfig configuration
- **Resource Management**: CPU caps, memory limits, and network isolation
- **Service Orchestration**: Coordinated service deployment across zones

#### Example Zone Configuration
```json
{
  "name": "web-zone",
  "brand": "sparse",
  "state": "running",
  "zonepath": "/export/zones/web-zone",
  "resources": {
    "cpu_cap": 2.0,
    "physical_memory_cap": "2G"
  },
  "sysconfig": {
    "system": {
      "hostname": "web-server"
    },
    "software": {
      "packages_to_install": ["web/server/apache-24"]
    },
    "users": [
      {
        "name": "webadmin",
        "sudo": "deny",
        "authentication": {
          "ssh_keys": ["ssh-rsa AAAAB3..."]
        }
      }
    ]
  }
}
```

### 2. `advanced-freebsd-config.json`

Showcases FreeBSD-specific features including:

#### FreeBSD Jails
- **Traditional Jail Management**: Using FreeBSD's native jail system
- **Jail Networking**: Dedicated IP addresses and interface assignments
- **Service Isolation**: Database, web, monitoring, and backup services in separate jails
- **Nested Configuration**: Full sysconfig deployment inside each jail

#### Storage Management
- **Boot Pool Configuration**: ZFS root pool with cache devices
- **Data Pool Setup**: RAIDZ storage pools with log and cache devices
- **Jail Storage**: Dedicated datasets for each jail with appropriate properties

#### Example Jail Configuration
```json
{
  "name": "db-jail",
  "path": "/usr/jails/db-jail",
  "hostname": "db.example.com",
  "ip_addresses": ["192.168.1.52"],
  "parameters": {
    "sysvmsg": "inherit",
    "sysvsem": "inherit",
    "sysvshm": "inherit"
  },
  "sysconfig": {
    "storage": {
      "zfs_datasets": [
        {
          "name": "storage/jails/db-jail/data",
          "properties": {
            "recordsize": "16K",
            "primarycache": "metadata"
          },
          "quota": "100G"
        }
      ]
    },
    "software": {
      "packages_to_install": ["mysql80-server"]
    }
  }
}
```

### 3. `advanced-linux-config.json`

Demonstrates Linux container orchestration with:

#### Docker Container Management
- **Multi-Container Architecture**: Web server, database, cache, monitoring, and backup services
- **Container Networking**: Custom networks and port mappings
- **Volume Management**: Named volumes and bind mounts
- **Resource Constraints**: CPU and memory limits

#### Container Orchestration
```json
{
  "name": "web-server",
  "image": "nginx:1.21-alpine",
  "runtime": "docker",
  "state": "running",
  "volumes": [
    {
      "source": "/data/web/html",
      "target": "/usr/share/nginx/html",
      "mount_type": "bind"
    }
  ],
  "sysconfig": {
    "scripts": {
      "main_scripts": [
        {
          "id": "setup_web_content",
          "content": "#!/bin/sh\necho '<h1>Welcome</h1>' > /usr/share/nginx/html/index.html"
        }
      ]
    }
  }
}
```

## Key Features Demonstrated

### Advanced ZFS Storage Management

#### 1. Complex Pool Topologies
```json
{
  "topology": {
    "data": [
      {
        "vdev_type": "raidz2",
        "devices": ["/dev/ada3", "/dev/ada4", "/dev/ada5", "/dev/ada6"]
      }
    ],
    "log": [
      {
        "vdev_type": "mirror", 
        "devices": ["/dev/ada7", "/dev/ada8"]
      }
    ],
    "cache": [
      {
        "vdev_type": "stripe",
        "devices": ["/dev/ada9"]
      }
    ],
    "spare": ["/dev/ada10"]
  }
}
```

#### 2. Dataset Hierarchies
```json
{
  "name": "tank/data",
  "dataset_type": "filesystem",
  "quota": "500G",
  "children": [
    {
      "name": "tank/data/databases",
      "properties": {
        "recordsize": "8K",
        "primarycache": "metadata"
      },
      "reservation": "50G"
    }
  ]
}
```

#### 3. Automated Snapshots
```json
{
  "dataset": "tank/data/databases",
  "name": "daily-backup",
  "recursive": true,
  "properties": {
    "com.example:retention": "30d",
    "com.example:backup_type": "daily"
  }
}
```

#### 4. Remote Replication
```json
{
  "source_dataset": "tank/data/databases",
  "target": "backup-server:backup/databases",
  "replication_type": "incremental",
  "ssh_config": {
    "user": "backup",
    "host": "backup-server.example.com",
    "key_path": "/root/.ssh/backup_rsa"
  }
}
```

### Nested Sysconfig Configuration

The nested sysconfig feature allows complete configuration management inside containers, zones, and jails. This enables:

- **Service Configuration**: Install and configure services within containers
- **User Management**: Create users and set up authentication inside containers
- **Storage Management**: Configure container-specific storage (ZFS datasets, etc.)
- **Script Execution**: Run initialization and setup scripts within containers
- **Network Configuration**: Set up container-specific networking

#### Benefits

1. **Isolation**: Each container has its own complete configuration
2. **Consistency**: Same configuration language across host and containers
3. **Automation**: Full infrastructure-as-code for containerized services
4. **Flexibility**: Mix and match different configurations per container

### Cross-Platform Support

The unified provisioning system provides consistent configuration syntax across:

- **illumos/Solaris**: Zones with SMF services and IPS packages
- **FreeBSD**: Jails with rc.d services and pkg packages  
- **Linux**: Docker/Podman containers with systemd services

## Getting Started

### 1. Basic Testing

```bash
# Test configuration validation
sysconfig provision --config-file advanced-illumos-config.json --dry-run

# Apply configuration 
sysconfig provision --config-file advanced-freebsd-config.json
```

### 2. Container Management

```bash
# Deploy Linux container stack
sysconfig provision --config-file advanced-linux-config.json

# Check container status
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
```

### 3. Storage Management

```bash
# Verify ZFS pools and datasets
zpool status
zfs list -t all

# Check snapshots
zfs list -t snapshot
```

## Architecture Notes

### Zone/Jail Implementation

The system uses platform-native container management:

- **illumos**: Uses the `oxide/zone` Rust crate for type-safe zone management
- **FreeBSD**: Uses native `jail(8)` and `jexec(8)` commands
- **Linux**: Uses Docker API for container lifecycle management

### Nested Configuration Deployment

When nested sysconfig is specified:

1. Configuration is serialized to JSON
2. Copied into the container/zone/jail at `/etc/sysconfig/nested-config.json`
3. Sysconfig provisioning is executed within the container context
4. Results are logged and reported back to the host

### Storage Management

Advanced ZFS features are implemented using:

- Native `zpool(8)` and `zfs(8)` commands
- Topology-aware pool creation
- Property management and inheritance
- Snapshot lifecycle management
- SSH-based replication setup

## Security Considerations

### Container Isolation

- Containers run with appropriate resource limits
- Network isolation using dedicated networks/VLANs
- File system isolation using dedicated datasets/volumes

### SSH Key Management

- SSH keys are managed through the authentication configuration
- Private keys for replication are stored securely
- Key-based authentication is preferred over passwords

### Service Hardening

- Services run as dedicated users with minimal privileges
- Sudo access is configured with specific command restrictions
- System services are configured with appropriate security settings

## Troubleshooting

### Common Issues

1. **Permission Errors**: Ensure sysconfig runs with appropriate privileges
2. **Network Connectivity**: Verify container networks and firewall rules
3. **Storage Issues**: Check ZFS pool status and available space
4. **Service Startup**: Review container logs and service status

### Debugging Commands

```bash
# Check sysconfig logs
tail -f /var/log/sysconfig/provisioning.log

# Container debugging
docker logs <container_name>
jexec <jail_name> /bin/sh  # FreeBSD
zlogin <zone_name>         # illumos

# Storage debugging  
zpool status -v
zfs get all <dataset>
```

## Contributing

To extend these examples:

1. Add new container configurations to demonstrate additional services
2. Enhance storage configurations with new ZFS features
3. Include more complex networking scenarios
4. Add monitoring and observability configurations

## License

These examples are provided under the same license as the main sysconfig project.