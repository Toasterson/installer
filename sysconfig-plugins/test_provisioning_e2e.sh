#!/bin/bash

# End-to-end test script for sysconfig provisioning plugin
# Tests the complete provisioning pipeline with dry-run mode

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALLER_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

echo -e "${CYAN}=============================================="
echo "Sysconfig Provisioning Plugin E2E Test"
echo -e "==============================================${NC}"
echo ""

# Setup runtime directory based on user
if [ "$EUID" -eq 0 ]; then
    echo -e "${YELLOW}⚠ Running as root. Dry-run mode will NOT be automatically enabled in base plugin."
    echo -e "  Consider running as non-root user for safer testing.${NC}"
    read -p "Continue as root? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
    RUNTIME_DIR="/var/run"
else
    echo -e "${GREEN}✓ Running as non-root user (UID: $EUID)${NC}"
    echo -e "${GREEN}✓ Base plugin will auto-enable dry-run mode${NC}"

    # Setup XDG_RUNTIME_DIR
    if [ -z "$XDG_RUNTIME_DIR" ]; then
        export XDG_RUNTIME_DIR="/tmp/run-$EUID"
        echo -e "${YELLOW}ℹ Setting XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR${NC}"
    fi
    RUNTIME_DIR="$XDG_RUNTIME_DIR"
fi

# Create runtime directory if needed
mkdir -p "$RUNTIME_DIR"
chmod 700 "$RUNTIME_DIR"

# Define socket paths and config files
SYSCONFIG_SOCKET="$RUNTIME_DIR/sysconfig.sock"
BASE_PLUGIN_SOCKET="$RUNTIME_DIR/sysconfig-illumos-base.sock"
PROVISIONING_PLUGIN_SOCKET="$RUNTIME_DIR/sysconfig-provisioning.sock"
STATE_FILE="$RUNTIME_DIR/sysconfig-state.json"
PROVISIONING_CONFIG="$SCRIPT_DIR/test-provisioning-local.json"
PROVISIONING_KDL="$SCRIPT_DIR/test-provisioning-config.kdl"

# Test data directories
TEST_DATA_DIR="$RUNTIME_DIR/test-provisioning-data"
CLOUD_INIT_DIR="$TEST_DATA_DIR/cloud-init"

echo -e "${BLUE}Configuration:${NC}"
echo "  Runtime dir:        $RUNTIME_DIR"
echo "  Sysconfig socket:   $SYSCONFIG_SOCKET"
echo "  Base plugin socket: $BASE_PLUGIN_SOCKET"
echo "  Prov plugin socket: $PROVISIONING_PLUGIN_SOCKET"
echo "  Config file:        $PROVISIONING_CONFIG"
echo "  KDL config:         $PROVISIONING_KDL"
echo "  Test data dir:      $TEST_DATA_DIR"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"

    # Kill processes
    [ ! -z "$PROVISIONING_PID" ] && kill $PROVISIONING_PID 2>/dev/null || true
    [ ! -z "$BASE_PLUGIN_PID" ] && kill $BASE_PLUGIN_PID 2>/dev/null || true
    [ ! -z "$SYSCONFIG_PID" ] && kill $SYSCONFIG_PID 2>/dev/null || true

    # Wait a moment for processes to exit
    sleep 1

    # Clean up sockets
    rm -f "$SYSCONFIG_SOCKET" "$BASE_PLUGIN_SOCKET" "$PROVISIONING_PLUGIN_SOCKET" 2>/dev/null || true

    echo -e "${GREEN}Cleanup complete${NC}"
}

trap cleanup EXIT INT TERM

# Setup test data directories
echo -e "${BLUE}Setting up test data...${NC}"
mkdir -p "$TEST_DATA_DIR"
mkdir -p "$CLOUD_INIT_DIR"

# Create cloud-init test data
cat > "$CLOUD_INIT_DIR/meta-data" << 'EOF'
instance-id: test-instance-001
local-hostname: cloud-init-test-host
public-keys:
  - ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC7CloudInit... cloud-init@test
network:
  version: 1
  config:
    - type: physical
      name: eth0
      mac_address: "52:54:00:11:22:33"
      subnets:
        - type: dhcp4
        - type: dhcp6
EOF

cat > "$CLOUD_INIT_DIR/user-data" << 'EOF'
#cloud-config
hostname: cloud-init-provisioned
users:
  - name: cloud-user
    uid: 3000
    groups: [wheel, users]
    shell: /bin/bash
    ssh_authorized_keys:
      - ssh-rsa AAAAB3NzaC1yc2ECloudUser... cloud-user@test
write_files:
  - path: /tmp/cloud-init-test
    content: |
      Cloud-init provisioning test
      Timestamp: $(date)
    permissions: '0644'
runcmd:
  - echo "Cloud-init provisioning complete" > /tmp/cloud-init-done
EOF

cat > "$CLOUD_INIT_DIR/network-config" << 'EOF'
version: 1
config:
  - type: physical
    name: eth0
    mac_address: "52:54:00:11:22:33"
    subnets:
      - type: static
        address: 10.0.1.100
        netmask: 255.255.255.0
        gateway: 10.0.1.1
        dns_nameservers:
          - 8.8.8.8
          - 8.8.4.4
  - type: nameserver
    address:
      - 1.1.1.1
      - 1.0.0.1
    search:
      - cloud.local
      - test.cloud
EOF

echo -e "${GREEN}✓ Test data created${NC}"
echo ""

# Copy the test provisioning config to the expected location
if [ -f "$PROVISIONING_CONFIG" ]; then
    cp "$PROVISIONING_CONFIG" "/tmp/test-provisioning-local.json" 2>/dev/null || true
fi

# Build everything
echo -e "${BLUE}Building components...${NC}"

# Build sysconfig
echo "  Building sysconfig..."
cd "$INSTALLER_ROOT/sysconfig"
cargo build --bin sysconfig 2>&1 | grep -E "(Compiling|Finished)" || true

# Build illumos-base-plugin
echo "  Building illumos-base-plugin..."
cd "$SCRIPT_DIR"
cargo build --bin illumos-base-plugin 2>&1 | grep -E "(Compiling|Finished)" || true

# Build provisioning plugin
echo "  Building provisioning-plugin..."
cd "$INSTALLER_ROOT/sysconfig-provisioning"
cargo build --bin provisioning-plugin 2>&1 | grep -E "(Compiling|Finished)" || true

echo -e "${GREEN}✓ All components built${NC}"
echo ""

# Set log levels
export RUST_LOG=info,sysconfig=debug,illumos_base_plugin=debug,provisioning_plugin=debug

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

# Start illumos-base-plugin
echo -e "${BLUE}Starting illumos-base-plugin...${NC}"
rm -f "$BASE_PLUGIN_SOCKET" 2>/dev/null || true

"$SCRIPT_DIR/target/debug/illumos-base-plugin" \
    --socket "$BASE_PLUGIN_SOCKET" \
    --service-socket "$SYSCONFIG_SOCKET" \
    2>&1 | sed 's/^/[base-plugin] /' &

BASE_PLUGIN_PID=$!
sleep 2

if ! ps -p $BASE_PLUGIN_PID > /dev/null 2>&1; then
    echo -e "${RED}ERROR: Base plugin failed to start${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Base plugin started (PID: $BASE_PLUGIN_PID)${NC}"

# Start provisioning plugin
echo -e "${BLUE}Starting provisioning-plugin...${NC}"
rm -f "$PROVISIONING_PLUGIN_SOCKET" 2>/dev/null || true

# Create a minimal provisioning config if the full one doesn't exist
if [ ! -f "$PROVISIONING_CONFIG" ]; then
    echo -e "${YELLOW}Creating minimal provisioning config...${NC}"
    cat > "$PROVISIONING_CONFIG" << 'EOF'
{
  "hostname": "minimal-test-host",
  "nameservers": ["8.8.8.8", "1.1.1.1"],
  "interfaces": {
    "net0": {
      "addresses": [
        {"type": "dhcp4", "primary": true}
      ]
    }
  }
}
EOF
fi

"$INSTALLER_ROOT/sysconfig-provisioning/target/debug/provisioning-plugin" \
    --socket "$PROVISIONING_PLUGIN_SOCKET" \
    --service-socket "$SYSCONFIG_SOCKET" \
    --config-file "$PROVISIONING_CONFIG" \
    2>&1 | sed 's/^/[provisioning] /' &

PROVISIONING_PID=$!
sleep 3

if ! ps -p $PROVISIONING_PID > /dev/null 2>&1; then
    echo -e "${RED}ERROR: Provisioning plugin failed to start${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Provisioning plugin started (PID: $PROVISIONING_PID)${NC}"

echo ""
echo -e "${MAGENTA}=============================================="
echo "All Services Running - System Ready for Testing"
echo -e "==============================================${NC}"
echo ""

echo -e "${CYAN}Service Status:${NC}"
echo "  • sysconfig:           PID $SYSCONFIG_PID"
echo "  • illumos-base-plugin: PID $BASE_PLUGIN_PID"
echo "  • provisioning-plugin: PID $PROVISIONING_PID"
echo ""

echo -e "${CYAN}Test Data Locations:${NC}"
echo "  • Local config:     $PROVISIONING_CONFIG"
echo "  • Cloud-init dir:   $CLOUD_INIT_DIR"
echo "  • KDL config:       $PROVISIONING_KDL"
echo ""

if [ "$EUID" -ne 0 ]; then
    echo -e "${YELLOW}DRY-RUN MODE ACTIVE:${NC}"
    echo "  All base plugin operations will be simulated."
    echo "  Check [base-plugin] output for DRY-RUN messages."
    echo ""
fi

echo -e "${CYAN}Testing Instructions:${NC}"
echo ""
echo "1. Test provisioning with local config:"
echo -e "   ${GREEN}$INSTALLER_ROOT/sysconfig-provisioning/target/debug/provisioning-plugin \\
      --apply-now \\
      --config-file $PROVISIONING_CONFIG${NC}"
echo ""

echo "2. Test with cloud-init data:"
echo -e "   ${GREEN}# First, set cloud-init paths
   export CLOUD_INIT_META_DATA=$CLOUD_INIT_DIR/meta-data
   export CLOUD_INIT_USER_DATA=$CLOUD_INIT_DIR/user-data
   export CLOUD_INIT_NETWORK_CONFIG=$CLOUD_INIT_DIR/network-config

   # Then trigger provisioning
   $INSTALLER_ROOT/sysconfig-provisioning/target/debug/provisioning-plugin \\
      --apply-now \\
      --enable cloud-init${NC}"
echo ""

echo "3. Test with sysconfig-cli:"
echo -e "   ${GREEN}export SYSCONFIG_SOCKET=$SYSCONFIG_SOCKET
   $INSTALLER_ROOT/sysconfig-cli/target/debug/sysconfig-cli status
   $INSTALLER_ROOT/sysconfig-cli/target/debug/sysconfig-cli apply --file $PROVISIONING_KDL${NC}"
echo ""

echo "4. Monitor logs:"
echo "   Watch the terminal output for:"
echo "   • [sysconfig] - Core service logs"
echo "   • [base-plugin] - DRY-RUN operations"
echo "   • [provisioning] - Provisioning plugin logs"
echo ""

echo "5. Check applied configuration:"
echo -e "   ${GREEN}cat $STATE_FILE | jq .${NC}"
echo ""

echo -e "${CYAN}Tips:${NC}"
echo "• Use Ctrl+C to stop all services"
echo "• Logs show which source provided each config item"
echo "• DRY-RUN messages indicate what would be changed"
echo "• State file shows current configuration state"
echo ""

echo -e "${MAGENTA}=============================================="
echo "Press Ctrl+C to stop all services"
echo -e "==============================================${NC}"

# Keep running and show logs
wait
