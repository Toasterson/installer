# Machine Configuration Overview

The Machine Configuration (`machineconfig`) component is responsible for defining the overall configuration of an illumos system during installation. It provides a structured way to specify various aspects of the system, including storage, system image, and boot environment.

## Purpose

The primary purpose of the Machine Configuration component is to:

1. Define ZFS storage pools and their configuration
2. Specify the system image to be installed
3. Configure the boot environment
4. Integrate with the System Configuration (`sysconfig`) component

By providing a single, unified configuration format, the Machine Configuration component simplifies the process of defining and deploying illumos systems.

## Configuration Format

Machine Configuration uses the KDL (Kubernetes Definition Language) format for its configuration files. KDL is a human-friendly configuration language that is both easy to read and write, while also being structured enough for machine parsing.

A basic Machine Configuration file might look like this:

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

sysconfig {
    hostname "myhost"
    nameserver "8.8.8.8"
    nameserver "8.8.4.4"
    interface "net0" {
        address name="v4" kind="dhcp4"
    }
}
```

## Components

The Machine Configuration consists of several key components:

### ZFS Pools

ZFS pools are defined using the `pool` element, which specifies the name of the pool, the virtual devices (vdevs) that make up the pool, and any pool options.

For more information, see [ZFS Pool Configuration](pools.md).

### System Image

The system image to be installed is specified using the `image` element, which takes an OCI (Open Container Initiative) URL as its argument.

For more information, see [System Image](image.md).

### Boot Environment

The boot environment can be configured using the `boot-environment-name` element, which specifies the name of the boot environment.

For more information, see [Boot Environment](boot-environment.md).

### System Configuration

The system configuration is specified using the `sysconfig` element, which integrates with the System Configuration component to define system settings such as hostname, network configuration, and other aspects of system configuration.

For more information, see the [System Configuration](../sysconfig/overview.md) section.

## Implementation

The Machine Configuration component is implemented as a Rust library that uses the `knus` crate for parsing KDL files. The library defines a set of structs that represent the various components of the configuration, and provides functions for parsing and validating configuration files.

## Next Steps

- Learn about the [Configuration Format](format.md) in detail
- Understand how to configure [ZFS Pools](pools.md)
- See [Examples](examples.md) of Machine Configuration files