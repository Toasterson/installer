#!/bin/bash

# Test script for the new CLI-based sysconfig provisioning tool
# This script demonstrates the new workflow where provisioning is a CLI
# that writes to sysconfig rather than a plugin server

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROVISIONING_ROOT="$SCRIPT_DIR"
INSTALLER_ROOT="$(dirname "$PROVISIONING_ROOT")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

echo -e "${CYAN}=============================================="
echo "Sysconfig Provisioning CLI Test"
echo -e "==============================================${NC}"
echo ""

# Setup runtime directory
if [ "$EUID" -eq 0 ]; then
    RUNTIME_DIR="/var/run"
else
    echo -e "${GREEN}✓ Running as non-root user (UID: $EUID)${NC}"
    if [ -z "$XDG_RUNTIME_DIR" ]; then
        export XDG_RUNTIME_DIR="/tmp/run-$EUID"
        echo -e "${YELLOW}ℹ Setting XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR${NC}"
    fi
    RUNTIME_DIR="$XDG_RUNTIME_DIR"
fi

# Create runtime directory if needed
mkdir -p "$RUNTIME_DIR"
chmod 700 "$RUNTIME_DIR"

# Define paths
SYSCONFIG_SOCKET="$RUNTIME_DIR/sysconfig.sock"
export SYSCONFIG_SOCKET
TEST_DATA_DIR="$RUNTIME_DIR/test-provisioning-data"
TEST_KDL_CONFIG="$TEST_DATA_DIR/test-config.kdl"
TEST_JSON_CONFIG="$TEST_DATA_DIR/test-config.json"
TEST_YAML_CONFIG="$TEST_DATA_DIR/test-config.yaml"

echo -e "${BLUE}Configuration:${NC}"
echo "  Runtime dir:        $RUNTIME_DIR"
echo "  Sysconfig socket:   $SYSCONFIG_SOCKET"
echo "  Test data dir:      $TEST_DATA_DIR"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"

    # Kill sysconfig if running
    [ ! -z "$SYSCONFIG_PID" ] && kill $SYSCONFIG_PID 2>/dev/null || true

    # Wait a moment for process to exit
    sleep 1

    # Clean up socket
    rm -f "$SYSCONFIG_SOCKET" 2>/dev/null || true

    echo -e "${GREEN}Cleanup complete${NC}"
}

trap cleanup EXIT INT TERM

# Setup test data
echo -e "${BLUE}Setting up test data...${NC}"
mkdir -p "$TEST_DATA_DIR"

# Create test KDL config
cat > "$TEST_KDL_CONFIG" << 'EOF'
// Test KDL configuration for provisioning
hostname "test-provisioned-host"

nameservers "8.8.8.8" "1.1.1.1"

interface "net0" {
    address "dhcp" primary=true
    mtu 1500
    enabled true
}

interface "net1" {
    address "192.168.1.100/24" "192.168.1.1"
    mtu 9000
    enabled true
}

// SSH keys for the root user
ssh-keys {
    root "ssh-rsa AAAAB3NzaC1yc2EAAAATest... test@provisioned"
}

// NTP servers
ntp-servers "pool.ntp.org" "time.google.com"

// Timezone
timezone "UTC"
EOF

# Create test JSON config
cat > "$TEST_JSON_CONFIG" << 'EOF'
{
  "hostname": "json-provisioned-host",
  "nameservers": ["8.8.8.8", "8.8.4.4"],
  "interfaces": {
    "net0": {
      "addresses": [
        {
          "addr_type": "Dhcp4",
          "primary": true
        }
      ],
      "enabled": true,
      "mtu": 1500
    },
    "net1": {
      "addresses": [
        {
          "addr_type": "Static",
          "address": "10.0.0.10/24",
          "gateway": "10.0.0.1"
        }
      ],
      "enabled": true
    }
  },
  "ssh_authorized_keys": [
    "ssh-rsa AAAAB3NzaC1yc2EJSONTest... json@test"
  ],
  "metadata": {
    "source": "local-json",
    "version": "1.0"
  }
}
EOF

# Create test YAML config
cat > "$TEST_YAML_CONFIG" << 'EOF'
hostname: yaml-provisioned-host
nameservers:
  - 1.1.1.1
  - 1.0.0.1
interfaces:
  net0:
    addresses:
      - addr_type: Dhcp4
        primary: true
    enabled: true
    mtu: 9000
  net1:
    addresses:
      - addr_type: Static
        address: 172.16.0.10/24
        gateway: 172.16.0.1
    enabled: true
ssh_authorized_keys:
  - ssh-ed25519 AAAAC3NzaC1lZDI1NTE5YAML... yaml@test
ntp_servers:
  - time.cloudflare.com
  - pool.ntp.org
timezone: Europe/London
EOF

echo -e "${GREEN}✓ Test data created${NC}"
echo ""

# Build sysconfig and provisioning CLI
echo -e "${BLUE}Building components...${NC}"

# Build sysconfig
echo "  Building sysconfig..."
cd "$INSTALLER_ROOT/sysconfig"
cargo build --bin sysconfig 2>&1 | grep -E "(Compiling|Finished)" || true

# Build provisioning CLI
echo "  Building provisioning CLI..."
cd "$PROVISIONING_ROOT"
cargo build --bin provisioning-plugin 2>&1 | grep -E "(Compiling|Finished)" || true

echo -e "${GREEN}✓ All components built${NC}"
echo ""

# Set log levels
export RUST_LOG=info,sysconfig=debug,provisioning=debug

# Start sysconfig service
echo -e "${BLUE}Starting sysconfig service...${NC}"
rm -f "$SYSCONFIG_SOCKET" 2>/dev/null || true

"$INSTALLER_ROOT/sysconfig/target/debug/sysconfig" \
    --socket "$SYSCONFIG_SOCKET" \
    2>&1 | sed 's/^/[sysconfig] /' &

SYSCONFIG_PID=$!
sleep 2

if [ ! -S "$SYSCONFIG_SOCKET" ]; then
    echo -e "${RED}ERROR: Sysconfig failed to start${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Sysconfig started (PID: $SYSCONFIG_PID)${NC}"
echo ""

PROVISIONING_CLI="$PROVISIONING_ROOT/target/debug/provisioning-plugin"

# Function to run a test
run_test() {
    local test_name="$1"
    local test_cmd="$2"

    echo -e "${CYAN}Running test: $test_name${NC}"
    echo -e "${BLUE}Command: $test_cmd${NC}"

    if eval "$test_cmd"; then
        echo -e "${GREEN}✓ $test_name passed${NC}"
    else
        echo -e "${RED}✗ $test_name failed${NC}"
        return 1
    fi
    echo ""
}

echo -e "${MAGENTA}=============================================="
echo "Running Provisioning CLI Tests"
echo -e "==============================================${NC}"
echo ""

# Test 1: Parse KDL configuration
run_test "Parse KDL config" \
    "$PROVISIONING_CLI parse --config $TEST_KDL_CONFIG --format pretty"

# Test 2: Parse JSON configuration
run_test "Parse JSON config" \
    "$PROVISIONING_CLI parse --config $TEST_JSON_CONFIG --format pretty"

# Test 3: Parse YAML configuration
run_test "Parse YAML config" \
    "$PROVISIONING_CLI parse --config $TEST_YAML_CONFIG --format pretty"

# Test 4: Detect available sources
run_test "Detect local sources" \
    "$PROVISIONING_CLI detect --format pretty"

# Test 5: Check provisioning status
run_test "Check initial status" \
    "$PROVISIONING_CLI status --format pretty"

# Test 6: Apply KDL configuration (dry run)
run_test "Apply KDL config (dry-run)" \
    "$PROVISIONING_CLI apply --config $TEST_KDL_CONFIG --dry-run"

# Test 7: Apply JSON configuration (dry run)
run_test "Apply JSON config (dry-run)" \
    "$PROVISIONING_CLI apply --config $TEST_JSON_CONFIG --dry-run"

# Test 8: Apply YAML configuration (dry run)
run_test "Apply YAML config (dry-run)" \
    "$PROVISIONING_CLI apply --config $TEST_YAML_CONFIG --dry-run"

# Test 9: Apply KDL configuration (real)
run_test "Apply KDL config" \
    "$PROVISIONING_CLI apply --config $TEST_KDL_CONFIG"

# Test 10: Check status after apply
run_test "Check status after apply" \
    "$PROVISIONING_CLI status --format pretty"

# Test 11: Auto-detect without network
run_test "Auto-detect (no network)" \
    "$PROVISIONING_CLI autodetect --dry-run"

# Test 12: Detect with network check (if available)
if ping -c 1 8.8.8.8 &> /dev/null; then
    run_test "Detect network sources" \
        "$PROVISIONING_CLI detect --network --format pretty"
else
    echo -e "${YELLOW}Skipping network source detection (no network)${NC}"
    echo ""
fi

echo -e "${MAGENTA}=============================================="
echo "Test Usage Examples"
echo -e "==============================================${NC}"
echo ""

echo -e "${CYAN}1. Parse and validate configuration files:${NC}"
echo "   $PROVISIONING_CLI parse --config /path/to/config.kdl"
echo "   $PROVISIONING_CLI parse --config /path/to/config.json"
echo "   $PROVISIONING_CLI parse --config /path/to/config.yaml"
echo ""

echo -e "${CYAN}2. Detect available provisioning sources:${NC}"
echo "   $PROVISIONING_CLI detect [--network]"
echo ""

echo -e "${CYAN}3. Apply configuration from specific sources:${NC}"
echo "   $PROVISIONING_CLI apply --sources local,cloud-init"
echo ""

echo -e "${CYAN}4. Auto-detect and apply on boot:${NC}"
echo "   $PROVISIONING_CLI autodetect --check-network"
echo ""

echo -e "${CYAN}5. Apply with specific config file (auto-detects format):${NC}"
echo "   $PROVISIONING_CLI apply --config /etc/sysconfig.kdl"
echo "   $PROVISIONING_CLI apply --config /etc/provisioning.json"
echo "   $PROVISIONING_CLI apply --config /etc/config.yaml"
echo ""

echo -e "${CYAN}6. Check current provisioning state:${NC}"
echo "   $PROVISIONING_CLI status"
echo ""

echo -e "${MAGENTA}=============================================="
echo "Boot-time Provisioning Workflow"
echo -e "==============================================${NC}"
echo ""

cat << 'EOF'
The typical boot-time provisioning workflow:

1. Early boot: Check for local configuration
   provisioning-plugin autodetect --check-network

2. If network required: Setup minimal network
   - The autodetect command will detect this
   - Apply minimal DHCP config to primary interface
   - Wait for network to come up

3. Fetch cloud configuration (if available)
   provisioning-plugin apply --sources ec2,azure,gcp,cloud-init

4. Apply full configuration to sysconfig
   - The provisioning CLI writes directly to sysconfig
   - Sysconfig distributes to appropriate plugins

5. Verify configuration applied
   provisioning-plugin status

This can be integrated into systemd/SMF as:
- A oneshot service that runs on boot
- Ordered after network-pre.target but before network.target
- Can detect and configure network if needed
EOF

echo ""
echo -e "${GREEN}All tests complete!${NC}"
echo ""
echo -e "${YELLOW}Note: The provisioning tool is now a CLI that:${NC}"
echo "  • Parses multiple config formats (KDL, JSON, YAML, TOML)"
echo "  • Auto-detects cloud environments"
echo "  • Writes configuration directly to sysconfig"
echo "  • Can run standalone without being a plugin server"
echo "  • Knows when network setup is needed first"
echo ""

# Keep running to show logs
echo -e "${CYAN}Press Ctrl+C to stop sysconfig and exit${NC}"
wait
