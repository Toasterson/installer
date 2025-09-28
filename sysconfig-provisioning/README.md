# Sysconfig Provisioning CLI

A command-line tool for system provisioning that parses configuration from multiple sources and writes it to sysconfig. This tool replaces the previous plugin-server architecture with a simpler CLI approach that can be used during boot or manually.

## Overview

The provisioning CLI is designed to:
- Parse KDL configuration files and convert them to sysconfig state
- Auto-detect cloud environments and fetch their metadata
- Determine if network setup is required before cloud provisioning
- Write configuration directly to sysconfig via Unix socket
- Support multiple configuration sources with priority-based merging

## Architecture

Unlike the previous plugin-based approach, this tool operates as a standalone CLI that:
1. Gathers configuration from various sources
2. Merges them based on priority
3. Writes the final state to sysconfig
4. Exits after completion

This design simplifies the provisioning workflow and makes it easier to integrate into boot processes.

## Installation

```bash
cd sysconfig-provisioning
cargo build --release
sudo cp target/release/provisioning-plugin /usr/local/bin/sysconfig-provision
```

## Usage

### Basic Commands

#### Parse a KDL configuration file
```bash
sysconfig-provision parse --config /etc/sysconfig.kdl
```

#### Auto-detect and apply configuration
```bash
sysconfig-provision autodetect --check-network
```

#### Apply from specific sources
```bash
sysconfig-provision apply --sources local,ec2,cloud-init
```

#### Check current provisioning status
```bash
sysconfig-provision status
```

#### Detect available sources
```bash
sysconfig-provision detect [--network]
```

### Command Details

#### `apply` - Apply provisioning configuration
```bash
sysconfig-provision apply [OPTIONS]

Options:
  -c, --config <PATH>           Path to KDL configuration file
  --sources <LIST>              Enable specific sources (comma-separated)
  --disable-sources <LIST>      Disable specific sources (comma-separated)
  -d, --dry-run                 Show what would be applied without making changes
  --force                       Force apply even if no changes detected
```

#### `autodetect` - Auto-detect and apply provisioning
```bash
sysconfig-provision autodetect [OPTIONS]

Options:
  --check-network               Check if network setup is required first
  -d, --dry-run                 Show what would be applied without making changes
  --network-timeout <SECONDS>   Max time to wait for network sources (default: 30)
```

#### `parse` - Parse and validate a KDL config
```bash
sysconfig-provision parse [OPTIONS]

Options:
  -c, --config <PATH>           Path to KDL configuration file (required)
  -f, --format <FORMAT>         Output format: json or pretty (default: pretty)
```

#### `detect` - Detect available provisioning sources
```bash
sysconfig-provision detect [OPTIONS]

Options:
  --network                     Check network sources (requires network)
  -f, --format <FORMAT>         Output format: json or pretty (default: pretty)
```

#### `status` - Show current provisioning status
```bash
sysconfig-provision status [OPTIONS]

Options:
  -f, --format <FORMAT>         Output format: json or pretty (default: pretty)
```

## Configuration Sources

The provisioning CLI supports multiple configuration sources, each with a priority level:

| Source | Priority | Description |
|--------|----------|-------------|
| Local KDL | 1 | Local KDL configuration files |
| Cloud-Init | 10 | Cloud-init metadata and userdata |
| EC2 | 20 | Amazon EC2 instance metadata |
| Azure | 21 | Microsoft Azure instance metadata |
| GCP | 22 | Google Cloud Platform metadata |
| DigitalOcean | 23 | DigitalOcean droplet metadata |
| OpenStack | 24 | OpenStack metadata service |
| SmartOS | 30 | SmartOS metadata service |

Lower priority numbers take precedence when merging configurations.

## KDL Configuration Format

The provisioning CLI accepts KDL (KDL Document Language) configuration files:

```kdl
// System hostname
hostname "my-server"

// DNS nameservers
nameservers "8.8.8.8" "1.1.1.1"

// Network interfaces
interface "net0" {
    address "dhcp" primary=true
    mtu 1500
    enabled true
}

interface "net1" {
    address "192.168.1.100/24" "192.168.1.1"
    mtu 9000
    enabled true
}

// SSH authorized keys
ssh-keys {
    root "ssh-rsa AAAAB3NzaC1... user@host"
    admin "ssh-ed25519 AAAAC3Nza... admin@host"
}

// NTP servers
ntp-servers "pool.ntp.org" "time.google.com"

// Timezone
timezone "America/Los_Angeles"
```

## Boot-Time Integration

### Systemd Service

Create `/etc/systemd/system/sysconfig-provision.service`:

```ini
[Unit]
Description=System Provisioning
After=network-pre.target
Before=network.target
Wants=sysconfig.service
After=sysconfig.service

[Service]
Type=oneshot
ExecStart=/usr/local/bin/sysconfig-provision autodetect --check-network
RemainAfterExit=yes
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### SMF Service (illumos)

Create an SMF manifest for automatic provisioning on boot:

```xml
<?xml version="1.0"?>
<!DOCTYPE service_bundle SYSTEM "/usr/share/lib/xml/dtd/service_bundle.dtd.1">
<service_bundle type='manifest' name='sysconfig-provision'>
  <service name='system/sysconfig-provision' type='service' version='1'>
    <dependency name='sysconfig' type='service' grouping='require_all' restart_on='none'>
      <service_fmri value='svc:/system/sysconfig:default' />
    </dependency>
    
    <dependency name='filesystem' type='service' grouping='require_all' restart_on='none'>
      <service_fmri value='svc:/system/filesystem/local:default' />
    </dependency>
    
    <exec_method type='method' name='start' exec='/usr/local/bin/sysconfig-provision autodetect --check-network' timeout_seconds='60' />
    <exec_method type='method' name='stop' exec=':true' timeout_seconds='10' />
    
    <instance name='default' enabled='true' />
  </service>
</service_bundle>
```

## Workflow

### Boot-Time Provisioning

1. **Early Boot Detection**: The tool checks for local configuration files first
2. **Network Assessment**: Determines if network is required for cloud metadata
3. **Minimal Network Setup**: If needed, configures basic DHCP on primary interface
4. **Source Detection**: Identifies available configuration sources
5. **Configuration Fetch**: Retrieves configuration from all available sources
6. **Priority Merge**: Merges configurations based on priority
7. **State Application**: Writes final configuration to sysconfig
8. **Plugin Distribution**: Sysconfig distributes state to appropriate plugins

### Manual Provisioning

```bash
# Check what sources are available
sysconfig-provision detect --network

# Apply configuration from a specific KDL file
sysconfig-provision apply --config /path/to/config.kdl

# Apply from cloud sources only
sysconfig-provision apply --sources ec2,azure,gcp

# Dry run to see what would change
sysconfig-provision apply --config /path/to/config.kdl --dry-run
```

## Environment Variables

- `SYSCONFIG_SOCKET`: Path to sysconfig Unix socket (default: auto-detected)
- `RUST_LOG`: Logging level (e.g., `info`, `debug`, `trace`)
- `XDG_RUNTIME_DIR`: Runtime directory for non-root users

## Network Detection Logic

The tool uses several methods to determine if network setup is required:

1. **Local Configuration Check**: Looks for local config files that don't require network
2. **Cloud Environment Detection**: Checks DMI/SMBIOS for cloud vendor strings
3. **Metadata Service Probing**: Attempts to reach known metadata endpoints
4. **SmartOS Detection**: Checks for SmartOS-specific tools and local metadata

If network is required but not configured, the tool can:
- Apply a minimal DHCP configuration to the primary interface
- Wait for network to come up
- Then fetch cloud metadata

## Development

### Building from Source

```bash
git clone <repository>
cd sysconfig-provisioning
cargo build --release
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration test with sysconfig
./test_cli.sh
```

### Adding a New Source

To add support for a new configuration source:

1. Create a new module in `src/sources/`
2. Implement the source trait with `load()` and `is_available()` methods
3. Add the source to `SourceManager` in `src/sources.rs`
4. Define its priority in the `SourcePriority` enum
5. Update the documentation

## Troubleshooting

### Common Issues

**Tool can't connect to sysconfig**
- Ensure sysconfig service is running
- Check socket path with `SYSCONFIG_SOCKET` environment variable
- Verify permissions on the socket file

**Network sources not detected**
- Check network connectivity
- Verify metadata service endpoints are accessible
- Look for firewall rules blocking metadata services
- Check the `--network-timeout` parameter

**Configuration not being applied**
- Run with `--dry-run` to see what would be changed
- Check sysconfig logs for errors
- Verify the configuration syntax is valid
- Use `parse` command to validate KDL files

### Debug Logging

Enable detailed logging:
```bash
RUST_LOG=debug sysconfig-provision autodetect --check-network
```

## License

See LICENSE file in the repository root.