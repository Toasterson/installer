# Development tasks and toolchains with mise

This repository uses mise as a unified toolchain manager and task runner.

- Website: https://mise.jdx.dev/
- Why: consistent Rust/Node toolchains, easy-to-discover `mise run <task>` commands, and fewer ad‑hoc scripts.

## Install mise

- macOS (Homebrew): `brew install mise`
- Linux: `curl https://mise.jdx.dev/install.sh | sh`
- Other options: see the official docs above

After installing, ensure `mise` is on your PATH (restart your shell if needed).

## Trust this repository and install tools

```bash
cd /path/to/illumos/installer
# Trust local tasks in this repo
mise trust
# Install toolchains defined in .mise.toml (Rust stable, Node LTS)
mise run tools:install
# Alternatively, just: mise install
```

Toolchain defaults are defined in `.mise.toml`:
- Rust: `stable`
- Node: `lts` (the Angular UI uses Yarn 4 via Corepack)

## Common tasks

### Initial setup (image-builder and dataset)
- `mise run image:setup -- [--dataset <DS>] [--builder-dir <DIR>] [--no-build] [--update]`
  - Clones and builds `image-builder` into `~/.cargo/bin` and ensures the ZFS dataset root exists.
  - Notes:
    - Subsequent tasks resolve the `image-builder` binary robustly, preferring `~/.cargo/bin/image-builder` if present.
    - If not found, tasks will suggest running `mise run image:setup`.

Run any task with `mise run <task>`. Tasks are defined as file tasks under `.mise/tasks/`.

### VM lifecycle
- `mise run vm:help` — Show available Makefile targets for the VM
- `mise run vm:up` — Create/start the dev VM (libvirt via Makefile)
- `mise run vm:status` — Show VM status
- `mise run vm:ssh` — SSH into the VM
- `mise run vm:console` — Serial console
- `mise run vm:destroy` — Stop and undefine the VM
- `mise run vm:download` — Download the OpenIndiana cloud image
- `mise run vm:clean` — Remove generated and downloaded files
- `mise run vm:cloud-init-iso` — Build cloud-init ISO
- `mise run vm:bhyve:start -- -i <image> -r <repo>` — Start dev VM with bhyve (9P)

### Rust workspace helpers
- `mise run rust:build-all` — Build all crates (debug)
- `mise run rust:test-all` — Run tests for all crates
- `mise run rust:fmt` — Format with rustfmt
- `mise run rust:clippy` — Lint with clippy (deny warnings)
- `mise run rust:check` — Type-check all crates

These commands iterate known crates in the repo and call Cargo for each.

### Frontend (Angular)
- `mise run ui:install` — Install dependencies (Yarn 4 via Corepack)
- `mise run ui:dev` — Start dev server
- `mise run ui:build` — Build production bundle

### Dev image helpers
- `mise run dev:build-image -- -d rpool/images` — Build the development cloud image
- `mise run dev:prepare-binaries` — Prepare development binaries into dev-bin
- `mise run dev:test-setup` — Run validation checks for dev setup

### SysConfig release build tasks (cloud image)
These tasks produce a release-grade cloud image using SysConfig + cloud-init provisioning, without the legacy metadata agent or 9p share.

- Build and stage SysConfig binaries for the image:
  - `mise run build:sysconfig -- --clean`
  - Options: `--clean` (wipe staging), `--features <cargo_features>` (pass to Cargo)
  - Staging: `image/staging/sysconfig-release/...` (consumed via `-E image/staging`)

- Build bootstrap artifacts (strap pipeline):
  - `mise run image:bootstrap` (incremental)
  - `mise run image:bootstrap -- --reset full` (full reset)
  - Flags: `--with-build-tools`, `--with-extra`

- Build the cloud image with SysConfig:
  - `mise run image:cloud_sysconfig` (incremental image rebuild)
  - `mise run image:cloud_sysconfig -- full` (full reset for this image)
  - `mise run image:cloud_sysconfig -- reset` (destroy work-in-progress only)
  - Common options:
    - `--dataset <DS>` (default: `$DATASET` or `rpool/images`)
    - `--output-name <name>` (default: `ttya-openindiana-hipster`)
    - `--templates <dir>` (default: `image/templates`)
    - `--external-src <dir>` (default: `image/staging`)

- One-shot full build pipeline:
  - `mise run build:all` — default incremental (reuses bootstrap; resets cloud image only)
  - `mise run build:all full` or `mise run build:all -- --full-reset` — full reset of both bootstrap and cloud image

Notes:
- The cloud image template is `image/templates/cloudimage/ttya-openindiana-hipster.json` and includes `sysconfig-release` (manifests/config) and `sysconfig-release-bins` (copies staged binaries via `extsrc`).
- The image builder is invoked with `-E image/staging` so the include can fetch staged binaries.
- The `build:sysconfig` task discovers Cargo's `target_directory` via `cargo metadata --format-version=1 --no-deps | jq -r .target_directory` (preferred). If `jq` is unavailable, it falls back to a basic extractor. Installing `jq` is recommended for reliability: `pkg install jq` (OmniOS/OpenIndiana) or your platform's package manager.
- Outputs are written under `${DATASET}/output`. See `image/IMAGE_BUILDER.md` for details.

## Tips
- First run may prompt to download toolchains; use `mise run tools:install`.
- To see all available tasks: `mise tasks`
- You can override tool versions per‑dir with a `.mise.local.toml` (ignored by VCS).
