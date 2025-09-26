# Sysconfig CLI Quick Start Guide

## Installation

```bash
# Build from source
cd sysconfig-cli
cargo build --release

# Install system-wide (requires sudo)
sudo make install

# Or install to user directory
INSTALL_DIR=~/.local/bin make install
```

## Automatic Socket Detection

The CLI automatically detects the correct socket path:
- **Root**: `/var/run/sysconfig.sock`
- **User**: `$XDG_RUNTIME_DIR/sysconfig.sock` or `/run/user/$UID/sysconfig.sock`

Check detected configuration:
```bash
sysconfig-cli info
```

## Essential Commands

### View State

```bash
# Get entire state
sysconfig-cli get

# Get specific path
sysconfig-cli get --path /network

# Output as JSON
sysconfig-cli get --format json
```

### Modify State

```bash
# Set simple value
sysconfig-cli set '$.network.hostname' '"myhost"'

# Set number
sysconfig-cli set '$.network.mtu' '1500'

# Set boolean
sysconfig-cli set '$.services.ssh.enabled' 'true'

# Set complex object
sysconfig-cli set '$.network.interfaces.eth0' '{
  "ip": "192.168.1.100",
  "netmask": "255.255.255.0",
  "gateway": "192.168.1.1"
}'

# Preview changes (dry run)
sysconfig-cli set '$.test.value' '"example"' --dry-run
```

### Apply Configuration

```bash
# From file
sysconfig-cli apply --file config.json

# From stdin
cat config.json | sysconfig-cli apply --stdin

# Dry run
sysconfig-cli apply --file config.json --dry-run

# Verbose output
sysconfig-cli apply --file config.json --verbose
```

### Monitor Changes

```bash
# Watch all changes
sysconfig-cli watch

# Watch specific path
sysconfig-cli watch --path /network

# JSON output for scripting
sysconfig-cli watch --format json
```

### Compare States

```bash
# Compare with file
sysconfig-cli diff --file desired-state.json

# From stdin
cat desired-state.json | sysconfig-cli diff --stdin
```

## Common Use Cases

### Testing Plugin Reactions

```bash
# Terminal 1: Monitor changes
sysconfig-cli watch

# Terminal 2: Make changes
sysconfig-cli set '$.plugins.test.trigger' 'true'
sysconfig-cli set '$.plugins.test.value' '42'
```

### Bulk Configuration Update

```bash
# Preview changes
sysconfig-cli apply --file new-config.json --dry-run

# Apply if everything looks good
sysconfig-cli apply --file new-config.json
```

### Network Configuration

```bash
# Set hostname
sysconfig-cli set '$.network.hostname' '"prod-server-01"'

# Configure interface
sysconfig-cli set '$.network.interfaces.eth0.ip' '"10.0.0.100"'
sysconfig-cli set '$.network.interfaces.eth0.gateway' '"10.0.0.1"'

# Set DNS servers
sysconfig-cli set '$.network.dns.nameservers' '["8.8.8.8", "8.8.4.4"]'
```

### Service Configuration

```bash
# Enable SSH
sysconfig-cli set '$.services.ssh.enabled' 'true'
sysconfig-cli set '$.services.ssh.port' '22'

# Configure firewall
sysconfig-cli set '$.services.firewall.enabled' 'true'
sysconfig-cli set '$.services.firewall.defaultPolicy' '"deny"'
```

## JSONPath Examples

```bash
# Top-level field
$.field

# Nested field
$.parent.child

# Deep nesting
$.level1.level2.level3

# Array element (basic)
$.items[0]

# Complex path
$.network.interfaces.eth0.ip
```

## Tips

### Value Format
- **Strings**: Must be quoted twice: `'"value"'`
- **Numbers**: Direct: `'42'` or `'3.14'`
- **Booleans**: Direct: `'true'` or `'false'`
- **Arrays**: JSON format: `'["a", "b", "c"]'`
- **Objects**: JSON format: `'{"key": "value"}'`

### Custom Socket
```bash
# Override auto-detected socket
sysconfig-cli --socket /tmp/custom.sock get
```

### Debugging
```bash
# Enable verbose logging
sysconfig-cli --verbose get

# Or use environment variable
RUST_LOG=debug sysconfig-cli get
```

## Quick Test

```bash
# 1. Check configuration
sysconfig-cli info

# 2. Get current state
sysconfig-cli get

# 3. Set a test value
sysconfig-cli set '$.test.hello' '"world"'

# 4. Verify the change
sysconfig-cli get --path /test
```

## Help

```bash
# General help
sysconfig-cli --help

# Command-specific help
sysconfig-cli set --help
```

## Common Issues

**Connection refused**: Make sure sysconfig service is running
```bash
cd ../sysconfig && cargo run
```

**Permission denied**: Check socket permissions or use correct user

**Invalid JSON**: Ensure proper quoting for strings in shell
```bash
# Wrong: '$.field' 'value'
# Right: '$.field' '"value"'
```
