# Hostname Configuration

The hostname is a fundamental part of system configuration that identifies the system on a network. This page provides detailed information about configuring the hostname in the System Configuration component.

## Basic Hostname Configuration

A basic hostname configuration consists of the `hostname` element with a name as its argument:

```
hostname my-host
```

This configuration sets the system hostname to "my-host".

## Hostname Format

The hostname should follow the rules for valid hostnames in DNS:

1. It can contain letters (a-z, A-Z), digits (0-9), and hyphens (-).
2. It cannot start or end with a hyphen.
3. It cannot contain other special characters, including spaces.
4. It should be no longer than 63 characters.

Examples of valid hostnames:
- `myhost`
- `my-host`
- `host123`
- `server-01`

Examples of invalid hostnames:
- `-myhost` (starts with a hyphen)
- `myhost-` (ends with a hyphen)
- `my_host` (contains an underscore)
- `my host` (contains a space)

## Fully Qualified Domain Names

You can also use a fully qualified domain name (FQDN) as the hostname:

```
hostname myhost.example.com
```

This configuration sets the system hostname to "myhost.example.com".

When using an FQDN, the same rules apply, with the addition that:
1. The FQDN consists of one or more labels separated by dots.
2. Each label follows the hostname rules above.
3. The total length of the FQDN should not exceed 255 characters.

## Hostname Resolution

The hostname is used in various ways on the system:

1. It is stored in the `/etc/hostname` file.
2. It is used by the `hostname` command.
3. It is often mapped to the loopback address (127.0.0.1) in the `/etc/hosts` file.

The System Configuration component ensures that the hostname is properly configured in all these places.

## Integration with DNS

For proper network operation, the hostname should be resolvable to the system's IP address through DNS or the local hosts file. This is especially important for services that rely on reverse DNS lookups.

The System Configuration component does not automatically configure DNS records for the hostname. You need to ensure that the hostname is properly registered in your DNS system or added to the `/etc/hosts` file on other systems that need to communicate with this system.

## Best Practices

When configuring the hostname, follow these best practices:

1. **Use Descriptive Names**: Choose a hostname that describes the purpose or role of the system.

2. **Be Consistent**: Use a consistent naming scheme across all your systems.

3. **Avoid Special Characters**: Stick to letters, digits, and hyphens to avoid compatibility issues.

4. **Consider DNS Integration**: Ensure that the hostname can be properly resolved in your network environment.

5. **Use Lowercase**: While hostnames are case-insensitive in DNS, it's a good practice to use lowercase for consistency.

6. **Include Domain Information**: In environments with multiple domains, consider using FQDNs to avoid ambiguity.

## Examples

### Basic Hostname

```
hostname server01
```

This sets the hostname to "server01".

### Hostname with Domain

```
hostname server01.example.com
```

This sets the hostname to "server01.example.com".

### Hostname in a Complete Configuration

```
hostname webserver

nameserver 8.8.8.8
nameserver 8.8.4.4

interface eth0 {
    address {
        name = eth0
        kind = Static
        192.168.1.100/24
    }
}
```

This configuration:
- Sets the hostname to "webserver"
- Configures two DNS nameservers
- Sets up the eth0 interface with a static IP address

## Next Steps

- Learn about [Nameserver Configuration](nameservers.md)
- Understand how to configure [Network Interfaces](interfaces.md)
- See [Examples](examples.md) of System Configuration files