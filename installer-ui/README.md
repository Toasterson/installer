# illumos Installer UI

A modern web-based installer interface for illumos systems built with Dioxus and Rust.

## Overview

This installer UI provides a step-by-step guided interface for installing illumos on target machines. It allows users to:

- **Select and claim target machines** from available machined servers
- **Configure ZFS storage pools** with various vdev types and options
- **Set up network interfaces** with static, DHCP, or auto-configuration
- **Configure system settings** like hostname and DNS servers
- **Review configuration** before installation
- **Monitor installation progress** with real-time logs

## Architecture

The installer UI is built with:

- **Dioxus**: Modern Rust framework for building cross-platform UIs
- **Server Functions**: Backend integration with machined servers
- **State Management**: Centralized installer state using Dioxus signals
- **Responsive Design**: Works on desktop, web, and mobile platforms

## Project Structure

```
installer-ui/
├─ assets/           # CSS styles and static assets
│  └─ main.css      # Main stylesheet with installer theme
├─ src/
│  ├─ main.rs       # Main application entry point
│  ├─ components/   # UI components organized by functionality
│  │  ├─ mod.rs     # Component module exports
│  │  ├─ layout.rs  # MainLayout, navigation, and app structure
│  │  └─ pages/     # Individual page components
│  │     ├─ mod.rs  # Page component exports
│  │     ├─ welcome.rs                # Welcome page
│  │     ├─ server_selection.rs       # Server selection page
│  │     ├─ storage_configuration.rs  # Storage configuration page
│  │     ├─ network_configuration.rs  # Network configuration page
│  │     ├─ system_configuration.rs   # System configuration page
│  │     ├─ review_configuration.rs   # Review configuration page
│  │     └─ installation.rs           # Installation progress page
│  ├─ routes/       # Route definitions and navigation
│  │  └─ mod.rs     # Route enum and navigation helpers
│  ├─ state/        # Application state management
│  │  └─ mod.rs     # InstallerState and data structures
│  └─ server/       # Server communication functions
│     └─ mod.rs     # machined server integration
├─ Cargo.toml       # Dependencies and features
└─ Dioxus.toml      # Dioxus configuration
```

## Modular Architecture

The application has been refactored into a clean modular structure:

### Components (`src/components/`)
- **Layout components**: MainLayout, navigation, progress indicators
- **Page components**: Individual pages for each installation step
- **Reusable components**: Cards, forms, validation summaries

### State Management (`src/state/`)
- **InstallerState**: Central application state
- **Data structures**: Server, storage, network configuration types
- **Validation helpers**: Methods for validating configuration steps

### Server Functions (`src/server/`)
- **Discovery**: Load and discover available machines
- **Communication**: Claim servers and manage installation
- **Config conversion**: Transform UI state to machine configuration

### Routing (`src/routes/`)
- **Route definitions**: All application routes and navigation
- **Route helpers**: Step validation, progress tracking, navigation utilities

## Installation Flow

The installer guides users through 7 steps:

1. **Welcome** - Introduction and overview
2. **Server Selection** - Choose and claim a target machine
3. **Storage Configuration** - Configure ZFS pools and datasets
4. **Network Configuration** - Set up network interfaces
5. **System Configuration** - Configure hostname and DNS
6. **Review Configuration** - Verify all settings
7. **Installation** - Execute the installation process

## Development

### Prerequisites

- Rust 1.70+ with Cargo
- Dioxus CLI (`dx`) tool

### Running the Application

For desktop development:
```bash
dx serve --platform desktop
```

For web development:
```bash
dx serve --platform web
```

For mobile development:
```bash
dx serve --platform mobile
```

### Window Sizing Options

The installer UI is optimized for different window sizes:

#### Using the launcher script:
```bash
./launcher.sh           # Interactive menu with presets
./run.sh --compact      # 900x650 - Good for laptops
./run.sh --small        # 800x600 - Minimal space
./run.sh                # 1024x768 - Standard desktop
./run.sh --large        # 1200x900 - Large monitors
./run.sh --fullhd       # 1920x1080 - Full HD displays
```

#### Custom window sizes:
```bash
./run.sh --width 1024 --height 600    # Custom dimensions
./run.sh --platform web               # Web browser (no window size)
```

The UI automatically adapts to smaller windows with:
- Compact layouts for windows under 800px height
- Ultra-compact mode for windows under 500px height
- Responsive design for mobile devices
- Optimized content density and spacing

### Building

To build the application:
```bash
cargo build --release
```

## Configuration

The installer generates a machine configuration in KDL format that includes:

- **Storage pools** with vdevs and ZFS options
- **Network interfaces** with addressing configuration
- **System settings** like hostname and DNS
- **OCI image** specification for installation

Example generated configuration:
```kdl
pool "rpool" {
    vdev "mirror" {
        disks "c5t0d0" "c6t0d0"
    }
    options {
        compression "zstd"
    }
}

image "oci://aopc.cloud/openindiana/hipster:2024.12"

sysconfig {
    hostname "node01"
    nameserver "9.9.9.9"
    nameserver "149.112.112.112"
    interface "net0" selector="mac:00:00:00:00" {
        address name="v4" kind="static" "192.168.1.200/24"
    }
}
```

## Server Integration

The UI communicates with machined servers through server functions:

- `load_available_servers()` - Discover available machines
- `claim_server(server_id)` - Claim a machine for installation
- `perform_installation(config)` - Execute the installation

## Features

### Storage Configuration
- Support for various vdev types (mirror, raidz, etc.)
- Configurable ZFS options (compression, dedup, etc.)
- Multiple pool support
- Disk selection and validation

### Network Configuration
- Multiple interface support
- DHCP v4/v6, static, and auto-configuration
- MAC address selectors
- IPv4 and IPv6 addressing

### System Configuration
- Hostname validation
- Multiple DNS servers
- Timezone and locale settings (planned)

### Installation Monitoring
- Real-time progress tracking
- Live log streaming
- Error handling and recovery

## Styling

The UI uses a modern dark theme with:
- Gradient backgrounds and glassmorphism effects
- Responsive grid layouts
- Smooth animations and transitions
- Accessible color contrasts
- Mobile-first responsive design

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

This project is licensed under the same terms as the parent illumos installer project.