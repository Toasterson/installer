# SysConfig

SysConfig is a service for configuring illumos systems. It provides a plugin-based architecture that allows different components to manage different aspects of system configuration.

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
sysconfig -s /path/to/socket
```

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

SysConfig uses a specific format for configuration files. These files define the system settings that will be applied by the service and its plugins.

For detailed information on how to write configuration files, including syntax, structure, and examples, see the [Configuration File Format Guide](docs/config-file-format.md).

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
