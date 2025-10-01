#!/bin/sh
#
# Sysconfig Services Installation Script
#
# This script installs sysconfig, platform base plugins, and the provisioning CLI
# with appropriate service configurations for your operating system.
#
# Usage: ./install.sh [options]
#
# Options:
#   --prefix PATH         Installation prefix (default: /usr/local or /usr)
#   --no-build           Skip building binaries (assume already built)
#   --no-enable          Don't enable services after installation
#   --no-start           Don't start services after installation
#   --dry-run            Show what would be done without doing it
#   --uninstall          Remove installed components
#   --help               Show this help message

set -e

# Get script directory and source common utilities
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALLER_ROOT=$(dirname "$SCRIPT_DIR")
source "${INSTALLER_ROOT}/lib/common.sh"

# Default options
PREFIX=""
BUILD=true
ENABLE=true
START=true
DRY_RUN=false
UNINSTALL=false
OS=""
INIT=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command line arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        --no-build)
            BUILD=false
            shift
            ;;
        --no-enable)
            ENABLE=false
            shift
            ;;
        --no-start)
            START=false
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        --help|-h)
            sed -n '3,17p' "$0" | sed 's/^# //'
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Detect operating system and init system
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS="linux"
        if command -v systemctl >/dev/null 2>&1; then
            INIT="systemd"
        else
            echo "${RED}Error: Linux detected but systemd not found${NC}"
            exit 1
        fi
    elif [ "$(uname -s)" = "FreeBSD" ]; then
        OS="freebsd"
        INIT="rc"
    elif [ "$(uname -s)" = "SunOS" ]; then
        OS="illumos"
        INIT="smf"
    else
        echo "${RED}Error: Unsupported operating system$(NC)"
        exit 1
    fi

    # Set default prefix based on OS
    if [ -z "$PREFIX" ]; then
        case "$OS" in
            linux|illumos)
                PREFIX="/usr"
                ;;
            freebsd)
                PREFIX="/usr/local"
                ;;
        esac
    fi

    echo "${GREEN}Detected: $OS with $INIT init system${NC}"
    echo "${GREEN}Installation prefix: $PREFIX${NC}"
}

# Run command or show what would be run
run_cmd() {
    if [ "$DRY_RUN" = true ]; then
        echo "${BLUE}[DRY RUN] $*${NC}"
    else
        echo "${YELLOW}Running: $*${NC}"
        eval "$@"
    fi
}

# Build binaries
build_binaries() {
    if [ "$BUILD" = false ]; then
        echo "${YELLOW}Skipping build (--no-build specified)${NC}"
        return
    fi

    echo "${BLUE}Building binaries...${NC}"

    # Find the installer root directory
    SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
    INSTALLER_ROOT=$(dirname "$(dirname "$SCRIPT_DIR")")

    # Build sysconfig
    echo "Building sysconfig..."
    run_cmd "cd '$INSTALLER_ROOT/sysconfig' && cargo build --release"

    # Build appropriate base plugin
    case "$OS" in
        linux)
            echo "Building Linux base plugin..."
            run_cmd "cd '$INSTALLER_ROOT/sysconfig-plugins' && cargo build --release --bin linux-base-plugin"
            ;;
        freebsd)
            echo "Building FreeBSD base plugin..."
            run_cmd "cd '$INSTALLER_ROOT/sysconfig-plugins' && cargo build --release --bin freebsd-base-plugin"
            ;;
        illumos)
            echo "Building illumos base plugin..."
            run_cmd "cd '$INSTALLER_ROOT/sysconfig-plugins' && cargo build --release --bin illumos-base-plugin"
            ;;
    esac

    # Build provisioning CLI
    echo "Building provisioning CLI..."
    run_cmd "cd '$INSTALLER_ROOT/sysconfig-provisioning' && cargo build --release"

    echo "${GREEN}Build complete${NC}"
}

# Install binaries
install_binaries() {
    echo "${BLUE}Installing binaries...${NC}"

    SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
    # Create directories
    run_cmd "mkdir -p '$PREFIX/lib/sysconfig/plugins'"
    run_cmd "mkdir -p '$PREFIX/bin'"

    # Get dynamic target directories
    SYSCONFIG_TARGET_DIR=$(get_crate_target_dir "${INSTALLER_ROOT}/sysconfig")
    PLUGINS_TARGET_DIR=$(get_crate_target_dir "${INSTALLER_ROOT}/sysconfig-plugins")
    PROVISIONING_TARGET_DIR=$(get_crate_target_dir "${INSTALLER_ROOT}/sysconfig-provisioning")

    # Install sysconfig
    SYSCONFIG_BINARY="${SYSCONFIG_TARGET_DIR}/release/sysconfig"
    run_cmd "cp '$SYSCONFIG_BINARY' '$PREFIX/lib/sysconfig/sysconfig'"
    run_cmd "chmod 755 '$PREFIX/lib/sysconfig/sysconfig'"

    # Install base plugin
    case "$OS" in
        linux)
            PLUGIN_BINARY="${PLUGINS_TARGET_DIR}/release/linux-base-plugin"
            run_cmd "cp '$PLUGIN_BINARY' '$PREFIX/lib/sysconfig/plugins/linux-base-plugin'"
            ;;
        freebsd)
            PLUGIN_BINARY="${PLUGINS_TARGET_DIR}/release/freebsd-base-plugin"
            run_cmd "cp '$PLUGIN_BINARY' '$PREFIX/lib/sysconfig/plugins/freebsd-base-plugin'"
            ;;
        illumos)
            PLUGIN_BINARY="${PLUGINS_TARGET_DIR}/release/illumos-base-plugin"
            run_cmd "cp '$PLUGIN_BINARY' '$PREFIX/lib/sysconfig/plugins/illumos-base-plugin'"
            ;;
    esac
    run_cmd "chmod 755 '$PREFIX/lib/sysconfig/plugins/'*"

    # Install provisioning CLI
    PROVISIONING_BINARY="${PROVISIONING_TARGET_DIR}/release/provisioning-plugin"
    run_cmd "cp '$PROVISIONING_BINARY' '$PREFIX/lib/sysconfig/sysconfig-provision'"
    run_cmd "chmod 755 '$PREFIX/lib/sysconfig/sysconfig-provision'"

    # Create symlink for CLI
    run_cmd "ln -sf '$PREFIX/lib/sysconfig/sysconfig-provision' '$PREFIX/bin/sysconfig-provision'"

    # Create state/config directories
    run_cmd "mkdir -p /var/lib/sysconfig"
    run_cmd "mkdir -p /etc/sysconfig.d"

    echo "${GREEN}Binaries installed${NC}"
}

# Install systemd services
install_systemd() {
    echo "${BLUE}Installing systemd services...${NC}"

    SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

    run_cmd "cp '$SCRIPT_DIR/systemd/sysconfig.service' /etc/systemd/system/"
    run_cmd "cp '$SCRIPT_DIR/systemd/sysconfig-linux-base.service' /etc/systemd/system/"
    run_cmd "cp '$SCRIPT_DIR/systemd/sysconfig-provision.service' /etc/systemd/system/"

    # Adjust paths in service files if non-standard prefix
    if [ "$PREFIX" != "/usr" ]; then
        run_cmd "sed -i 's|/usr/lib/sysconfig|$PREFIX/lib/sysconfig|g' /etc/systemd/system/sysconfig*.service"
    fi

    run_cmd "systemctl daemon-reload"

    if [ "$ENABLE" = true ]; then
        run_cmd "systemctl enable sysconfig.service"
        run_cmd "systemctl enable sysconfig-linux-base.service"
        run_cmd "systemctl enable sysconfig-provision.service"
    fi

    if [ "$START" = true ]; then
        run_cmd "systemctl start sysconfig.service"
        run_cmd "systemctl start sysconfig-linux-base.service"
        # Don't auto-start provision - it runs on boot or manually
    fi

    echo "${GREEN}systemd services installed${NC}"
}

# Install SMF manifests
install_smf() {
    echo "${BLUE}Installing SMF manifests...${NC}"

    SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

    # Adjust paths in manifests if needed
    for manifest in "$SCRIPT_DIR/smf/"*.xml; do
        if [ "$PREFIX" != "/usr" ]; then
            # Create temporary modified manifest
            sed "s|/usr/lib/sysconfig|$PREFIX/lib/sysconfig|g" "$manifest" > "/tmp/$(basename "$manifest")"
            run_cmd "svccfg import '/tmp/$(basename "$manifest")'"
            rm -f "/tmp/$(basename "$manifest")"
        else
            run_cmd "svccfg import '$manifest'"
        fi
    done

    if [ "$ENABLE" = true ]; then
        run_cmd "svcadm enable sysconfig"
        run_cmd "svcadm enable sysconfig/illumos-base"
        # Provision service is transient, runs on boot
    fi

    echo "${GREEN}SMF manifests installed${NC}"
}

# Install FreeBSD rc scripts
install_rc() {
    echo "${BLUE}Installing FreeBSD rc.d scripts...${NC}"

    SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

    run_cmd "cp '$SCRIPT_DIR/freebsd/sysconfig' /usr/local/etc/rc.d/"
    run_cmd "cp '$SCRIPT_DIR/freebsd/sysconfig-freebsd-base' /usr/local/etc/rc.d/"
    run_cmd "cp '$SCRIPT_DIR/freebsd/sysconfig-provision' /usr/local/etc/rc.d/"
    run_cmd "chmod +x /usr/local/etc/rc.d/sysconfig*"

    # Adjust paths if non-standard prefix
    if [ "$PREFIX" != "/usr/local" ]; then
        run_cmd "sed -i '' 's|/usr/local/lib/sysconfig|$PREFIX/lib/sysconfig|g' /usr/local/etc/rc.d/sysconfig*"
    fi

    if [ "$ENABLE" = true ]; then
        if ! grep -q "sysconfig_enable" /etc/rc.conf; then
            echo "" >> /etc/rc.conf
            echo "# Sysconfig services" >> /etc/rc.conf
            echo "sysconfig_enable=\"YES\"" >> /etc/rc.conf
            echo "sysconfig_freebsd_base_enable=\"YES\"" >> /etc/rc.conf
            echo "sysconfig_provision_enable=\"YES\"" >> /etc/rc.conf
        fi
    fi

    if [ "$START" = true ]; then
        run_cmd "service sysconfig start"
        run_cmd "service sysconfig-freebsd-base start"
        # Don't auto-start provision - it runs on boot or manually
    fi

    echo "${GREEN}FreeBSD rc.d scripts installed${NC}"
}

# Uninstall components
uninstall() {
    echo "${RED}Uninstalling sysconfig services...${NC}"

    case "$INIT" in
        systemd)
            run_cmd "systemctl stop sysconfig-linux-base.service || true"
            run_cmd "systemctl stop sysconfig.service || true"
            run_cmd "systemctl disable sysconfig-provision.service || true"
            run_cmd "systemctl disable sysconfig-linux-base.service || true"
            run_cmd "systemctl disable sysconfig.service || true"
            run_cmd "rm -f /etc/systemd/system/sysconfig*.service"
            run_cmd "systemctl daemon-reload"
            ;;
        smf)
            run_cmd "svcadm disable -s sysconfig/illumos-base || true"
            run_cmd "svcadm disable -s sysconfig || true"
            run_cmd "svccfg delete sysconfig/provision || true"
            run_cmd "svccfg delete sysconfig/illumos-base || true"
            run_cmd "svccfg delete sysconfig || true"
            ;;
        rc)
            run_cmd "service sysconfig-freebsd-base stop || true"
            run_cmd "service sysconfig stop || true"
            run_cmd "rm -f /usr/local/etc/rc.d/sysconfig*"
            echo "${YELLOW}Remember to remove sysconfig entries from /etc/rc.conf${NC}"
            ;;
    esac

    # Remove binaries
    run_cmd "rm -rf '$PREFIX/lib/sysconfig'"
    run_cmd "rm -f '$PREFIX/bin/sysconfig-provision'"

    echo "${GREEN}Uninstall complete${NC}"
    echo "${YELLOW}Note: Configuration files in /etc/sysconfig.d and state in /var/lib/sysconfig were preserved${NC}"
}

# Main installation flow
main() {
    detect_os

    if [ "$UNINSTALL" = true ]; then
        uninstall
        exit 0
    fi

    echo "${BLUE}Installing Sysconfig Services${NC}"
    echo "OS: $OS"
    echo "Init: $INIT"
    echo "Prefix: $PREFIX"
    echo ""

    # Check for root/sudo
    if [ "$(id -u)" != "0" ] && [ "$DRY_RUN" = false ]; then
        echo "${RED}Error: This script must be run as root${NC}"
        echo "Try: sudo $0 $*"
        exit 1
    fi

    # Build and install
    build_binaries
    install_binaries

    # Install service configurations
    case "$INIT" in
        systemd)
            install_systemd
            ;;
        smf)
            install_smf
            ;;
        rc)
            install_rc
            ;;
    esac

    echo ""
    echo "${GREEN}Installation complete!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Create /etc/sysconfig.kdl with your configuration"
    echo "2. Test provisioning: sysconfig-provision detect"
    echo "3. Apply configuration: sysconfig-provision apply --config /etc/sysconfig.kdl"
    echo ""
    echo "Services will automatically start on next boot."
    echo ""

    # Show status
    case "$INIT" in
        systemd)
            echo "Check status with:"
            echo "  systemctl status sysconfig"
            echo "  systemctl status sysconfig-linux-base"
            echo "  systemctl status sysconfig-provision"
            ;;
        smf)
            echo "Check status with:"
            echo "  svcs sysconfig"
            echo "  svcs sysconfig/illumos-base"
            echo "  svcs sysconfig/provision"
            ;;
        rc)
            echo "Check status with:"
            echo "  service sysconfig status"
            echo "  service sysconfig-freebsd-base status"
            echo "  service sysconfig-provision status"
            ;;
    esac
}

# Run main function
main
