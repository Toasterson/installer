//! Unified Configuration Schema for Cloud Instance Provisioning
//!
//! This crate provides a type-safe, unified configuration schema that replaces
//! the complex and redundant structure of cloud-init with a clean, hierarchical,
//! and domain-driven approach.
//!
//! The schema is designed with the following principles:
//! - **Orthogonality**: One concept, one configuration path
//! - **Clarity and Explicitness**: Self-documenting field names and structures
//! - **Type Safety**: Maps directly to strongly-typed language constructs
//! - **Composition over Proliferation**: Structured by functional domain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The root configuration for a cloud instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedConfig {
    /// Configuration related to the system's identity and environment.
    pub system: Option<SystemConfig>,
    /// Configuration for storage devices, partitions, and filesystems.
    pub storage: Option<StorageConfig>,
    /// Configuration for network services like DNS and interfaces.
    pub networking: Option<NetworkingConfig>,
    /// Configuration for software packages, repositories, and updates.
    pub software: Option<SoftwareConfig>,
    /// A list of user accounts to create and configure.
    pub users: Vec<UserConfig>,
    /// Configuration for running imperative scripts at various boot stages.
    pub scripts: Option<ScriptConfig>,
    /// Configuration for bootstrapping third-party tools like Ansible or Puppet.
    pub integrations: Option<IntegrationConfig>,
    /// Configuration for containers, zones, and jails.
    pub containers: Option<ContainerConfig>,
    /// Defines the final power state of the machine after initialization.
    pub power_state: Option<PowerStateConfig>,
}

/// Configuration related to the system's identity and environment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemConfig {
    /// The system hostname (short name).
    pub hostname: Option<String>,
    /// The fully qualified domain name.
    pub fqdn: Option<String>,
    /// The system timezone (e.g., "UTC", "America/New_York").
    pub timezone: Option<String>,
    /// The system locale (e.g., "en_US.UTF-8").
    pub locale: Option<String>,
    /// Additional environment variables to set system-wide.
    pub environment: HashMap<String, String>,
}

/// Configuration for storage devices, partitions, and filesystems.
/// Configuration for storage management.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageConfig {
    /// List of filesystems to create or manage.
    pub filesystems: Vec<FilesystemConfig>,
    /// List of storage pools (e.g., ZFS pools).
    pub pools: Vec<StoragePoolConfig>,
    /// List of mount points and their configuration.
    pub mounts: Vec<MountConfig>,
    /// List of ZFS datasets with advanced configuration.
    pub zfs_datasets: Vec<ZfsDatasetConfig>,
    /// List of ZFS snapshots to create.
    pub zfs_snapshots: Vec<ZfsSnapshotConfig>,
    /// ZFS replication configuration.
    pub zfs_replication: Vec<ZfsReplicationConfig>,
}

/// Configuration for a filesystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilesystemConfig {
    /// The device or pool/dataset path.
    pub device: String,
    /// The filesystem type.
    pub fstype: FilesystemType,
    /// Filesystem creation options.
    pub options: HashMap<String, String>,
    /// Whether to format the device.
    pub format: bool,
}

/// Supported filesystem types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilesystemType {
    #[serde(rename = "zfs")]
    Zfs,
    #[serde(rename = "ufs")]
    Ufs,
    #[serde(rename = "ext4")]
    Ext4,
    #[serde(rename = "xfs")]
    Xfs,
    #[serde(rename = "btrfs")]
    Btrfs,
    #[serde(rename = "ntfs")]
    Ntfs,
    #[serde(rename = "fat32")]
    Fat32,
}

/// Configuration for a storage pool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoragePoolConfig {
    /// The name of the storage pool.
    pub name: String,
    /// The type of storage pool.
    pub pool_type: StoragePoolType,
    /// List of devices that make up the pool.
    pub devices: Vec<String>,
    /// Pool-specific properties.
    pub properties: HashMap<String, String>,
    /// ZFS pool topology configuration.
    pub topology: Option<ZfsPoolTopology>,
}

/// Supported storage pool types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StoragePoolType {
    #[serde(rename = "zpool")]
    ZfsPool,
    #[serde(rename = "lvm")]
    Lvm,
}

/// Configuration for a mount point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MountConfig {
    /// The source device, filesystem, or path.
    pub source: String,
    /// The target mount point.
    pub target: String,
    /// The filesystem type for the mount.
    pub fstype: Option<String>,
    /// Mount options.
    pub options: Vec<String>,
    /// Whether to persist the mount in fstab/vfstab.
    pub persistent: bool,
}

/// Advanced ZFS dataset configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZfsDatasetConfig {
    /// The name of the dataset (pool/dataset/path).
    pub name: String,
    /// Dataset type.
    pub dataset_type: ZfsDatasetType,
    /// Dataset properties.
    pub properties: HashMap<String, String>,
    /// Dataset quotas.
    pub quota: Option<String>,
    /// Dataset reservations.
    pub reservation: Option<String>,
    /// Child datasets to create.
    pub children: Vec<ZfsDatasetConfig>,
}

/// ZFS dataset types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZfsDatasetType {
    #[serde(rename = "filesystem")]
    Filesystem,
    #[serde(rename = "volume")]
    Volume { size: String },
}

/// ZFS pool topology configuration for advanced layouts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZfsPoolTopology {
    /// Data vdevs configuration.
    pub data: Vec<ZfsVdevConfig>,
    /// Log vdevs configuration.
    pub log: Vec<ZfsVdevConfig>,
    /// Cache vdevs configuration.
    pub cache: Vec<ZfsVdevConfig>,
    /// Spare vdevs configuration.
    pub spare: Vec<String>,
}

/// ZFS vdev configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZfsVdevConfig {
    /// Vdev type.
    pub vdev_type: ZfsVdevType,
    /// List of devices in this vdev.
    pub devices: Vec<String>,
}

/// ZFS vdev types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZfsVdevType {
    #[serde(rename = "stripe")]
    Stripe,
    #[serde(rename = "mirror")]
    Mirror,
    #[serde(rename = "raidz")]
    Raidz,
    #[serde(rename = "raidz2")]
    Raidz2,
    #[serde(rename = "raidz3")]
    Raidz3,
}

/// ZFS snapshot configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZfsSnapshotConfig {
    /// The dataset to snapshot.
    pub dataset: String,
    /// Snapshot name.
    pub name: String,
    /// Whether to create snapshots recursively.
    pub recursive: bool,
    /// Properties to set on the snapshot.
    pub properties: HashMap<String, String>,
}

/// ZFS replication configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZfsReplicationConfig {
    /// Source dataset.
    pub source_dataset: String,
    /// Target system and dataset.
    pub target: String,
    /// Replication type.
    pub replication_type: ZfsReplicationType,
    /// SSH configuration for remote replication.
    pub ssh_config: Option<SshConfig>,
    /// Properties to exclude from replication.
    pub exclude_properties: Vec<String>,
}

/// ZFS replication types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZfsReplicationType {
    #[serde(rename = "send")]
    Send,
    #[serde(rename = "incremental")]
    Incremental,
    #[serde(rename = "full")]
    Full,
}

/// SSH configuration for remote operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SshConfig {
    /// SSH username.
    pub user: String,
    /// SSH host.
    pub host: String,
    /// SSH port.
    pub port: Option<u16>,
    /// SSH key path.
    pub key_path: Option<String>,
}

/// Configuration for network services and interfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkingConfig {
    /// List of network interfaces to configure.
    pub interfaces: Vec<NetworkInterfaceConfig>,
    /// List of DNS nameservers.
    pub nameservers: Vec<String>,
    /// List of DNS search domains.
    pub search_domains: Vec<String>,
    /// Static routes to configure.
    pub routes: Vec<RouteConfig>,
    /// NTP servers for time synchronization.
    pub ntp_servers: Vec<String>,
}

/// Configuration for a network interface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkInterfaceConfig {
    /// The interface name (e.g., "eth0", "net0").
    pub name: String,
    /// Optional MAC address selector for interface identification.
    pub mac_address: Option<String>,
    /// List of IP addresses to configure on this interface.
    pub addresses: Vec<AddressConfig>,
    /// Default gateway for this interface.
    pub gateway: Option<String>,
    /// MTU size for the interface.
    pub mtu: Option<u16>,
    /// Human-readable description.
    pub description: Option<String>,
    /// VLAN configuration if this is a VLAN interface.
    pub vlan: Option<VlanConfig>,
}

/// Configuration for an IP address on an interface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddressConfig {
    /// A name for this address configuration.
    pub name: String,
    /// The type of address configuration.
    pub kind: AddressKind,
}

/// Types of address configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AddressKind {
    /// Static IP address with CIDR notation.
    #[serde(rename = "static")]
    Static(String),
    /// DHCPv4 configuration.
    #[serde(rename = "dhcp4")]
    Dhcp4,
    /// DHCPv6 configuration.
    #[serde(rename = "dhcp6")]
    Dhcp6,
    /// IPv6 address autoconfiguration.
    #[serde(rename = "addrconf")]
    Addrconf,
}

/// VLAN configuration for an interface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VlanConfig {
    /// VLAN ID.
    pub id: u16,
    /// Parent interface name.
    pub parent: String,
}

/// Configuration for a static route.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteConfig {
    /// Destination network in CIDR notation.
    pub destination: String,
    /// Gateway IP address.
    pub gateway: String,
    /// Interface to use for this route.
    pub interface: Option<String>,
    /// Route metric/priority.
    pub metric: Option<u32>,
}

/// Configuration for software packages, repositories, and updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoftwareConfig {
    /// If true, runs the package manager's update command on boot.
    pub update_on_boot: bool,
    /// If true, runs the package manager's upgrade command on boot.
    pub upgrade_on_boot: bool,
    /// A list of packages to install.
    pub packages_to_install: Vec<String>,
    /// A list of packages to remove.
    pub packages_to_remove: Vec<String>,
    /// Distribution-specific repository configurations.
    pub repositories: Option<RepositoryConfig>,
}

/// A container for distribution-specific repository settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepositoryConfig {
    /// APT configuration for Debian/Ubuntu.
    pub apt: Option<AptRepositoryConfig>,
    /// YUM/DNF configuration for RedHat/CentOS/Fedora.
    pub yum: Option<YumRepositoryConfig>,
    /// APK configuration for Alpine Linux.
    pub apk: Option<ApkRepositoryConfig>,
    /// IPS configuration for illumos/Solaris.
    pub ips: Option<IpsRepositoryConfig>,
    /// PKG configuration for FreeBSD.
    pub pkg: Option<PkgRepositoryConfig>,
}

/// Configuration for APT (Debian/Ubuntu).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AptRepositoryConfig {
    /// The URL of the HTTP/HTTPS proxy to use for APT.
    pub proxy: Option<String>,
    /// A list of PPA identifiers to add (e.g., "ppa:deadsnakes/ppa").
    pub ppas: Vec<String>,
    /// A list of custom APT source repositories.
    pub sources: Vec<AptSource>,
    /// APT preferences/pinning configuration.
    pub preferences: HashMap<String, i32>,
}

/// Defines a single custom APT source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AptSource {
    /// Used for the filename in /etc/apt/sources.list.d/
    pub name: String,
    /// Repository URI.
    pub uri: String,
    /// Distribution suites.
    pub suites: Vec<String>,
    /// Repository components.
    pub components: Vec<String>,
    /// GPG key ID from a keyserver.
    pub key_id: Option<String>,
    /// Keyserver to fetch the key from.
    pub key_server: Option<String>,
    /// Direct GPG key content.
    pub key_content: Option<String>,
}

/// Configuration for YUM/DNF (RedHat/CentOS/Fedora).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YumRepositoryConfig {
    /// The URL of the HTTP/HTTPS proxy to use for YUM.
    pub proxy: Option<String>,
    /// A list of custom YUM repositories.
    pub repositories: Vec<YumRepository>,
    /// GPG check configuration.
    pub gpgcheck: bool,
}

/// Defines a single YUM repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YumRepository {
    /// Repository ID.
    pub id: String,
    /// Repository name/description.
    pub name: String,
    /// Base URL for the repository.
    pub baseurl: String,
    /// Whether the repository is enabled.
    pub enabled: bool,
    /// GPG key URL for signature verification.
    pub gpgkey: Option<String>,
}

/// Configuration for APK (Alpine Linux).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApkRepositoryConfig {
    /// The URL of the HTTP/HTTPS proxy to use for APK.
    pub proxy: Option<String>,
    /// A list of custom APK repositories.
    pub repositories: Vec<String>,
    /// APK cache directory.
    pub cache_dir: Option<String>,
}

/// Configuration for IPS (Image Packaging System - illumos/Solaris).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpsRepositoryConfig {
    /// The URL of the HTTP/HTTPS proxy to use for IPS.
    pub proxy: Option<String>,
    /// A list of IPS publishers to configure.
    pub publishers: Vec<IpsPublisher>,
    /// Whether to enable signature verification.
    pub signature_verification: bool,
}

/// Defines a single IPS publisher.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpsPublisher {
    /// Publisher name.
    pub name: String,
    /// Publisher origin URI.
    pub origin: String,
    /// Whether the publisher is enabled.
    pub enabled: bool,
    /// Whether the publisher is the preferred publisher.
    pub preferred: bool,
    /// SSL certificate path for HTTPS repositories.
    pub ssl_cert: Option<String>,
    /// SSL key path for HTTPS repositories.
    pub ssl_key: Option<String>,
}

/// Configuration for PKG (FreeBSD).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PkgRepositoryConfig {
    /// The URL of the HTTP/HTTPS proxy to use for PKG.
    pub proxy: Option<String>,
    /// A list of custom PKG repositories.
    pub repositories: Vec<PkgRepository>,
    /// Package signature verification mode.
    pub signature_type: PkgSignatureType,
}

/// Defines a single PKG repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PkgRepository {
    /// Repository name/identifier.
    pub name: String,
    /// Repository URL.
    pub url: String,
    /// Whether the repository is enabled.
    pub enabled: bool,
    /// Repository priority.
    pub priority: Option<i32>,
    /// Signature verification settings for this repository.
    pub signature_type: Option<PkgSignatureType>,
}

/// PKG signature verification types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PkgSignatureType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "fingerprints")]
    Fingerprints,
    #[serde(rename = "pubkey")]
    Pubkey,
}

/// Defines a single user account and all its properties.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserConfig {
    /// The username of the account (e.g., "admin").
    pub name: String,
    /// The user's full name or description (maps to GECOS field).
    pub description: Option<String>,
    /// The user's default login shell (e.g., "/bin/bash").
    pub shell: Option<String>,
    /// A list of supplementary groups to which the user belongs.
    pub groups: Vec<String>,
    /// The user's primary group. If not set, a group with the same name as the user is typically created.
    pub primary_group: Option<String>,
    /// Specifies whether this is a system account (often with no home directory).
    pub system_user: bool,
    /// Overrides the default home directory path.
    pub home_directory: Option<String>,
    /// The user's UID. If not specified, the system will assign one.
    pub uid: Option<u32>,
    /// Whether to create the home directory if it doesn't exist.
    pub create_home: bool,
    /// Defines the user's sudo privileges.
    pub sudo: Option<SudoConfig>,
    /// Contains all authentication-related settings for the user.
    pub authentication: AuthenticationConfig,
}

/// An enum representing the user's sudo privilege level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SudoConfig {
    /// The user is explicitly denied sudo access.
    #[serde(rename = "deny")]
    Deny,
    /// The user is granted passwordless, unrestricted sudo access.
    #[serde(rename = "unrestricted")]
    Unrestricted,
    /// The user is granted sudo access according to a list of custom rules.
    #[serde(rename = "custom")]
    Custom(Vec<String>),
}

/// A unified structure for all user authentication methods.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthenticationConfig {
    /// Configuration for the user's password.
    pub password: Option<PasswordConfig>,
    /// A list of full SSH public key strings to add to the user's authorized_keys file.
    pub ssh_keys: Vec<String>,
    /// A list of IDs to import from services like GitHub ("gh:username") or Launchpad ("lp:username").
    pub ssh_import_ids: Vec<String>,
}

/// Defines a user's password, enforcing security best practices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PasswordConfig {
    /// The pre-computed password hash string (e.g., in SHA-512 crypt format).
    pub hash: String,
    /// If true, the user will be forced to change their password on their first login.
    pub expire_on_first_login: bool,
}

/// Configuration for running imperative scripts at various boot stages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptConfig {
    /// Scripts to run early in the boot process.
    pub early_scripts: Vec<Script>,
    /// Scripts to run during the main configuration phase.
    pub main_scripts: Vec<Script>,
    /// Scripts to run late in the boot process.
    pub late_scripts: Vec<Script>,
    /// Scripts to run on every boot.
    pub always_scripts: Vec<Script>,
}

/// Defines a script to be executed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Script {
    /// Unique identifier for the script.
    pub id: String,
    /// The script content.
    pub content: String,
    /// The interpreter to use (e.g., "/bin/bash", "/bin/sh").
    pub interpreter: Option<String>,
    /// Working directory for script execution.
    pub working_directory: Option<String>,
    /// Environment variables to set for the script.
    pub environment: HashMap<String, String>,
    /// Whether to run only once (persistent across reboots).
    pub run_once: bool,
    /// File path to capture script output.
    pub output_file: Option<String>,
    /// Timeout for script execution in seconds.
    pub timeout: Option<u64>,
}

/// Configuration for bootstrapping third-party tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntegrationConfig {
    /// Ansible integration configuration.
    pub ansible: Option<AnsibleConfig>,
    /// Puppet integration configuration.
    pub puppet: Option<PuppetConfig>,
    /// Chef integration configuration.
    pub chef: Option<ChefConfig>,
}

/// Configuration for Ansible integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnsibleConfig {
    /// Git repository URL containing Ansible playbooks.
    pub repository_url: String,
    /// Branch or tag to checkout.
    pub revision: Option<String>,
    /// Path to the playbook within the repository.
    pub playbook_path: String,
    /// Ansible vault password for encrypted content.
    pub vault_password: Option<String>,
    /// Extra variables to pass to ansible-playbook.
    pub extra_vars: HashMap<String, String>,
}

/// Configuration for Puppet integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PuppetConfig {
    /// Puppet server hostname or IP.
    pub server: String,
    /// Puppet environment to use.
    pub environment: Option<String>,
    /// Certificate name for this agent.
    pub certname: Option<String>,
    /// Whether to run Puppet agent as a daemon.
    pub daemon: bool,
}

/// Configuration for Chef integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChefConfig {
    /// Chef server URL.
    pub server_url: String,
    /// Node name for this client.
    pub node_name: String,
    /// Validation client name.
    pub validation_client_name: String,
    /// Validation key content.
    pub validation_key: String,
    /// Run list for this node.
    pub run_list: Vec<String>,
}

/// Defines the final power state of the machine after initialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerStateConfig {
    /// The desired power state.
    pub mode: PowerStateMode,
    /// Delay before executing the power state change (in seconds).
    pub delay: Option<u64>,
    /// Custom message to display before power state change.
    pub message: Option<String>,
}

/// Configuration for container management across platforms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerConfig {
    /// Solaris/illumos zones.
    pub zones: Vec<ZoneConfig>,
    /// FreeBSD jails.
    pub jails: Vec<JailConfig>,
    /// Linux containers.
    pub containers: Vec<LinuxContainerConfig>,
}

/// Solaris/illumos zone configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneConfig {
    /// Zone name.
    pub name: String,
    /// Zone brand (sparse, whole-root, etc.).
    pub brand: String,
    /// Zone state (configured, installed, running).
    pub state: ZoneState,
    /// Zone root path.
    pub zonepath: String,
    /// Network configuration.
    pub networks: Vec<ZoneNetworkConfig>,
    /// Resource controls.
    pub resources: Option<ZoneResourceConfig>,
    /// Zone-specific properties.
    pub properties: HashMap<String, String>,
    /// Nested sysconfig configuration for the zone.
    pub sysconfig: Option<Box<UnifiedConfig>>,
}

/// Zone states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZoneState {
    #[serde(rename = "configured")]
    Configured,
    #[serde(rename = "installed")]
    Installed,
    #[serde(rename = "running")]
    Running,
}

/// Zone network configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneNetworkConfig {
    /// Network interface name in the zone.
    pub interface: String,
    /// Physical interface or VNIC to use.
    pub physical: String,
    /// IP address configuration.
    pub address: Option<String>,
    /// Default router.
    pub defrouter: Option<String>,
}

/// Zone resource controls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneResourceConfig {
    /// CPU cap (percentage).
    pub cpu_cap: Option<f64>,
    /// CPU shares.
    pub cpu_shares: Option<u32>,
    /// Physical memory cap.
    pub physical_memory_cap: Option<String>,
    /// Swap memory cap.
    pub swap_memory_cap: Option<String>,
}

/// FreeBSD jail configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JailConfig {
    /// Jail name.
    pub name: String,
    /// Jail ID (optional, auto-assigned if not specified).
    pub jid: Option<u32>,
    /// Jail root path.
    pub path: String,
    /// Hostname inside the jail.
    pub hostname: String,
    /// IP addresses assigned to the jail.
    pub ip_addresses: Vec<String>,
    /// Network interfaces.
    pub interfaces: Vec<String>,
    /// Jail parameters.
    pub parameters: HashMap<String, String>,
    /// Whether to start the jail automatically.
    pub auto_start: bool,
    /// Nested sysconfig configuration for the jail.
    pub sysconfig: Option<Box<UnifiedConfig>>,
}

/// Linux container configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinuxContainerConfig {
    /// Container name.
    pub name: String,
    /// Container image.
    pub image: String,
    /// Container runtime (docker, podman, containerd).
    pub runtime: ContainerRuntime,
    /// Container state (created, running, stopped).
    pub state: ContainerState,
    /// Environment variables.
    pub environment: HashMap<String, String>,
    /// Volume mounts.
    pub volumes: Vec<ContainerVolumeConfig>,
    /// Port mappings.
    pub ports: Vec<ContainerPortConfig>,
    /// Network configuration.
    pub networks: Vec<String>,
    /// Resource limits.
    pub resources: Option<ContainerResourceConfig>,
    /// Nested sysconfig configuration for the container.
    pub sysconfig: Option<Box<UnifiedConfig>>,
}

/// Container runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerRuntime {
    #[serde(rename = "docker")]
    Docker,
    #[serde(rename = "podman")]
    Podman,
    #[serde(rename = "containerd")]
    Containerd,
}

/// Container states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerState {
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopped")]
    Stopped,
}

/// Container volume configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerVolumeConfig {
    /// Host path or volume name.
    pub source: String,
    /// Container path.
    pub target: String,
    /// Mount type (bind, volume, tmpfs).
    pub mount_type: ContainerMountType,
    /// Mount options.
    pub options: Vec<String>,
}

/// Container mount types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerMountType {
    #[serde(rename = "bind")]
    Bind,
    #[serde(rename = "volume")]
    Volume,
    #[serde(rename = "tmpfs")]
    Tmpfs,
}

/// Container port configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerPortConfig {
    /// Host port.
    pub host_port: u16,
    /// Container port.
    pub container_port: u16,
    /// Protocol (tcp, udp).
    pub protocol: ContainerProtocol,
    /// Host IP to bind to.
    pub host_ip: Option<String>,
}

/// Container protocols.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerProtocol {
    #[serde(rename = "tcp")]
    Tcp,
    #[serde(rename = "udp")]
    Udp,
}

/// Container resource configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerResourceConfig {
    /// CPU limit (cores).
    pub cpu_limit: Option<f64>,
    /// Memory limit.
    pub memory_limit: Option<String>,
    /// Memory swap limit.
    pub memory_swap_limit: Option<String>,
}

/// Available power states after initialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PowerStateMode {
    /// No action (default).
    #[serde(rename = "noop")]
    Noop,
    /// Halt the system.
    #[serde(rename = "halt")]
    Halt,
    /// Power off the system.
    #[serde(rename = "poweroff")]
    Poweroff,
    /// Reboot the system.
    #[serde(rename = "reboot")]
    Reboot,
}

/// Error types for configuration validation and processing.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl UnifiedConfig {
    /// Create a new empty unified configuration.
    pub fn new() -> Self {
        Self {
            system: None,
            storage: None,
            networking: None,
            software: None,
            users: Vec::new(),
            scripts: None,
            integrations: None,
            containers: None,
            power_state: None,
        }
    }

    /// Validate the configuration for consistency and completeness.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate user names are unique
        let mut user_names = std::collections::HashSet::new();
        for user in &self.users {
            if !user_names.insert(&user.name) {
                return Err(ConfigError::ValidationError(format!(
                    "Duplicate user name: {}",
                    user.name
                )));
            }
        }

        // Validate network interface names are unique
        if let Some(networking) = &self.networking {
            let mut interface_names = std::collections::HashSet::new();
            for interface in &networking.interfaces {
                if !interface_names.insert(&interface.name) {
                    return Err(ConfigError::ValidationError(format!(
                        "Duplicate interface name: {}",
                        interface.name
                    )));
                }
            }
        }

        // Validate storage pool names are unique
        if let Some(storage) = &self.storage {
            let mut pool_names = std::collections::HashSet::new();
            for pool in &storage.pools {
                if !pool_names.insert(&pool.name) {
                    return Err(ConfigError::ValidationError(format!(
                        "Duplicate storage pool name: {}",
                        pool.name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String, ConfigError> {
        serde_json::to_string_pretty(self).map_err(ConfigError::from)
    }

    /// Create from JSON string.
    pub fn from_json(json: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_json::from_str(json)?;
        config.validate()?;
        Ok(config)
    }
}

impl Default for UnifiedConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            filesystems: Vec::new(),
            pools: Vec::new(),
            mounts: Vec::new(),
            zfs_datasets: Vec::new(),
            zfs_snapshots: Vec::new(),
            zfs_replication: Vec::new(),
        }
    }
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            zones: Vec::new(),
            jails: Vec::new(),
            containers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_config_creation() {
        let config = UnifiedConfig::new();
        assert_eq!(config.users.len(), 0);
        assert!(config.system.is_none());
    }

    #[test]
    fn test_user_config_validation() {
        let mut config = UnifiedConfig::new();

        // Add two users with the same name
        config.users.push(UserConfig {
            name: "admin".to_string(),
            description: None,
            shell: None,
            groups: vec![],
            primary_group: None,
            system_user: false,
            home_directory: None,
            uid: None,
            create_home: true,
            sudo: None,
            authentication: AuthenticationConfig {
                password: None,
                ssh_keys: vec![],
                ssh_import_ids: vec![],
            },
        });

        config.users.push(UserConfig {
            name: "admin".to_string(),
            description: None,
            shell: None,
            groups: vec![],
            primary_group: None,
            system_user: false,
            home_directory: None,
            uid: None,
            create_home: true,
            sudo: None,
            authentication: AuthenticationConfig {
                password: None,
                ssh_keys: vec![],
                ssh_import_ids: vec![],
            },
        });

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_json_serialization() {
        let mut config = UnifiedConfig::new();
        config.system = Some(SystemConfig {
            hostname: Some("test-host".to_string()),
            fqdn: Some("test-host.example.com".to_string()),
            timezone: Some("UTC".to_string()),
            locale: Some("en_US.UTF-8".to_string()),
            environment: HashMap::new(),
        });

        let json = config.to_json().unwrap();
        let deserialized = UnifiedConfig::from_json(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
