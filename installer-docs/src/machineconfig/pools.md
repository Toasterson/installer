# ZFS Pool Configuration

ZFS pools are a fundamental part of the storage configuration in illumos systems. The Machine Configuration component allows you to define ZFS pools with various configurations to meet your storage needs.

## Basic Pool Configuration

A basic ZFS pool configuration consists of a pool name and one or more virtual devices (vdevs):

```kdl
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0" "c6t0d0"
    }
}
```

This configuration creates a mirrored pool named "rpool" using the disks "c5t0d0" and "c6t0d0".

## Pool Name

The pool name is specified as an argument to the `pool` node:

```kdl
pool "rpool" {
    // ...
}
```

The pool name should be a descriptive name that identifies the purpose of the pool. Common pool names include:

- `rpool` - The root pool, which contains the operating system
- `data` - A pool for storing data
- `backup` - A pool for backups

## Virtual Devices (vdevs)

Virtual devices (vdevs) are the building blocks of ZFS pools. Each vdev consists of one or more physical devices (disks) arranged in a specific configuration.

The `vdev` node specifies the type of vdev and the disks that make up the vdev:

```kdl
vdev "mirror" {
    disks "c5t0d0" "c6t0d0"
}
```

### Vdev Types

The Machine Configuration component supports the following vdev types:

- `mirror` - A mirrored vdev, which provides redundancy by storing a copy of the data on each disk
- `raidz` - A RAID-Z vdev, which provides redundancy similar to RAID-5
- `raidz1` - Equivalent to `raidz`
- `raidz2` - A RAID-Z2 vdev, which provides redundancy similar to RAID-6
- `raidz3` - A RAID-Z3 vdev, which provides even more redundancy than RAID-Z2
- `spare` - A spare vdev, which contains disks that can be used as replacements for failed disks
- `log` - A log vdev, which contains devices used for the ZFS Intent Log (ZIL)
- `cache` - A cache vdev, which contains devices used for the L2ARC (Level 2 Adaptive Replacement Cache)

### Disks

The `disks` node specifies the physical devices that make up the vdev:

```kdl
disks "c5t0d0" "c6t0d0"
```

Each argument to the `disks` node is a device identifier. The format of the device identifier depends on the system, but common formats include:

- `c#t#d#` - The traditional illumos device identifier (e.g., `c0t0d0`)
- `/dev/dsk/c#t#d#s#` - The full path to the device (e.g., `/dev/dsk/c0t0d0s0`)
- `/dev/zvol/dsk/pool/volume` - The path to a ZFS volume (e.g., `/dev/zvol/dsk/rpool/swap`)

## Pool Options

Pool options can be specified using the `options` node:

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
```

Each child node of the `options` node specifies a pool option. The node name is the option name, and the argument is the option value.

### Common Pool Options

Here are some common pool options:

- `compression` - The compression algorithm to use (e.g., `lz4`, `gzip`, `zstd`)
- `atime` - Whether to update access times on files (`on` or `off`)
- `dedup` - Whether to enable deduplication (`on` or `off`)
- `autoreplace` - Whether to automatically replace failed devices with spares (`on` or `off`)

## Multiple Pools

You can define multiple pools in a single configuration file:

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
    vdev "raidz" {
        disks "c7t0d0" "c8t0d0" "c9t0d0"
    }
    options {
        compression "lz4"
        atime "off"
    }
}
```

This configuration creates two pools: a mirrored pool named "rpool" and a RAID-Z pool named "data".

## Best Practices

When configuring ZFS pools, follow these best practices:

1. **Use Redundancy**: Use mirrored or RAID-Z vdevs to provide redundancy and protect against disk failures.

2. **Match Disk Sizes**: Use disks of the same size in a vdev to avoid wasting space.

3. **Consider Performance**: Different vdev types have different performance characteristics. Mirrored vdevs generally provide better performance for random I/O, while RAID-Z vdevs provide better space efficiency.

4. **Use Appropriate Options**: Choose pool options that match your workload. For example, use `compression` to save space, but be aware that it may impact performance.

5. **Plan for Growth**: Consider how you will expand the pool in the future. Adding new vdevs is generally better than replacing disks with larger ones.

## Next Steps

- Learn about [System Image Configuration](image.md)
- Understand how to configure [Boot Environments](boot-environment.md)
- See [Examples](examples.md) of Machine Configuration files