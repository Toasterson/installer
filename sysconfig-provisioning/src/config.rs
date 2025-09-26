use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main provisioning configuration that aggregates all settings
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvisioningConfig {
    /// System hostname
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    /// DNS nameservers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nameservers: Vec<String>,

    /// DNS search domains
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub search_domains: Vec<String>,

    /// Network interface configurations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub interfaces: HashMap<String, InterfaceConfig>,

    /// SSH authorized keys (typically for root user)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ssh_authorized_keys: Vec<String>,

    /// User accounts to create/configure
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<UserConfig>,

    /// User data script (cloud-init style)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<String>,

    /// User data script (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data_base64: Option<String>,

    /// Arbitrary metadata key-value pairs
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Static routes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<RouteConfig>,

    /// NTP servers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ntp_servers: Vec<String>,

    /// Timezone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterfaceConfig {
    /// MAC address (for interface matching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,

    /// MTU size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u32>,

    /// List of addresses for this interface
    #[serde(default)]
    pub addresses: Vec<AddressConfig>,

    /// Whether this interface should be enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// VLAN ID (if this is a VLAN interface)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_id: Option<u16>,

    /// Parent interface (for VLANs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// Address configuration for an interface
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddressConfig {
    /// Address type: dhcp4, dhcp6, static, slaac, addrconf
    #[serde(rename = "type")]
    pub addr_type: AddressType,

    /// IP address with CIDR (for static addresses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    /// Gateway (for static addresses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,

    /// Whether this is the primary address
    #[serde(default)]
    pub primary: bool,
}

/// Address type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AddressType {
    #[serde(rename = "dhcp4")]
    Dhcp4,
    #[serde(rename = "dhcp6")]
    Dhcp6,
    #[serde(rename = "dhcp")]
    Dhcp,
    Static,
    #[serde(rename = "slaac")]
    Slaac,
    #[serde(rename = "addrconf")]
    Addrconf,
}

impl Default for AddressType {
    fn default() -> Self {
        AddressType::Dhcp4
    }
}

/// User configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserConfig {
    /// Username
    pub name: String,

    /// User's full name (GECOS field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gecos: Option<String>,

    /// User ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,

    /// Primary group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,

    /// Additional groups
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,

    /// Home directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,

    /// Shell
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,

    /// SSH authorized keys for this user
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ssh_authorized_keys: Vec<String>,

    /// Whether to create the home directory
    #[serde(default = "default_true")]
    pub create_home: bool,

    /// Password hash (not plaintext)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,

    /// Whether the user should have sudo privileges
    #[serde(default)]
    pub sudo: bool,
}

/// Static route configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteConfig {
    /// Destination network (CIDR)
    pub destination: String,

    /// Gateway/nexthop
    pub gateway: String,

    /// Interface to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,

    /// Metric/priority
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<u32>,
}

/// Cloud vendor detection result
#[derive(Debug, Clone, PartialEq)]
pub enum CloudVendor {
    EC2,
    Azure,
    GCP,
    DigitalOcean,
    Oracle,
    OpenStack,
    SmartOS,
    VMware,
    Unknown,
}

impl std::fmt::Display for CloudVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudVendor::EC2 => write!(f, "Amazon EC2"),
            CloudVendor::Azure => write!(f, "Microsoft Azure"),
            CloudVendor::GCP => write!(f, "Google Cloud Platform"),
            CloudVendor::DigitalOcean => write!(f, "DigitalOcean"),
            CloudVendor::Oracle => write!(f, "Oracle Cloud"),
            CloudVendor::OpenStack => write!(f, "OpenStack"),
            CloudVendor::SmartOS => write!(f, "SmartOS"),
            CloudVendor::VMware => write!(f, "VMware"),
            CloudVendor::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Cloud-init network configuration version 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfigV1 {
    pub version: u8,
    pub config: Vec<NetworkConfigV1Item>,
}

/// Cloud-init network config v1 item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NetworkConfigV1Item {
    #[serde(rename = "physical")]
    Physical {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mac_address: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mtu: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subnets: Option<Vec<SubnetConfig>>,
    },
    #[serde(rename = "vlan")]
    Vlan {
        name: String,
        vlan_id: u16,
        vlan_link: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subnets: Option<Vec<SubnetConfig>>,
    },
    #[serde(rename = "bond")]
    Bond {
        name: String,
        bond_interfaces: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<HashMap<String, serde_json::Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subnets: Option<Vec<SubnetConfig>>,
    },
    #[serde(rename = "nameserver")]
    Nameserver {
        #[serde(skip_serializing_if = "Option::is_none")]
        address: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        search: Option<Vec<String>>,
    },
    #[serde(rename = "route")]
    Route {
        destination: String,
        gateway: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metric: Option<u32>,
    },
}

/// Subnet configuration for cloud-init
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubnetConfig {
    #[serde(rename = "type")]
    pub subnet_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netmask: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_nameservers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_search: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<Vec<RouteConfig>>,
}

/// KDL configuration structure (matches sysconfig format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdlConfig {
    pub hostname: Option<String>,
    pub nameservers: Vec<String>,
    pub interfaces: Vec<KdlInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdlInterface {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    pub addresses: Vec<KdlAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdlAddress {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

// Helper functions
fn default_true() -> bool {
    true
}

impl ProvisioningConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another configuration into this one
    /// The other configuration takes precedence for non-empty values
    pub fn merge(&mut self, other: ProvisioningConfig) {
        if other.hostname.is_some() {
            self.hostname = other.hostname;
        }

        for ns in other.nameservers {
            if !self.nameservers.contains(&ns) {
                self.nameservers.push(ns);
            }
        }

        for domain in other.search_domains {
            if !self.search_domains.contains(&domain) {
                self.search_domains.push(domain);
            }
        }

        for (name, iface) in other.interfaces {
            self.interfaces.insert(name, iface);
        }

        for key in other.ssh_authorized_keys {
            if !self.ssh_authorized_keys.contains(&key) {
                self.ssh_authorized_keys.push(key);
            }
        }

        for user in other.users {
            if !self.users.iter().any(|u| u.name == user.name) {
                self.users.push(user);
            }
        }

        if other.user_data.is_some() {
            self.user_data = other.user_data;
        }

        if other.user_data_base64.is_some() {
            self.user_data_base64 = other.user_data_base64;
        }

        for (key, value) in other.metadata {
            self.metadata.insert(key, value);
        }

        for route in other.routes {
            if !self
                .routes
                .iter()
                .any(|r| r.destination == route.destination && r.gateway == route.gateway)
            {
                self.routes.push(route);
            }
        }

        for ntp in other.ntp_servers {
            if !self.ntp_servers.contains(&ntp) {
                self.ntp_servers.push(ntp);
            }
        }

        if other.timezone.is_some() {
            self.timezone = other.timezone;
        }
    }
}
