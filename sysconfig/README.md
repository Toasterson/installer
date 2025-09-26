# SysConfig

SysConfig is a service for configuring illumos systems. It provides a plugin-based architecture that allows different components to manage different aspects of system configuration.

## KDL Configuration Support

SysConfig now supports KDL (KDL Document Language) configuration files, providing a modern and expressive way to define system configurations. KDL files can be loaded at startup or watched for changes, making system configuration more dynamic and maintainable.

## Features

- Plugin-based architecture for extensibility
- RPC over Unix sockets for communication
- State management with locking to prevent conflicts
- Event-based notifications for state changes
- Support for executing actions on the system

## Architecture

SysConfig consists of a central service that manages plugins and state. Plugins can register with the service and provide functionality for managing specific aspects of system configuration.

### Components

- **SysConfigService**: The main service that manages plugins and state
- **Plugins**: Components that provide functionality for managing specific aspects of system configuration
- **State**: The system state that is managed by the service and plugins
- **Actions**: Operations that can be executed on the system

### Communication

SysConfig uses gRPC over Unix sockets for communication between the service and plugins. This provides a well-defined API and efficient binary serialization.

## Usage

### Starting the Service

To start the SysConfig service:

```bash
# Start with default socket
sysconfig

# Start with custom socket
sysconfig -s /path/to/socket

# Start with KDL configuration file
sysconfig -c /path/to/config.kdl

# Validate KDL configuration without applying (dry-run)
sysconfig -c /path/to/config.kdl --dry-run

# Watch KDL configuration file for changes
sysconfig -c /path/to/config.kdl --watch

# Show configuration summary
sysconfig -c /path/to/config.kdl --summary
```

### KDL Configuration Format

SysConfig supports KDL configuration files with the following structure:

```kdl
sysconfig {
    hostname "my-host"
    
    nameserver "8.8.8.8"
    nameserver "8.8.4.4"
    
    interface "eth0" {
        address name="v4" kind="dhcp4"
    }
    
    interface "eth1" selector="mac:00:11:22:33:44:55" {
        address name="v4" kind="static" "192.168.1.100/24"
        address name="v6" kind="static" "2001:db8::1/64"
    }
}
```

### KDL Configuration Options

- **hostname**: Sets the system hostname
- **nameserver**: Adds a DNS nameserver (can be specified multiple times)
- **interface**: Configures a network interface
  - `selector`: Optional MAC address selector for hardware-independent configuration
  - `address`: Configures an address on the interface
    - `name`: Address identifier
    - `kind`: Address type (`dhcp4`, `dhcp6`, `static`, `addrconf`)
    - Address value (required for static addresses)

### Writing Plugins

Plugins are separate binaries that implement the `PluginService` gRPC service. They can register with the SysConfig service to provide functionality for managing specific aspects of system configuration.

For detailed information on how to develop plugins, including how to create plugins that read configuration files, see the [Plugin Development Guide](docs/plugin-development.md).

Here's an example of a simple plugin:

```rust
use sysconfig::{PluginClient, PluginTrait, Result};
use async_trait::async_trait;

struct MyPlugin;

#[async_trait]
impl PluginTrait for MyPlugin {
    async fn initialize(&self, plugin_id: &str, service_socket_path: &str) -> Result<()> {
        // Initialize the plugin
        Ok(())
    }

    async fn get_config(&self) -> Result<String> {
        // Return the plugin's configuration
        Ok("{}".to_string())
    }

    async fn diff_state(&self, current_state: &str, desired_state: &str) -> Result<Vec<StateChange>> {
        // Diff the current state with the desired state
        Ok(vec![])
    }

    async fn apply_state(&self, state: &str, dry_run: bool) -> Result<Vec<StateChange>> {
        // Apply a new state
        Ok(vec![])
    }

    async fn execute_action(&self, action: &str, parameters: &str) -> Result<String> {
        // Execute an action
        Ok("".to_string())
    }

    async fn notify_state_change(&self, event: StateChangeEvent) -> Result<()> {
        // Handle a state change notification
        Ok(())
    }
}
```

### Configuration Files

SysConfig supports two configuration file formats:

1. **KDL Format** (Recommended): A modern, expressive configuration language that integrates well with system provisioning tools. KDL files use the `.kdl` extension.

2. **Legacy Format**: The original configuration format documented in the [Configuration File Format Guide](docs/config-file-format.md).

#### Example KDL Configurations

##### Minimal Configuration
```kdl
sysconfig {
    hostname "minimal-host"
    nameserver "9.9.9.9"
    interface "eth0" {
        address name="v4" kind="dhcp4"
    }
}
```

##### Production Configuration
```kdl
sysconfig {
    hostname "production-server"
    
    // DNS configuration with fallbacks
    nameserver "9.9.9.9"
    nameserver "149.112.112.112"
    nameserver "1.1.1.1"
    
    // Primary interface with static IPs
    interface "net0" selector="mac:00:0c:29:3e:4f:50" {
        address name="v4-primary" kind="static" "192.168.1.200/24"
        address name="v6-primary" kind="static" "2001:db8:1::200/64"
    }
    
    // Management interface
    interface "net1" selector="mac:00:0c:29:3e:4f:51" {
        address name="mgmt" kind="static" "10.0.0.200/24"
    }
}
```

For more examples, see the `examples/` directory.

### SMF Integration

SysConfig can be integrated with the Service Management Facility (SMF) on illumos systems. An SMF manifest is provided in the `image/templates/files` directory.

## Development

### Building

To build SysConfig:

```bash
cargo build
```

### Testing

To run the tests:

```bash
cargo test
```

### Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
