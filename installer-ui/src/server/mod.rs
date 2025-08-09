//! Server communication functions for the Illumos installer
//!
//! This module handles all communication with machined servers,
//! including discovery, claiming, and installation operations.

use crate::state::{AddressKind, InstallerState, MachineServer, VDevType};
use dioxus::prelude::*;

/// Load available servers from machined discovery service
#[server(LoadAvailableServers)]
pub async fn load_available_servers() -> Result<Vec<MachineServer>, ServerFnError> {
    // This would connect to machined discovery service
    // For now, return mock data
    Ok(vec![
        MachineServer {
            id: "server-001".to_string(),
            name: "Machine 001".to_string(),
            address: "192.168.1.100".to_string(),
            status: crate::state::ServerStatus::Available,
            specs: crate::state::ServerSpecs {
                cpu_cores: 8,
                memory_gb: 32,
                storage_gb: 500,
                network_interfaces: 2,
            },
        },
        MachineServer {
            id: "server-002".to_string(),
            name: "Machine 002".to_string(),
            address: "192.168.1.101".to_string(),
            status: crate::state::ServerStatus::Busy,
            specs: crate::state::ServerSpecs {
                cpu_cores: 16,
                memory_gb: 64,
                storage_gb: 1000,
                network_interfaces: 4,
            },
        },
    ])
}

/// Claim a specific server for installation
#[server(ClaimServer)]
pub async fn claim_server(server_id: String) -> Result<(), ServerFnError> {
    // This would send a claim request to the specific machined server
    // Implementation would use the instcomd client to claim the server
    log::info!("Claiming server: {}", server_id);
    Ok(())
}

/// Perform installation on the claimed server
#[server(PerformInstallation)]
pub async fn perform_installation(config: InstallerState) -> Result<(), ServerFnError> {
    // Convert InstallerState to MachineConfig
    let machine_config = convert_to_machine_config(config)?;

    // Send configuration to the claimed server
    // This would use the machined client to start installation
    log::info!("Starting installation with config: {:?}", machine_config);

    // In a real implementation, this would:
    // 1. Convert the config to KDL format
    // 2. Send it to the machined server
    // 3. Monitor installation progress
    // 4. Stream logs back to the UI

    Ok(())
}

/// Convert InstallerState to MachineConfig format
fn convert_to_machine_config(
    state: InstallerState,
) -> Result<machineconfig::MachineConfig, ServerFnError> {
    // Convert pools
    let pools = state
        .pools
        .into_iter()
        .map(|pool| machineconfig::Pool {
            name: pool.name,
            vdevs: pool
                .vdevs
                .into_iter()
                .map(|vdev| machineconfig::VDev {
                    kind: match vdev.kind {
                        VDevType::Mirror => machineconfig::VDevType::Mirror,
                        VDevType::RaidZ => machineconfig::VDevType::RaidZ,
                        VDevType::RaidZ1 => machineconfig::VDevType::RaidZ1,
                        VDevType::RaidZ2 => machineconfig::VDevType::RaidZ2,
                        VDevType::RaidZ3 => machineconfig::VDevType::RaidZ3,
                        VDevType::Spare => machineconfig::VDevType::Spare,
                        VDevType::Log => machineconfig::VDevType::Log,
                        VDevType::Dedup => machineconfig::VDevType::Debup,
                        VDevType::Special => machineconfig::VDevType::Special,
                        VDevType::Cache => machineconfig::VDevType::Cache,
                    },
                    disks: vdev.disks,
                })
                .collect(),
            options: pool
                .options
                .into_iter()
                .map(|(k, v)| machineconfig::PoolOption { name: k, value: v })
                .collect(),
        })
        .collect();

    // Convert sysconfig
    let sysconfig = sysconfig::config::SysConfig {
        hostname: state.hostname,
        nameservers: state.nameservers,
        interfaces: state
            .interfaces
            .into_iter()
            .map(|iface| sysconfig::config::Interface {
                name: Some(iface.name),
                selector: iface.selector,
                addresses: iface
                    .addresses
                    .into_iter()
                    .map(|addr| sysconfig::config::AddressObject {
                        name: addr.name,
                        kind: match addr.kind {
                            AddressKind::Dhcp4 => sysconfig::config::AddressKind::Dhcp4,
                            AddressKind::Dhcp6 => sysconfig::config::AddressKind::Dhcp6,
                            AddressKind::Addrconf => sysconfig::config::AddressKind::Addrconf,
                            AddressKind::Static => sysconfig::config::AddressKind::Static,
                        },
                        address: addr.address,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(machineconfig::MachineConfig {
        pools,
        image: state.image,
        boot_environment_name: state.boot_environment_name,
        sysconfig,
    })
}
