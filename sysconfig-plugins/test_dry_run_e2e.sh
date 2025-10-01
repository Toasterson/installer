#!/bin/bash

# End-to-end test script for illumos-base-plugin dry-run mode with sysconfig
# This script starts both sysconfig and the plugin, with automatic registration

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALLER_ROOT="$(dirname "$SCRIPT_DIR")"

# Source common functions
source "${INSTALLER_ROOT}/lib/common.sh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=============================================="
echo "End-to-End Testing: sysconfig + illumos-base-plugin"
echo -e "==============================================${NC}"
echo ""

# Setup runtime directory
if [ "$EUID" -eq 0 ]; then
    echo -e "${RED}WARNING: Running as root. Dry-run mode will NOT be automatically enabled."
    echo "         The plugin will make actual system changes unless --dry-run is specified."
    echo -e "         Consider running as a non-root user for safe testing.${NC}"
    read -p "Continue as root? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
    RUNTIME_DIR="/var/run"
    SOCKET_DIR="$RUNTIME_DIR"
else
    echo -e "${GREEN}✓ Running as non-root user (UID: $EUID)${NC}"
    echo -e "${GREEN}✓ Dry-run mode will be automatically enabled in plugin${NC}"

    # Setup XDG_RUNTIME_DIR if not set
    if [ -z "$XDG_RUNTIME_DIR" ]; then
        export XDG_RUNTIME_DIR="/tmp/run-$EUID"
        echo -e "${YELLOW}ℹ XDG_RUNTIME_DIR not set, using: $XDG_RUNTIME_DIR${NC}"
    fi

    RUNTIME_DIR="$XDG_RUNTIME_DIR"
    SOCKET_DIR="$RUNTIME_DIR"

    # Create runtime directory if it doesn't exist
    if [ ! -d "$RUNTIME_DIR" ]; then
        echo "Creating runtime directory: $RUNTIME_DIR"
        mkdir -p "$RUNTIME_DIR"
        chmod 700 "$RUNTIME_DIR"
    fi
fi

# Define socket paths
SYSCONFIG_SOCKET="$SOCKET_DIR/sysconfig.sock"
PLUGIN_SOCKET="$SOCKET_DIR/sysconfig-illumos-base.sock"
STATE_FILE="$SOCKET_DIR/sysconfig-state.json"

echo ""
echo -e "${BLUE}Configuration:${NC}"
echo "  Runtime directory: $RUNTIME_DIR"
echo "  Sysconfig socket:  $SYSCONFIG_SOCKET"
echo "  Plugin socket:     $PLUGIN_SOCKET"
echo "  State file:        $STATE_FILE"
echo ""

# Function to cleanup on exit
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"

    if [ ! -z "$PLUGIN_PID" ] && ps -p $PLUGIN_PID > /dev/null 2>&1; then
        echo "Stopping illumos-base-plugin (PID: $PLUGIN_PID)..."
        kill $PLUGIN_PID 2>/dev/null || true
    fi

    if [ ! -z "$SYSCONFIG_PID" ] && ps -p $SYSCONFIG_PID > /dev/null 2>&1; then
        echo "Stopping sysconfig (PID: $SYSCONFIG_PID)..."
        kill $SYSCONFIG_PID 2>/dev/null || true
    fi

    # Clean up sockets
    rm -f "$SYSCONFIG_SOCKET" "$PLUGIN_SOCKET" 2>/dev/null || true

    echo -e "${GREEN}Cleanup complete${NC}"
}

# Set up trap for cleanup
trap cleanup EXIT INT TERM

# Get dynamic target directories
SYSCONFIG_TARGET_DIR=$(get_crate_target_dir "$INSTALLER_ROOT/sysconfig")
PLUGINS_TARGET_DIR=$(get_crate_target_dir "$SCRIPT_DIR")

# Build sysconfig
echo -e "${BLUE}Building sysconfig...${NC}"
cd "$INSTALLER_ROOT/sysconfig"
cargo build --bin sysconfig 2>&1 | grep -E "(Compiling|Finished)" || true

# Build the plugin
echo -e "${BLUE}Building illumos-base-plugin...${NC}"
cd "$SCRIPT_DIR"
cargo build --bin illumos-base-plugin 2>&1 | grep -E "(Compiling|Finished)" || true

# Build cloud-init-plugin if it exists
if [ -f "$SCRIPT_DIR/src/bin/cloud-init-plugin.rs" ]; then
    echo -e "${BLUE}Building cloud-init-plugin...${NC}"
    cargo build --bin cloud-init-plugin 2>&1 | grep -E "(Compiling|Finished)" || true
fi

echo ""

# Set logging level
export RUST_LOG=info,sysconfig=debug,illumos_base_plugin=debug

# Start sysconfig service
echo -e "${BLUE}Starting sysconfig service...${NC}"
rm -f "$SYSCONFIG_SOCKET" 2>/dev/null || true

"$SYSCONFIG_TARGET_DIR/debug/sysconfig" \
    --socket "$SYSCONFIG_SOCKET" \
    2>&1 | sed 's/^/[sysconfig] /' &

SYSCONFIG_PID=$!

# Wait for sysconfig to start
echo "Waiting for sysconfig to start..."
for i in {1..10}; do
    if [ -S "$SYSCONFIG_SOCKET" ]; then
        echo -e "${GREEN}✓ Sysconfig started (PID: $SYSCONFIG_PID)${NC}"
        break
    fi
    sleep 0.5
done

if [ ! -S "$SYSCONFIG_SOCKET" ]; then
    echo -e "${RED}ERROR: Sysconfig failed to start or create socket${NC}"
    exit 1
fi

echo ""

# Start the illumos-base-plugin (it will auto-register)
echo -e "${BLUE}Starting illumos-base-plugin...${NC}"
rm -f "$PLUGIN_SOCKET" 2>/dev/null || true

"$PLUGINS_TARGET_DIR/debug/illumos-base-plugin" \
    --socket "$PLUGIN_SOCKET" \
    --service-socket "$SYSCONFIG_SOCKET" \
    2>&1 | sed 's/^/[plugin] /' &

PLUGIN_PID=$!

# Wait for plugin to start and register
echo "Waiting for plugin to start and register..."
sleep 2

# Check if plugin is running
if ! ps -p $PLUGIN_PID > /dev/null 2>&1; then
    echo -e "${RED}ERROR: Plugin failed to start${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Plugin started (PID: $PLUGIN_PID)${NC}"
echo ""

# Display status
echo -e "${GREEN}=============================================="
echo "System is ready for testing!"
echo -e "==============================================${NC}"
echo ""
echo -e "${BLUE}Service Status:${NC}"
echo "  • sysconfig:          Running (PID: $SYSCONFIG_PID)"
echo "  • illumos-base-plugin: Running (PID: $PLUGIN_PID)"
echo ""
echo -e "${BLUE}Socket Paths:${NC}"
echo "  • Sysconfig:  $SYSCONFIG_SOCKET"
echo "  • Plugin:     $PLUGIN_SOCKET"
echo ""

if [ "$EUID" -ne 0 ]; then
    echo -e "${YELLOW}DRY-RUN MODE:${NC}"
    echo "  All plugin operations will be simulated."
    echo "  Check the [plugin] output above for dry-run messages."
    echo ""
fi

echo -e "${BLUE}Testing with cloud-init:${NC}"
if [ -f "$PLUGINS_TARGET_DIR/debug/cloud-init-plugin" ]; then
    echo "  You can now run the cloud-init plugin in another terminal:"
    echo ""
    echo "  export RUST_LOG=info"
    if [ ! -z "$XDG_RUNTIME_DIR" ]; then
        echo "  export XDG_RUNTIME_DIR='$XDG_RUNTIME_DIR'"
    fi
    echo "  $PLUGINS_TARGET_DIR/debug/cloud-init-plugin \\"
    echo "    --service-socket '$SYSCONFIG_SOCKET' \\"
    echo "    --config /path/to/cloud-init-config.yaml"
else
    echo "  Cloud-init plugin not found. You can test with other clients."
fi

echo ""
echo -e "${BLUE}Testing with sysconfig-cli:${NC}"
SYSCONFIG_CLI_TARGET_DIR=$(get_crate_target_dir "$INSTALLER_ROOT/sysconfig-cli")
if [ -f "$SYSCONFIG_CLI_TARGET_DIR/debug/sysconfig-cli" ]; then
    echo "  You can use sysconfig-cli to interact with the system:"
    echo ""
    echo "  export SYSCONFIG_SOCKET='$SYSCONFIG_SOCKET'"
    echo "  $SYSCONFIG_CLI_TARGET_DIR/debug/sysconfig-cli status"
    echo "  $SYSCONFIG_CLI_TARGET_DIR/debug/sysconfig-cli apply --file config.kdl"
fi

echo ""
echo -e "${BLUE}Example test configuration (save as test-config.json):${NC}"
cat << 'EOF'
{
  "network": {
    "settings": {
      "hostname": "test-host",
      "dns": {
        "nameservers": ["8.8.8.8", "1.1.1.1"],
        "search": ["example.com"]
      }
    }
  },
  "files": [
    {
      "path": "/etc/test-file.conf",
      "ensure": "present",
      "content": "# Test configuration\ntest_value=123\n",
      "mode": "0644",
      "uid": 0,
      "gid": 0
    }
  ]
}
EOF

echo ""
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
echo -e "${BLUE}=============================================${NC}"

# Keep the script running and show logs
wait
