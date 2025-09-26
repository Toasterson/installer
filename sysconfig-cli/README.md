# Sysconfig CLI

A command-line interface tool for interacting with the Sysconfig service. This tool allows you to inspect, modify, and monitor system configuration state, making it easy to test how plugins react to state changes.

## Features

- **Get State**: Retrieve current system configuration state
- **Apply State**: Apply configuration changes from JSON files or stdin
- **Set Values**: Update specific configuration values using JSONPath syntax
- **Watch Changes**: Monitor real-time state changes as they happen
- **Diff States**: Compare current state with desired state
- **Dry Run Support**: Preview changes before applying them

## Installation

Build the tool from source:

```bash
cd sysconfig-cli
cargo build --release
```

The binary will be available at `target/release/sysconfig-cli`.

## Usage

### Basic Syntax

```bash
sysconfig-cli [OPTIONS] <COMMAND>
```

### Global Options

- `-s, --socket <PATH>`: Path to the Unix socket for the Sysconfig service (auto-detected based on user)
- `-v, --verbose`: Enable verbose logging
- `-h, --help`: Print help information
- `-V, --version`: Print version information

### Commands

#### Get State

Retrieve the current system state or a specific part of it:

```bash
# Get entire state
sysconfig-cli get

# Get specific path
sysconfig-cli get --path "/network"

# Output as JSON
sysconfig-cli get --format json

# Pretty print (default)
sysconfig-cli get --format pretty
```

#### Apply State

Apply a new state configuration to the system:

```bash
# Apply from file
sysconfig-cli apply --file state.json

# Apply from stdin
cat state.json | sysconfig-cli apply --stdin

# Dry run to preview changes
sysconfig-cli apply --file state.json --dry-run

# Show verbose change details
sysconfig-cli apply --file state.json --verbose
```

#### Set Value

Set a specific value using JSONPath syntax:

```bash
# Set a simple value
sysconfig-cli set '$.network.hostname' '"myhost"'

# Set a complex object
sysconfig-cli set '$.network.interfaces.eth0' '{"ip": "192.168.1.100", "netmask": "255.255.255.0"}'

# Set with dry run
sysconfig-cli set '$.system.timezone' '"UTC"' --dry-run
```

**Note**: Values must be valid JSON. Strings need to be quoted twice (once for shell, once for JSON).

#### Watch State Changes

Monitor state changes in real-time:

```bash
# Watch all changes
sysconfig-cli watch

# Watch specific path
sysconfig-cli watch --path "/network"

# Output as JSON stream
sysconfig-cli watch --format json
```

#### Diff States

Compare current state with a desired state:

```bash
# Diff with file
sysconfig-cli diff --file desired-state.json

# Diff from stdin
cat desired-state.json | sysconfig-cli diff --stdin
```

## Examples

### Example 1: Testing Network Configuration

```bash
# Check current network state
sysconfig-cli get --path "/network"

# Set a new hostname
sysconfig-cli set '$.network.hostname' '"test-host"'

# Watch for plugin reactions
sysconfig-cli watch --path "/network"
```

### Example 2: Applying Complex Configuration

Create a state file (`new-config.json`):

```json
{
  "network": {
    "hostname": "myserver",
    "interfaces": {
      "eth0": {
        "ip": "10.0.0.100",
        "netmask": "255.255.255.0",
        "gateway": "10.0.0.1"
      }
    }
  },
  "system": {
    "timezone": "America/New_York",
    "locale": "en_US.UTF-8"
  }
}
```

Apply it:

```bash
# Preview changes first
sysconfig-cli apply --file new-config.json --dry-run

# Apply if everything looks good
sysconfig-cli apply --file new-config.json
```

### Example 3: Testing Plugin Behavior

```bash
# Terminal 1: Start watching for changes
sysconfig-cli watch

# Terminal 2: Make changes and observe plugin reactions
sysconfig-cli set '$.test.value' '42'
sysconfig-cli set '$.test.trigger' 'true'
```

## JSONPath Syntax

The `set` command uses JSONPath expressions to target specific values:

- `$.field` - Top-level field
- `$.parent.child` - Nested field
- `$.array[0]` - Array element (basic array indexing supported)
- `$.network.interfaces.eth0` - Deep nested path

## Output Formats

### Pretty Format (Default)

Human-readable output with colors and formatting.

### JSON Format

Machine-readable JSON output, useful for scripting:

```bash
# Get state as JSON
sysconfig-cli get --format json | jq '.network.hostname'

# Watch changes as JSON stream
sysconfig-cli watch --format json | jq '.path'
```

## Socket Path

The tool automatically detects the appropriate socket path based on the current user:

- **Root user**: `/var/run/sysconfig.sock`
- **Regular users**: 
  - Uses `$XDG_RUNTIME_DIR/sysconfig.sock` if XDG_RUNTIME_DIR is set
  - Otherwise uses `/run/user/$UID/sysconfig.sock`

This matches the behavior of the sysconfig service itself, ensuring the CLI automatically connects to the correct socket.

You can override the automatic detection with a custom path:

```bash
# Use custom socket path
sysconfig-cli --socket /tmp/sysconfig.sock get
```

## Debugging

Enable verbose logging to troubleshoot issues:

```bash
# Enable debug logging
sysconfig-cli --verbose get

# Or set environment variable
RUST_LOG=debug sysconfig-cli get
```

## Error Handling

The CLI provides clear error messages:

- **Connection errors**: Check if the Sysconfig service is running and the socket path is correct
- **JSON parse errors**: Ensure your input is valid JSON
- **JSONPath errors**: Verify your path expressions are correct
- **State application errors**: Review the error message for plugin-specific issues

## Contributing

This tool is part of the illumos installer project. Contributions are welcome!

## License

See the main project LICENSE file for details.