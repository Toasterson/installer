# System Configuration Examples

This page provides examples of complete System Configuration files for various scenarios. These examples can be used as templates for your own configurations.

## Basic Configuration

Here's a basic System Configuration file that sets up a hostname, nameservers, and a network interface:

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
- Configures two DNS nameservers (Google's public DNS)
- Sets up the eth0 interface to use DHCP for IPv4

## Advanced Configuration

Here's a more advanced System Configuration file that sets up multiple network interfaces with different configurations:

```
hostname server01

nameserver 192.168.1.1
nameserver 8.8.8.8
nameserver 2001:4860:4860::8888

interface {
    selector = "mac=00:11:22:33:44:55"
    
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

interface {
    selector = "mac=00:11:22:33:44:66"
    
    address {
        name = eth1
        kind = Dhcp4
    }
    
    address {
        name = eth1
        kind = Dhcp6
    }
}
```

This configuration:
- Sets the hostname to "server01"
- Configures three DNS nameservers (local router, Google IPv4, and Google IPv6)
- Sets up an interface identified by MAC address with static IPv4 and IPv6 addresses and default gateways
- Sets up another interface identified by MAC address with DHCP for both IPv4 and IPv6

## Server Configuration

Here's a System Configuration file for a server with multiple network interfaces and specific requirements:

```
hostname webserver

nameserver 192.168.1.1
nameserver 192.168.1.2

interface {
    selector = "mac=00:11:22:33:44:55"
    
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
        gateway = 192.168.1.1
    }
}

interface {
    selector = "mac=00:11:22:33:44:66"
    
    address {
        name = eth1
        kind = Static
        10.0.0.100/24
    }
}

interface {
    selector = "mac=00:11:22:33:44:77"
    
    address {
        name = eth2
        kind = Static
        172.16.0.100/24
    }
}
```

This configuration:
- Sets the hostname to "webserver"
- Configures two DNS nameservers
- Sets up three interfaces with static IPv4 addresses, with the first interface having a default gateway

## Workstation Configuration

Here's a System Configuration file for a workstation with both wired and wireless interfaces:

```
hostname workstation

nameserver 192.168.1.1
nameserver 8.8.8.8
nameserver 8.8.4.4

interface eth0 {
    address {
        name = eth0
        kind = Dhcp4
    }
    
    address {
        name = eth0
        kind = Dhcp6
    }
}

interface wlan0 {
    wireless {
        ssid = "MyHomeNetwork"
        security = "wpa2"
        passphrase = "MySecretPassphrase"
    }
    
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
- Sets the hostname to "workstation"
- Configures three DNS nameservers
- Sets up the eth0 interface to use DHCP for both IPv4 and IPv6
- Sets up the wlan0 interface with wireless settings and DHCP for both IPv4 and IPv6

## Minimal Configuration

Here's a minimal System Configuration file that sets only the essential settings:

```
hostname minimal

interface eth0 {
    address {
        name = eth0
        kind = Dhcp4
    }
}
```

This configuration:
- Sets the hostname to "minimal"
- Sets up the eth0 interface to use DHCP for IPv4
- Uses default values for all other settings

## Complex Network Configuration

Here's a System Configuration file with complex network configurations, including VLANs and bridges:

```
hostname network-server

nameserver 192.168.1.1
nameserver 8.8.8.8

interface eth0 {
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
        gateway = 192.168.1.1
    }
}

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

interface eth0.200 {
    vlan {
        parent = eth0
        id = 200
    }
    
    address {
        name = eth0.200
        kind = Static
        192.168.200.1/24
    }
}

interface br0 {
    bridge {
        members = ["eth1", "eth2"]
    }
    
    address {
        name = br0
        kind = Static
        10.0.0.1/24
    }
}
```

This configuration:
- Sets the hostname to "network-server"
- Configures two DNS nameservers
- Sets up the eth0 interface with a static IPv4 address and default gateway
- Creates two VLAN interfaces (eth0.100 and eth0.200) with static IPv4 addresses
- Creates a bridge interface (br0) with two member interfaces and a static IPv4 address

## Integration with Machine Configuration

System Configuration can be integrated with Machine Configuration to provide a complete system configuration solution. Here's an example of how System Configuration is specified in a Machine Configuration file:

```kdl
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0" "c6t0d0"
    }
    options {
        compression "zstd"
    }
}

image "oci://aopc.cloud/openindiana/hipster:2024.12"

boot-environment-name "initial"

sysconfig {
    hostname "myhost"
    nameserver "8.8.8.8"
    nameserver "8.8.4.4"
    interface "net0" {
        address name="v4" kind="dhcp4"
    }
}
```

This Machine Configuration:
- Creates a mirrored root pool named "rpool"
- Specifies the system image to be installed
- Names the boot environment "initial"
- Configures the system with a hostname, nameservers, and a network interface

## Best Practices

When creating System Configuration files, follow these best practices:

1. **Keep It Simple**: Start with a simple configuration and add complexity as needed.

2. **Use Selectors**: Use selectors instead of hardcoded interface names to make your configuration more portable.

3. **Configure Multiple Nameservers**: Configure at least two nameservers for redundancy.

4. **Consider IPv6**: Configure IPv6 addresses when possible for better compatibility.

5. **Document Special Requirements**: Document any special network requirements or dependencies.

6. **Test Your Configuration**: Test your configuration before deploying it to production systems.

7. **Use Version Control**: Store your configuration files in a version control system to track changes over time.

## Next Steps

- Explore the [Glossary](../appendix/glossary.md) for definitions of terms
- Check the [Troubleshooting](../appendix/troubleshooting.md) guide if you encounter issues