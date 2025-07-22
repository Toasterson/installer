# Machine Configuration Examples

This page provides examples of complete Machine Configuration files for various scenarios. These examples can be used as templates for your own configurations.

## Basic Configuration

Here's a basic Machine Configuration file that sets up a mirrored root pool, specifies a system image, and configures basic system settings:

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

This configuration:
- Creates a mirrored root pool named "rpool" using the disks "c5t0d0" and "c6t0d0" with zstd compression
- Installs the OpenIndiana Hipster 2024.12 image
- Names the boot environment "initial"
- Sets the hostname to "myhost"
- Configures two DNS nameservers
- Sets up the net0 interface to use DHCP for IPv4

## Advanced Configuration

Here's a more advanced Machine Configuration file that sets up multiple pools, specifies a custom image, and configures more detailed system settings:

```kdl
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0" "c6t0d0"
    }
    options {
        compression "zstd"
        atime "off"
    }
}

pool "data" {
    vdev "raidz" {
        disks "c7t0d0" "c8t0d0" "c9t0d0"
    }
    options {
        compression "lz4"
        atime "off"
        dedup "on"
    }
}

image "oci://registry.example.com/custom/image:1.0"

boot-environment-name "custom-initial"

sysconfig {
    hostname "server01"
    nameserver "192.168.1.1"
    nameserver "192.168.1.2"
    
    interface "net0" selector="mac:00:11:22:33:44:55" {
        address name="v4" kind="static" "192.168.1.100/24"
        address name="v6" kind="static" "2001:db8::1/64"
    }
    
    interface "net1" selector="mac:00:11:22:33:44:66" {
        address name="v4" kind="dhcp4"
        address name="v6" kind="addrconf"
    }
}
```

This configuration:
- Creates a mirrored root pool named "rpool" with zstd compression and atime disabled
- Creates a RAID-Z data pool named "data" with lz4 compression, atime disabled, and deduplication enabled
- Installs a custom image from a private registry
- Names the boot environment "custom-initial"
- Sets the hostname to "server01"
- Configures two DNS nameservers
- Sets up the net0 interface with static IPv4 and IPv6 addresses, identified by MAC address
- Sets up the net1 interface to use DHCP for IPv4 and SLAAC for IPv6, identified by MAC address

## Minimal Configuration

Here's a minimal Machine Configuration file that sets up a single disk root pool and uses defaults for most settings:

```kdl
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0"
    }
}

image "oci://aopc.cloud/openindiana/hipster:latest"

sysconfig {
    hostname "minimal"
    interface "net0" {
        address name="v4" kind="dhcp4"
    }
}
```

This configuration:
- Creates a single-disk root pool named "rpool" using the disk "c5t0d0"
- Installs the latest OpenIndiana Hipster image
- Sets the hostname to "minimal"
- Sets up the net0 interface to use DHCP for IPv4
- Uses default values for all other settings

## Server Configuration

Here's a Machine Configuration file for a server with multiple network interfaces and a focus on performance:

```kdl
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0" "c6t0d0"
    }
    options {
        compression "zstd"
        atime "off"
    }
}

pool "data" {
    vdev "raidz2" {
        disks "c7t0d0" "c8t0d0" "c9t0d0" "c10t0d0" "c11t0d0" "c12t0d0"
    }
    options {
        compression "lz4"
        atime "off"
        recordsize "128K"
    }
}

pool "log" {
    vdev "mirror" {
        disks "c13t0d0" "c14t0d0"
    }
    options {
        compression "off"
        sync "always"
    }
}

image "oci://registry.omnios.org/omnios/omnios:r151046"

boot-environment-name "omnios-r151046"

sysconfig {
    hostname "server01"
    nameserver "192.168.1.1"
    nameserver "192.168.1.2"
    
    interface "net0" selector="mac:00:11:22:33:44:55" {
        address name="v4" kind="static" "192.168.1.100/24"
    }
    
    interface "net1" selector="mac:00:11:22:33:44:66" {
        address name="v4" kind="static" "10.0.0.100/24"
    }
    
    interface "net2" selector="mac:00:11:22:33:44:77" {
        address name="v4" kind="static" "172.16.0.100/24"
    }
    
    interface "net3" selector="mac:00:11:22:33:44:88" {
        address name="v4" kind="static" "192.168.2.100/24"
    }
}
```

This configuration:
- Creates a mirrored root pool named "rpool" with zstd compression and atime disabled
- Creates a RAID-Z2 data pool named "data" with lz4 compression, atime disabled, and a larger recordsize for better performance with large files
- Creates a mirrored log pool named "log" with compression disabled and sync always enabled for better performance with synchronous writes
- Installs the OmniOS r151046 image
- Names the boot environment "omnios-r151046"
- Sets the hostname to "server01"
- Configures two DNS nameservers
- Sets up four network interfaces with static IPv4 addresses, identified by MAC address

## Development Workstation Configuration

Here's a Machine Configuration file for a development workstation with a focus on flexibility:

```kdl
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0" "c6t0d0"
    }
    options {
        compression "zstd"
    }
}

pool "data" {
    vdev "mirror" {
        disks "c7t0d0" "c8t0d0"
    }
    options {
        compression "lz4"
        atime "off"
    }
}

image "oci://aopc.cloud/openindiana/hipster-dev:2024.12"

boot-environment-name "dev-2024.12"

sysconfig {
    hostname "devbox"
    nameserver "8.8.8.8"
    nameserver "8.8.4.4"
    
    interface "net0" {
        address name="v4" kind="dhcp4"
        address name="v6" kind="dhcp6"
    }
    
    interface "wlan0" {
        address name="v4" kind="dhcp4"
        address name="v6" kind="dhcp6"
    }
}
```

This configuration:
- Creates a mirrored root pool named "rpool" with zstd compression
- Creates a mirrored data pool named "data" with lz4 compression and atime disabled
- Installs the OpenIndiana Hipster development image for 2024.12
- Names the boot environment "dev-2024.12"
- Sets the hostname to "devbox"
- Configures two DNS nameservers
- Sets up the net0 interface to use DHCP for both IPv4 and IPv6
- Sets up the wlan0 interface to use DHCP for both IPv4 and IPv6

## Next Steps

Now that you've seen examples of Machine Configuration files, you can:

- Learn more about [System Configuration](../sysconfig/overview.md)
- Explore the [Glossary](../appendix/glossary.md) for definitions of terms
- Check the [Troubleshooting](../appendix/troubleshooting.md) guide if you encounter issues