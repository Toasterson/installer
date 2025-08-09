//! State management and data structures for the Illumos installer
//!
//! This module contains all the data structures used throughout the installer UI,
//! including server information, storage configuration, and installation state.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main state structure for the installer application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstallerState {
    pub selected_server: Option<String>,
    pub server_list: Vec<MachineServer>,
    pub pools: Vec<Pool>,
    pub image: String,
    pub boot_environment_name: Option<String>,
    pub hostname: String,
    pub nameservers: Vec<String>,
    pub interfaces: Vec<NetworkInterface>,
    pub installation_progress: f32,
    pub installation_log: Vec<String>,
}

/// Represents a machine server that can be used for installation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MachineServer {
    pub id: String,
    pub name: String,
    pub address: String,
    pub status: ServerStatus,
    pub specs: ServerSpecs,
}

/// Status of a machine server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerStatus {
    Available,
    Busy,
    Offline,
    Installing,
}

/// Hardware specifications of a machine server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerSpecs {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub network_interfaces: u32,
}

/// ZFS storage pool configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pool {
    pub name: String,
    pub vdevs: Vec<VDev>,
    pub options: HashMap<String, String>,
}

/// Virtual device configuration for ZFS pools
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VDev {
    pub kind: VDevType,
    pub disks: Vec<String>,
}

/// Types of virtual devices supported by ZFS
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VDevType {
    Mirror,
    RaidZ,
    RaidZ1,
    RaidZ2,
    RaidZ3,
    Spare,
    Log,
    Dedup,
    Special,
    Cache,
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkInterface {
    pub name: String,
    pub selector: Option<String>,
    pub addresses: Vec<AddressObject>,
}

/// Network address configuration object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddressObject {
    pub name: String,
    pub kind: AddressKind,
    pub address: Option<String>,
}

/// Types of network address configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AddressKind {
    Dhcp4,
    Dhcp6,
    Addrconf,
    Static,
}

impl Default for InstallerState {
    fn default() -> Self {
        Self {
            selected_server: None,
            server_list: vec![],
            pools: vec![Pool {
                name: "rpool".to_string(),
                vdevs: vec![],
                options: HashMap::new(),
            }],
            image: "oci://aopc.cloud/openindiana/hipster:2024.12".to_string(),
            boot_environment_name: None,
            hostname: "".to_string(),
            nameservers: vec!["9.9.9.9".to_string(), "149.112.112.112".to_string()],
            interfaces: vec![],
            installation_progress: 0.0,
            installation_log: vec![],
        }
    }
}

impl ServerStatus {
    /// Returns true if the server is available for installation
    pub fn is_available(&self) -> bool {
        matches!(self, ServerStatus::Available)
    }

    /// Returns a human-readable status string
    pub fn as_str(&self) -> &'static str {
        match self {
            ServerStatus::Available => "Available",
            ServerStatus::Busy => "Busy",
            ServerStatus::Offline => "Offline",
            ServerStatus::Installing => "Installing",
        }
    }

    /// Returns CSS class for status styling
    pub fn css_class(&self) -> &'static str {
        match self {
            ServerStatus::Available => "status-available",
            ServerStatus::Busy => "status-busy",
            ServerStatus::Offline => "status-offline",
            ServerStatus::Installing => "status-installing",
        }
    }
}

impl VDevType {
    /// Returns all available VDev types
    pub fn all() -> Vec<VDevType> {
        vec![
            VDevType::Mirror,
            VDevType::RaidZ,
            VDevType::RaidZ1,
            VDevType::RaidZ2,
            VDevType::RaidZ3,
            VDevType::Spare,
            VDevType::Log,
            VDevType::Dedup,
            VDevType::Special,
            VDevType::Cache,
        ]
    }

    /// Returns a human-readable name for the VDev type
    pub fn display_name(&self) -> &'static str {
        match self {
            VDevType::Mirror => "Mirror",
            VDevType::RaidZ => "RAID-Z",
            VDevType::RaidZ1 => "RAID-Z1",
            VDevType::RaidZ2 => "RAID-Z2",
            VDevType::RaidZ3 => "RAID-Z3",
            VDevType::Spare => "Spare",
            VDevType::Log => "Log",
            VDevType::Dedup => "Dedup",
            VDevType::Special => "Special",
            VDevType::Cache => "Cache",
        }
    }

    /// Returns the minimum number of disks required for this VDev type
    pub fn min_disks(&self) -> usize {
        match self {
            VDevType::Mirror => 2,
            VDevType::RaidZ | VDevType::RaidZ1 => 3,
            VDevType::RaidZ2 => 4,
            VDevType::RaidZ3 => 5,
            VDevType::Spare => 1,
            VDevType::Log => 1,
            VDevType::Dedup => 1,
            VDevType::Special => 1,
            VDevType::Cache => 1,
        }
    }
}

impl AddressKind {
    /// Returns all available address kinds
    pub fn all() -> Vec<AddressKind> {
        vec![
            AddressKind::Dhcp4,
            AddressKind::Dhcp6,
            AddressKind::Addrconf,
            AddressKind::Static,
        ]
    }

    /// Returns a human-readable name for the address kind
    pub fn display_name(&self) -> &'static str {
        match self {
            AddressKind::Dhcp4 => "DHCP IPv4",
            AddressKind::Dhcp6 => "DHCP IPv6",
            AddressKind::Addrconf => "IPv6 Auto-configuration",
            AddressKind::Static => "Static IP",
        }
    }

    /// Returns true if this address kind requires manual IP configuration
    pub fn requires_address(&self) -> bool {
        matches!(self, AddressKind::Static)
    }
}
