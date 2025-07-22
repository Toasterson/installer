# System Configuration Overview

The System Configuration (`sysconfig`) component is responsible for configuring various aspects of an illumos system. It provides a plugin-based architecture that allows different components to manage different aspects of system configuration.

## Purpose

The primary purpose of the System Configuration component is to:

1. Provide a unified interface for configuring system settings
2. Support a plugin-based architecture for extensibility
3. Manage system state with locking to prevent conflicts
4. Provide event-based notifications for state changes
5. Support executing actions on the system

By providing a flexible and extensible configuration system, the System Configuration component makes it easier to manage illumos systems in a consistent and reliable way.

## Architecture

The System Configuration component consists of a central service that manages plugins and state. Plugins can register with the service and provide functionality for managing specific aspects of system configuration.

### Components

- **SysConfigService**: The main service that manages plugins and state
- **Plugins**: Components that provide functionality for managing specific aspects of system configuration
- **State**: The system state that is managed by the service and plugins
- **Actions**: Operations that can be executed on the system

### Communication

System Configuration uses gRPC over Unix sockets for communication between the service and plugins. This provides a well-defined API and efficient binary serialization.

## Configuration Format

System Configuration uses a structured configuration format to define system settings. The format is similar to a simplified XML or YAML, but with its own syntax rules.

A basic System Configuration file might look like this:

```
hostname my-host

nameserver 8.8.8.8
nameserver 8.8.4.4

interface eth0 {
    address {
        name = eth0
        kind = Dhcp4
    }
}
```

This configuration:
- Sets the hostname to "my-host"
- Configures two DNS nameservers
- Sets up the eth0 interface to use DHCP for IPv4

For more information about the configuration format, see [Configuration File Format](format.md).

## Configuration Options

The System Configuration component supports various configuration options, including:

### Hostname

The hostname is defined as a child element with an argument:

```
hostname my-host
```

For more information, see [Hostname Configuration](hostname.md).

### Nameservers

Nameservers are defined as multiple child elements, each with an argument:

```
nameserver 8.8.8.8
nameserver 8.8.4.4
```

For more information, see [Nameserver Configuration](nameservers.md).

### Network Interfaces

Network interfaces are defined as child elements with optional arguments and properties:

```
interface eth0 {
    selector = "mac=00:11:22:33:44:55"
    
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
    }
}
```

For more information, see [Network Interface Configuration](interfaces.md).

## Plugin Architecture

The System Configuration component uses a plugin-based architecture that allows different components to manage different aspects of system configuration. Plugins are separate binaries that implement the `PluginService` gRPC service.

For more information about the plugin architecture, see [Plugin Architecture](plugins.md).

## Integration with Machine Configuration

The System Configuration component integrates with the Machine Configuration component to provide a complete system configuration solution. In a Machine Configuration file, the System Configuration is specified using the `sysconfig` element:

```kdl
sysconfig {
    hostname "myhost"
    nameserver "8.8.8.8"
    nameserver "8.8.4.4"
    interface "net0" {
        address name="v4" kind="dhcp4"
    }
}
```

For more information about the Machine Configuration component, see the [Machine Configuration](../machineconfig/overview.md) section.

## Next Steps

- Learn about the [Configuration File Format](format.md) in detail
- Understand how to configure [Hostname](hostname.md)
- See [Examples](examples.md) of System Configuration files