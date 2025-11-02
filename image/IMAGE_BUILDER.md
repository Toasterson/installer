# illumos Image Builder — User Guide

This directory contains templates and helper scripts for building illumos-based images using the `image-builder` tool. The `image-builder` is a template-driven utility that can assemble:

- ISO images for installers or live media
- UFS/FAT filesystems (for boot archives or El Torito images)
- ZFS pools or datasets (to stage roots and pack tars)

The scripts here wrap `image-builder` with the correct template root and sane defaults for this project.

## Quick start

1) Ensure prerequisites

- An illumos system with ZFS (needs privilege for ZFS operations)
- Rust toolchain (`cargo`) in your PATH
- Ability to run privileged commands (these scripts use `pfexec`)

2) Install the `image-builder` binary and create the working dataset

Using mise (recommended):
```
mise run image:setup -- [--dataset <DS>] [--builder-dir <DIR>] [--no-build] [--update]
```

The `image:setup` task will:
- Clone `https://github.com/illumos/image-builder` into `~/image-builder` (or `--builder-dir`) if needed
- Build and install the `image-builder` binary into `~/.cargo/bin`
- Ensure the root ZFS dataset exists (defaults to `${DATASET:-rpool/images}`, override with `--dataset`)

Legacy script `image/setup.sh` is deprecated. Please use the mise task above.

3) Configure (optional)

Defaults live in `image/etc/defaults.sh`. You can override them by creating `image/etc/config.sh`. For example, to use a different ZFS dataset root:

```
# image/etc/config.sh
DATASET=tank/images
```

4) Build common artifacts

- Build the multi-stage ramdisk and installer content (strap pipeline):

```
# from repo root
mise run image:bootstrap -- [--reset none|full] [--with-build-tools] [--with-extra]
```

Flags:
- `--reset full` Full reset (destroys prior work-in-progress datasets for a clean rebuild)
- `--with-build-tools` Include software build tools (enables `build` feature)
- `--with-extra` Enable OmniOS Extra publisher (enables `extra` feature)

- Build the boot archive (UFS):

```
cd image
./boot_archive.sh
```

- Build the ISO (including El Torito EFI content):

```
cd image
./iso.sh
```

Outputs will be written under the `output` dataset within your configured dataset root. For example, if `DATASET=rpool/images` the output ZFS dataset is `rpool/images/output`. To find its mountpoint:

```
pfexec zfs get -Ho value mountpoint rpool/images/output
```

## Direct `image-builder` usage

The scripts above are thin wrappers. You can also invoke `image-builder` directly. The primary subcommand is `build` and it accepts these options:

Required options:
- `-g, --group GROUPNAME` — template group (e.g. `installer`)
- `-n, --name IMAGENAME` — template name (e.g. `generic-iso`, `eltorito-efi`)
- `-d, --dataset DATASET` — ZFS dataset root to use for work/output (e.g. `rpool/images`)

Common optional options:
- `-N, --output-name IMAGENAME` — overrides the default output name
- `-T, --templates DIR` — template root directory (use `image/templates` from this repo)
- `-F, --feature` — add/remove feature definitions; supports multiple, e.g. `-F name=installer`, `-F extra`, `-F build`
- `-E, --external-src DIR` — additional source directory to locate external files; can be given multiple times
- `-r, --reset` — destroy any work-in-progress dataset for this output
- `-x, --fullreset` — destroy the full work dataset for this output
- `-S, --svccfg PATH` — path to `svccfg` (or `svccfg-native` from an illumos build); defaults to `/usr/sbin/svccfg`

Example: build the generic installer ISO components from this repository’s templates

```
cd image
pfexec image-builder \
  build \
  -d "${DATASET:-rpool/images}" \
  -g installer \
  -n generic-iso \
  -T "$PWD/templates" \
  -N generic \
  -F name=installer
```

Notes:
- Template root discovery: if `-T` is not specified, the tool attempts to locate a `templates` directory relative to the binary. In this repository, prefer passing `-T "$PWD/templates"` from the `image/` directory.
- Outputs: files are created under `${DATASET}/output` with names derived from `group` and `output-name`, e.g. `installer-generic.iso`, `installer-eltorito-efi.pcfs`, or `installer-generic-ttya-ufs.ufs` (depending on template type).

## Template layout in this repo

Templates live under `image/templates/` and are grouped by the `group` argument.

Examples:
- `image/templates/installer/eltorito-efi.json`
- `image/templates/installer/generic-iso.json`
- `image/templates/installer/ramdisk-01-strap.json`
- `image/templates/openindiana/hipster-01-strap.json`

Template JSON describes build steps (creating datasets, unpacking tars, setting SMF manifests, packaging, assembling ISOs, etc.) that `image-builder` executes.

## Troubleshooting

- Missing dataset: create it with `pfexec zfs create <DATASET>` or run `mise run image:setup`.
- Permission errors: ensure you are using `pfexec` (or an equivalent with the required ZFS and loopback device privileges).
- `image-builder` not found: run `mise run image:setup` to install it, or pass `IMAGE_BUILDER=/absolute/path/to/image-builder`. Tasks also search `~/.cargo/bin/image-builder` automatically.
- SMF/`svccfg` issues: pass `-S /path/to/svccfg-native` built from your illumos tree if system `svccfg` is incompatible.

## See also

- Scripts in this directory: `setup.sh`, `boot_archive.sh`, `iso.sh`
- Task: use `mise run image:bootstrap` for the strap pipeline
- Upstream template references (historical inspiration):
  - https://github.com/oxidecomputer/helios-engvm/blob/main/image/templates
  - https://github.com/jclulow/omnios-image-builder/blob/main/templates
