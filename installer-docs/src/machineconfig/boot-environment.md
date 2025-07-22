# Boot Environment Configuration

Boot environments are a key feature of illumos systems that allow for safe system updates and rollbacks. The Machine Configuration component allows you to configure the boot environment for your system.

## Basic Boot Environment Configuration

A basic boot environment configuration consists of the `boot-environment-name` node with a name as its argument:

```kdl
boot-environment-name "initial"
```

This configuration specifies that the boot environment should be named "initial".

## Boot Environment Name

The boot environment name is specified as an argument to the `boot-environment-name` node:

```kdl
boot-environment-name "initial"
```

The boot environment name should be a descriptive name that identifies the purpose or state of the boot environment. Common boot environment names include:

- `initial` - The initial boot environment created during installation
- `upgrade` - A boot environment created for an upgrade
- `backup` - A backup boot environment created before making changes
- `test` - A boot environment created for testing purposes

## Optional Configuration

The `boot-environment-name` node is optional in the Machine Configuration. If it is not specified, the installer will use a default name for the boot environment, typically based on the current date and time or the name of the system image.

```kdl
// No boot-environment-name specified, a default name will be used
pool "rpool" {
    // ...
}

image "oci://aopc.cloud/openindiana/hipster:2024.12"

sysconfig {
    // ...
}
```

## Boot Environment Management

After installation, boot environments can be managed using the `beadm` command-line tool. This tool allows you to:

- Create new boot environments
- Activate boot environments
- Mount boot environments
- List boot environments
- Destroy boot environments
- Rename boot environments

For example, to list all boot environments:

```bash
beadm list
```

To create a new boot environment:

```bash
beadm create -e sourceBE targetBE
```

To activate a boot environment (which will become active on the next boot):

```bash
beadm activate targetBE
```

## Best Practices

When configuring boot environments, follow these best practices:

1. **Use Descriptive Names**: Use descriptive names for boot environments to make it easier to identify their purpose or state.

2. **Create Backups**: Create backup boot environments before making significant changes to the system. **NOTE:** IPS does this automatically during updates

3. **Keep History**: Keep a history of boot environments to allow for rollbacks if needed.

4. **Clean Up**: Regularly clean up old boot environments to free up space.

## Next Steps

- See [Examples](examples.md) of Machine Configuration files