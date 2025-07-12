# Plugin Development Guide

This guide explains how to develop plugins for the SysConfig service, with a focus on creating plugins that read configuration files.

## Table of Contents

- [Introduction](#introduction)
- [Plugin Architecture](#plugin-architecture)
- [Creating a Basic Plugin](#creating-a-basic-plugin)
- [Reading Configuration Files](#reading-configuration-files)
- [Plugin Lifecycle](#plugin-lifecycle)
- [Communication with SysConfig Service](#communication-with-sysconfig-service)
- [Advanced Topics](#advanced-topics)
- [Best Practices](#best-practices)

## Introduction

SysConfig plugins are separate binaries that implement the `PluginService` gRPC interface. They communicate with the SysConfig service over Unix sockets and can manage different aspects of system configuration.

Plugins can read configuration files from various sources, parse them, and apply the configuration to the system. This guide focuses on creating plugins that read configuration files and integrate them with the SysConfig service.

## Plugin Architecture

A SysConfig plugin consists of the following components:

1. **Plugin Implementation**: A Rust struct that implements the `PluginTrait` trait
2. **gRPC Server**: A server that listens on a Unix socket and handles requests from the SysConfig service
3. **Configuration Parser**: Code that reads and parses configuration files
4. **State Management**: Code that manages the system state based on the configuration

The plugin communicates with the SysConfig service using the gRPC protocol defined in `sysconfig.proto`.

## Creating a Basic Plugin

To create a basic plugin, you need to:

1. Create a new Rust project
2. Add the necessary dependencies
3. Implement the `PluginTrait` trait
4. Set up a gRPC server to handle requests

Here's a basic example of a plugin:

```rust
use sysconfig::{PluginTrait, Result, StateChange, StateChangeEvent};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

// Define your plugin struct
struct MyPlugin {
    // Add fields for your plugin state
    config: Arc<Mutex<String>>,
}

impl MyPlugin {
    // Create a new instance of your plugin
    fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new("{}".to_string())),
        }
    }
}

// Implement the PluginTrait for your plugin
#[async_trait]
impl PluginTrait for MyPlugin {
    async fn initialize(&self, plugin_id: &str, service_socket_path: &str) -> Result<()> {
        // Initialize your plugin
        println!("Initializing plugin with ID: {}", plugin_id);
        println!("Service socket path: {}", service_socket_path);
        Ok(())
    }

    async fn get_config(&self) -> Result<String> {
        // Return your plugin's configuration
        let config = self.config.lock().unwrap();
        Ok(config.clone())
    }

    async fn diff_state(&self, current_state: &str, desired_state: &str) -> Result<Vec<StateChange>> {
        // Compare the current state with the desired state
        // Return the changes that would be made
        Ok(vec![])
    }

    async fn apply_state(&self, state: &str, dry_run: bool) -> Result<Vec<StateChange>> {
        // Apply the new state to the system
        // If dry_run is true, don't actually make any changes
        Ok(vec![])
    }

    async fn execute_action(&self, action: &str, parameters: &str) -> Result<String> {
        // Execute an action with the given parameters
        Ok("Action executed".to_string())
    }

    async fn notify_state_change(&self, event: StateChangeEvent) -> Result<()> {
        // Handle a state change notification
        println!("State changed: {:?}", event);
        Ok(())
    }
}

// Main function to start the plugin
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Create your plugin
    let plugin = MyPlugin::new();

    // Create a gRPC server
    let socket_path = "/tmp/my-plugin.sock";
    let plugin_service = sysconfig::plugin_service_server::PluginServiceServer::new(plugin);

    // Start the server
    tonic::transport::Server::builder()
        .add_service(plugin_service)
        .serve_with_incoming(tokio_stream::wrappers::UnixListenerStream::new(
            tokio::net::UnixListener::bind(socket_path)?
        ))
        .await?;

    Ok(())
}
```

## Reading Configuration Files

One of the key features of a plugin is the ability to read and parse configuration files. SysConfig provides built-in support for parsing configuration files using the `knus` library.

For detailed information on the configuration file format, including syntax, structure, and examples, see the [Configuration File Format Guide](config-file-format.md).

### Using the knus Library

The `knus` library allows you to parse configuration files with a specific format. Here's an example of how to use it:

```rust
use sysconfig::{parse_config, Result, SysConfig};
use std::fs;

fn read_config_file(path: &str) -> Result<SysConfig> {
    // Read the file content
    let content = fs::read_to_string(path)?;

    // Parse the configuration
    let config = parse_config(path, &content)?;

    Ok(config)
}
```

### Custom Configuration Formats

If you need to support a different configuration format, you can implement your own parser. Here's an example using JSON:

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use sysconfig::Result;

#[derive(Debug, Serialize, Deserialize)]
struct MyConfig {
    name: String,
    version: String,
    settings: Vec<Setting>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Setting {
    key: String,
    value: String,
}

fn read_json_config(path: &str) -> Result<MyConfig> {
    // Read the file content
    let content = fs::read_to_string(path)?;

    // Parse the JSON
    let config: MyConfig = serde_json::from_str(&content)?;

    Ok(config)
}
```

### Integrating Configuration with Plugin

To integrate configuration reading with your plugin, you can update your plugin implementation:

```rust
use sysconfig::{PluginTrait, Result, StateChange, StateChangeEvent};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::fs;

struct ConfigPlugin {
    config_path: String,
    config: Arc<Mutex<String>>,
}

impl ConfigPlugin {
    fn new(config_path: &str) -> Self {
        Self {
            config_path: config_path.to_string(),
            config: Arc::new(Mutex::new("{}".to_string())),
        }
    }

    fn read_config(&self) -> Result<()> {
        // Read the configuration file
        let content = fs::read_to_string(&self.config_path)?;

        // Store the configuration
        let mut config = self.config.lock().unwrap();
        *config = content;

        Ok(())
    }
}

#[async_trait]
impl PluginTrait for ConfigPlugin {
    async fn initialize(&self, plugin_id: &str, service_socket_path: &str) -> Result<()> {
        // Read the configuration file during initialization
        self.read_config()?;
        Ok(())
    }

    async fn get_config(&self) -> Result<String> {
        // Return the stored configuration
        let config = self.config.lock().unwrap();
        Ok(config.clone())
    }

    // Implement other methods...
}
```

## Plugin Lifecycle

The lifecycle of a SysConfig plugin consists of the following stages:

1. **Startup**: The plugin is started as a separate process.
2. **Registration**: The plugin registers with the SysConfig service.
3. **Initialization**: The SysConfig service initializes the plugin.
4. **Operation**: The plugin handles requests from the SysConfig service.
5. **Shutdown**: The plugin is shut down when the SysConfig service is stopped.

### Startup

During startup, the plugin should:

1. Parse command-line arguments
2. Set up logging
3. Create an instance of the plugin struct
4. Start a gRPC server to listen for requests

### Registration

The plugin is registered with the SysConfig service either:

1. Automatically by the SysConfig service discovering the plugin
2. Manually by an administrator registering the plugin

During registration, the plugin provides:

- A unique identifier
- A name and description
- The socket path where it's listening
- The state paths it wants to manage

### Initialization

After registration, the SysConfig service initializes the plugin by calling the `initialize` method with:

- The plugin ID assigned by the service
- The socket path where the service is listening

During initialization, the plugin should:

1. Store the plugin ID and service socket path
2. Read its configuration files
3. Connect to the SysConfig service
4. Perform any other necessary setup

### Operation

During operation, the plugin handles requests from the SysConfig service:

- `get_config`: Return the plugin's configuration
- `diff_state`: Compare the current state with the desired state
- `apply_state`: Apply a new state to the system
- `execute_action`: Execute an action
- `notify_state_change`: Handle state change notifications

### Shutdown

When the SysConfig service is stopped, the plugin should:

1. Clean up any resources
2. Close connections
3. Exit gracefully

## Communication with SysConfig Service

Plugins communicate with the SysConfig service using gRPC over Unix sockets. The communication is bidirectional:

1. The SysConfig service sends requests to the plugin
2. The plugin can also send requests to the SysConfig service

### Receiving Requests from SysConfig

The plugin receives requests from the SysConfig service through the gRPC server it sets up. These requests are handled by the methods implemented in the `PluginTrait`.

### Sending Requests to SysConfig

The plugin can send requests to the SysConfig service using the `PluginClient`:

```rust
use sysconfig::{PluginClient, Result};

async fn connect_to_service(socket_path: &str) -> Result<PluginClient> {
    let client = PluginClient::connect(socket_path).await?;
    Ok(client)
}

async fn get_state(client: &mut PluginClient, path: &str) -> Result<String> {
    let state = client.get_state(path).await?;
    Ok(state)
}
```

## Advanced Topics

### Watching for Configuration Changes

To watch for changes in configuration files, you can use the `notify` crate:

```rust
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

fn watch_config_file(path: &str) -> Result<()> {
    // Create a channel to receive events
    let (tx, rx) = channel();

    // Create a watcher
    let mut watcher = watcher(tx, Duration::from_secs(10))?;

    // Watch the file
    watcher.watch(path, RecursiveMode::NonRecursive)?;

    // Handle events
    loop {
        match rx.recv() {
            Ok(event) => println!("Event: {:?}", event),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
```

### Handling Multiple Configuration Sources

If your plugin needs to handle multiple configuration sources, you can implement a configuration manager:

```rust
struct ConfigManager {
    sources: Vec<Box<dyn ConfigSource>>,
}

trait ConfigSource {
    fn read_config(&self) -> Result<String>;
}

struct FileConfigSource {
    path: String,
}

impl ConfigSource for FileConfigSource {
    fn read_config(&self) -> Result<String> {
        let content = fs::read_to_string(&self.path)?;
        Ok(content)
    }
}

struct HttpConfigSource {
    url: String,
}

impl ConfigSource for HttpConfigSource {
    fn read_config(&self) -> Result<String> {
        // Fetch configuration from HTTP
        Ok("{}".to_string())
    }
}
```

## Best Practices

When developing plugins for SysConfig, follow these best practices:

1. **Error Handling**: Use proper error handling and propagation.
2. **Logging**: Log important events and errors.
3. **Configuration Validation**: Validate configuration files before applying them.
4. **Idempotency**: Make sure your state changes are idempotent.
5. **Resource Management**: Clean up resources properly.
6. **Security**: Handle sensitive information securely.
7. **Testing**: Write tests for your plugin.

### Error Handling

Use the `Result` type for error handling:

```rust
fn process_config(path: &str) -> Result<()> {
    let config = read_config_file(path)?;
    // Process the configuration
    Ok(())
}
```

### Logging

Use the `tracing` crate for logging:

```rust
fn process_config(path: &str) -> Result<()> {
    tracing::info!("Processing configuration file: {}", path);
    let config = read_config_file(path)?;
    tracing::debug!("Configuration: {:?}", config);
    // Process the configuration
    tracing::info!("Configuration processed successfully");
    Ok(())
}
```

### Configuration Validation

Validate configuration files before applying them:

```rust
fn validate_config(config: &SysConfig) -> Result<()> {
    // Validate the configuration
    if config.hostname.is_empty() {
        return Err(Error::Plugin("Hostname cannot be empty".to_string()));
    }
    Ok(())
}
```

### Idempotency

Make sure your state changes are idempotent:

```rust
fn apply_hostname(hostname: &str) -> Result<()> {
    // Check if the hostname is already set
    let current_hostname = get_current_hostname()?;
    if current_hostname == hostname {
        return Ok(());
    }

    // Set the hostname
    set_hostname(hostname)?;
    Ok(())
}
```

### Resource Management

Clean up resources properly:

```rust
fn create_temp_file() -> Result<std::fs::File> {
    let file = tempfile::NamedTempFile::new()?;
    Ok(file.into_file())
}
```

### Security

Handle sensitive information securely:

```rust
fn read_credentials(path: &str) -> Result<Credentials> {
    // Read credentials from a secure location
    let content = fs::read_to_string(path)?;
    let credentials: Credentials = serde_json::from_str(&content)?;
    Ok(credentials)
}
```

### Testing

Write tests for your plugin:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_config() {
        let config = read_config_file("test_data/config.txt").unwrap();
        assert_eq!(config.hostname, "test-host");
    }
}
```
