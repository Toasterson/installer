# Sysconfig CLI Tool - Summary

## Overview

The `sysconfig-cli` is a command-line interface tool designed to interact with the Sysconfig service for testing and debugging purposes. It provides a simple and intuitive way to inspect, modify, and monitor system configuration state, making it easy to test how plugins react to state changes.

## Key Features

### 1. **State Inspection (`get` command)**
- Retrieve the entire system configuration state
- Query specific paths within the state hierarchy
- Support for both JSON and pretty-printed output formats

### 2. **State Modification (`set` command)**
- Update specific values using JSONPath syntax
- Support for simple values (strings, numbers, booleans)
- Support for complex objects and arrays
- Dry-run mode to preview changes before applying

### 3. **Bulk State Application (`apply` command)**
- Apply configuration from JSON files
- Read configuration from stdin for pipeline operations
- Verbose mode to see detailed change information
- Dry-run support for validation

### 4. **Real-time Monitoring (`watch` command)**
- Monitor state changes as they happen
- Filter by specific paths
- Timestamped event logging
- Support for both JSON and pretty-printed output

### 5. **State Comparison (`diff` command)**
- Compare current state with desired state
- Show what changes would be made
- Useful for pre-deployment validation

## Technical Implementation

### Architecture
- Built with Rust for performance and safety
- Uses gRPC (via Tonic) for communication with Sysconfig service
- Protocol Buffers for message serialization
- Unix socket for local communication with automatic path detection

### Automatic Socket Detection
The CLI automatically detects the appropriate socket path based on the current user, matching the sysconfig service behavior:
- **Root users**: `/var/run/sysconfig.sock`
- **Regular users**: `$XDG_RUNTIME_DIR/sysconfig.sock` or `/run/user/$UID/sysconfig.sock`
- This ensures seamless connectivity without requiring manual configuration

### Dependencies
- `clap` - Command-line argument parsing
- `tonic` - gRPC client implementation
- `serde_json` - JSON parsing and manipulation
- `jsonpath_lib` - JSONPath expression evaluation
- `colored` - Terminal output formatting
- `tokio` - Async runtime

### JSONPath Support
The tool supports JSONPath expressions for targeting specific values:
- `$.field` - Top-level fields
- `$.parent.child` - Nested fields
- `$.deep.nested.path` - Arbitrary depth
- Path expressions with or without the `$` prefix

## Usage Examples

### Basic Operations
```bash
# Show detected socket and configuration
sysconfig-cli info

# Get current state (automatically uses correct socket)
sysconfig-cli get

# Get specific path
sysconfig-cli get --path /network

# Set a value
sysconfig-cli set '$.network.hostname' '"my-host"'

# Apply from file
sysconfig-cli apply --file config.json

# Watch for changes
sysconfig-cli watch
```

### Testing Plugin Reactions
```bash
# Terminal 1: Start monitoring
sysconfig-cli watch

# Terminal 2: Make changes
sysconfig-cli set '$.test.trigger' 'true'
sysconfig-cli set '$.test.value' '42'
```

### Complex Configuration
```bash
# Set nested object
sysconfig-cli set '$.network.interfaces.eth0' \
  '{"ip": "192.168.1.100", "netmask": "255.255.255.0"}'

# Preview changes with dry-run
sysconfig-cli apply --file new-config.json --dry-run
```

## Installation

### From Source
```bash
cd sysconfig-cli
cargo build --release
sudo make install
```

### Custom Installation Directory
```bash
INSTALL_DIR=~/.local/bin make install
```

### Using Install Script
```bash
./install.sh
# Or for custom directory:
INSTALL_DIR=~/.local/bin ./install.sh
```

## Project Structure

```
sysconfig-cli/
├── Cargo.toml          # Rust dependencies
├── build.rs            # Proto compilation
├── src/
│   └── main.rs         # CLI implementation
├── examples/
│   └── state.json      # Example configuration
├── Makefile            # Build automation
├── install.sh          # Installation script
├── demo.sh             # Interactive demo
├── test_jsonpath.sh    # JSONPath testing
└── README.md           # Documentation
```

## Key Design Decisions

### 1. **Automatic Socket Detection**
The CLI automatically detects the correct socket path based on the user context (root vs regular user), matching the sysconfig service behavior. This eliminates configuration friction and ensures the CLI "just works" regardless of how the service was started.

### 2. **JSONPath for Value Setting**
Using JSONPath syntax provides a familiar and powerful way to target specific values in the configuration tree without requiring knowledge of the internal state structure.

### 3. **Dry-Run Support**
All modification commands support dry-run mode, allowing users to validate changes before applying them - crucial for testing and debugging.

### 4. **Unix Socket Communication**
Direct communication via Unix socket ensures low latency and secure local-only access to the Sysconfig service. The socket path is intelligently chosen based on user permissions.

### 5. **Colored Output**
Terminal colors make it easy to distinguish between different types of information (changes, errors, values) at a glance.

### 6. **Streaming for Watch Command**
Uses gRPC streaming to efficiently monitor state changes in real-time without polling.

## Testing Workflow

1. **Start the Sysconfig service** (in one terminal):
   ```bash
   cd sysconfig
   cargo run  # Automatically uses user-appropriate socket
   ```

2. **Use the CLI to interact** (in another terminal):
   ```bash
   # Check configuration and socket detection
   sysconfig-cli info
   
   # Check current state (automatically connects to correct socket)
   sysconfig-cli get

   # Make changes
   sysconfig-cli set '$.test.value' '123'

   # Watch for plugin reactions
   sysconfig-cli watch
   ```

3. **Test plugin behavior**:
   - Make state changes via CLI
   - Observe how plugins react to changes
   - Validate plugin state synchronization
   - Test error conditions and edge cases

## Benefits for Development

- **Rapid Testing**: Quickly modify state without writing code
- **Plugin Development**: Test plugin reactions to state changes
- **Debugging**: Monitor state changes in real-time
- **Validation**: Preview changes before applying them
- **Scripting**: Automate testing scenarios with shell scripts
- **CI/CD Integration**: Use in automated testing pipelines

## Future Enhancements

Potential improvements for the tool:

1. **Advanced JSONPath**: Support for filters and wildcards
2. **State History**: Track and replay state changes
3. **Batch Operations**: Apply multiple changes atomically
4. **State Export/Import**: Save and restore complete states
5. **Plugin Commands**: Direct interaction with specific plugins
6. **Interactive Mode**: REPL-like interface for exploration
7. **State Validation**: Schema validation before applying changes
8. **Undo/Redo**: Revert recent changes

## Conclusion

The `sysconfig-cli` tool provides a comprehensive interface for interacting with the Sysconfig service, making it invaluable for testing, debugging, and development. Its support for JSONPath expressions, dry-run mode, and real-time monitoring makes it particularly useful for understanding and testing how plugins react to state changes in the system.