#!/bin/bash

# Sysconfig CLI Demo Script
# This script demonstrates the various features of the sysconfig-cli tool

set -e

# Get script directory and source common utilities
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALLER_ROOT=$(dirname "$SCRIPT_DIR")
source "${INSTALLER_ROOT}/lib/common.sh"

# Get dynamic target directory
TARGET_DIR=$(get_crate_target_dir "$SCRIPT_DIR")

# Check if binary exists, build if not
DEBUG_CLI="$TARGET_DIR/debug/sysconfig-cli"
RELEASE_CLI="$TARGET_DIR/release/sysconfig-cli"

if [ ! -f "$DEBUG_CLI" ] && [ ! -f "$RELEASE_CLI" ]; then
    echo -e "${YELLOW}Building sysconfig-cli...${NC}"
    cargo build
fi

# Use release if available, debug otherwise
if [ -f "$RELEASE_CLI" ]; then
    CLI="$RELEASE_CLI"
else
    CLI="$DEBUG_CLI"
fi

# Socket path (auto-detected based on user, can be overridden by environment variable)
if [ -z "$SYSCONFIG_SOCKET" ]; then
    # Auto-detect socket path based on user, matching sysconfig service behavior
    if [ -n "$XDG_RUNTIME_DIR" ]; then
        SOCKET="$XDG_RUNTIME_DIR/sysconfig.sock"
    elif [ "$EUID" -eq 0 ]; then
        SOCKET="/var/run/sysconfig.sock"
    else
        SOCKET="/run/user/$EUID/sysconfig.sock"
    fi
else
    SOCKET="$SYSCONFIG_SOCKET"
fi

echo -e "${BLUE}=== Sysconfig CLI Demo ===${NC}\n"
echo "Using socket: $SOCKET"
echo "Using CLI: $CLI"
echo ""

# Function to run a command and show it
run_demo() {
    echo -e "${GREEN}$ $1${NC}"
    eval "$1" || true
    echo ""
    read -p "Press Enter to continue..."
    echo ""
}

# Function to show section header
section() {
    echo -e "${YELLOW}### $1 ###${NC}\n"
}

# Check if service is running
section "Checking Service Connection"
if ! $CLI --socket "$SOCKET" get --path "/" > /dev/null 2>&1; then
    echo -e "${RED}Error: Cannot connect to sysconfig service at $SOCKET${NC}"
    echo "Please ensure the sysconfig service is running."
    echo ""
    echo "To start the service in another terminal:"
    echo "  cd ../sysconfig"
    echo "  cargo run"
    echo ""
    echo "The service will automatically use the same socket path based on your user."
    exit 1
fi
echo -e "${GREEN}✓ Connected to sysconfig service${NC}\n"

# Demo 1: Get current state
section "1. Get Current State"
echo "Retrieve the entire configuration state:"
run_demo "$CLI --socket '$SOCKET' get --format pretty"

echo "Get a specific path (network configuration):"
run_demo "$CLI --socket '$SOCKET' get --path '/network' --format pretty"

# Demo 2: Set values using JSONPath
section "2. Set Values with JSONPath"
echo "Set a simple string value (hostname):"
run_demo "$CLI --socket '$SOCKET' set '$.network.hostname' '\"demo-host\"'"

echo "Set a complex object (network interface):"
run_demo "$CLI --socket '$SOCKET' set '$.network.interfaces.demo0' '{\"ip\": \"10.0.0.50\", \"netmask\": \"255.255.255.0\", \"enabled\": true}'"

echo "Dry run - preview changes without applying:"
run_demo "$CLI --socket '$SOCKET' set '$.system.timezone' '\"UTC\"' --dry-run"

# Demo 3: Apply state from file
section "3. Apply State from File"
echo "Apply the example state configuration:"
run_demo "$CLI --socket '$SOCKET' apply --file examples/state.json --dry-run"

echo "Apply with verbose output:"
run_demo "$CLI --socket '$SOCKET' apply --file examples/state.json --dry-run --verbose"

# Demo 4: Diff states
section "4. Compare States"
echo "Show differences between current state and desired state:"
run_demo "$CLI --socket '$SOCKET' diff --file examples/state.json"

# Demo 5: Watch for changes
section "5. Watch State Changes (Interactive)"
echo "This will watch for state changes in real-time."
echo "Open another terminal and make changes to see them appear here."
echo ""
echo "Example commands to run in another terminal:"
echo "  $CLI --socket '$SOCKET' set '\$.test.counter' '1'"
echo "  $CLI --socket '$SOCKET' set '\$.test.counter' '2'"
echo "  $CLI --socket '$SOCKET' set '\$.test.message' '\"Hello, World!\"'"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop watching${NC}"
read -p "Press Enter to start watching..."
$CLI --socket "$SOCKET" watch --format pretty || true

echo ""
echo -e "${BLUE}=== Demo Complete ===${NC}"
echo ""
echo "You can now use the sysconfig-cli tool to:"
echo "  • Inspect system configuration state"
echo "  • Apply configuration changes"
echo "  • Set specific values using JSONPath"
echo "  • Monitor state changes in real-time"
echo "  • Test how plugins react to state changes"
echo ""
echo "For more information, see the README.md file."
