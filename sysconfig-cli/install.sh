#!/bin/bash

# Sysconfig CLI Installation Script
# This script builds and installs the sysconfig-cli tool

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default installation directory
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root for system-wide installation
check_permissions() {
    if [ "$INSTALL_DIR" = "/usr/local/bin" ] || [ "$INSTALL_DIR" = "/usr/bin" ]; then
        if [ "$EUID" -ne 0 ]; then
            print_error "System-wide installation requires root privileges"
            echo "Please run with sudo or set INSTALL_DIR to a user-writable location:"
            echo "  INSTALL_DIR=~/.local/bin $0"
            exit 1
        fi
    fi
}

# Check for required dependencies
check_dependencies() {
    print_info "Checking dependencies..."

    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed"
        echo "Please install Rust and Cargo from https://rustup.rs/"
        exit 1
    fi

    if ! command -v protoc &> /dev/null; then
        print_warning "protoc (Protocol Buffers compiler) not found"
        echo "The build might fail without protoc. Install it with:"
        echo "  Ubuntu/Debian: sudo apt-get install protobuf-compiler"
        echo "  Fedora: sudo dnf install protobuf-compiler"
        echo "  macOS: brew install protobuf"
        echo ""
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    print_success "Dependencies check complete"
}

# Build the project
build_project() {
    print_info "Building sysconfig-cli..."

    if [ ! -f "Cargo.toml" ]; then
        print_error "Cargo.toml not found. Please run this script from the sysconfig-cli directory"
        exit 1
    fi

    # Check if sysconfig proto file exists
    if [ ! -f "../sysconfig/proto/sysconfig.proto" ]; then
        print_error "Cannot find sysconfig.proto file at ../sysconfig/proto/"
        echo "Please ensure you're running this from the correct directory structure"
        exit 1
    fi

    # Build in release mode
    if cargo build --release; then
        print_success "Build completed successfully"
    else
        print_error "Build failed"
        exit 1
    fi
}

# Install the binary
install_binary() {
    local binary_path="target/release/sysconfig-cli"

    if [ ! -f "$binary_path" ]; then
        print_error "Binary not found at $binary_path"
        exit 1
    fi

    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        print_info "Creating installation directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi

    print_info "Installing to $INSTALL_DIR/sysconfig-cli"

    # Copy the binary
    if cp "$binary_path" "$INSTALL_DIR/sysconfig-cli"; then
        chmod +x "$INSTALL_DIR/sysconfig-cli"
        print_success "Installation completed"
    else
        print_error "Failed to copy binary to $INSTALL_DIR"
        exit 1
    fi
}

# Verify installation
verify_installation() {
    print_info "Verifying installation..."

    # Check if the binary is in PATH
    if command -v sysconfig-cli &> /dev/null; then
        print_success "sysconfig-cli is available in PATH"
        echo ""
        sysconfig-cli --version
    else
        print_warning "sysconfig-cli is not in PATH"
        echo "Add $INSTALL_DIR to your PATH by adding this to your shell configuration:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
        echo "Or run the tool directly:"
        echo "  $INSTALL_DIR/sysconfig-cli --help"
    fi
}

# Uninstall function
uninstall() {
    print_info "Uninstalling sysconfig-cli..."

    if [ -f "$INSTALL_DIR/sysconfig-cli" ]; then
        if rm "$INSTALL_DIR/sysconfig-cli"; then
            print_success "sysconfig-cli has been removed from $INSTALL_DIR"
        else
            print_error "Failed to remove sysconfig-cli from $INSTALL_DIR"
            echo "You may need to run with sudo"
            exit 1
        fi
    else
        print_warning "sysconfig-cli not found at $INSTALL_DIR"
    fi
}

# Main script
main() {
    echo -e "${BLUE}=== Sysconfig CLI Installer ===${NC}"
    echo ""

    # Parse arguments
    case "${1:-}" in
        uninstall|--uninstall|-u)
            check_permissions
            uninstall
            exit 0
            ;;
        help|--help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --uninstall, -u    Uninstall sysconfig-cli"
            echo "  --help, -h         Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  INSTALL_DIR        Installation directory (default: /usr/local/bin)"
            echo ""
            echo "Examples:"
            echo "  # Install system-wide (requires sudo)"
            echo "  sudo $0"
            echo ""
            echo "  # Install to user directory"
            echo "  INSTALL_DIR=~/.local/bin $0"
            echo ""
            echo "  # Uninstall"
            echo "  sudo $0 --uninstall"
            exit 0
            ;;
    esac

    # Installation process
    check_permissions
    check_dependencies
    build_project
    install_binary
    verify_installation

    echo ""
    echo -e "${GREEN}=== Installation Complete ===${NC}"
    echo ""
    echo "Quick start:"
    echo "  sysconfig-cli --help           # Show help"
    echo "  sysconfig-cli get              # Get current state"
    echo "  sysconfig-cli watch            # Watch for state changes"
    echo ""
    echo "For more information, see the README.md file"
}

# Run main function
main "$@"
