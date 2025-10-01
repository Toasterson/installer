#!/bin/bash
#
# Development build script for illumos installer cloud image with sysconfig support
#
# This script builds the sysconfig binary and then creates a cloud image that:
# 1. Mounts a 9p filesystem from the host containing this repo
# 2. Runs sysconfig daemons as SMF services inside the VM
# 3. Allows testing modifications without rebuilding the image
#
# Usage:
#   ./dev-build.sh [options]
#
# Options:
#   -d, --dataset DATASET    ZFS dataset for image builder (required)
#   -o, --output-dir DIR     Output directory for images (optional)
#   -h, --help               Show this help message
#

set -o errexit
set -o pipefail
set -o nounset

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

# Default values
DATASET=""
OUTPUT_DIR=""

# Get dynamic target directories
IMAGE_BUILDER_TARGET_DIR=$(get_crate_target_dir "${SCRIPT_DIR}/image-builder")
IMAGE_BUILDER="${IMAGE_BUILDER_TARGET_DIR}/release/image-builder"

usage() {
    cat << EOF
Usage: $0 [options]

Development build script for illumos installer cloud image with sysconfig support.

This script builds the sysconfig binary and then creates a cloud image that:
1. Mounts a 9p filesystem from the host containing this repo
2. Runs sysconfig daemons as SMF services inside the VM
3. Allows testing modifications without rebuilding the image

Options:
  -d, --dataset DATASET    ZFS dataset for image builder (required)
  -o, --output-dir DIR     Output directory for images (optional)
  -h, --help               Show this help message

Examples:
  $0 -d rpool/images
  $0 -d tank/build -o /export/images

Prerequisites:
- ZFS dataset must exist for image builder workspace
- OpenIndiana hipster tarball must be available (built with openindiana templates)
- Rust toolchain must be installed for building sysconfig
- image-builder binary must be built

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--dataset)
            DATASET="$2"
            shift 2
            ;;
        -o|--output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Error: Unknown option $1"
            usage
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$DATASET" ]]; then
    echo "Error: Dataset is required"
    usage
    exit 1
fi

# Check if dataset exists
if ! zfs list "$DATASET" >/dev/null 2>&1; then
    echo "Error: ZFS dataset '$DATASET' does not exist"
    exit 1
fi

echo "=== Development Cloud Image Build ==="
echo "Dataset: $DATASET"
echo "Script directory: $SCRIPT_DIR"
echo ""

# Step 1: Build image-builder if it doesn't exist
echo "Step 1: Checking image-builder..."
if [[ ! -f "$IMAGE_BUILDER" ]]; then
    echo "Building image-builder..."
    cd "${SCRIPT_DIR}/image-builder"
    cargo build --release
    cd "$SCRIPT_DIR"
else
    echo "image-builder already exists"
fi

# Step 2: Build all sysconfig components
echo ""
echo "Step 2: Building sysconfig components..."

echo "  Building main sysconfig daemon..."
cd "${SCRIPT_DIR}/sysconfig"
cargo build --release
cd "$SCRIPT_DIR"

echo "  Building sysconfig-plugins..."
cd "${SCRIPT_DIR}/sysconfig-plugins"
cargo build --release
cd "$SCRIPT_DIR"

echo "  Building sysconfig-provisioning..."
cd "${SCRIPT_DIR}/sysconfig-provisioning"
cargo build --release
cd "$SCRIPT_DIR"

# Verify all binaries were created
SYSCONFIG_TARGET_DIR=$(get_crate_target_dir "${SCRIPT_DIR}/sysconfig")
SYSCONFIG_BINARY="${SYSCONFIG_TARGET_DIR}/release/sysconfig"
if [[ ! -f "${SYSCONFIG_BINARY}" ]]; then
    echo "Error: Failed to build sysconfig binary at ${SYSCONFIG_BINARY}"
    exit 1
fi

# Check for actual plugin binaries that exist
PLUGINS_TARGET_DIR=$(get_crate_target_dir "${SCRIPT_DIR}/sysconfig-plugins")
ILLUMOS_PLUGIN_BINARY="${PLUGINS_TARGET_DIR}/release/illumos-base-plugin"
if [[ ! -f "${ILLUMOS_PLUGIN_BINARY}" ]]; then
    echo "Error: Failed to build illumos-base-plugin binary at ${ILLUMOS_PLUGIN_BINARY}"
    exit 1
fi

PROVISIONING_TARGET_DIR=$(get_crate_target_dir "${SCRIPT_DIR}/sysconfig-provisioning")
PROVISIONING_PLUGIN_BINARY="${PROVISIONING_TARGET_DIR}/release/provisioning-plugin"
if [[ ! -f "${PROVISIONING_PLUGIN_BINARY}" ]]; then
    echo "Error: Failed to build provisioning-plugin binary at ${PROVISIONING_PLUGIN_BINARY}"
    exit 1
fi

# Step 3: Check for required tarball
echo ""
echo "Step 3: Checking for OpenIndiana hipster tarball..."
TARBALL_NAME="openindiana-hipster.tar.gz"
OUTPUT_PATH="${DATASET}/output"

# Check if tarball exists in dataset output
if ! ls "/$(zfs get -H -o value mountpoint $OUTPUT_PATH 2>/dev/null || echo "nonexistent")/${TARBALL_NAME}" 2>/dev/null; then
    echo "Warning: ${TARBALL_NAME} not found in dataset output."
    echo "You may need to build the OpenIndiana templates first:"
    echo ""
    echo "  $IMAGE_BUILDER build -d $DATASET -g openindiana -n hipster-01-strap"
    echo "  $IMAGE_BUILDER build -d $DATASET -g openindiana -n hipster-02-image"
    echo "  $IMAGE_BUILDER build -d $DATASET -g openindiana -n hipster-03-archive"
    echo ""
    echo "Continuing anyway - the image build will fail if the tarball is missing."
fi

# Step 4: Build the development cloud image
echo ""
echo "Step 4: Building development cloud image..."

BUILD_ARGS=(
    "build"
    "-d" "$DATASET"
    "-g" "cloudimage"
    "-n" "ttya-openindiana-hipster-dev"
    "-E" "$SCRIPT_DIR"
)

# Add output directory if specified
if [[ -n "$OUTPUT_DIR" ]]; then
    # Create output directory if it doesn't exist
    mkdir -p "$OUTPUT_DIR"
    BUILD_ARGS+=("-o" "$OUTPUT_DIR")
fi

echo "Running: $IMAGE_BUILDER ${BUILD_ARGS[*]}"
"$IMAGE_BUILDER" "${BUILD_ARGS[@]}"

echo ""
echo "=== Build Complete ==="
echo ""
echo "Development cloud image has been created with the following features:"
echo "- 9P filesystem support for mounting host directories"
echo "- sysconfig daemon installed as SMF service"
echo "- sysconfig-plugins (illumos-base-plugin) as separate SMF service"
echo "- sysconfig-provisioning oneshot CLI service"
echo "- Development configuration loading KDL from /repo"
echo ""
echo "To use the image:"
echo "1. Boot the image in bhyve or libvirt with 9p filesystem sharing"
echo "2. The VM will automatically mount the repo at /repo via 9p"
echo "3. All sysconfig components start with proper SMF dependencies"
echo "4. Configuration loaded from /repo/sysconfig-plugins/test-provisioning-config.kdl"
echo "5. Make changes to any component on the host"
echo "6. Rebuild: cd <component> && cargo build --release"
echo "7. Restart services in VM: svcadm restart svc:/system/installer/sysconfig"
echo "8. Or restart plugins: svcadm restart svc:/system/sysconfig/illumos-base"
echo "9. Or restart provisioning: svcadm restart svc:/system/sysconfig-provisioning"
echo ""
echo "Example bhyve command with 9p support:"
echo "bhyve -c 2 -m 2048M -w -H \\"
echo "  -s 0,hostbridge \\"
echo "  -s 1,lpc \\"
echo "  -s 2,ahci-hd,/path/to/image.raw \\"
echo "  -s 3,virtio-9p,repo=/path/to/repo \\"
echo "  -l com1,stdio \\"
echo "  -l bootrom,/usr/share/bhyve/BHYVE_UEFI.fd \\"
echo "  vm-name"
echo ""
echo "SMF Services in the VM:"
echo "- svc:/system/dev-9p-mount (mounts /repo)"
echo "- svc:/system/installer/sysconfig (main daemon)"
echo "- svc:/system/sysconfig/illumos-base (base plugin)"
echo "- svc:/system/sysconfig-provisioning (oneshot provisioning CLI)"
