# Integration Guide for illumos Installer UI

This document explains how to integrate the installer UI with machined servers and other components of the illumos installer ecosystem.

## Overview

The installer UI is designed to work with the following components:

- **machined servers**: Physical or virtual machines running the machined daemon
- **instcomd**: Installation command daemon for orchestrating installations
- **sysconfig service**: System configuration management service
- **OCI registries**: Container image repositories for illumos images

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Installer UI  │    │   machined      │    │   Target        │
│                 │    │   Discovery     │    │   Machine       │
│   - Web/Desktop │◄──►│   Service       │◄──►│                 │
│   - Mobile      │    │                 │    │   - Hardware    │
│   - State Mgmt  │    │   - Claims      │    │   - Storage     │
└─────────────────┘    │   - Status      │    │   - Network     │
         │              │   - Install     │    └─────────────────┘
         │              └─────────────────┘
         │
         ▼
┌─────────────────┐    ┌─────────────────┐
│   instcomd      │    │   OCI Registry  │
│                 │    │                 │
│   - Config      │◄──►│   - Images      │
│   - Validation  │    │   - Manifests   │
│   - Execution   │    │   - Layers      │
└─────────────────┘    └─────────────────┘
```

## Server Functions

The UI uses Dioxus server functions to communicate with backend services. These functions provide a clean abstraction layer between the frontend and backend systems.

### Machine Discovery

```rust
#[server(LoadAvailableServers)]
async fn load_available_servers() -> Result<Vec<MachineServer>, ServerFnError> {
    // Implementation connects to machined discovery service
    // Returns list of available machines with specs and status
}
```

**Integration Points:**
- Connects to machined discovery endpoint (typically UDP multicast)
- Queries machine specifications (CPU, RAM, storage, NICs)
- Filters by availability status
- Handles authentication/authorization

### Machine Claiming

```rust
#[server(ClaimServer)]
async fn claim_server(server_id: String) -> Result<(), ServerFnError> {
    // Implementation sends claim request to specific machined server
    // Uses instcomd client to reserve the machine
}
```

**Integration Points:**
- HTTP/gRPC connection to machined server
- Authentication with server certificates
- Claim timeout and renewal mechanisms
- Conflict resolution for simultaneous claims

### Installation Execution

```rust
#[server(PerformInstallation)]
async fn perform_installation(config: InstallerState) -> Result<(), ServerFnError> {
    // Converts UI state to machine configuration
    // Sends to claimed server for installation
}
```

**Integration Points:**
- Configuration validation and sanitization
- KDL serialization of machine config
- Progress monitoring and log streaming
- Error handling and rollback procedures

## Configuration Format

The UI generates machine configurations in KDL (KDL Document Language) format that are consumed by the machined servers.

### Structure

```kdl
// Storage configuration
pool "rpool" {
    vdev "mirror" {
        disks "disk1" "disk2"
    }
    options {
        compression "zstd"
    }
}

// Image specification
image "oci://registry.example.com/illumos/base:latest"

// System configuration
sysconfig {
    hostname "machine-01"
    nameserver "8.8.8.8"
    interface "net0" {
        address name="primary" kind="static" "192.168.1.100/24"
    }
}
```

### Validation Rules

1. **Pool Names**: Must be unique and valid ZFS pool names
2. **Disk Identifiers**: Must match available disks on target machine
3. **Network Interfaces**: Must correspond to physical NICs
4. **IP Addresses**: Must be valid and not conflict with existing networks
5. **Hostnames**: Must be valid DNS names

## Network Discovery Protocol

The UI discovers available machines through a custom discovery protocol:

### Message Format

```json
{
  "type": "discovery_request",
  "timestamp": "2024-12-09T10:30:00Z",
  "client_id": "ui-session-12345"
}
```

### Response Format

```json
{
  "type": "discovery_response",
  "machine_id": "server-001",
  "hostname": "machine-01.local",
  "address": "192.168.1.100",
  "status": "available",
  "specs": {
    "cpu_cores": 16,
    "memory_gb": 64,
    "storage_gb": 2000,
    "network_interfaces": 4
  },
  "capabilities": ["zfs", "bhyve", "zones"],
  "timestamp": "2024-12-09T10:30:01Z"
}
```

## Authentication and Security

### TLS Configuration

All communication between components uses TLS with mutual authentication:

```toml
[tls]
cert_file = "/etc/installer/client.crt"
key_file = "/etc/installer/client.key"
ca_file = "/etc/installer/ca.crt"
verify_peer = true
```

### Authorization

The installer UI must be authorized to:
- Discover available machines
- Claim machines for installation
- Execute installation commands
- Monitor installation progress

## Error Handling

### Network Errors

- Connection timeouts: Retry with exponential backoff
- Discovery failures: Fall back to manual machine entry
- Certificate errors: Display clear error messages

### Validation Errors

- Invalid configurations: Highlight specific fields
- Resource conflicts: Suggest alternatives
- Hardware incompatibility: Show requirements

### Installation Errors

- Disk failures: Offer alternative disk configurations
- Network issues: Provide network troubleshooting
- Image pull failures: Suggest alternative registries

## Development Setup

### Prerequisites

1. **machined simulator**: For testing without physical hardware
2. **Local OCI registry**: For testing image installations
3. **Mock discovery service**: For UI development

### Running the Stack

```bash
# Start the discovery service
./scripts/start-discovery.sh

# Start a mock machined server
./scripts/start-mock-machined.sh

# Start the installer UI
cd installer-ui
dx serve --platform web
```

### Testing

```bash
# Run integration tests
cargo test --test integration

# Test with physical machines
./scripts/test-physical.sh

# Load testing
./scripts/load-test.sh
```

## Production Deployment

### Infrastructure Requirements

- **Load balancer**: For multiple UI instances
- **Service discovery**: For finding machined servers
- **Monitoring**: For tracking installation success rates
- **Logging**: Centralized logging for troubleshooting

### Configuration Management

```yaml
# installer-ui.yaml
discovery:
  multicast_group: "239.255.255.250"
  port: 1900
  timeout: 30s

machined:
  default_port: 8443
  connection_timeout: 60s
  claim_timeout: 300s

installation:
  max_concurrent: 10
  progress_interval: 5s
  log_level: "info"
```

### Monitoring and Alerting

Key metrics to monitor:

- **Discovery success rate**: Percentage of successful machine discoveries
- **Claim success rate**: Percentage of successful machine claims
- **Installation success rate**: Percentage of successful installations
- **Installation duration**: Time from start to completion
- **Error rates**: By category (network, storage, image, etc.)

### Backup and Recovery

- **Configuration backup**: Regular backup of validated configurations
- **State recovery**: Ability to resume interrupted installations
- **Rollback procedures**: Steps to undo failed installations

## API Reference

### REST Endpoints

```
GET /api/machines           # List available machines
POST /api/machines/{id}/claim # Claim a machine
GET /api/machines/{id}/status # Get machine status
POST /api/install           # Start installation
GET /api/install/{id}/logs  # Get installation logs
GET /api/install/{id}/progress # Get installation progress
```

### WebSocket Events

```
machines.discovered         # New machine discovered
machines.claimed           # Machine claimed
machines.released          # Machine released
installation.started       # Installation started
installation.progress      # Installation progress update
installation.completed     # Installation completed
installation.failed        # Installation failed
```

## Troubleshooting

### Common Issues

1. **No machines discovered**
   - Check network connectivity
   - Verify multicast routing
   - Check firewall rules

2. **Cannot claim machine**
   - Machine may be already claimed
   - Check authentication credentials
   - Verify machine is in available state

3. **Installation fails**
   - Check disk availability
   - Verify network configuration
   - Check image accessibility

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug dx serve --platform desktop
```

### Log Analysis

Key log patterns to watch for:

```
[INFO] Machine discovered: server-001 at 192.168.1.100
[WARN] Claim timeout for machine server-001
[ERROR] Installation failed: disk c0t0d0 not found
[DEBUG] Configuration validated successfully
```
