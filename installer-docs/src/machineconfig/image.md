# System Image Configuration

The system image is a crucial part of the Machine Configuration. It specifies the operating system image that will be installed on the system. This page provides detailed information about configuring system images in the Machine Configuration.

## Basic Image Configuration

A basic system image configuration consists of the `image` node with an OCI (Open Container Initiative) URL as its argument:

```kdl
image "oci://aopc.cloud/openindiana/hipster:2024.12"
```

This configuration specifies that the OpenIndiana Hipster 2024.12 image should be installed on the system.

## OCI URLs

The system image is specified using an OCI (Open Container Initiative) URL. OCI URLs have the following format:

```
oci://<registry>/<repository>:<tag>
```

Where:
- `<registry>` is the hostname of the OCI registry (e.g., `aopc.cloud`)
- `<repository>` is the name of the repository (e.g., `openindiana/hipster`)
- `<tag>` is the tag of the image (e.g., `2024.12`)

### Tags

Tags are used to specify the version of the system image. Common tag formats include:

- Year-based tags (e.g., `2024.12`)
- Semantic versioning tags (e.g., `r151046`)
- Special tags like `latest` or `stable`

## Image Variants

Some system images may have variants that provide different sets of packages or configurations. Variants are typically specified as part of the repository name or tag:

```kdl
image "oci://aopc.cloud/openindiana/hipster-minimal:2024.10"
```

This configuration specifies the minimal variant of the OpenIndiana Hipster 2024.10 image.

## Custom Images

You can also use custom images that you've built yourself or that are provided by your organization:

```kdl
image "oci://registry.example.com/custom/image:1.0"
```

This configuration specifies a custom image from a private registry.

## Local Images

In some cases, you may want to use a local image that's already available on the system:

```kdl
image "file:///path/to/local/image.tar"
```

This configuration specifies a local image file.

## Image Authentication

This is currently not supported. Documentation will be updated with information once it is.

## Next Steps

- Learn about [Boot Environment Configuration](boot-environment.md)
- See [Examples](examples.md) of Machine Configuration files