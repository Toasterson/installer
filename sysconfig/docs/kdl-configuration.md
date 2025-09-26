# KDL Configuration Guide

## Introduction

SysConfig supports KDL (KDL Document Language) as its modern configuration format. KDL is a human-friendly configuration language that combines the simplicity of INI files with the structure of XML/JSON, making it ideal for system configuration.

## Why KDL?

KDL was chosen for SysConfig because it offers:

- **Human-readable syntax** - Easy to write and understand
- **Type safety** - Strong typing for values
- **Comments** - Built-in support for documentation
- **Hierarchical structure** - Natural nesting for complex configurations
- **Minimal syntax** - Less verbose than JSON/XML
- **Error messages** - Clear, helpful error reporting

## Configuration Structure

A KDL configuration file for SysConfig follows this general structure:

```kdl
sysconfig {
    // Global system settings
    hostname "system-name"
    
    // DNS configuration
    nameserver "dns-ip-1"
    nameserver "dns-ip-2"
    
    // Network interface configuration
    interface "interface-name" selector="optional-selector" {
        address name="address-name" kind="address-type" "optional-address"
    }
}
```

## Configuration Options

### Hostname

Sets the system hostname.

```kdl
sysconfig {
    hostname "my-server"
}
```

**Validation:**
- Must be non-empty
- Maximum 255 characters
- Should contain only alphanumeric characters, hyphens, and dots (warning issued for other characters)

### Nameservers

Configures DNS nameservers. Multiple nameservers can be specified for redundancy.

```kdl
sysconfig {
    nameserver "8.8.8.8"
    nameserver "8.8.4.4"
    nameserver "2001:4860:4860::8888"  // IPv6 supported
}
```

**Validation:**
- Must be non-empty
- Should be valid IP addresses (IPv4 or IPv6)

### Network Interfaces

Configures network interfaces with various addressing options.

```kdl
sysconfig {
    interface "eth0" {
        address name="primary" kind="dhcp4"
    }
}
```

#### Interface Properties

- **name** (required): The interface name (e.g., "eth0", "net0", "wlan0")
- **selector** (optional): Hardware selector for interface matching

#### Selector Format

The selector property allows hardware-independent configuration by matching interfaces based on MAC address:

```kdl
interface "net0" selector="mac:00:11:22:33:44:55" {
    // configuration
}
```

This is useful when:
- Interface names might change between boots
- Deploying configurations across multiple machines
- Ensuring consistent configuration regardless of hardware detection order

#### Address Configuration

Each interface can have multiple addresses configured:

```kdl
interface "eth0" {
    address name="v4" kind="dhcp4"
    address name="v6" kind="dhcp6"
    address name="link-local" kind="addrconf"
    address name="static-v4" kind="static" "192.168.1.100/24"
}
```

**Address Properties:**
- **name** (required): Unique identifier for the address
- **kind** (required): Type of address configuration
  - `dhcp4`: DHCPv4 client
  - `dhcp6`: DHCPv6 client
  - `addrconf`: IPv6 stateless address autoconfiguration (SLAAC)
  - `static`: Static IP address
- **address** (required for static): The IP address and prefix length (e.g., "192.168.1.100/24")

## Complete Examples

### Basic Desktop Configuration

```kdl
sysconfig {
    hostname "desktop-01"
    
    nameserver "1.1.1.1"
    nameserver "1.0.0.1"
    
    interface "eth0" {
        address name="v4" kind="dhcp4"
        address name="v6" kind="dhcp6"
    }
}
```

### Static Server Configuration

```kdl
sysconfig {
    hostname "web-server-01"
    
    // Use reliable DNS servers
    nameserver "9.9.9.9"
    nameserver "149.112.112.112"
    
    // Primary network interface with static configuration
    interface "eth0" selector="mac:00:0c:29:3e:4f:50" {
        address name="primary-v4" kind="static" "192.168.1.100/24"
        address name="primary-v6" kind="static" "2001:db8::100/64"
    }
}
```

### Multi-Homed Server

```kdl
sysconfig {
    hostname "multi-homed-server"
    
    nameserver "8.8.8.8"
    nameserver "8.8.4.4"
    
    // Public interface
    interface "net0" selector="mac:00:11:22:33:44:55" {
        address name="public-v4" kind="static" "203.0.113.10/24"
        address name="public-v6" kind="static" "2001:db8:1::10/64"
    }
    
    // Management interface
    interface "net1" selector="mac:00:11:22:33:44:56" {
        address name="mgmt" kind="static" "10.0.0.10/24"
    }
    
    // Storage network
    interface "net2" selector="mac:00:11:22:33:44:57" {
        address name="storage" kind="static" "172.16.0.10/24"
    }
    
    // Backup network with DHCP
    interface "net3" selector="mac:00:11:22:33:44:58" {
        address name="backup" kind="dhcp4"
    }
}
```

### High Availability Configuration

```kdl
sysconfig {
    hostname "ha-node-01"
    
    // Multiple nameservers for redundancy
    nameserver "9.9.9.9"
    nameserver "149.112.112.112"
    nameserver "1.1.1.1"
    nameserver "1.0.0.1"
    
    // Primary interface with multiple IPs for failover
    interface "net0" selector="mac:aa:bb:cc:dd:ee:ff" {
        address name="primary" kind="static" "192.168.1.100/24"
        address name="vip1" kind="static" "192.168.1.101/24"
        address name="vip2" kind="static" "192.168.1.102/24"
        address name="v6" kind="static" "2001:db8::100/64"
    }
    
    // Heartbeat network for cluster communication
    interface "net1" selector="mac:aa:bb:cc:dd:ef:00" {
        address name="heartbeat" kind="static" "10.0.0.100/24"
    }
}
```

## Command Line Usage

### Loading Configuration

```bash
# Apply configuration
sysconfig -c /path/to/config.kdl

# Validate without applying (dry run)
sysconfig -c /path/to/config.kdl --dry-run

# Show configuration summary
sysconfig -c /path/to/config.kdl --summary
```

### Watching for Changes

The `--watch` flag enables automatic reloading when the configuration file changes:

```bash
sysconfig -c /path/to/config.kdl --watch
```

This is useful for:
- Development and testing
- Dynamic configuration updates
- Configuration management systems

## Best Practices

### 1. Use Selectors for Hardware Independence

Instead of hardcoding interface names, use MAC address selectors:

```kdl
// Good - works regardless of interface naming
interface "net0" selector="mac:00:11:22:33:44:55" {
    address name="primary" kind="static" "192.168.1.100/24"
}

// Less portable - depends on specific interface name
interface "e1000g0" {
    address name="primary" kind="static" "192.168.1.100/24"
}
```

### 2. Document Your Configuration

Use KDL's comment support to document your configuration:

```kdl
sysconfig {
    // Production web server - managed by ops team
    hostname "prod-web-01"
    
    // Primary DNS servers (datacenter local)
    nameserver "10.0.1.53"
    nameserver "10.0.2.53"
    
    // Fallback DNS (public)
    nameserver "8.8.8.8"
}
```

### 3. Group Related Configuration

Organize interfaces logically:

```kdl
sysconfig {
    hostname "app-server"
    
    // === DNS Configuration ===
    nameserver "10.0.1.53"
    nameserver "10.0.2.53"
    
    // === Production Network ===
    interface "net0" selector="mac:00:11:22:33:44:55" {
        address name="prod-v4" kind="static" "192.168.1.100/24"
        address name="prod-v6" kind="static" "2001:db8:1::100/64"
    }
    
    // === Management Network ===
    interface "net1" selector="mac:00:11:22:33:44:56" {
        address name="mgmt" kind="static" "10.0.0.100/24"
    }
}
```

### 4. Use Descriptive Address Names

Choose meaningful names for addresses to make the configuration self-documenting:

```kdl
interface "net0" {
    address name="primary-v4" kind="static" "192.168.1.100/24"
    address name="secondary-v4" kind="static" "192.168.1.101/24"
    address name="ipv6-global" kind="static" "2001:db8::100/64"
    address name="ipv6-link-local" kind="addrconf"
}
```

### 5. Validate Before Deployment

Always test configurations with `--dry-run` before applying:

```bash
# Validate configuration
sysconfig -c new-config.kdl --dry-run

# If validation passes, apply
sysconfig -c new-config.kdl
```

## Migration from Legacy Format

If you have existing configurations in the legacy format, here's how to migrate:

### Legacy Format
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

### KDL Format
```kdl
sysconfig {
    hostname "my-host"
    
    nameserver "8.8.8.8"
    nameserver "8.8.4.4"
    
    interface "eth0" {
        address name="eth0" kind="dhcp4"
    }
}
```

Key differences:
- Wrap everything in a `sysconfig` block
- Quote string values
- Use lowercase for address kinds
- Simplified address syntax

## Troubleshooting

### Common Errors

#### Missing sysconfig Node
```
Error: Missing sysconfig node
```

**Solution:** Ensure your configuration is wrapped in a `sysconfig { ... }` block.

#### Empty Hostname
```
Error: Hostname cannot be empty
```

**Solution:** Provide a non-empty hostname value.

#### Missing Static Address
```
Error: Static address on interface eth0 requires an address value
```

**Solution:** When using `kind="static"`, always provide an address:
```kdl
address name="v4" kind="static" "192.168.1.100/24"
```

### Validation Tips

1. **Use --dry-run**: Always validate before applying
2. **Check logs**: Enable debug logging with `RUST_LOG=debug`
3. **Start simple**: Begin with a minimal configuration and add complexity
4. **Test incrementally**: Add one interface at a time when debugging

## Integration with System Provisioning

KDL configuration files can be integrated with broader system provisioning:

```kdl
// Full system configuration (example)
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0" "c6t0d0"
    }
}

image "oci://registry.example.com/os/base:latest"

sysconfig {
    hostname "provisioned-system"
    nameserver "10.0.1.53"
    
    interface "net0" {
        address name="v4" kind="dhcp4"
    }
}
```

This allows sysconfig to be part of a larger infrastructure-as-code approach.

## Advanced Features

### Multiple Addresses per Interface

Interfaces can have multiple addresses of different types:

```kdl
interface "net0" {
    // IPv4 addresses
    address name="primary-v4" kind="static" "192.168.1.100/24"
    address name="alias-v4" kind="static" "192.168.1.101/24"
    
    // IPv6 addresses
    address name="v6-static" kind="static" "2001:db8::100/64"
    address name="v6-dhcp" kind="dhcp6"
    address name="v6-slaac" kind="addrconf"
}
```

### Mixed Configuration Strategies

Combine different addressing methods for flexibility:

```kdl
interface "net0" {
    // Get IPv4 from DHCP
    address name="v4-dynamic" kind="dhcp4"
    
    // But use static IPv6
    address name="v6-static" kind="static" "2001:db8::100/64"
    
    // Also configure link-local
    address name="v6-link" kind="addrconf"
}
```

## Future Enhancements

The KDL configuration format is designed to be extensible. Future versions may support:

- Route configuration
- Firewall rules
- VPN settings
- Service configuration
- User management
- Package installation

These can be added without breaking existing configurations.

## Reference

### Address Kinds

| Kind | Description | Requires Address |
|------|-------------|-----------------|
| `dhcp4` | DHCPv4 client | No |
| `dhcp6` | DHCPv6 client | No |
| `addrconf` | IPv6 SLAAC | No |
| `static` | Static IP | Yes |

### Configuration Limits

| Setting | Maximum |
|---------|---------|
| Hostname length | 255 characters |
| Nameservers | Unlimited |
| Interfaces | Unlimited |
| Addresses per interface | Unlimited |

## Getting Help

For additional help:

1. Run `sysconfig --help` for command-line options
2. Check example configurations in `examples/` directory
3. Enable debug logging: `RUST_LOG=debug sysconfig -c config.kdl`
4. Validate configurations: `sysconfig -c config.kdl --dry-run`
