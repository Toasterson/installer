# Troubleshooting

This page provides guidance for troubleshooting common issues with the Illumos Installer configuration.

## Machine Configuration Issues

### Invalid Configuration Format

**Issue**: The Machine Configuration file has syntax errors or invalid format.

**Symptoms**:
- Error messages about invalid syntax
- Failure to parse the configuration file

**Solutions**:
1. Verify that the KDL syntax is correct
2. Check for missing or mismatched braces `{}`
3. Ensure that all strings are properly quoted
4. Validate the configuration file using a KDL validator

### Missing Required Elements

**Issue**: The Machine Configuration is missing required elements.

**Symptoms**:
- Error messages about missing required elements
- Failure to apply the configuration

**Solutions**:
1. Ensure that the configuration includes a `pool` element
2. Ensure that the configuration includes an `image` element
3. Check that all required child elements are present

### Invalid Disk Identifiers

**Issue**: The disk identifiers specified in the configuration are invalid or not found.

**Symptoms**:
- Error messages about invalid or not found disks
- Failure to create ZFS pools

**Solutions**:
1. Verify that the disk identifiers are correct
2. Use `format` or `diskinfo` commands to list available disks
3. Ensure that the disks are accessible to the installer

### Invalid Image URL

**Issue**: The image URL specified in the configuration is invalid or not accessible.

**Symptoms**:
- Error messages about invalid or inaccessible image
- Failure to download or access the image

**Solutions**:
1. Verify that the image URL is correct
2. Ensure that the image repository is accessible
3. Check for network connectivity issues
4. Verify authentication credentials if required

## System Configuration Issues

### Invalid Interface Configuration

**Issue**: The network interface configuration is invalid or not applicable.

**Symptoms**:
- Error messages about invalid interface configuration
- Network interfaces not configured correctly

**Solutions**:
1. Verify that the interface names or selectors are correct
2. Check that the address configuration is valid
3. Ensure that the specified interfaces exist on the system
4. Use `dladm show-phys` and `dladm show-link` to list available interfaces

### DNS Resolution Issues

**Issue**: DNS resolution is not working correctly.

**Symptoms**:
- Unable to resolve domain names
- Network connectivity issues

**Solutions**:
1. Verify that nameservers are correctly configured
2. Check that the nameservers are accessible
3. Test DNS resolution using `nslookup` or `dig`
4. Check the `/etc/resolv.conf` file for correct configuration

### Static IP Configuration Issues

**Issue**: Static IP configuration is not working correctly.

**Symptoms**:
- Network connectivity issues
- Unable to reach default gateway
- IP address conflicts

**Solutions**:
1. Verify that the IP address, subnet mask, and gateway are correct
2. Ensure that the IP address is not already in use on the network
3. Check that the gateway is accessible
4. Test network connectivity using `ping` and `traceroute`

## Plugin Issues

### Plugin Registration Failure

**Issue**: Plugins fail to register with the SysConfig service.

**Symptoms**:
- Error messages about plugin registration failure
- Missing functionality in the system configuration

**Solutions**:
1. Verify that the plugin binary is accessible
2. Check that the plugin has the correct permissions
3. Ensure that the SysConfig service is running
4. Check the plugin logs for error messages

### Plugin Configuration Issues

**Issue**: Plugins are not correctly configured.

**Symptoms**:
- Error messages about plugin configuration
- Unexpected behavior in the system configuration

**Solutions**:
1. Verify that the plugin configuration is correct
2. Check the plugin logs for error messages
3. Ensure that the plugin dependencies are satisfied
4. Test the plugin configuration in isolation

## General Troubleshooting Tips

### Check Logs

The installer and its components generate logs that can help diagnose issues:

- Check the installer logs for error messages
- Check the SysConfig service logs for configuration issues
- Check the plugin logs for plugin-specific issues

### Validate Configuration

Before applying a configuration, validate it to ensure it's correct:

- Use configuration validators if available
- Test the configuration in a non-production environment
- Start with a minimal configuration and add complexity incrementally

### Network Diagnostics

For network-related issues, use these diagnostic tools:

- `ping`: Test basic network connectivity
- `traceroute`: Trace the route to a destination
- `dladm`: Show physical and virtual network interfaces
- `ipadm`: Show IP addresses and network configuration
- `netstat`: Show network statistics and connections

### Storage Diagnostics

For storage-related issues, use these diagnostic tools:

- `format`: Show disk information and partitions
- `diskinfo`: Show detailed disk information
- `zpool status`: Show ZFS pool status
- `zfs list`: Show ZFS filesystems and properties

## Getting Help

If you're unable to resolve an issue using this troubleshooting guide, consider these resources:

- Check the [Illumos Installer repository](https://github.com/toasterson/installer) for known issues
- Search for similar issues in the issue tracker
- Ask for help in the Illumos community forums or mailing lists
- File a bug report if you believe you've found a bug