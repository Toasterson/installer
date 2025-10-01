#!/bin/bash
#
# Prepare development binaries for the development VM
# This script finds the dynamically built binaries and copies them to
# predictable locations that the VM's SMF services can find
#

set -o errexit
set -o pipefail
set -o nounset

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Source common functions for dynamic target directory detection
source "${SCRIPT_DIR}/lib/common.sh"

echo -e "${BLUE}Preparing development binaries for VM...${NC}"
echo ""

# Create the dev-bin directory in the repo
DEV_BIN_DIR="${SCRIPT_DIR}/dev-bin"
mkdir -p "$DEV_BIN_DIR"

# Function to copy binary if it exists
copy_binary() {
    local crate_path="$1"
    local binary_name="$2"
    local dest_name="${3:-$binary_name}"

    echo -n "  ${binary_name}... "

    if [[ ! -d "$crate_path" ]]; then
        echo -e "${RED}SKIP (crate not found)${NC}"
        return 1
    fi

    local target_dir
    target_dir=$(get_crate_target_dir "$crate_path")

    # Try release first, then debug
    local binary_path
    if [[ -f "$target_dir/release/$binary_name" ]]; then
        binary_path="$target_dir/release/$binary_name"
        echo -e "${GREEN}OK (release)${NC}"
    elif [[ -f "$target_dir/debug/$binary_name" ]]; then
        binary_path="$target_dir/debug/$binary_name"
        echo -e "${YELLOW}OK (debug)${NC}"
    else
        echo -e "${RED}MISSING${NC}"
        echo "    Build with: cd $crate_path && cargo build [--release]"
        return 1
    fi

    cp "$binary_path" "$DEV_BIN_DIR/$dest_name"
    chmod +x "$DEV_BIN_DIR/$dest_name"
    return 0
}

# Track if any binaries are missing
missing_binaries=0

echo "Copying binaries to $DEV_BIN_DIR:"

# Copy sysconfig binary
if ! copy_binary "${SCRIPT_DIR}/sysconfig" "sysconfig"; then
    ((missing_binaries++))
fi

# Copy illumos-base-plugin binary
if ! copy_binary "${SCRIPT_DIR}/sysconfig-plugins" "illumos-base-plugin"; then
    ((missing_binaries++))
fi

# Copy provisioning-plugin binary
if ! copy_binary "${SCRIPT_DIR}/sysconfig-provisioning" "provisioning-plugin"; then
    ((missing_binaries++))
fi

echo ""

if [[ $missing_binaries -eq 0 ]]; then
    echo -e "${GREEN}✓ All binaries prepared successfully${NC}"
    echo ""
    echo "Binaries are ready in $DEV_BIN_DIR:"
    ls -la "$DEV_BIN_DIR/"
    echo ""
    echo "These will be available to the VM at /repo/dev-bin/ when mounted via 9P"
else
    echo -e "${YELLOW}⚠ $missing_binaries binaries are missing${NC}"
    echo ""
    echo "To build missing binaries, run:"
    echo "  ./dev-build.sh --build-only"
    echo ""
    echo "Or build individual components:"
    echo "  cd sysconfig && cargo build --release"
    echo "  cd sysconfig-plugins && cargo build --release"
    echo "  cd sysconfig-provisioning && cargo build --release"
    exit 1
fi
