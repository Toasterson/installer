### Purpose
This document is a hybrid how‑to and reference for authoring image‑builder templates and for generating a machine‑parsable JSON Schema describing them. It is based on the code in `src/main.rs` (Template, Step, and processing logic) and related modules.

- Template files are JSON documents.
- They can reference features/macros, include other partial templates, and conditionally run steps.
- The builder loads templates from a templates root, expands includes, applies feature guards, and executes each step.

### Where templates live
- Default root: the process looks for a templates directory (see `find_template_root` using `jmclib::dirs::rootpath("templates")`). You can override with `-T, --templates DIR`.
- File layout per group and name: `{TEMPLATES_ROOT}/{group}/{name}.json`.
- Include files can be resolved from:
    - `{group}/include/{name}.json`, or
    - `include/{name}.json` at the root.

### Template file structure (high level)
Top‑level object fields:
- `dataset` (object, optional): ZFS dataset target parameters.
- `pool` (object, optional): ZFS pool target parameters.
- `ufs` (object, optional): UFS target parameters.
- `pcfs` (object, optional): FAT (PCFS) target parameters.
- `iso` (object, optional): ISO image parameters.
- `steps` (array, required): Ordered list of step objects to execute.

Constraints:
- In a main template (not an include), you may specify at most one of `pool` and `dataset`. Both set is an error. Neither is permitted when the template’s steps do not need a pool/dataset (e.g., ISO flow).
- In an include file, `pool` and `dataset` are not allowed at all.
- Steps may be guarded by features via `with`/`without` fields; see “Feature guards and expansion”.

### Target objects reference
Dataset:
- `name` (string, required): Dataset name suffix; builder uses the temporary pool and creates `"{temp_pool}/{name}"`.
- `output_snapshot` (string, optional): Name of snapshot to create at the end.
- `input_snapshot` (string, optional): Name of snapshot to roll back to at the start.

Pool:
- `name` (string, required): Pool name.
- `size` (integer, required): Size of the lofi image (bytes). See `recreate_lofi` usage.
- `uefi` (boolean, optional, default false): Whether to create an EFI System Partition (`zpool create -B` behavior disabled when false).
- `ashift` (integer, optional, default 9): Sector size as log2 (9 = 512 B, 12 = 4 KiB).
- `bename` (string, optional): Boot environment name.
- `partition_only` (boolean, optional, default false): If true and labeling is enabled, create partitions only.
- `no_features` (boolean, optional, default true): Disable new pool features if true.
- `compression` (string, optional, default "on"): zfs compression property.
- `label` (boolean, optional, default true): Create labeled lofi device.
- `autoexpand` (boolean, optional, default false).
- `trim` (boolean, optional, default true).
- `options` (array of string, optional, default []): Extra `zpool create` options.
- `fsoptions` (array of string, optional, default []): Extra filesystem options.

Ufs:
- `size` (integer, required): Image size (bytes).
- `inode_density` (integer, required): Bytes per inode (mkfs `-i`).

Pcfs (FAT):
- `label` (string, required): Volume label.
- `size` (integer, required): Image size (bytes).

Iso:
- `boot_bios` (string, optional): Path to BIOS El Torito boot image.
- `boot_uefi` (string, optional): Path to UEFI boot image.
- `volume_id` (string, optional): ISO volume ID.
- `hybrid` (object, optional): See `Hybrid` below.

Hybrid:
- `stage1` (string, required): Path to bootblock stage1.
- `stage2` (string, required): Path to bootblock stage2.

### Steps: common fields
Every step object has at least:
- `t` (string, required): Step type discriminator.
- `with` (string, optional): Only run if feature named here is present.
- `without` (string, optional): Only run if feature named here is absent.
- Plus type‑specific fields (documented below).

The loader also supports a special step type `include` to splice another template’s steps inline.

### Step catalog and arguments
Below are the step types implemented in `run_steps` and their argument schemas. Unless specified, string fields can use feature macro expansion (see “Feature expansion” section) and filesystem paths must be absolute when targeting inside the image unless noted.

1) `include`
- Fields:
    - `t: "include"`
    - `name` (string, required) OR `file` (string, optional): The include name within the group or a fully qualified absolute path to a JSON file. You may provide only one; if both omitted, error; if `file` provided it must be absolute. `with`/`without` guards are honored before expanding `file`.
- Effect: Loads the include’s steps from `{group}/include/{name}.json` or `include/{name}.json` and inserts them. Include files must not contain `pool` or `dataset`.

2) `create_be`
- Purpose: Create a ZFS ROOT container and a boot environment dataset, mount it at the image root, set BE properties, and assign a generated UUID.
- Args: none beyond `t` and optional guards.

3) `create_dataset`
- Fields: `name` (string, required), `mountpoint` (string, optional). Creates dataset under temp pool and optionally sets mountpoint.

4) `remove_files`
- Fields: exactly one of:
    - `file` (absolute path, string), or
    - `dir` (absolute path, string), or
    - `pattern` (glob pattern, string). Pattern matches file basenames anywhere under the image root; matched files are removed.

5) `unpack_tar`
- Fields: `name` (string, required), `into_tmp` (boolean, optional, default false).
- Behavior: Unpacks an output tar file named by `name` into the image root, or into a temporary directory if `into_tmp: true` (for subsequent `ensure_file` with `tarsrc`). `.gz` suffix controls `z` flag.

6) `pack_tar`
- Fields: `name` (string, required), `include` (array of string, optional): list of relative paths inside the image root to include. If `include` omitted, packs entire image root. `.gz` suffix compresses.

7) `onu`
- Fields: `repo` (string, required), `publisher` (string, required), `uninstall` (array of string, default empty).
- Behavior: Configures an ONU publisher, sets a nightly publisher, refreshes, optionally uninstalls listed packages, sets `onu.ooceonly=false`, and `pkg update` then purge history.

8) `devfsadm`
- Fields: none. Runs `devfsadm -r` on image root to populate `/dev` scaffolding.

9) `assemble_files`
- Fields: `dir` (absolute path, string), `output` (absolute path, string), `prefix` (string, optional).
- Behavior: Concats trimmed non‑empty contents of files in `dir` (optionally filtered by filename `prefix`) in sorted order into `output`, ensuring final newline behavior like OmniOS bootadm.

10) `shadow`
- Fields: `username` (string, required), `password` (string, optional).
- Behavior: Edits `/etc/shadow` inside the image to set a user’s password hash. Leaves perms at `0400`.

11) `gzip`
- Fields: `target` (absolute path, string), `src` (relative string; resolved in output area), `owner` (string), `group` (string), `mode` (octal string, e.g., "0644").
- Behavior: Gzip‑compresses source file from the builder output area into `target` within the image and sets perms.

12) `digest`
- Fields: `algorithm` (string: `"sha1"` or `"md5"`), `target` (absolute path), `src` (relative string resolved in output area), `owner`, `group`, `mode` (octal string).
- Behavior: Writes hex digest plus newline to `target`.

13) `ensure_symlink`
- Fields: `link` (absolute path), `target` (string; symlink target as given), `owner`, `group`.

14) `ensure_perms`
- Fields: `path` (absolute path), `owner`, `group`, `mode` (octal string). Applies perms.

15) `ensure_directory` (alias: `ensure_dir`)
- Fields: `dir` (absolute path), `owner`, `group`, `mode` (octal string). Ensures directory with perms.

16) `ensure_file`
- Fields:
    - One of the following sources (exactly one required):
        - `src` (relative path under templates root or `{group}/`), or
        - `extsrc` (relative path under one of the `-E/--external-src` directories), or
        - `outputsrc` (relative path under the builder output area), or
        - `imagesrc` (absolute path inside the current image), or
        - `tarsrc` (absolute path inside the last `unpack_tar` temporary directory used with `into_tmp: true`), or
        - `contents` (string literal to write).
    - And the destination + perms:
        - `file` (absolute path, required), `owner` (string), `group` (string), `mode` (octal string).
- Errors if no source is specified. Paths must obey: `src`, `extsrc`, `outputsrc` are relative; `imagesrc`, `tarsrc`, and `file` must be absolute.

17) `make_bootable`
- Fields: none. Sets ZFS bootfs, activates BE, installs bootloader, updates archive.

18) `pkg_image_create`
- Fields: `publisher` (string, optional), `uri` (string, optional). Either provide both or neither. If both provided, initializes an image root with that publisher.

19) `pkg_install`
- Fields: `pkgs` (array of string, required; each supports macro expansion), `include_optional` (boolean, default false), `strip_optional_publishers` (boolean, optional; default true when `include_optional` is true). Installs packages; can resolve and include optional dependencies.

20) `pkg_set_property`
- Fields: `name` (string), `value` (string). Runs `pkg set-property` inside image.

21) `pkg_set_publisher`
- Fields:
    - `publisher` (string, required)
    - Either `uri` (string) or `uris` (array of string), but not both.
    - Either `mirror_uri` (string) or `mirror_uris` (array of string), but not both.
- At least one of origins or mirrors must be provided. The logic atomically replaces origins/mirrors as appropriate.

22) `pkg_approve_ca_cert`
- Fields: `publisher` (string, required), `certfile` (relative path under templates, required). Approves a CA cert for the publisher.

23) `pkg_uninstall`
- Fields: `pkgs` (array of string, required). Uninstalls listed packages.

24) `pkg_change_variant`
- Fields: `variant` (string), `value` (string). Ensures IPS variant value.

25) `pkg_change_facet`
- Fields: `facet` (string), `value` (string). Ensures IPS facet value.

26) `pkg_purge_history`
- Fields: none. Runs `pkg purge-history`.

27) `seed_smf`
- Fields:
    - `debug` (boolean, optional, default false)
    - `apply_site` (boolean, optional, default false)
    - `apply_profiles` (array of string, default empty)
    - `seed` (string, optional; default "global")
    - `skip_seed` (boolean, optional, default false)
- Constraints:
    - `apply_site: true` is mutually exclusive with any non‑empty `apply_profiles`.
    - Profiles are validated to be among: `generic`, `platform`, `site` (as of current code).
    - Empty strings in `apply_profiles` are ignored (enables optional macro‑driven profiles).

### Feature guards and expansion
- Any step can include `with` and/or `without`:
    - `with: "feat"` runs the step only if feature `feat` exists.
    - `without: "feat"` runs only if `feat` does not exist.
- Macro expansion:
    - Many string fields are expanded via `Features.expand`/`expandm` before use. Single‑value features are supported in single string contexts; lists use `expandm` and may expand to zero, one, or multiple values.

### Authoring a simple template
Minimal example (creates a directory, writes a file, sets perms):
```json
{
  "steps": [
    { "t": "ensure_dir", "dir": "/etc/myapp", "owner": "root", "group": "root", "mode": "0755" },
    { "t": "ensure_file", "file": "/etc/myapp/config.ini", "owner": "root", "group": "root", "mode": "0644", "contents": "key=value\n" }
  ]
}
```

Example with include and features:
```json
{
  "pool": { "name": "rpool", "size": 10737418240 },
  "steps": [
    { "t": "create_be" },
    { "t": "pkg_image_create", "publisher": "example", "uri": "https://pkg.example.org/" },
    { "t": "pkg_install", "pkgs": ["group/system/posix"] },
    { "t": "include", "name": "common" },
    { "t": "ensure_file", "with": "debug", "file": "/etc/motd", "owner": "root", "group": "root", "mode": "0644", "contents": "Debug build\n" }
  ]
}
```

### JSON Schema for templates
Below is a JSON Schema (Draft 2020‑12) that captures the current format. You can save it as `schema/template.schema.json`. This schema enforces the discriminator `t` on steps, the mutual exclusions noted above, and common constraints. Adjust patterns/enum values as your policy requires.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://example.org/image-builder/template.schema.json",
  "title": "Image Builder Template",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "dataset": { "$ref": "#/$defs/Dataset" },
    "pool": { "$ref": "#/$defs/Pool" },
    "ufs": { "$ref": "#/$defs/Ufs" },
    "pcfs": { "$ref": "#/$defs/Pcfs" },
    "iso": { "$ref": "#/$defs/Iso" },
    "steps": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/$defs/Step" }
    }
  },
  "required": ["steps"],
  "allOf": [
    {
      "if": { "required": ["pool", "dataset"] },
      "then": { "description": "pool and dataset cannot both be present", "not": {} }
    }
  ],
  "$defs": {
    "Dataset": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "name": { "type": "string", "minLength": 1 },
        "output_snapshot": { "type": "string" },
        "input_snapshot": { "type": "string" }
      },
      "required": ["name"]
    },
    "Pool": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "name": { "type": "string", "minLength": 1 },
        "size": { "type": "integer", "minimum": 1 },
        "uefi": { "type": "boolean" },
        "ashift": { "type": "integer", "minimum": 0 },
        "bename": { "type": "string" },
        "partition_only": { "type": "boolean" },
        "no_features": { "type": "boolean" },
        "compression": { "type": "string" },
        "label": { "type": "boolean" },
        "autoexpand": { "type": "boolean" },
        "trim": { "type": "boolean" },
        "options": { "type": "array", "items": { "type": "string" } },
        "fsoptions": { "type": "array", "items": { "type": "string" } }
      },
      "required": ["name", "size"]
    },
    "Ufs": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "size": { "type": "integer", "minimum": 1 },
        "inode_density": { "type": "integer", "minimum": 1 }
      },
      "required": ["size", "inode_density"]
    },
    "Pcfs": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "label": { "type": "string" },
        "size": { "type": "integer", "minimum": 1 }
      },
      "required": ["label", "size"]
    },
    "Iso": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "boot_bios": { "type": ["string", "null"] },
        "boot_uefi": { "type": ["string", "null"] },
        "volume_id": { "type": ["string", "null"] },
        "hybrid": { "$ref": "#/$defs/Hybrid" }
      }
    },
    "Hybrid": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "stage1": { "type": "string" },
        "stage2": { "type": "string" }
      },
      "required": ["stage1", "stage2"]
    },
    "StepBase": {
      "type": "object",
      "properties": {
        "t": { "type": "string" },
        "with": { "type": "string" },
        "without": { "type": "string" }
      },
      "required": ["t"],
      "additionalProperties": true
    },
    "Step": {
      "allOf": [
        { "$ref": "#/$defs/StepBase" },
        {
          "oneOf": [
            { "$ref": "#/$defs/StepInclude" },
            { "$ref": "#/$defs/StepCreateBe" },
            { "$ref": "#/$defs/StepCreateDataset" },
            { "$ref": "#/$defs/StepRemoveFiles" },
            { "$ref": "#/$defs/StepUnpackTar" },
            { "$ref": "#/$defs/StepPackTar" },
            { "$ref": "#/$defs/StepOnu" },
            { "$ref": "#/$defs/StepDevfsadm" },
            { "$ref": "#/$defs/StepAssembleFiles" },
            { "$ref": "#/$defs/StepShadow" },
            { "$ref": "#/$defs/StepGzip" },
            { "$ref": "#/$defs/StepDigest" },
            { "$ref": "#/$defs/StepEnsureSymlink" },
            { "$ref": "#/$defs/StepEnsurePerms" },
            { "$ref": "#/$defs/StepEnsureDir" },
            { "$ref": "#/$defs/StepEnsureFile" },
            { "$ref": "#/$defs/StepMakeBootable" },
            { "$ref": "#/$defs/StepPkgImageCreate" },
            { "$ref": "#/$defs/StepPkgInstall" },
            { "$ref": "#/$defs/StepPkgSetProperty" },
            { "$ref": "#/$defs/StepPkgSetPublisher" },
            { "$ref": "#/$defs/StepPkgApproveCaCert" },
            { "$ref": "#/$defs/StepPkgUninstall" },
            { "$ref": "#/$defs/StepPkgChangeVariant" },
            { "$ref": "#/$defs/StepPkgChangeFacet" },
            { "$ref": "#/$defs/StepPkgPurgeHistory" },
            { "$ref": "#/$defs/StepSeedSmf" }
          ]
        }
      ]
    },

    "AbsPath": { "type": "string", "pattern": "^/" },
    "Octal": { "type": "string", "pattern": "^[0-7]{3,4}$" },

    "StepInclude": {
      "type": "object",
      "properties": { "t": { "const": "include" }, "name": { "type": "string" }, "file": { "type": "string", "pattern": "^/" } },
      "additionalProperties": false,
      "oneOf": [ { "required": ["name"] }, { "required": ["file"] } ]
    },
    "StepCreateBe": { "type": "object", "properties": { "t": { "const": "create_be" } }, "additionalProperties": false },
    "StepCreateDataset": {
      "type": "object",
      "properties": { "t": { "const": "create_dataset" }, "name": { "type": "string" }, "mountpoint": { "type": "string" } },
      "required": ["name"], "additionalProperties": false
    },
    "StepRemoveFiles": {
      "type": "object",
      "properties": { "t": { "const": "remove_files" }, "file": { "$ref": "#/$defs/AbsPath" }, "dir": { "$ref": "#/$defs/AbsPath" }, "pattern": { "type": "string" } },
      "additionalProperties": false,
      "oneOf": [ { "required": ["file"] }, { "required": ["dir"] }, { "required": ["pattern"] } ]
    },
    "StepUnpackTar": { "type": "object", "properties": { "t": { "const": "unpack_tar" }, "name": { "type": "string" }, "into_tmp": { "type": "boolean" } }, "required": ["name"], "additionalProperties": false },
    "StepPackTar": { "type": "object", "properties": { "t": { "const": "pack_tar" }, "name": { "type": "string" }, "include": { "type": "array", "items": { "type": "string" } } }, "required": ["name"], "additionalProperties": false },
    "StepOnu": { "type": "object", "properties": { "t": { "const": "onu" }, "repo": { "type": "string" }, "publisher": { "type": "string" }, "uninstall": { "type": "array", "items": { "type": "string" }, "default": [] } }, "required": ["repo", "publisher"], "additionalProperties": false },
    "StepDevfsadm": { "type": "object", "properties": { "t": { "const": "devfsadm" } }, "additionalProperties": false },
    "StepAssembleFiles": { "type": "object", "properties": { "t": { "const": "assemble_files" }, "dir": { "$ref": "#/$defs/AbsPath" }, "output": { "$ref": "#/$defs/AbsPath" }, "prefix": { "type": "string" } }, "required": ["dir", "output"], "additionalProperties": false },
    "StepShadow": { "type": "object", "properties": { "t": { "const": "shadow" }, "username": { "type": "string" }, "password": { "type": "string" } }, "required": ["username"], "additionalProperties": false },
    "StepGzip": { "type": "object", "properties": { "t": { "const": "gzip" }, "target": { "$ref": "#/$defs/AbsPath" }, "src": { "type": "string", "not": { "pattern": "^/" } }, "owner": { "type": "string" }, "group": { "type": "string" }, "mode": { "$ref": "#/$defs/Octal" } }, "required": ["target", "src", "owner", "group", "mode"], "additionalProperties": false },
    "StepDigest": { "type": "object", "properties": { "t": { "const": "digest" }, "algorithm": { "type": "string", "enum": ["sha1", "md5"] }, "target": { "$ref": "#/$defs/AbsPath" }, "src": { "type": "string", "not": { "pattern": "^/" } }, "owner": { "type": "string" }, "group": { "type": "string" }, "mode": { "$ref": "#/$defs/Octal" } }, "required": ["algorithm", "target", "src", "owner", "group", "mode"], "additionalProperties": false },
    "StepEnsureSymlink": { "type": "object", "properties": { "t": { "const": "ensure_symlink" }, "link": { "$ref": "#/$defs/AbsPath" }, "target": { "type": "string" }, "owner": { "type": "string" }, "group": { "type": "string" } }, "required": ["link", "target", "owner", "group"], "additionalProperties": false },
    "StepEnsurePerms": { "type": "object", "properties": { "t": { "const": "ensure_perms" }, "path": { "$ref": "#/$defs/AbsPath" }, "owner": { "type": "string" }, "group": { "type": "string" }, "mode": { "$ref": "#/$defs/Octal" } }, "required": ["path", "owner", "group", "mode"], "additionalProperties": false },
    "StepEnsureDir": { "type": "object", "properties": { "t": { "enum": ["ensure_directory", "ensure_dir"] }, "dir": { "$ref": "#/$defs/AbsPath" }, "owner": { "type": "string" }, "group": { "type": "string" }, "mode": { "$ref": "#/$defs/Octal" } }, "required": ["dir", "owner", "group", "mode"], "additionalProperties": false },
    "StepEnsureFile": {
      "type": "object",
      "properties": {
        "t": { "const": "ensure_file" },
        "file": { "$ref": "#/$defs/AbsPath" },
        "owner": { "type": "string" },
        "group": { "type": "string" },
        "mode": { "$ref": "#/$defs/Octal" },
        "src": { "type": "string", "not": { "pattern": "^/" } },
        "imagesrc": { "$ref": "#/$defs/AbsPath" },
        "tarsrc": { "$ref": "#/$defs/AbsPath" },
        "outputsrc": { "type": "string", "not": { "pattern": "^/" } },
        "extsrc": { "type": "string", "not": { "pattern": "^/" } },
        "contents": { "type": "string" }
      },
      "required": ["file", "owner", "group", "mode"],
      "additionalProperties": false,
      "oneOf": [
        { "required": ["src"] },
        { "required": ["imagesrc"] },
        { "required": ["tarsrc"] },
        { "required": ["outputsrc"] },
        { "required": ["extsrc"] },
        { "required": ["contents"] }
      ]
    },
    "StepMakeBootable": { "type": "object", "properties": { "t": { "const": "make_bootable" } }, "additionalProperties": false },
    "StepPkgImageCreate": { "type": "object", "properties": { "t": { "const": "pkg_image_create" }, "publisher": { "type": "string" }, "uri": { "type": "string" } }, "additionalProperties": false },
    "StepPkgInstall": { "type": "object", "properties": { "t": { "const": "pkg_install" }, "pkgs": { "type": "array", "items": { "type": "string" } }, "include_optional": { "type": "boolean" }, "strip_optional_publishers": { "type": "boolean" } }, "required": ["pkgs"], "additionalProperties": false },
    "StepPkgSetProperty": { "type": "object", "properties": { "t": { "const": "pkg_set_property" }, "name": { "type": "string" }, "value": { "type": "string" } }, "required": ["name", "value"], "additionalProperties": false },
    "StepPkgSetPublisher": {
      "type": "object",
      "properties": {
        "t": { "const": "pkg_set_publisher" },
        "publisher": { "type": "string" },
        "uri": { "type": "string" },
        "uris": { "type": "array", "items": { "type": "string" } },
        "mirror_uri": { "type": "string" },
        "mirror_uris": { "type": "array", "items": { "type": "string" } }
      },
      "required": ["publisher"],
      "allOf": [
        { "not": { "required": ["uri", "uris"] } },
        { "not": { "required": ["mirror_uri", "mirror_uris"] } }
      ],
      "additionalProperties": false
    },
    "StepPkgApproveCaCert": { "type": "object", "properties": { "t": { "const": "pkg_approve_ca_cert" }, "publisher": { "type": "string" }, "certfile": { "type": "string", "not": { "pattern": "^/" } } }, "required": ["publisher", "certfile"], "additionalProperties": false },
    "StepPkgUninstall": { "type": "object", "properties": { "t": { "const": "pkg_uninstall" }, "pkgs": { "type": "array", "items": { "type": "string" } } }, "required": ["pkgs"], "additionalProperties": false },
    "StepPkgChangeVariant": { "type": "object", "properties": { "t": { "const": "pkg_change_variant" }, "variant": { "type": "string" }, "value": { "type": "string" } }, "required": ["variant", "value"], "additionalProperties": false },
    "StepPkgChangeFacet": { "type": "object", "properties": { "t": { "const": "pkg_change_facet" }, "facet": { "type": "string" }, "value": { "type": "string" } }, "required": ["facet", "value"], "additionalProperties": false },
    "StepPkgPurgeHistory": { "type": "object", "properties": { "t": { "const": "pkg_purge_history" } }, "additionalProperties": false },
    "StepSeedSmf": {
      "type": "object",
      "properties": {
        "t": { "const": "seed_smf" },
        "debug": { "type": "boolean" },
        "apply_site": { "type": "boolean" },
        "apply_profiles": { "type": "array", "items": { "type": "string" } },
        "seed": { "type": "string" },
        "skip_seed": { "type": "boolean" }
      },
      "additionalProperties": false
    }
  }
}
```

Notes on schema fidelity:
- The runtime enforces `pool`/`dataset` exclusivity differently for includes vs. mains. This schema flags both present as invalid but does not distinguish includes. You can maintain separate schemas for includes if you want stricter validation.
- Owner/group are strings resolved at runtime to numeric IDs; schema leaves them as strings.
- Octal modes are represented as strings (e.g., `"0644"`).

### How to generate and ship the schema JSON file
- Copy the schema above into your repo at, for example, `docs/template.schema.json`.
- Optionally publish it at a stable URL and update the `$id` accordingly.
- If you want the builder to emit its schema automatically in the future, consider adding a `--print-schema` CLI that prints this JSON; for now, manual management is fine.

### Validating templates against the schema
Using Node.js `ajv` (v8+):
```bash
npm i -g ajv-cli
ajv validate -s docs/template.schema.json -d templates/mygroup/mytemplate.json --strict=false
```

Using Python `jsonschema`:
```bash
python - <<'PY'
import json, sys
from jsonschema import Draft202012Validator
schema = json.load(open('docs/template.schema.json'))
doc = json.load(open('templates/mygroup/mytemplate.json'))
Draft202012Validator(schema).validate(doc)
print('OK')
PY
```

### Tips, pitfalls, and best practices
- Keep includes small and focused; they cannot contain `pool`/`dataset`.
- Use `with`/`without` to create feature‑specific variants instead of duplicating templates.
- Prefer `ensure_dir` and `ensure_file` over running ad‑hoc shell commands; the builder normalizes permissions and handles idempotency.
- When using `ensure_file` sources, double‑check absolute vs relative rules:
    - Relative: `src`, `extsrc`, `outputsrc`.
    - Absolute: `imagesrc`, `tarsrc`, dest `file`.
- `pack_tar` and `unpack_tar` work together to move artifacts between builds (e.g., create an ESP in one build, consume in another via `outputsrc`).
- `pkg_set_publisher` with a single origin will use `-O` to replace origins atomically; with multiple origins it clears and re‑adds.

### Appendix: Full template example
```json
{
  "pool": { "name": "rpool", "size": 17179869184, "uefi": true, "bename": "omnios" },
  "iso": { "volume_id": "OMNIOS", "boot_uefi": "boot/uefi.img" },
  "steps": [
    { "t": "create_be" },
    { "t": "pkg_image_create", "publisher": "omnios", "uri": "https://pkg.omnios.org/" },
    { "t": "pkg_install", "pkgs": ["entire@r151048"], "include_optional": true },
    { "t": "ensure_dir", "dir": "/etc/rc3.d", "owner": "root", "group": "root", "mode": "0755" },
    { "t": "ensure_file", "src": "common/motd.txt", "file": "/etc/motd", "owner": "root", "group": "root", "mode": "0644" },
    { "t": "devfsadm" },
    { "t": "make_bootable" },
    { "t": "pack_tar", "name": "rpool.tar.gz", "include": ["boot", "usr", "etc"] }
  ]
}
```

If you’d like, I can also drop the schema into your repo at a path you prefer or split it into main/include variants. Would you like me to do that now?