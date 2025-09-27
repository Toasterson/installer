# Dry-Run Testing Mode for illumos-base-plugin

## Overview

The illumos-base-plugin now includes an automatic dry-run testing mode that activates when the plugin is run as a non-root user. This feature allows safe testing of system configuration changes without actually modifying the system.

## Features

### Automatic Detection
- **Root Detection**: The plugin automatically detects if it's running as root (UID 0)
- **Auto Dry-Run**: When running as non-root, dry-run mode is automatically enabled
- **XDG_RUNTIME_DIR Support**: Uses standard user runtime directory for socket paths when non-root

### Enhanced Logging
- Detailed logging of all operations that would be performed
- Clear "DRY-RUN" prefixed messages for simulated operations
- Shows both current and proposed values for changes
- File content previews (truncated for large files)

### Safe Testing
- No actual system changes when in dry-run mode
- All file writes, permission changes, and system calls are simulated
- Perfect for CI/CD testing and development

## Socket Path Behavior

### As Root (Production)
```
/var/run/sysconfig.sock          # Sysconfig service
/var/run/sysconfig-illumos-base.sock  # Plugin socket
```

### As Non-Root (Testing)
```
$XDG_RUNTIME_DIR/sysconfig.sock          # Sysconfig service
$XDG_RUNTIME_DIR/sysconfig-illumos-base.sock  # Plugin socket
```

If `XDG_RUNTIME_DIR` is not set, falls back to `/tmp/run-$UID`.

## Test Scripts

### 1. `test_dry_run.sh`
Simple script to start just the plugin in dry-run mode.

```bash
./test_dry_run.sh
```

Features:
- Starts only the illumos-base-plugin
- Shows dry-run activation messages
- Good for testing plugin initialization

### 2. `test_dry_run_e2e.sh` (Recommended)
Complete end-to-end testing with both sysconfig and plugin.

```bash
./test_dry_run_e2e.sh
```

Features:
- Starts sysconfig service
- Starts illumos-base-plugin with auto-registration
- Sets up proper socket paths
- Provides instructions for testing with cloud-init
- Color-coded output for easy reading
- Automatic cleanup on exit

### 3. `test_apply_state.sh`
Helper script to apply test configurations.

```bash
# Use default test configuration
./test_apply_state.sh

# Use custom configuration file
./test_apply_state.sh my-config.json
```

## Example Test Configuration

### Network Settings and Files
```json
{
  "network": {
    "settings": {
      "hostname": "test-illumos-host",
      "dns": {
        "nameservers": ["8.8.8.8", "1.1.1.1"],
        "search": ["example.com", "test.local"]
      }
    }
  },
  "files": [
    {
      "path": "/etc/test-config.conf",
      "ensure": "present",
      "content": "# Test configuration\ntest_value=123\n",
      "mode": "0644",
      "uid": 0,
      "gid": 0
    },
    {
      "path": "/tmp/test-file",
      "ensure": "absent"
    }
  ]
}
```

## Testing with cloud-init-plugin

1. Start the test environment:
```bash
./test_dry_run_e2e.sh
```

2. In another terminal, run cloud-init-plugin:
```bash
export XDG_RUNTIME_DIR=/tmp/run-$(id -u)  # Or your runtime dir
export RUST_LOG=info

./target/debug/cloud-init-plugin \
  --service-socket $XDG_RUNTIME_DIR/sysconfig.sock \
  --config /path/to/cloud-init-config.yaml
```

## Example Dry-Run Output

When operations are simulated, you'll see output like:

```
[plugin] INFO DRY-RUN: Checking hostname configuration...
[plugin] INFO DRY-RUN: Would write hostname 'test-host' to /etc/nodename
[plugin] INFO DRY-RUN:   Current content: "old-hostname"
[plugin] INFO DRY-RUN:   New content: "test-host"
[plugin] INFO DRY-RUN: Would set runtime hostname from 'old-hostname' to 'test-host'
[plugin] INFO DRY-RUN: Would update /etc/resolv.conf with:
[plugin] INFO DRY-RUN:   Nameservers: ["8.8.8.8", "1.1.1.1"]
[plugin] INFO DRY-RUN:   Search domains: ["example.com"]
[plugin] INFO DRY-RUN: Would create file: /etc/test-config.conf
[plugin] INFO DRY-RUN:   Content: "# Test configuration\n..."
[plugin] INFO DRY-RUN: Would change permissions for /etc/test-config.conf from 755 to 644
```

## Development Workflow

### 1. Initial Setup
```bash
# Build everything
cd sysconfig-plugins
cargo build --bin illumos-base-plugin
cd ../sysconfig
cargo build --bin sysconfig
cd ../sysconfig-cli
cargo build
```

### 2. Start Test Environment
```bash
cd sysconfig-plugins
./test_dry_run_e2e.sh
```

### 3. Test Changes
```bash
# In another terminal
./test_apply_state.sh my-test-config.json
```

### 4. Check Logs
Watch the output in the terminal running `test_dry_run_e2e.sh` for:
- DRY-RUN prefixed messages
- Detailed operation logs
- Error messages

## Environment Variables

### Required for Non-Root Testing
- `XDG_RUNTIME_DIR`: User runtime directory (auto-detected or defaults to `/tmp/run-$UID`)

### Optional
- `RUST_LOG`: Log level (e.g., `info`, `debug`, `trace`)
- `SYSCONFIG_SOCKET`: Override default socket path

## Safety Features

1. **Automatic Detection**: No need to specify --dry-run flag when running as non-root
2. **Clear Warnings**: Prominent warnings when dry-run mode is active
3. **No Privilege Escalation**: Cannot accidentally run privileged operations
4. **Verbose Logging**: Every simulated operation is logged
5. **Rollback Safety**: Since no changes are made, there's nothing to roll back

## Troubleshooting

### Socket Not Found
```bash
# Check if XDG_RUNTIME_DIR is set
echo $XDG_RUNTIME_DIR

# Set it if not
export XDG_RUNTIME_DIR=/tmp/run-$(id -u)
mkdir -p $XDG_RUNTIME_DIR
```

### Permission Denied
```bash
# Ensure runtime directory has correct permissions
chmod 700 $XDG_RUNTIME_DIR
```

### Plugin Won't Start
```bash
# Check for stale sockets
rm -f $XDG_RUNTIME_DIR/sysconfig*.sock

# Check if ports are in use
lsof -U | grep sysconfig
```

## Best Practices

1. **Always test as non-root first** to ensure changes are correct
2. **Use meaningful test configurations** that match your production scenarios
3. **Check both the sysconfig and plugin logs** for complete information
4. **Save test configurations** for regression testing
5. **Use version control** for test configurations

## Integration with CI/CD

The dry-run mode is perfect for CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Test sysconfig changes
  run: |
    export XDG_RUNTIME_DIR=${{ runner.temp }}/runtime
    mkdir -p $XDG_RUNTIME_DIR
    ./test_dry_run_e2e.sh &
    sleep 5
    ./test_apply_state.sh ci-test-config.json
```

## Future Enhancements

- [ ] JSON diff output for automated testing
- [ ] Snapshot comparison for regression testing
- [ ] Mock filesystem for more detailed file operation testing
- [ ] Performance metrics in dry-run mode
- [ ] Integration with test frameworks