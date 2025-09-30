#!/bin/bash
#
# Example bhyve startup script for illumos installer development VM
#
# This script demonstrates how to start the development VM using bhyve
# with 9P filesystem sharing for rapid development and testing.
#

set -o errexit
set -o pipefail

# Configuration - CUSTOMIZE THESE PATHS
VM_NAME="illumos-installer-dev"
IMAGE_PATH="/path/to/cloudimage-ttya-openindiana-hipster-dev.raw"
REPO_PATH="/path/to/illumos/installer"
MEMORY="2048M"
CPUS="2"

# bhyve configuration
BHYVE_UEFI="/usr/share/bhyve/BHYVE_UEFI.fd"
TAP_DEVICE=""
BRIDGE_DEVICE=""

usage() {
    cat << EOF
Usage: $0 [options]

Start illumos installer development VM with bhyve.

Options:
  -i, --image PATH      Path to the development image (required)
  -r, --repo PATH       Path to the repository directory (required)
  -n, --name NAME       VM name (default: $VM_NAME)
  -m, --memory SIZE     Memory size (default: $MEMORY)
  -c, --cpus COUNT      Number of CPUs (default: $CPUS)
  -t, --tap DEVICE      TAP device for networking (optional)
  -b, --bridge DEVICE   Bridge device for networking (optional)
  -h, --help            Show this help message

Examples:
  $0 -i /images/dev.raw -r /src/installer
  $0 -i /images/dev.raw -r /src/installer -n my-dev-vm -m 4096M -c 4

Prerequisites:
- bhyve must be available and loaded
- UEFI firmware must be available at $BHYVE_UEFI
- Image file must exist and be accessible
- Repository directory must exist

EOF
}

check_prerequisites() {
    echo "Checking prerequisites..."

    # Check if bhyve is available
    if ! command -v bhyve >/dev/null 2>&1; then
        echo "Error: bhyve command not found"
        exit 1
    fi

    # Check if UEFI firmware exists
    if [[ ! -f "$BHYVE_UEFI" ]]; then
        echo "Error: UEFI firmware not found at $BHYVE_UEFI"
        echo "Please install bhyve-firmware or adjust BHYVE_UEFI path"
        exit 1
    fi

    # Check if image exists
    if [[ ! -f "$IMAGE_PATH" ]]; then
        echo "Error: Image file not found at $IMAGE_PATH"
        exit 1
    fi

    # Check if repo directory exists
    if [[ ! -d "$REPO_PATH" ]]; then
        echo "Error: Repository directory not found at $REPO_PATH"
        exit 1
    fi

    echo "Prerequisites check passed"
}

cleanup_vm() {
    echo "Cleaning up existing VM..."

    # Destroy existing VM if running
    if bhyvectl --vm="$VM_NAME" --get-stats >/dev/null 2>&1; then
        echo "Destroying existing VM instance..."
        bhyvectl --destroy --vm="$VM_NAME"
    fi

    # Clean up any existing device mappings
    if vmm_loaded=$(kldstat -q -m vmm 2>/dev/null); then
        echo "VMM module is loaded"
    fi
}

setup_networking() {
    # Basic networking setup - customize as needed
    if [[ -n "$TAP_DEVICE" ]] && [[ -n "$BRIDGE_DEVICE" ]]; then
        echo "Using custom networking: tap=$TAP_DEVICE, bridge=$BRIDGE_DEVICE"
        NETWORK_ARGS="-s 10,virtio-net,$TAP_DEVICE"
    else
        echo "Using default networking (no network interface)"
        NETWORK_ARGS=""
    fi
}

start_vm() {
    echo "Starting VM: $VM_NAME"
    echo "Image: $IMAGE_PATH"
    echo "Repository: $REPO_PATH"
    echo "Memory: $MEMORY"
    echo "CPUs: $CPUS"
    echo ""

    # Build bhyve command arguments
    local bhyve_args=(
        "-c" "$CPUS"
        "-m" "$MEMORY"
        "-w"  # Wire guest memory
        "-H"  # vmexit from HLT
        "-A"  # Generate ACPI tables
        "-P"  # Force virtio PCI to use MSI interrupts
        "-s" "0:0,hostbridge"
        "-s" "1:0,lpc"
        "-s" "2:0,ahci-hd,$IMAGE_PATH"
        "-s" "3:0,virtio-9p,repo=$REPO_PATH"
        "-l" "com1,stdio"
        "-l" "bootrom,$BHYVE_UEFI"
    )

    # Add network interface if configured
    if [[ -n "$NETWORK_ARGS" ]]; then
        bhyve_args+=($NETWORK_ARGS)
    fi

    # Add VM name
    bhyve_args+=("$VM_NAME")

    echo "Running: bhyve ${bhyve_args[*]}"
    echo ""
    echo "=== VM Console (Ctrl+C to exit) ==="
    echo ""

    # Run bhyve
    bhyve "${bhyve_args[@]}"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -i|--image)
            IMAGE_PATH="$2"
            shift 2
            ;;
        -r|--repo)
            REPO_PATH="$2"
            shift 2
            ;;
        -n|--name)
            VM_NAME="$2"
            shift 2
            ;;
        -m|--memory)
            MEMORY="$2"
            shift 2
            ;;
        -c|--cpus)
            CPUS="$2"
            shift 2
            ;;
        -t|--tap)
            TAP_DEVICE="$2"
            shift 2
            ;;
        -b|--bridge)
            BRIDGE_DEVICE="$2"
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
if [[ "$IMAGE_PATH" == "/path/to/cloudimage-ttya-openindiana-hipster-dev.raw" ]]; then
    echo "Error: Please specify the image path with -i or edit the script"
    usage
    exit 1
fi

if [[ "$REPO_PATH" == "/path/to/illumos/installer" ]]; then
    echo "Error: Please specify the repository path with -r or edit the script"
    usage
    exit 1
fi

# Trap to cleanup on exit
trap cleanup_vm EXIT

# Main execution
echo "=== bhyve Development VM Startup ==="
check_prerequisites
cleanup_vm
setup_networking
start_vm

echo ""
echo "VM has exited"
