# Service Configuration Files

This directory contains service configuration files for running sysconfig, platform-specific base plugins, and the provisioning CLI across different init systems.

## Overview

The sysconfig system consists of three main components:

1. **sysconfig** - Core configuration management service
2. **Base Plugin** - Platform-specific configuration plugin (illumos/Linux/FreeBSD)
3. **Provisioning CLI** - One-time boot provisioning tool

## Service Dependencies

```
┌─────────────────────┐
│   Boot Process      │
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│   sysconfig         │  (Core Service - Always Running)
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│   Base Plugin       │  (Platform Plugin - Always Running)
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│   Provisioning      │  (CLI Tool - Runs Once at Boot)
└─────────────────────┘
```

## Installation by Platform

### illumos/SmartOS (SMF)

#### Installation Steps

1. Copy the binaries:
```bash
# Core service
sudo cp sysconfig/target/release/sysconfig /usr/lib/sysconfig/sysconfig

# Base plugin
sudo cp sysconfig-plugins/target/release/illumos-base-plugin \
     /usr/lib/sysconfig/plugins/illumos-base-plugin

# Provisioning CLI
sudo cp sysconfig-provisioning/target/release/provisioning-plugin \
     /usr/lib/sysconfig/sysconfig-provision
```

2. Install the SMF manifests:
```bash
# Import manifests
sudo svccfg import service-configs/smf/sysconfig.xml
sudo svccfg import service-configs/smf/sysconfig-illumos-base.xml
sudo svccfg import service-configs/smf/sysconfig-provision.xml
```

3. Enable the services:
```bash
# Enable core service and plugin
sudo svcadm enable sysconfig
sudo svcadm enable sysconfig/illumos-base

# Provisioning runs automatically on next boot, or manually:
sudo svcadm enable -t sysconfig/provision
```

#### Configuration

```bash
# View current configuration
svccfg -s sysconfig:default listprop config/

# Change socket path
sudo svccfg -s sysconfig:default setprop config/socket_path = astring: "/var/run/sysconfig.sock"

# Apply configuration changes
sudo svcadm refresh sysconfig:default
sudo svcadm restart sysconfig:default
```

#### Monitoring

```bash
# Check service status
svcs -xv sysconfig
svcs -xv sysconfig/illumos-base
svcs -xv sysconfig/provision

# View logs
tail -f /var/svc/log/system-sysconfig:default.log
tail -f /var/svc/log/system-sysconfig-illumos-base:default.log
tail -f /var/svc/log/system-sysconfig-provision:default.log
```

### Linux (systemd)

#### Installation Steps

1. Copy the binaries:
```bash
# Core service
sudo cp sysconfig/target/release/sysconfig /usr/lib/sysconfig/sysconfig

# Base plugin (use appropriate binary for your system)
sudo cp sysconfig-plugins/target/release/linux-base-plugin \
     /usr/lib/sysconfig/plugins/linux-base-plugin

# Provisioning CLI
sudo cp sysconfig-provisioning/target/release/provisioning-plugin \
     /usr/lib/sysconfig/sysconfig-provision
```

2. Install systemd units:
```bash
# Copy service files
sudo cp service-configs/systemd/*.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload
```

3. Enable and start services:
```bash
# Enable and start core service
sudo systemctl enable --now sysconfig.service

# Enable and start base plugin
sudo systemctl enable --now sysconfig-linux-base.service

# Enable provisioning (runs on next boot)
sudo systemctl enable sysconfig-provision.service

# Or run provisioning immediately
sudo systemctl start sysconfig-provision.service
```

#### Configuration

```bash
# Edit service configuration
sudo systemctl edit sysconfig.service

# Add environment overrides:
[Service]
Environment="RUST_LOG=debug"
Environment="CUSTOM_SOCKET=/run/custom-sysconfig.sock"

# Reload and restart
sudo systemctl daemon-reload
sudo systemctl restart sysconfig.service
```

#### Monitoring

```bash
# Check service status
systemctl status sysconfig.service
systemctl status sysconfig-linux-base.service
systemctl status sysconfig-provision.service

# View logs
journalctl -u sysconfig -f
journalctl -u sysconfig-linux-base -f
journalctl -u sysconfig-provision -f

# View provisioning output from last boot
journalctl -b -u sysconfig-provision
```

### FreeBSD (rc.d)

#### Installation Steps

1. Copy the binaries:
```bash
# Core service
sudo cp sysconfig/target/release/sysconfig /usr/local/lib/sysconfig/sysconfig

# Base plugin
sudo cp sysconfig-plugins/target/release/freebsd-base-plugin \
     /usr/local/lib/sysconfig/plugins/freebsd-base-plugin

# Provisioning CLI
sudo cp sysconfig-provisioning/target/release/provisioning-plugin \
     /usr/local/lib/sysconfig/sysconfig-provision
```

2. Install rc.d scripts:
```bash
# Copy service scripts
sudo cp service-configs/freebsd/sysconfig /usr/local/etc/rc.d/
sudo cp service-configs/freebsd/sysconfig-freebsd-base /usr/local/etc/rc.d/
sudo cp service-configs/freebsd/sysconfig-provision /usr/local/etc/rc.d/

# Make executable
sudo chmod +x /usr/local/etc/rc.d/sysconfig*
```

3. Configure in rc.conf:
```bash
# Add to /etc/rc.conf
cat >> /etc/rc.conf << EOF
# Sysconfig services
sysconfig_enable="YES"
sysconfig_freebsd_base_enable="YES"
sysconfig_provision_enable="YES"
EOF
```

4. Start services:
```bash
# Start core service and plugin
sudo service sysconfig start
sudo service sysconfig-freebsd-base start

# Run provisioning
sudo service sysconfig-provision start
```

#### Configuration

Edit `/etc/rc.conf`:
```bash
# Sysconfig core options
sysconfig_socket="/var/run/sysconfig.sock"
sysconfig_state_dir="/var/db/sysconfig"
sysconfig_log_level="debug"

# Base plugin options
sysconfig_freebsd_base_dry_run="NO"

# Provisioning options
sysconfig_provision_check_network="YES"
sysconfig_provision_network_timeout="30"
sysconfig_provision_sources="local,cloud-init,ec2"
```

#### Monitoring

```bash
# Check service status
service sysconfig status
service sysconfig-freebsd-base status
service sysconfig-provision status

# View logs
tail -f /var/log/sysconfig.log
tail -f /var/log/sysconfig_freebsd_base.log

# Run provisioning detection
service sysconfig-provision detect

# Parse configuration
service sysconfig-provision parse
```

## Boot Sequence

### Typical Boot Flow

1. **System Boot**: Basic filesystem and network stack initialization
2. **sysconfig starts**: Core service becomes available
3. **Base Plugin starts**: Connects to sysconfig, ready to apply configurations
4. **Provisioning runs**: 
   - Detects environment (cloud, local, etc.)
   - Checks if network setup is needed
   - Fetches configuration from available sources
   - Writes configuration to sysconfig
   - Base plugin applies the configuration
5. **Network services start**: Using the provisioned configuration
6. **System ready**: Fully configured and operational

### Network Bootstrap

If the provisioning CLI detects it's in a cloud environment but has no network:

1. Applies minimal DHCP configuration to primary interface
2. Waits for network to come up
3. Fetches cloud metadata
4. Applies full configuration

## Configuration Priority

When multiple sources are available, they are merged with the following priority (lowest number wins):

| Priority | Source | Description |
|----------|--------|-------------|
| 1 | Local KDL | `/etc/sysconfig.kdl` or specified file |
| 10 | Cloud-Init | Cloud-init metadata and userdata |
| 20 | EC2 | Amazon EC2 instance metadata |
| 21 | Azure | Microsoft Azure instance metadata |
| 22 | GCP | Google Cloud Platform metadata |
| 23 | DigitalOcean | DigitalOcean droplet metadata |
| 24 | OpenStack | OpenStack metadata service |
| 30 | SmartOS | SmartOS metadata service |

## Troubleshooting

### Common Issues

#### Services Won't Start

**Check socket paths exist:**
```bash
# Linux/FreeBSD
ls -la /var/run/sysconfig*

# illumos
ls -la /var/run/sysconfig*
```

**Check permissions:**
```bash
# Ensure run directory is writable
ls -ld /var/run
```

**Check if ports are in use:**
```bash
# Check for existing processes
ps aux | grep sysconfig
```

#### Provisioning Not Running at Boot

**Check if already provisioned:**
```bash
# FreeBSD
ls -la /var/db/.sysconfig_provisioned

# Force re-provisioning
rm /var/db/.sysconfig_provisioned
service sysconfig-provision start
```

**Check service dependencies:**
```bash
# systemd
systemctl list-dependencies sysconfig-provision

# SMF
svcs -d sysconfig/provision
```

#### Configuration Not Applied

**Check sysconfig state:**
```bash
# Using sysconfig-cli
sysconfig-cli status

# Check provisioning status
sysconfig-provision status
```

**Enable debug logging:**
```bash
# systemd
systemctl edit sysconfig-provision
# Add: Environment="RUST_LOG=debug,provisioning=trace"

# SMF
svccfg -s sysconfig/provision:default setprop method_context/environment = astring: "RUST_LOG=trace"

# FreeBSD
# Edit /etc/rc.conf
sysconfig_provision_log_level="trace"
```

### Manual Testing

#### Test Provisioning Without Applying

```bash
# Dry run - shows what would be configured
sysconfig-provision autodetect --dry-run

# Parse a specific config file
sysconfig-provision parse --config /etc/sysconfig.kdl

# Detect available sources
sysconfig-provision detect --network
```

#### Test Specific Sources

```bash
# Apply from local file only
sysconfig-provision apply --config /etc/sysconfig.kdl

# Apply from cloud sources only
sysconfig-provision apply --sources ec2,azure,gcp

# Disable specific sources
sysconfig-provision apply --disable-sources cloud-init,smartos
```

## Security Considerations

### File Permissions

Ensure proper permissions on configuration files:
```bash
# Config files should be readable by root only
chmod 600 /etc/sysconfig.kdl
chown root:root /etc/sysconfig.kdl

# Socket should be accessible by services
chmod 666 /var/run/sysconfig.sock
```

### Network Security

The provisioning service connects to cloud metadata services. Ensure firewall rules allow:
- `169.254.169.254` (AWS, Azure, others)
- `metadata.google.internal` (GCP)
- Local metadata services (SmartOS, OpenStack)

### Privilege Separation

- Core sysconfig service runs as root (required for system configuration)
- Consider running in a restricted environment where possible
- Use capability restrictions (Linux) or privileges (illumos) to limit access

## Additional Resources

- [Sysconfig Documentation](../sysconfig/README.md)
- [Provisioning CLI Documentation](../sysconfig-provisioning/README.md)
- [KDL Configuration Format](../sysconfig/docs/kdl-configuration.md)
- [Plugin Development Guide](../sysconfig/docs/plugin-development.md)