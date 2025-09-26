# SysConfig Provisioning System

## Overview

The SysConfig Provisioning System is a comprehensive metadata aggregation solution that reads system configuration from multiple sources during boot and runtime. It supports local configuration files, cloud-init datasources, and various cloud vendor metadata services.

## Current Implementation Status

### ✅ Implemented

#### Core Infrastructure
- **SysConfig Service**: gRPC-based configuration management service with plugin architecture
- **Plugin System**: Support for platform-specific plugins (illumos, Linux, FreeBSD, Windows)
- **State Management**: JSON-based state storage with persistence and revision tracking
- **KDL Parser**: Configuration parsing using the KDL (KDL Document Language) format

#### Configuration Sources
1. **Local KDL File** (`/etc/sysconfig.kdl`)
   - Hostname configuration
   - Nameserver configuration
   - Network interface configuration with multiple address types

2. **Integration Points**
   - Machined installer writes initial configuration during installation
   - Plugins can read and apply configuration through gRPC APIs

### 🚧 Planned Implementation

#### Provisioning Plugin
A dedicated plugin (`provisioning-plugin`) that will aggregate configuration from multiple sources:

##### Priority Order (highest to lowest)
1. Local configuration files (`/etc/sysconfig.kdl`)
2. Cloud-init sources (NoCloud, ConfigDrive)
3. Cloud vendor metadata services

##### Planned Data Sources

**Cloud-Init Compatible**
- **NoCloud**: ISO/USB with `cidata` label
- **ConfigDrive**: OpenStack-style configuration drive
- **Network Config v1**: Basic network configuration
- **Network Config v2**: Netplan-style advanced networking

**Cloud Vendors**
- **Amazon EC2**: Instance metadata service (169.254.169.254)
- **DigitalOcean**: Metadata ISO and droplet configuration
- **Microsoft Azure**: Azure Instance Metadata Service
- **Google Cloud Platform**: GCP metadata server
- **Oracle Cloud**: OCI metadata service
- **SmartOS/Triton**: mdata-get integration
- **OpenStack**: Full metadata API support

**Local Sources**
- `/etc/sysconfig.kdl`: Primary local configuration
- `/etc/cloud/cloud.cfg`: Cloud-init configuration
- `/var/lib/cloud/`: Cloud-init runtime data

##### Supported Configuration Types
- **Hostname**: System hostname and FQDN
- **Network Configuration**:
  - Static IP addresses (IPv4/IPv6)
  - DHCP configuration
  - Gateway and routing
  - DNS servers and search domains
  - MTU settings
  - VLAN, bonding, bridging (planned)
- **SSH Configuration**:
  - Authorized keys management
  - User account provisioning
- **User Data**:
  - First-boot scripts
  - Cloud-init user-data
- **Package Management** (future):
  - Package installation
  - Repository configuration
- **Storage** (future):
  - Disk partitioning
  - Filesystem creation
  - Mount points

## Architecture

```
┌─────────────────────────────────────────────────┐
│                 Boot Process                     │
├─────────────────────────────────────────────────┤
│                                                  │
│  1. System Boot                                  │
│       ↓                                          │
│  2. SysConfig Service Starts                     │
│       ↓                                          │
│  3. Provisioning Plugin Starts                   │
│       ↓                                          │
│  4. Read Configuration Sources:                  │
│     • /etc/sysconfig.kdl (if exists)            │
│     • Cloud-init sources                         │
│     • Cloud vendor metadata                      │
│       ↓                                          │
│  5. Merge Configurations (priority-based)        │
│       ↓                                          │
│  6. Apply Through SysConfig API                  │
│       ↓                                          │
│  7. Platform Plugins Execute Changes             │
│                                                  │
└─────────────────────────────────────────────────┘
```

## Configuration Format

### KDL Configuration Example

```kdl
hostname "web-server-01"

nameserver "8.8.8.8"
nameserver "8.8.4.4"

interface "eth0" {
    address name="ipv4" kind="static" "192.168.1.100/24"
    address name="gateway" kind="static" "192.168.1.1"
}

interface "eth1" {
    address name="dhcp" kind="dhcp4"
}
```

### Cloud-Init Network Config v1 Example

```yaml
version: 1
config:
  - type: physical
    name: eth0
    mac_address: "52:54:00:12:34:56"
    subnets:
      - type: static
        address: 192.168.1.100/24
        gateway: 192.168.1.1
  - type: nameserver
    address: [8.8.8.8, 8.8.4.4]
    search: [example.com]
```

## Building

### Prerequisites
- Rust 1.70 or later
- Protocol Buffers compiler (`protoc`)

### Build Commands

```bash
# Build all components
cd installer
cargo build --release

# Build specific components
cargo build -p sysconfig --release
cargo build -p sysconfig-plugins --release
cargo build -p machineconfig --release
```

## Deployment

### illumos/SmartOS

1. Install the sysconfig service binary to `/usr/lib/sysconfig/sysconfig`
2. Install plugins to `/usr/lib/sysconfig/plugins/`
3. Create SMF manifest for the service
4. Import and enable the service:
   ```bash
   svccfg import /lib/svc/manifest/system/sysconfig.xml
   svcadm enable sysconfig
   ```

### Linux (systemd)

1. Install binaries to `/usr/lib/sysconfig/`
2. Create systemd unit files
3. Enable and start services:
   ```bash
   systemctl enable sysconfig.service
   systemctl start sysconfig.service
   ```

## Testing

### Manual Testing

```bash
# Test KDL parsing
cat > /tmp/test.kdl << EOF
hostname "test-host"
nameserver "1.1.1.1"
EOF

# Run sysconfig with test configuration
sysconfig --socket /tmp/sysconfig.sock &

# Use CLI to interact
sysconfig-cli --socket /tmp/sysconfig.sock get-state
```

### Integration Testing

```bash
# Test with cloud-init NoCloud datasource
mkdir -p /tmp/cidata
cat > /tmp/cidata/meta-data << EOF
instance-id: test-001
hostname: test-vm
EOF

cat > /tmp/cidata/user-data << EOF
#cloud-config
ssh_authorized_keys:
  - ssh-rsa AAAAB3... user@host
EOF

# Create ISO
genisoimage -o /tmp/cidata.iso -V cidata -r -J /tmp/cidata
```

## Roadmap

### Phase 1: Core Implementation (Current)
- [x] Basic sysconfig service
- [x] Plugin architecture
- [x] KDL configuration parser
- [x] Platform plugins (basic)

### Phase 2: Provisioning Plugin (Next)
- [ ] Multi-source configuration reader
- [ ] Cloud-init compatibility
- [ ] Major cloud vendor support
- [ ] Configuration merging logic

### Phase 3: Advanced Features
- [ ] Network configuration v2
- [ ] Package management integration
- [ ] Storage configuration
- [ ] Service management
- [ ] User/group management

### Phase 4: Enterprise Features
- [ ] Configuration validation
- [ ] Rollback capability
- [ ] Audit logging
- [ ] Remote configuration management
- [ ] Encrypted configuration support

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement your changes
4. Add tests
5. Submit a pull request

## License

This project is part of the illumos installer and follows the same licensing terms.

## References

- [Cloud-Init Documentation](https://cloud-init.io/)
- [KDL Specification](https://kdl.dev/)
- [illumos Project](https://illumos.org/)
- [EC2 Metadata](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-metadata.html)
- [Azure IMDS](https://docs.microsoft.com/en-us/azure/virtual-machines/linux/instance-metadata-service)
