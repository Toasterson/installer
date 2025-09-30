# Development Cloud Image for illumos Installer

This document describes how to build and use a development cloud image that allows rapid testing of the illumos installer's sysconfig components without rebuilding the entire image for each change.

## Overview

The development cloud image provides:

1. **9P Filesystem Support**: Mounts the host's repository directory inside the VM
2. **Live Sysconfig Testing**: Loads sysconfig plugins directly from the mounted repository
3. **SMF Integration**: Runs sysconfig as a proper illumos SMF service
4. **Development Configuration**: Debug logging and development-friendly settings

## Quick Start

### Prerequisites

1. **ZFS Dataset**: A ZFS dataset for the image builder workspace
2. **Rust Toolchain**: Required for building sysconfig components
3. **OpenIndiana Base**: The hipster tarball (built from openindiana templates)
4. **Virtualization Platform**: bhyve or libvirt with 9P support

### Building the Development Image

```bash
# Clone and navigate to the repository
git clone <repository-url>
cd illumos/installer

# Build the development image
./dev-build.sh -d rpool/images
```

This script will:
1. Build the image-builder if needed
2. Build the sysconfig binary
3. Check for required OpenIndiana tarballs
4. Create the development cloud image

## Detailed Build Process

### Step 1: Build Prerequisites

If you haven't built the OpenIndiana base images yet:

```bash
# Build the base OpenIndiana images (this takes a while)
./image-builder/target/release/image-builder build -d rpool/images -g openindiana -n hipster-01-strap
./image-builder/target/release/image-builder build -d rpool/images -g openindiana -n hipster-02-image  
./image-builder/target/release/image-builder build -d rpool/images -g openindiana -n hipster-03-archive
```

### Step 2: Build Development Image

```bash
# Build the development cloud image
./dev-build.sh -d rpool/images -o /export/images
```

Options:
- `-d, --dataset`: ZFS dataset for image builder workspace (required)
- `-o, --output-dir`: Output directory for final images (optional)
- `-h, --help`: Show help message

## Using the Development Image

### VM Configuration

The development image requires 9P filesystem support. Here are examples for different virtualization platforms:

#### bhyve

```bash
# Example bhyve command with 9P support
bhyve -c 2 -m 2048M -w -H \
  -s 0,hostbridge \
  -s 1,lpc \
  -s 2,ahci-hd,/path/to/cloudimage-ttya-openindiana-hipster-dev.raw \
  -s 3,virtio-9p,repo=/path/to/illumos/installer \
  -l com1,stdio \
  -l bootrom,/usr/share/bhyve/BHYVE_UEFI.fd \
  dev-vm
```

#### libvirt/QEMU

```xml
<domain type='kvm'>
  <name>illumos-dev</name>
  <memory unit='KiB'>2097152</memory>
  <vcpu placement='static'>2</vcpu>
  <os>
    <type arch='x86_64' machine='pc'>hvm</type>
    <loader readonly='yes' type='pflash'>/usr/share/OVMF/OVMF_CODE.fd</loader>
  </os>
  <devices>
    <disk type='file' device='disk'>
      <source file='/path/to/cloudimage-ttya-openindiana-hipster-dev.raw'/>
      <target dev='vda' bus='virtio'/>
    </disk>
    <filesystem type='mount' accessmode='passthrough'>
      <source dir='/path/to/illumos/installer'/>
      <target dir='repo'/>
    </filesystem>
    <serial type='pty'>
      <target port='0'/>
    </serial>
    <console type='pty'>
      <target type='serial' port='0'/>
    </console>
  </devices>
</domain>
```

### Development Workflow

1. **Boot the VM**: Start the VM with 9P filesystem sharing enabled
2. **Automatic Setup**: The VM will automatically:
   - Mount the repository at `/repo` via 9P
   - Start the sysconfig service
   - Load plugins from `/repo/sysconfig/target/release`

3. **Make Code Changes**: Edit sysconfig code on the host machine

4. **Test Changes**: 
   ```bash
   # On the host: rebuild sysconfig
   cd sysconfig
   cargo build --release
   
   # In the VM: restart the service
   svcadm restart svc:/system/installer/sysconfig
   
   # Check service status
   svcs -l svc:/system/installer/sysconfig
   
   # View service logs
   tail -f /var/svc/log/system-installer-sysconfig:default.log
   ```

5. **Debug**: View detailed logs and debug information:
   ```bash
   # In the VM: check 9P mount
   mount | grep 9pfs
   ls -la /repo
   
   # Check sysconfig configuration
   cat /etc/sysconfig.toml
   
   # Monitor system logs
   tail -f /var/adm/messages
   ```

## Image Configuration

The development image includes these key configurations:

### 9P Filesystem Setup

- **Mount Point**: `/repo`
- **Filesystem Type**: `9pfs`
- **Options**: `tag=repo,trans=virtio,version=9p2000.L`
- **Auto-mount**: Configured in `/etc/vfstab`

### SMF Services

1. **dev-9p-mount** (`svc:/system/dev-9p-mount`):
   - Mounts the 9P filesystem
   - Dependency for sysconfig service
   - Runs early in boot process

2. **sysconfig** (`svc:/system/installer/sysconfig`):
   - Main sysconfig daemon
   - Depends on 9P mount service
   - Loads plugins from mounted repository

### Development Configuration

**`/etc/sysconfig.toml`**:
```toml
[plugins]
path = "/repo/sysconfig/target/release"

[logging]
level = "debug"
output = "console"
```

## Troubleshooting

### Common Issues

**9P Mount Fails**:
```bash
# Check if 9P driver is loaded
modinfo | grep 9p

# Check 9P mount service
svcs -l svc:/system/dev-9p-mount

# Manual mount test
mount -F 9pfs -o tag=repo,trans=virtio,version=9p2000.L repo /mnt
```

**Sysconfig Service Issues**:
```bash
# Check service dependencies
svcs -d svc:/system/installer/sysconfig

# View service log
cat /var/svc/log/system-installer-sysconfig:default.log

# Manual service start
/usr/lib/sysconfig
```

**Build Issues**:
```bash
# Check required packages
pkg list | grep vio9p

# Verify external source files
ls -la /repo/sysconfig/target/release/sysconfig
ls -la /repo/sysconfig/image/templates/files/
```

### Debug Commands

```bash
# System information
uname -a
prtconf | grep -i memory

# Network status
dladm show-phys
ipadm show-addr

# ZFS status
zfs list
zpool status

# SMF status
svcs -xv
```

## Template Details

The development image is built using:

- **Base Template**: `cloudimage/ttya-openindiana-hipster-dev.json`
- **Include Template**: `include/sysconfig-dev.json`
- **External Sources**: Sysconfig binary and configuration files

### Key Template Steps

1. Create ZFS boot environment
2. Unpack OpenIndiana hipster base
3. Install development tools and drivers
4. Configure 9P filesystem mounting
5. Install sysconfig binary and SMF manifests
6. Set up development configuration
7. Finalize and seed SMF repository

## Advanced Usage

### Custom Plugin Development

1. Create new plugins in the sysconfig source tree
2. Build with `cargo build --release`
3. Restart sysconfig service in VM
4. Test plugin functionality

### Network Configuration Testing

The image includes network configuration plugins for testing:
- DHCP client configuration
- Static IP configuration
- DNS resolver setup
- Hostname management

### Storage Configuration Testing

Test storage-related functionality:
- ZFS pool management
- Filesystem mounting
- Swap configuration

## Performance Considerations

- **9P Performance**: 9P filesystem can be slower than native storage for large file operations
- **Memory Usage**: Development image uses more memory due to debug logging
- **Boot Time**: Additional services increase boot time slightly

## Security Notes

- **Development Only**: This image is intended for development and testing only
- **Root Access**: Default root password is set for easy access
- **Network Security**: No firewall configured by default
- **File Permissions**: 9P mounted files inherit host permissions

## Contributing

When modifying the development image:

1. Test changes in isolated VM environment
2. Verify 9P mounting works correctly
3. Ensure SMF services start properly
4. Update documentation for any configuration changes
5. Test the complete development workflow

For questions or issues, consult the main project documentation or open an issue in the project repository.