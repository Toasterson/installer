# illumos Installer Development Cloud Image Guide

This guide shows you how to use the development cloud image for rapid testing and development of the illumos installer's sysconfig ecosystem including the main daemon, plugins, and provisioning tools.

## Overview

The development cloud image allows you to:
- Mount your development repository directly into the VM via 9P filesystem
- Test changes to all sysconfig components without rebuilding the entire image
- Use proper illumos SMF services with correct dependencies for realistic testing
- Test plugin interactions and provisioning workflows
- Load KDL configurations from the mounted repository for testing
- Get fast feedback cycles (seconds instead of hours)

## Using mise (recommended)

mise is a unified toolchain manager and task runner we use to simplify common dev flows.

Quick setup:

```bash
# 1) Install mise (see TOOLING_MISE.md for options)
# 2) Trust the repo and install toolchains
mise trust
mise run tools:install

# 3) Start the dev VM (wraps Makefile)
mise run vm:up
# SSH or console
mise run vm:ssh
mise run vm:console
```

Common tasks:
- Rust: `mise run rust:build-all`, `mise run rust:test-all`, `mise run rust:fmt`, `mise run rust:clippy`
- UI: `mise run ui:install`, `mise run ui:dev`, `mise run ui:build`
- Dev image: `mise run dev:build-image -- -d rpool/images`, `mise run dev:test-setup`

See `TOOLING_MISE.md` for the full task catalog.

## Quick Start

### 1. Build the Development Image

```bash
# Build the development image (requires ZFS dataset)
mise run dev:build-image -- -d rpool/images
```

This creates `cloudimage-ttya-openindiana-hipster-dev.raw` in your dataset's output directory.

### 2. Start the VM

#### Using bhyve
```bash
mise run vm:bhyve:start -- \
  -i /path/to/cloudimage-ttya-openindiana-hipster-dev.raw \
  -r /path/to/illumos/installer
```

#### Using libvirt
```bash
# Edit examples/dev-vm-libvirt.xml with your paths, then:
virsh define examples/dev-vm-libvirt.xml
virsh start illumos-installer-dev
virsh console illumos-installer-dev
```

### 3. Develop and Test

```bash
# On host: Make code changes to any component
vim sysconfig/src/main.rs                    # Main daemon
vim sysconfig-plugins/src/bin/provisioning-plugin.rs  # Plugin
vim sysconfig-provisioning/src/main.rs       # Provisioning CLI

# On host: Rebuild the changed component
cd sysconfig && cargo build --release
# or
cd sysconfig-plugins && cargo build --release
# or
cd sysconfig-provisioning && cargo build --release

# In VM: Restart relevant services (via console)
svcadm restart svc:/system/installer/sysconfig        # Main daemon
svcadm restart svc:/system/sysconfig/provisioning     # Provisioning plugin
svcadm restart svc:/system/sysconfig/illumos-base     # Base plugin
svcadm restart svc:/system/sysconfig-provisioning     # Provisioning CLI

# In VM: Check logs
tail -f /var/svc/log/system-installer-sysconfig:default.log
tail -f /var/svc/log/system-sysconfig-provisioning:default.log
```

## What Happens During Boot

1. **VM Starts**: Normal illumos boot with ZFS root
2. **9P Mount**: Repository automatically mounted at `/repo` via `svc:/system/dev-9p-mount`
3. **Main Daemon**: `svc:/system/installer/sysconfig` starts with debug logging
4. **Plugin Services**: Individual SMF services start for each plugin:
   - `svc:/system/sysconfig/provisioning` (provisioning plugin)
   - `svc:/system/sysconfig/illumos-base` (base system plugin)
5. **Provisioning CLI**: `svc:/system/sysconfig-provisioning` runs oneshot provisioning
6. **Configuration Loading**: All components load KDL config from `/repo/sysconfig-plugins/test-provisioning-config.kdl`
7. **Ready for Development**: Make changes, rebuild, restart specific services

## Development Workflow

### Typical Development Cycle

```bash
# 1. Make changes to any component
vim sysconfig/src/main.rs                           # Main daemon
vim sysconfig-plugins/src/bin/provisioning-plugin.rs  # Plugins
vim sysconfig-provisioning/src/main.rs              # Provisioning CLI
vim sysconfig-plugins/test-provisioning-config.kdl  # Test configuration

# 2. Build the changed component (takes ~30 seconds)
cd sysconfig && cargo build --release
# or
cd sysconfig-plugins && cargo build --release
# or
cd sysconfig-provisioning && cargo build --release

# 3. Test in VM (restart relevant services)
# In VM console:
svcadm restart svc:/system/installer/sysconfig        # Main daemon
svcadm restart svc:/system/sysconfig/provisioning     # Provisioning plugin
svcadm restart svc:/system/sysconfig-provisioning     # Provisioning CLI

# 4. Verify (check service status and logs)
svcs -l svc:/system/installer/sysconfig
svcs -l svc:/system/sysconfig/provisioning
tail -f /var/svc/log/system-installer-sysconfig:default.log
tail -f /var/svc/log/system-sysconfig-provisioning:default.log
```

### Adding New Plugins

```bash
# 1. Create new plugin binary in sysconfig-plugins
vim sysconfig-plugins/src/bin/my-plugin.rs

# 2. Add binary entry to Cargo.toml
vim sysconfig-plugins/Cargo.toml
# Add: [[bin]]
#      name = "my-plugin"
#      path = "src/bin/my-plugin.rs"

# 3. Build
cd sysconfig-plugins && cargo build --release

# 4. Create SMF manifest (in development template)
# Add new plugin service in sysconfig-dev.json

# 5. Test in VM
# Plugin is available at /repo/sysconfig-plugins/target/release/my-plugin
# Create and enable SMF service for the plugin
```

### Testing Network Configuration

```bash
# In VM: Test DHCP configuration
dhcpinfo
ipadm show-addr

# Test static IP configuration
# (Make changes to network plugin, rebuild, restart)

# Test DNS resolution
nslookup google.com
```

## Troubleshooting

### 9P Mount Issues

```bash
# In VM: Check if 9P is mounted
mount | grep 9pfs
# Should show: repo on /repo type 9pfs ...

# Check 9P mount service
svcs -l svc:/system/dev-9p-mount

# Manual mount test
umount /repo
mount -F 9pfs -o tag=repo,trans=virtio,version=9p2000.L repo /repo
```

### Sysconfig Service Issues

```bash
# Check all sysconfig service statuses
svcs -xv svc:/system/installer/sysconfig
svcs -xv svc:/system/sysconfig/provisioning
svcs -xv svc:/system/sysconfig/illumos-base
svcs -xv svc:/system/sysconfig-provisioning

# View detailed logs for each service
cat /var/svc/log/system-installer-sysconfig:default.log
cat /var/svc/log/system-sysconfig-provisioning:default.log

# Manual service tests
/usr/lib/sysconfig/sysconfig --help
/usr/lib/sysconfig/plugins/provisioning-plugin --help
/usr/lib/sysconfig-provisioning --help

# Check configurations
cat /etc/sysconfig.toml
cat /repo/sysconfig-plugins/test-provisioning-config.kdl
```

### Build Issues

```bash
# On host: Check if all binaries exist
ls -la sysconfig/target/release/sysconfig
ls -la sysconfig-plugins/target/release/provisioning-plugin
ls -la sysconfig-plugins/target/release/illumos-base-plugin
ls -la sysconfig-provisioning/target/release/sysconfig-provisioning

# Check Rust toolchain
cargo --version
rustc --version

# Clean build all components
cd sysconfig && cargo clean && cargo build --release
cd ../sysconfig-plugins && cargo clean && cargo build --release
cd ../sysconfig-provisioning && cargo clean && cargo build --release
```

### VM Access Issues

```bash
# For bhyve: Connect to console
# Console is usually attached to stdio

# For libvirt: Connect to console
virsh console illumos-installer-dev

# For VNC access (if configured):
vncviewer localhost:5900
```

## Tips and Best Practices

### Performance

- **9P Performance**: Good for source code, slower for large binaries
- **Memory**: Use at least 2GB RAM for comfortable development
- **Storage**: VM uses ~2GB, builds need ~500MB additional space

### Development

- **Incremental Builds**: Use `cargo build` instead of `cargo build --release` during development
- **Logging**: Enable debug logging in `/etc/sysconfig.toml`
- **Testing**: Use VM snapshots before major changes

### Debugging

- **Console Output**: Services log to both SMF logs and console
- **System Logs**: Check `/var/adm/messages` for system-level issues
- **Service Dependencies**: Use `svcs -d` to check service dependencies

## File Locations in VM

| Path | Description |
|------|-------------|
| `/repo` | Mounted repository (your host development directory) |
| `/usr/lib/sysconfig/sysconfig` | Main sysconfig daemon |
| `/usr/lib/sysconfig/plugins/` | Plugin binaries directory |
| `/usr/lib/sysconfig-provisioning` | Provisioning CLI binary |
| `/etc/sysconfig.toml` | Main daemon configuration |
| `/etc/sysconfig/dev-test.kdl` | Development test configuration |
| `/repo/sysconfig-plugins/test-provisioning-config.kdl` | Loaded test configuration |
| `/lib/svc/manifest/system/sysconfig*.xml` | SMF manifests for all services |
| `/var/svc/log/system-installer-sysconfig:default.log` | Main daemon log |
| `/var/svc/log/system-sysconfig-*:default.log` | Plugin service logs |
| `/var/run/sysconfig/

## Advanced Usage

### Custom Plugin Development

1. Create plugin binary in `sysconfig-plugins/src/bin/my-plugin.rs`
2. Add binary entry to `sysconfig-plugins/Cargo.toml`
3. Implement plugin trait and gRPC server
4. Create SMF manifest for the plugin service
5. Build and test in VM: `cd sysconfig-plugins && cargo build --release`
6. Plugin configuration goes in KDL config files
7. Test with: `svcadm enable svc:/system/sysconfig/my-plugin`

### Network Testing

The VM includes network drivers and DHCP client:
- Test network configuration plugins
- Verify DNS resolution
- Test static IP assignment

### Storage Testing

- VM boots from ZFS
- Test ZFS-related configuration
- Verify filesystem mounting

## Getting Help

1. **Logs**: Always check SMF logs first
2. **Service Status**: Use `svcs -xv` to diagnose service issues
3. **Documentation**: See `DEV_CLOUD_IMAGE.md` for comprehensive details
4. **Validation**: Run `mise run dev:test-setup` to check your setup

## Example Session

```bash
# Terminal 1 (Host): Start development
cd illumos/installer
mise run dev:build-image -- -d rpool/images

# Terminal 2 (Host): Start VM
mise run vm:bhyve:start -- -i /images/dev.raw -r $PWD

# VM Console: Check services after boot
svcs svc:/system/installer/sysconfig
mount | grep repo

# Terminal 1 (Host): Make changes and build
vim sysconfig/src/main.rs
cd sysconfig && cargo build --release

# VM Console: Test changes
svcadm restart svc:/system/installer/sysconfig
tail -f /var/svc/log/system-installer-sysconfig:default.log
```

This development environment transforms your workflow from hours-long image rebuild cycles to seconds-long service restart cycles, making illumos installer development much more efficient and enjoyable.


## AI-authored documentation location

To keep the docs organized, any AI-authored guides or status reports that are not explicitly requested as permanent top-level docs must be placed under `docs/ai/` and named with a `YYYY-MM-DD-` date prefix. See `docs/DOCUMENTATION_GUIDELINES.md` for details.
