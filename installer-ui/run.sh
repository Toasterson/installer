#!/bin/bash

# illumos Installer UI Runner
# This script starts the installer UI with optimal window configuration

set -e

# Check if dx (Dioxus CLI) is available
if ! command -v dx &> /dev/null; then
    echo "Error: Dioxus CLI (dx) not found."
    echo "Install it with: cargo install dioxus-cli"
    exit 1
fi

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]] || [[ ! -f "src/main.rs" ]]; then
    echo "Error: Please run this script from the installer-ui directory"
    exit 1
fi

# Default configuration
PLATFORM="desktop"
WIDTH=1024
HEIGHT=768
HOT_RELOAD="true"
RELEASE="false"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        --width)
            WIDTH="$2"
            shift 2
            ;;
        --height)
            HEIGHT="$2"
            shift 2
            ;;
        --no-hot-reload)
            HOT_RELOAD="false"
            shift
            ;;
        --release)
            RELEASE="true"
            shift
            ;;
        --compact)
            WIDTH=900
            HEIGHT=650
            shift
            ;;
        --small)
            WIDTH=800
            HEIGHT=600
            shift
            ;;
        --large)
            WIDTH=1200
            HEIGHT=900
            shift
            ;;
        --fullhd)
            WIDTH=1920
            HEIGHT=1080
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --platform PLATFORM    Platform to run on (desktop, web, mobile) [default: desktop]"
            echo "  --width WIDTH          Window width in pixels [default: 1024]"
            echo "  --height HEIGHT        Window height in pixels [default: 768]"
            echo "  --no-hot-reload        Disable hot reload"
            echo "  --release              Build in release mode"
            echo "  --compact              Use compact window size (900x650)"
            echo "  --small                Use small window size (800x600)"
            echo "  --large                Use large window size (1200x900)"
            echo "  --fullhd               Use full HD window size (1920x1080)"
            echo "  --help                 Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                     # Run with default settings"
            echo "  $0 --compact           # Run in compact mode"
            echo "  $0 --small             # Run in small window"
            echo "  $0 --platform web      # Run as web app"
            echo "  $0 --no-hot-reload     # Disable hot reload"
            echo "  $0 --width 1024 --height 600  # Custom size"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Update Dioxus.toml with window configuration
if [[ "$PLATFORM" == "desktop" ]]; then
    cat > Dioxus.toml << EOF
[application]

[web.app]
title = "illumos Installer"

[desktop.app]
title = "illumos Installer"
width = $WIDTH
height = $HEIGHT
min_width = 800
min_height = 600
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

# Set environment variables for optimal performance
export RUST_LOG=info
export RUST_BACKTRACE=1

# Build arguments
BUILD_ARGS=""
if [[ "$RELEASE" == "true" ]]; then
    BUILD_ARGS="--release"
fi

# Serve arguments
SERVE_ARGS="--platform $PLATFORM"
if [[ "$HOT_RELOAD" == "false" ]]; then
    SERVE_ARGS="$SERVE_ARGS --hot-reload false"
fi

if [[ "$RELEASE" == "true" ]]; then
    SERVE_ARGS="$SERVE_ARGS $BUILD_ARGS"
fi

echo "🚀 Starting illumos Installer UI"
echo "   Platform: $PLATFORM"
if [[ "$PLATFORM" == "desktop" ]]; then
    echo "   Window: ${WIDTH}x${HEIGHT}"
fi
echo "   Hot Reload: $HOT_RELOAD"
echo "   Release Mode: $RELEASE"
echo ""

# Start the application
echo "📱 Launching installer..."
dx serve $SERVE_ARGS

# Clean up on exit
trap 'echo "🛑 Shutting down installer..."' EXIT
