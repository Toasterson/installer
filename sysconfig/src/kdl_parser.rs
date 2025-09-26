use kdl::{KdlDocument, KdlNode, KdlValue};
use serde::{Deserialize, Serialize};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KdlParseError {
    #[error("KDL parsing error: {0}")]
    Kdl(#[from] kdl::KdlError),

    #[error("Missing sysconfig node")]
    MissingSysconfig,

    #[error("Invalid configuration structure: {0}")]
    InvalidStructure(String),

    #[error("Invalid value type for {field}: expected {expected}")]
    InvalidValueType { field: String, expected: String },
}

/// Parsed system configuration from KDL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdlSysConfig {
    pub hostname: Option<String>,
    pub nameservers: Vec<String>,
    pub interfaces: Vec<KdlInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdlInterface {
    pub name: String,
    pub selector: Option<String>,
    pub addresses: Vec<KdlAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdlAddress {
    pub name: String,
    pub kind: String,
    pub address: Option<String>,
}

impl KdlSysConfig {
    /// Parse a KDL document and extract the sysconfig section
    pub fn from_kdl_str(content: &str) -> Result<Self, KdlParseError> {
        let doc: KdlDocument = content.parse()?;
        Self::from_kdl_document(&doc)
    }

    /// Parse from a KDL document
    pub fn from_kdl_document(doc: &KdlDocument) -> Result<Self, KdlParseError> {
        // Find the sysconfig node
        let sysconfig_node = doc
            .nodes()
            .iter()
            .find(|node| node.name().value() == "sysconfig")
            .ok_or(KdlParseError::MissingSysconfig)?;

        Self::parse_sysconfig_node(sysconfig_node)
    }

    /// Parse a sysconfig KDL node
    fn parse_sysconfig_node(node: &KdlNode) -> Result<Self, KdlParseError> {
        let mut config = KdlSysConfig {
            hostname: None,
            nameservers: Vec::new(),
            interfaces: Vec::new(),
        };

        // Process children nodes
        if let Some(children) = node.children() {
            for child in children.nodes() {
                match child.name().value() {
                    "hostname" => {
                        config.hostname = Some(Self::extract_string_arg(child, "hostname")?);
                    }
                    "nameserver" => {
                        config
                            .nameservers
                            .push(Self::extract_string_arg(child, "nameserver")?);
                    }
                    "interface" => {
                        config.interfaces.push(Self::parse_interface_node(child)?);
                    }
                    _ => {
                        // Ignore unknown nodes for forward compatibility
                    }
                }
            }
        }

        Ok(config)
    }

    /// Parse an interface KDL node
    fn parse_interface_node(node: &KdlNode) -> Result<KdlInterface, KdlParseError> {
        // Get the interface name from the first argument
        let name = Self::extract_string_arg(node, "interface")?;

        // Get the selector property if present
        let selector = node
            .entries()
            .iter()
            .find(|e| e.name().as_ref().map(|n| n.value()) == Some("selector"))
            .and_then(|e| Self::value_to_string(e.value()));

        let mut addresses = Vec::new();

        // Parse address children
        if let Some(children) = node.children() {
            for child in children.nodes() {
                if child.name().value() == "address" {
                    addresses.push(Self::parse_address_node(child)?);
                }
            }
        }

        Ok(KdlInterface {
            name,
            selector,
            addresses,
        })
    }

    /// Parse an address KDL node
    fn parse_address_node(node: &KdlNode) -> Result<KdlAddress, KdlParseError> {
        // Get properties
        let mut name = String::new();
        let mut kind = String::new();

        for entry in node.entries() {
            if let Some(entry_name) = entry.name() {
                match entry_name.value() {
                    "name" => {
                        name = Self::value_to_string(entry.value()).ok_or_else(|| {
                            KdlParseError::InvalidValueType {
                                field: "address.name".to_string(),
                                expected: "string".to_string(),
                            }
                        })?;
                    }
                    "kind" => {
                        kind = Self::value_to_string(entry.value()).ok_or_else(|| {
                            KdlParseError::InvalidValueType {
                                field: "address.kind".to_string(),
                                expected: "string".to_string(),
                            }
                        })?;
                    }
                    _ => {}
                }
            }
        }

        // Get the address from the first unnamed argument (if present)
        let address = node
            .entries()
            .iter()
            .find(|e| e.name().is_none())
            .and_then(|e| Self::value_to_string(e.value()));

        Ok(KdlAddress {
            name,
            kind,
            address,
        })
    }

    /// Extract a string argument from a node
    fn extract_string_arg(node: &KdlNode, field_name: &str) -> Result<String, KdlParseError> {
        node.entries()
            .first()
            .and_then(|e| {
                if e.name().is_none() {
                    Self::value_to_string(e.value())
                } else {
                    None
                }
            })
            .ok_or_else(|| KdlParseError::InvalidValueType {
                field: field_name.to_string(),
                expected: "string argument".to_string(),
            })
    }

    /// Convert a KdlValue to a String
    fn value_to_string(value: &KdlValue) -> Option<String> {
        match value {
            KdlValue::String(s) => Some(s.clone()),
            KdlValue::RawString(s) => Some(s.clone()),
            KdlValue::Base10(n) => Some(n.to_string()),
            KdlValue::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// Convert to the existing SysConfig format used by the config module
    pub fn to_sysconfig(&self) -> crate::config::SysConfig {
        let mut config = crate::config::SysConfig::default();

        if let Some(hostname) = &self.hostname {
            config.hostname = hostname.clone();
        }

        config.nameservers = self.nameservers.clone();

        config.interfaces = self
            .interfaces
            .iter()
            .map(|iface| {
                let mut interface = crate::config::Interface::default();
                interface.name = Some(iface.name.clone());
                interface.selector = iface.selector.clone();

                interface.addresses = iface
                    .addresses
                    .iter()
                    .map(|addr| {
                        let mut address = crate::config::AddressObject::default();
                        address.name = addr.name.clone();
                        address.kind = match addr.kind.as_str() {
                            "dhcp4" => crate::config::AddressKind::Dhcp4,
                            "dhcp6" => crate::config::AddressKind::Dhcp6,
                            "addrconf" => crate::config::AddressKind::Addrconf,
                            "static" | _ => crate::config::AddressKind::Static,
                        };
                        address.address = addr.address.clone();
                        address
                    })
                    .collect();

                interface
            })
            .collect();

        config
    }
}

/// Parse a KDL configuration file
pub fn parse_kdl_file(path: &std::path::Path) -> Result<KdlSysConfig, KdlParseError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| KdlParseError::InvalidStructure(format!("Failed to read file: {}", e)))?;
    KdlSysConfig::from_kdl_str(&content)
}

/// Parse a KDL configuration string
pub fn parse_kdl_str(content: &str) -> Result<KdlSysConfig, KdlParseError> {
    KdlSysConfig::from_kdl_str(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_kdl() {
        let kdl = r#"
            sysconfig {
                hostname "test-host"
                nameserver "8.8.8.8"
                nameserver "8.8.4.4"

                interface "eth0" {
                    address name="v4" kind="dhcp4"
                }
            }
        "#;

        let config = KdlSysConfig::from_kdl_str(kdl).unwrap();
        assert_eq!(config.hostname, Some("test-host".to_string()));
        assert_eq!(config.nameservers, vec!["8.8.8.8", "8.8.4.4"]);
        assert_eq!(config.interfaces.len(), 1);
        assert_eq!(config.interfaces[0].name, "eth0");
        assert_eq!(config.interfaces[0].addresses.len(), 1);
        assert_eq!(config.interfaces[0].addresses[0].kind, "dhcp4");
    }

    #[test]
    fn test_parse_complex_kdl() {
        let kdl = r#"
            pool "rpool" {
                vdev "mirror" {
                    disks "c5t0d0" "c6t0d0"
                }
            }

            sysconfig {
                hostname "node01"
                nameserver "9.9.9.9"
                nameserver "149.112.112.112"

                interface "net0" selector="mac:00:00:00:00" {
                    address name="v4" kind="static" "192.168.1.200/24"
                    address name="v6" kind="static" "fe80:01::1/64"
                }

                interface "net1" selector="mac:00:00:00:01" {
                    address name="v4" kind="dhcp4"
                    address name="v6" kind="dhcp6"
                    address name="addrconf" kind="addrconf"
                }
            }
        "#;

        let config = KdlSysConfig::from_kdl_str(kdl).unwrap();
        assert_eq!(config.hostname, Some("node01".to_string()));
        assert_eq!(config.nameservers, vec!["9.9.9.9", "149.112.112.112"]);
        assert_eq!(config.interfaces.len(), 2);

        let net0 = &config.interfaces[0];
        assert_eq!(net0.name, "net0");
        assert_eq!(net0.selector, Some("mac:00:00:00:00".to_string()));
        assert_eq!(net0.addresses.len(), 2);
        assert_eq!(
            net0.addresses[0].address,
            Some("192.168.1.200/24".to_string())
        );

        let net1 = &config.interfaces[1];
        assert_eq!(net1.name, "net1");
        assert_eq!(net1.selector, Some("mac:00:00:00:01".to_string()));
        assert_eq!(net1.addresses.len(), 3);
    }

    #[test]
    fn test_missing_sysconfig() {
        let kdl = r#"
            pool "rpool" {
                vdev "mirror" {
                    disks "c5t0d0" "c6t0d0"
                }
            }
        "#;

        let result = KdlSysConfig::from_kdl_str(kdl);
        assert!(matches!(result, Err(KdlParseError::MissingSysconfig)));
    }
}
