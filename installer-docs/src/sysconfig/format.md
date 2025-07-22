# Configuration File Format

This page explains how to write configuration files for the System Configuration (`sysconfig`) component. Configuration files are used to define system settings that will be applied by the SysConfig service and its plugins.

## File Format

Configuration files for SysConfig are text files with a specific structure. The format is similar to a simplified XML or YAML, but with its own syntax rules.

## Basic Structure

A configuration file consists of elements, which can have:
- Arguments (values that follow the element name)
- Properties (key-value pairs)
- Child elements (nested elements)

Here's the basic syntax:

```
element_name argument {
    property_name = property_value
    
    child_element child_argument {
        child_property = child_property_value
    }
}
```

## Element Types

The SysConfig configuration format supports several types of elements:

### Hostname

The hostname is defined as an element with an argument:

```
hostname my-host
```

This sets the system hostname to "my-host".

### Nameservers

Nameservers are defined as multiple elements, each with an argument:

```
nameserver 8.8.8.8
nameserver 8.8.4.4
```

This configures two DNS nameservers: 8.8.8.8 and 8.8.4.4.

### Network Interfaces

Network interfaces are defined as elements with optional arguments and properties:

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

Each interface can have:
- An optional name argument
- An optional selector property
- Multiple address child elements

Each address element can have:
- A name property
- A kind property (one of: Dhcp4, Dhcp6, Addrconf, Static)
- An optional address argument (required for Static)

## Property Values

Property values can be strings, numbers, or booleans:

```
property_string = "value"
property_number = 42
property_boolean = true
```

Strings can be quoted or unquoted, but must be quoted if they contain spaces or special characters:

```
property = value
property = "value with spaces"
```

## Selectors

Selectors are used to identify hardware components based on their attributes. For example, a network interface can be identified by its MAC address:

```
interface {
    selector = "mac=00:11:22:33:44:55"
    // ...
}
```

This allows the configuration to be hardware-independent, as it identifies interfaces by their attributes rather than by their names, which may change between systems or after hardware changes.

## Address Kinds

The `kind` property of an address element specifies how the address should be configured:

- `Dhcp4`: Use DHCP for IPv4
- `Dhcp6`: Use DHCP for IPv6
- `Addrconf`: Use stateless address autoconfiguration (SLAAC) for IPv6
- `Static`: Use a static address, which must be specified as an argument

For example:

```
address {
    name = eth0
    kind = Dhcp4
}
```

This configures the interface to use DHCP for IPv4.

```
address {
    name = eth0
    kind = Static
    192.168.1.100/24
}
```

This configures the interface with a static IPv4 address of 192.168.1.100 with a subnet mask of 255.255.255.0.

## Multiple Configurations

A configuration file can contain multiple elements of the same type:

```
nameserver 8.8.8.8
nameserver 8.8.4.4

interface eth0 {
    address {
        name = eth0
        kind = Dhcp4
    }
}

interface wlan0 {
    address {
        name = wlan0
        kind = Dhcp4
    }
}
```

This configures two nameservers and two network interfaces.

## Comments

The configuration format does not directly support comments. However, you can add explanatory text in a separate file or as part of your documentation.

## Examples

### Basic Configuration

Here's a basic configuration example:

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

### Advanced Configuration

Here's a more advanced configuration example:

```
hostname complex-host

nameserver 8.8.8.8
nameserver 8.8.4.4
nameserver 2001:4860:4860::8888

interface {
    selector = "mac=00:11:22:33:44:55"
    
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
    }
    
    address {
        name = eth0
        kind = Static
        2001:db8::1/64
    }
}

interface wlan0 {
    address {
        name = wlan0
        kind = Dhcp4
    }
    
    address {
        name = wlan0
        kind = Dhcp6
    }
}
```

This configuration:
- Sets the hostname to "complex-host"
- Configures three DNS nameservers (two IPv4 and one IPv6)
- Sets up an interface identified by MAC address with static IPv4 and IPv6 addresses
- Sets up the wlan0 interface to use DHCP for both IPv4 and IPv6

## Validation

The SysConfig service validates configuration files when they are loaded. If a configuration file is invalid, an error will be reported.

Common validation errors include:
- Missing required elements or properties
- Invalid property values
- Syntax errors

## Best Practices

When writing configuration files, follow these best practices:

1. **Use Descriptive Names**: Use descriptive names for interfaces and other elements to make the configuration easier to understand.

2. **Comment Your Configuration**: While the format doesn't directly support comments, you can add explanatory text in a separate file or as part of your documentation.

3. **Validate Before Deploying**: Test your configuration files before deploying them to production systems.

4. **Keep It Simple**: Start with a simple configuration and add complexity as needed.

5. **Use Version Control**: Store your configuration files in a version control system to track changes over time.

6. **Document Special Requirements**: If your configuration has special requirements or dependencies, document them clearly.

7. **Use Selectors for Hardware Independence**: When possible, use selectors (like MAC addresses) instead of hardcoded interface names to make your configuration more portable across different hardware.

## Next Steps

- Learn about [Hostname Configuration](hostname.md) in detail
- Understand how to configure [Nameservers](nameservers.md)
- Learn about [Network Interface Configuration](interfaces.md)
- See [Examples](examples.md) of System Configuration files