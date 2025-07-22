# Configuration Format

The Machine Configuration component uses the KDL (Kubernetes Definition Language) format for its configuration files. This page provides detailed information about the syntax and structure of Machine Configuration files.

## KDL Syntax

KDL is a document language with a syntax inspired by Rust, JavaScript, and TOML. It's designed to be easy to read and write, while also being structured enough for machine parsing.

### Basic Syntax

A KDL document consists of nodes. Each node has:
- A name
- Zero or more arguments
- Zero or more properties (key-value pairs)
- Zero or more child nodes

Here's a simple example:

```kdl
node "argument1" "argument2" key="value" {
    child "argument" key="value"
}
```

### Comments

KDL supports single-line comments using `//`:

```kdl
// This is a comment
node "argument" // This is also a comment
```

### Strings

Strings in KDL are enclosed in double quotes:

```kdl
node "this is a string"
```

### Numbers

Numbers in KDL can be integers or floating-point:

```kdl
node 42 3.14
```

### Booleans

Booleans in KDL are represented as `true` or `false`:

```kdl
node true false
```

### Null

Null in KDL is represented as `null`:

```kdl
node null
```

## Machine Configuration Structure

A Machine Configuration file consists of several top-level nodes:

1. `pool` - Defines a ZFS storage pool
2. `image` - Specifies the system image to be installed
3. `boot-environment-name` (optional) - Configures the boot environment
4. `sysconfig` - Integrates with the System Configuration component

### Pool Node

The `pool` node defines a ZFS storage pool:

```kdl
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0" "c6t0d0"
    }
    options {
        compression "zstd"
    }
}
```

The `pool` node has:
- An argument specifying the name of the pool
- Child `vdev` nodes defining the virtual devices that make up the pool
- An optional `options` node specifying pool options

### Image Node

The `image` node specifies the system image to be installed:

```kdl
image "oci://aopc.cloud/openindiana/hipster:2024.12"
```

The `image` node has a single argument specifying the OCI (Open Container Initiative) URL of the image.

### Boot Environment Name Node

The `boot-environment-name` node configures the boot environment:

```kdl
boot-environment-name "be-name"
```

The `boot-environment-name` node has a single argument specifying the name of the boot environment.

### SysConfig Node

The `sysconfig` node integrates with the System Configuration component:

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

The `sysconfig` node contains child nodes that define system settings such as hostname, nameservers, and network interfaces. For more information about the System Configuration format, see the [System Configuration Format](../sysconfig/format.md) page.

## Parsing

The Machine Configuration component uses the `knus` crate to parse KDL files. The `knus` crate provides a set of macros and functions for parsing KDL into Rust structs.

Here's a simplified example of how the Machine Configuration component parses KDL files:

```rust
#[derive(Debug, knus::Decode, Default)]
pub struct MachineConfig {
    #[knus(children(name = "pool"))]
    pub pools: Vec<Pool>,

    #[knus(child, unwrap(argument))]
    pub image: String,

    #[knus(child, unwrap(argument))]
    pub boot_environment_name: Option<String>,

    #[knus(child)]
    pub sysconfig: SysConfig,
}
```

## Validation

The Machine Configuration component validates configuration files when they are parsed. If a configuration file is invalid, an error will be reported.

Common validation errors include:
- Missing required nodes or arguments
- Invalid argument values
- Syntax errors

## Best Practices

When writing Machine Configuration files, follow these best practices:

1. **Use Descriptive Names**: Use descriptive names for pools, vdevs, and other elements to make the configuration easier to understand.

2. **Comment Your Configuration**: Use comments to explain complex or non-obvious parts of your configuration.

3. **Validate Before Deploying**: Test your configuration files before deploying them to production systems.

4. **Keep It Simple**: Start with a simple configuration and add complexity as needed.

5. **Use Version Control**: Store your configuration files in a version control system to track changes over time.

## Next Steps

- Learn about [ZFS Pool Configuration](pools.md)
- Understand how to configure [System Images](image.md)
- See [Examples](examples.md) of Machine Configuration files