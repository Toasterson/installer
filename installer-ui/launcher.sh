#!/bin/bash

# illumos Installer UI Launcher
# Quick launcher with predefined window configurations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to show menu
show_menu() {
    clear
    print_color $BLUE "╔══════════════════════════════════════╗"
    print_color $BLUE "║        illumos Installer UI         ║"
    print_color $BLUE "╚══════════════════════════════════════╝"
    echo
    print_color $GREEN "Select window configuration:"
    echo
    echo "1) Compact     (900x650)  - Good for laptops"
    echo "2) Default     (1024x768) - Standard desktop"
    echo "3) Large       (1200x900) - Big monitors"
    echo "4) Small       (800x600)  - Minimal space"
    echo "5) Web         (Browser)  - Web platform"
    echo "6) Custom      (Specify)  - Custom dimensions"
    echo
    echo "q) Quit"
    echo
    print_color $YELLOW "Enter your choice [1-6/q]: "
}

# Function to run with specific configuration
run_installer() {
    local width=$1
    local height=$2
    local platform=${3:-desktop}

    print_color $GREEN "🚀 Starting illumos Installer UI..."
    print_color $BLUE "   Platform: $platform"
    if [[ "$platform" == "desktop" ]]; then
        print_color $BLUE "   Window Size: ${width}x${height}"
    fi
    echo

    # Check if in correct directory
    if [[ ! -f "Cargo.toml" ]]; then
        print_color $RED "Error: Not in installer-ui directory"
        exit 1
    fi

    # Update Dioxus.toml for desktop
    if [[ "$platform" == "desktop" ]]; then
        cat > Dioxus.toml << EOF
[application]

[web.app]
title = "illumos Installer"

[desktop.app]
title = "illumos Installer"
width = $width
height = $height
min_width = 800
min_height = 500
resizable = true
maximized = false
decorations = true

[web.resource]
style = []
script = []

[web.resource.dev]
script = []
EOF
    fi

    # Set environment
    export RUST_LOG=info
    export RUST_BACKTRACE=1

    # Check for dx command
    if ! command -v dx &> /dev/null; then
        print_color $RED "Error: Dioxus CLI (dx) not found"
        print_color $YELLOW "Install with: cargo install dioxus-cli"
        exit 1
    fi

    # Launch
    dx serve --platform $platform
}

# Function to get custom dimensions
get_custom_dimensions() {
    echo
    print_color $YELLOW "Enter custom window dimensions:"
    read -p "Width (pixels): " width
    read -p "Height (pixels): " height

    # Validate input
    if ! [[ "$width" =~ ^[0-9]+$ ]] || ! [[ "$height" =~ ^[0-9]+$ ]]; then
        print_color $RED "Invalid dimensions. Using defaults."
        width=1024
        height=768
    fi

    # Apply minimums
    if (( width < 800 )); then width=800; fi
    if (( height < 500 )); then height=500; fi

    echo "Using: ${width}x${height}"
    run_installer $width $height
}

# Main menu loop
while true; do
    show_menu
    read -n 1 choice
    echo

    case $choice in
        1)
            print_color $GREEN "Selected: Compact Mode"
            run_installer 900 650
            break
            ;;
        2)
            print_color $GREEN "Selected: Default Mode"
            run_installer 1024 768
            break
            ;;
        3)
            print_color $GREEN "Selected: Large Mode"
            run_installer 1200 900
            break
            ;;
        4)
            print_color $GREEN "Selected: Small Mode"
            run_installer 800 600
            break
            ;;
        5)
            print_color $GREEN "Selected: Web Mode"
            run_installer 0 0 web
            break
            ;;
        6)
            get_custom_dimensions
            break
            ;;
        q|Q)
            print_color $YELLOW "Goodbye!"
            exit 0
            ;;
        *)
            print_color $RED "Invalid choice. Please try again."
            sleep 1
            ;;
    esac
done
