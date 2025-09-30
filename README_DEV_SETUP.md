# Development Cloud Image Setup

This directory contains a development cloud image template that enables rapid testing of the illumos installer's sysconfig components without rebuilding the entire image for each change.

## What This Provides

- **Live Code Testing**: Mount the host repository via 9P filesystem into the VM
- **Complete Architecture Testing**: Main daemon, plugins, and provisioning CLI as separate SMF services
- **Fast Development Cycle**: Make changes on host, rebuild, restart specific services in VM
- **KDL Configuration Loading**: Test configs loaded directly from mounted repository
- **Verifiable File Organization**: All configuration files as separate, editable files

## Quick Start

```bash
# Build the development image (requires ZFS dataset)
./dev-build.sh -d rpool/images

# Boot with 9P sharing (example for bhyve)
bhyve -c 2 -m 2048M -w -H \
  -s 0,hostbridge \
  -s 1,lpc \
  -s 2,ahci-hd,/path/to/image.raw \
  -s 3,virtio-9p,repo=/path/to/repo \
  -l com1,stdio \
  -l bootrom,/usr/share/bhyve/BHYVE_UEFI.fd \
  dev-vm
```

## Files Created

### Templates
- `machined/image/templates/cloudimage/ttya-openindiana-hipster-dev.json` - Main development image template
- `machined/image/templates/include/sysconfig-dev.json` - Development-specific configurations
- `machined/image/templates/files/default_init.utc` - UTC timezone configuration

### Configuration Files
- `machined/image/templates/files/sysconfig-dev/` - Directory containing all dev configuration files:
  - `dev-9p-mount` - 9P filesystem mount script (executable)
  - `dev-9p-mount.xml` - SMF manifest for 9P mount service
  - `sysconfig-illumos-base-plugin.xml` - SMF manifest for illumos base plugin
  - `sysconfig-provisioning.xml` - SMF manifest for provisioning CLI
  - `sysconfig.toml` - Main sysconfig daemon configuration
  - `dev-test.kdl` - Development KDL test configuration
  - `vfstab` - System mount table with 9P entry

### Scripts & Documentation
- `dev-build.sh` - Automated build script for all sysconfig components
- `test-dev-setup.sh` - Comprehensive validation script (34 tests)
- `DEV_CLOUD_IMAGE.md` - Comprehensive usage and troubleshooting guide
- `IMPROVED_FILE_ORGANIZATION.md` - Details on file organization improvements
- `README_DEV_SETUP.md` - This summary file

## Development Workflow

1. **Build**: `./dev-build.sh -d <dataset>` - Builds all components (sysconfig, sysconfig-plugins, sysconfig-provisioning)
2. **Boot VM**: With 9P filesystem sharing the repo directory at `/repo`
3. **Code**: Edit any sysconfig component code on the host
4. **Test**: Build specific component, then restart corresponding service in VM:
   - Main daemon: `svcadm restart svc:/system/installer/sysconfig`
   - Plugins: `svcadm restart svc:/system/sysconfig/illumos-base`
   - Provisioning: `svcadm restart svc:/system/sysconfig-provisioning`
5. **Debug**: View service-specific logs and test functionality

## Key Features

- **9P Mount**: Repository automatically mounted at `/repo` in VM via SMF service
- **Complete SMF Architecture**: 
  - `svc:/system/dev-9p-mount` - Mounts 9P filesystem
  - `svc:/system/installer/sysconfig` - Main daemon
  - `svc:/system/sysconfig/illumos-base` - illumos base plugin
  - `svc:/system/sysconfig-provisioning` - Provisioning CLI service
- **KDL Config Loading**: Test configurations loaded from `/repo/sysconfig-plugins/test-provisioning-*.kdl`
- **Verifiable Files**: All SMF manifests, scripts, and configs as separate readable files
- **Comprehensive Testing**: 34 validation tests ensure all components work correctly
- **Console Access**: Serial console configured for easy VM interaction

## File Organization Benefits

- **Readable Templates**: No more unreadable inline JSON content
- **Independent Validation**: Each file can be validated with proper tools
- **Better Version Control**: Individual files show meaningful diffs
- **Easier Maintenance**: Edit configuration files with appropriate syntax highlighting

See `DEV_CLOUD_IMAGE.md` for detailed documentation and `IMPROVED_FILE_ORGANIZATION.md` for technical details on file structure improvements.