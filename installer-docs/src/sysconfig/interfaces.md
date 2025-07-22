# Network Interface Configuration

Network interfaces are essential for connecting systems to networks. This page provides detailed information about configuring network interfaces in the System Configuration component.

## Basic Interface Configuration

Network interfaces are defined using the `interface` element, which can have an optional name argument, properties, and child elements:

```
interface eth0 {
    address {
        name = eth0
        kind = Dhcp4
    }
}
```

This configuration sets up the eth0 interface to use DHCP for IPv4 addressing.

## Interface Identification

Interfaces can be identified in two ways:

### By Name

You can identify an interface by its name:

```
interface eth0 {
    // ...
}
```

This configures the interface named "eth0".

### By Selector

You can also identify an interface by its attributes using a selector:

```
interface {
    selector = "mac=00:11:22:33:44:55"
    // ...
}
```

This configures the interface with the MAC address 00:11:22:33:44:55, regardless of its name.

## Selectors

Selectors are a powerful way to identify interfaces based on their attributes, making your configuration more portable across different systems or after hardware changes.

### MAC Address Selector

The most common selector is the MAC address selector:

```
selector = "mac=00:11:22:33:44:55"
```

This selects the interface with the MAC address 00:11:22:33:44:55.

### Multiple Selectors

You can combine multiple selectors using the `&` (AND) operator:

```
selector = "mac=00:11:22:33:44:55 & driver=e1000"
```

This selects the interface with the MAC address 00:11:22:33:44:55 and the driver e1000.

## Address Configuration

Interfaces can have one or more address configurations, defined using the `address` element:

```
address {
    name = eth0
    kind = Dhcp4
}
```

Each address element has:
- A `name` property, which should match the interface name
- A `kind` property, which specifies the addressing method
- An optional address argument (required for Static kind)

### Address Kinds

The `kind` property specifies how the address should be configured:

#### DHCP for IPv4

```
address {
    name = eth0
    kind = Dhcp4
}
```

This configures the interface to use DHCP for IPv4 addressing.

#### DHCP for IPv6

```
address {
    name = eth0
    kind = Dhcp6
}
```

This configures the interface to use DHCP for IPv6 addressing.

#### Stateless Address Autoconfiguration (SLAAC) for IPv6

```
address {
    name = eth0
    kind = Addrconf
}
```

This configures the interface to use SLAAC for IPv6 addressing.

#### Static IPv4 Address

```
address {
    name = eth0
    kind = Static
    192.168.1.100/24
}
```

This configures the interface with a static IPv4 address of 192.168.1.100 with a subnet mask of 255.255.255.0.

#### Static IPv6 Address

```
address {
    name = eth0
    kind = Static
    2001:db8::1/64
}
```

This configures the interface with a static IPv6 address of 2001:db8::1 with a prefix length of 64.

## Multiple Addresses

An interface can have multiple address configurations:

```
interface eth0 {
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
```

This configures the eth0 interface with both IPv4 and IPv6 static addresses.

## Default Gateway

The default gateway is specified as part of the address configuration:

```
address {
    name = eth0
    kind = Static
    192.168.1.100/24
    gateway = 192.168.1.1
}
```

This configures the interface with a static IPv4 address and sets the default gateway to 192.168.1.1.

## Multiple Interfaces

You can configure multiple interfaces in a single configuration:

```
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

This configures both the eth0 and wlan0 interfaces to use DHCP for IPv4 addressing.

## Wireless Interfaces

Wireless interfaces may require additional configuration, such as SSID and security settings:

```
interface wlan0 {
    wireless {
        ssid = "MyNetwork"
        security = "wpa2"
        passphrase = "MySecretPassphrase"
    }
    
    address {
        name = wlan0
        kind = Dhcp4
    }
}
```

This configures the wlan0 interface to connect to the wireless network "MyNetwork" using WPA2 security and DHCP for IPv4 addressing.

## VLAN Interfaces

VLAN interfaces are configured by specifying the parent interface and VLAN ID:

```
interface eth0.100 {
    vlan {
        parent = eth0
        id = 100
    }
    
    address {
        name = eth0.100
        kind = Static
        192.168.100.1/24
    }
}
```

This configures a VLAN interface eth0.100 with VLAN ID 100 on the parent interface eth0, and assigns it a static IPv4 address.

## Bridge Interfaces

Bridge interfaces are configured by specifying the member interfaces:

```
interface br0 {
    bridge {
        members = ["eth0", "eth1"]
    }
    
    address {
        name = br0
        kind = Static
        192.168.1.100/24
    }
}
```

This configures a bridge interface br0 with member interfaces eth0 and eth1, and assigns it a static IPv4 address.

## Best Practices

When configuring network interfaces, follow these best practices:

1. **Use Selectors**: Use selectors instead of hardcoded interface names to make your configuration more portable.

2. **Configure Multiple Address Types**: Configure both IPv4 and IPv6 addresses when possible for better compatibility.

3. **Use DHCP When Appropriate**: Use DHCP for dynamic environments and static addresses for servers or infrastructure.

4. **Document Network Requirements**: Document any special network requirements or dependencies.

5. **Test Connectivity**: After configuring interfaces, test network connectivity to ensure it's working correctly.

6. **Consider Security**: Configure appropriate security settings for wireless interfaces.

7. **Plan for Redundancy**: Consider configuring redundant interfaces or failover mechanisms for critical systems.

## Examples

### Basic DHCP Configuration

```
interface eth0 {
    address {
        name = eth0
        kind = Dhcp4
    }
}
```

This configures the eth0 interface to use DHCP for IPv4 addressing.

### Static IP Configuration

```
interface eth0 {
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
        gateway = 192.168.1.1
    }
}
```

This configures the eth0 interface with a static IPv4 address and default gateway.

### Dual-Stack Configuration

```
interface eth0 {
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
        gateway = 192.168.1.1
    }
    
    address {
        name = eth0
        kind = Static
        2001:db8::1/64
        gateway = 2001:db8::ffff
    }
}
```

This configures the eth0 interface with both IPv4 and IPv6 static addresses and default gateways.

### Selector-Based Configuration

```
interface {
    selector = "mac=00:11:22:33:44:55"
    
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
    }
}
```

This configures the interface with the MAC address 00:11:22:33:44:55 with a static IPv4 address.

### Multiple Interface Configuration

```
interface eth0 {
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
    }
}

interface eth1 {
    address {
        name = eth1
        kind = Dhcp4
    }
}

interface wlan0 {
    wireless {
        ssid = "MyNetwork"
        security = "wpa2"
        passphrase = "MySecretPassphrase"
    }
    
    address {
        name = wlan0
        kind = Dhcp4
    }
}
```

This configures three interfaces: eth0 with a static IPv4 address, eth1 with DHCP for IPv4, and wlan0 with wireless settings and DHCP for IPv4.

## Next Steps

- Learn about the [Plugin Architecture](plugins.md)
- See [Examples](examples.md) of System Configuration files