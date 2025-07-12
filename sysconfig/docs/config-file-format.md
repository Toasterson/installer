# Configuration File Format Guide

This guide explains how to write configuration files for the SysConfig service. Configuration files are used to define system settings that will be applied by the SysConfig service and its plugins.

## Table of Contents

- [Introduction](#introduction)
- [File Format](#file-format)
- [Basic Structure](#basic-structure)
- [Configuration Options](#configuration-options)
  - [Hostname](#hostname)
  - [Nameservers](#nameservers)
  - [Network Interfaces](#network-interfaces)
- [Examples](#examples)
  - [Basic Configuration](#basic-configuration)
  - [Advanced Configuration](#advanced-configuration)
- [Validation](#validation)
- [Best Practices](#best-practices)

## Introduction

SysConfig uses a structured configuration file format to define system settings. The configuration files are parsed using the `knus` library, which provides a flexible and powerful way to define hierarchical configurations.

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

## Configuration Options

The SysConfig service supports the following configuration options:

### Hostname

The hostname is defined as a child element with an argument:

```
hostname my-host
```

### Nameservers

Nameservers are defined as multiple child elements, each with an argument:

```
nameserver 8.8.8.8
nameserver 8.8.4.4
```

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

Each interface can have:
- An optional name argument
- An optional selector property
- Multiple address child elements

Each address element can have:
- A name property
- A kind property (one of: Dhcp4, Dhcp6, Addrconf, Static)
- An optional address argument (required for Static)

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