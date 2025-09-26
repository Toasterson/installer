use std::path::Path;
use sysconfig::kdl_loader::KdlConfigLoader;
use sysconfig::kdl_parser::KdlSysConfig;

#[test]
fn test_parse_minimal_example() {
    let mut loader = KdlConfigLoader::new();
    let result = loader.load_file(Path::new("examples/minimal.kdl"));

    assert!(result.is_ok(), "Failed to load minimal.kdl: {:?}", result);

    let config = loader.get_config().expect("Config should be loaded");
    assert_eq!(config.hostname, Some("minimal-host".to_string()));
    assert_eq!(config.nameservers, vec!["9.9.9.9"]);
    assert_eq!(config.interfaces.len(), 1);
    assert_eq!(config.interfaces[0].name, "eth0");
    assert_eq!(config.interfaces[0].addresses.len(), 1);
    assert_eq!(config.interfaces[0].addresses[0].kind, "dhcp4");

    // Validate the configuration
    assert!(loader.validate().is_ok());
}

#[test]
fn test_parse_config_example() {
    let mut loader = KdlConfigLoader::new();
    let result = loader.load_file(Path::new("examples/config.kdl"));

    assert!(result.is_ok(), "Failed to load config.kdl: {:?}", result);

    let config = loader.get_config().expect("Config should be loaded");
    assert_eq!(config.hostname, Some("example-host".to_string()));
    assert_eq!(config.nameservers.len(), 3);
    assert!(config.nameservers.contains(&"8.8.8.8".to_string()));
    assert!(config.nameservers.contains(&"1.1.1.1".to_string()));

    // Check interfaces
    assert_eq!(config.interfaces.len(), 4);

    // Check eth0
    let eth0 = config
        .interfaces
        .iter()
        .find(|i| i.name == "eth0")
        .expect("eth0 should exist");
    assert_eq!(eth0.addresses.len(), 2);

    // Check eth1 with selector
    let eth1 = config
        .interfaces
        .iter()
        .find(|i| i.name == "eth1")
        .expect("eth1 should exist");
    assert_eq!(eth1.selector, Some("mac:00:11:22:33:44:55".to_string()));
    assert_eq!(eth1.addresses.len(), 2);

    // Check static addresses
    let static_addr = eth1
        .addresses
        .iter()
        .find(|a| a.kind == "static" && a.name == "v4")
        .expect("Static v4 address should exist");
    assert_eq!(static_addr.address, Some("192.168.1.100/24".to_string()));

    // Validate the configuration
    assert!(loader.validate().is_ok());
}

#[test]
fn test_parse_full_system_example() {
    let mut loader = KdlConfigLoader::new();
    let result = loader.load_file(Path::new("examples/full-system.kdl"));

    assert!(
        result.is_ok(),
        "Failed to load full-system.kdl: {:?}",
        result
    );

    let config = loader.get_config().expect("Config should be loaded");
    assert_eq!(config.hostname, Some("production-server-01".to_string()));

    // Check nameservers including IPv6
    assert_eq!(config.nameservers.len(), 6);
    assert!(config.nameservers.contains(&"9.9.9.9".to_string()));
    assert!(config
        .nameservers
        .contains(&"2606:4700:4700::1111".to_string()));

    // Check interfaces
    assert_eq!(config.interfaces.len(), 6);

    // Check net0 with multiple addresses
    let net0 = config
        .interfaces
        .iter()
        .find(|i| i.name == "net0")
        .expect("net0 should exist");
    assert_eq!(net0.selector, Some("mac:00:0c:29:3e:4f:50".to_string()));
    assert_eq!(net0.addresses.len(), 4);

    // Check various address types
    let v4_primary = net0
        .addresses
        .iter()
        .find(|a| a.name == "v4-primary")
        .expect("v4-primary should exist");
    assert_eq!(v4_primary.kind, "static");
    assert_eq!(v4_primary.address, Some("192.168.1.200/24".to_string()));

    let v6_primary = net0
        .addresses
        .iter()
        .find(|a| a.name == "v6-primary")
        .expect("v6-primary should exist");
    assert_eq!(v6_primary.kind, "static");
    assert_eq!(v6_primary.address, Some("2001:db8:1::200/64".to_string()));

    // Check net4 with DHCP
    let net4 = config
        .interfaces
        .iter()
        .find(|i| i.name == "net4")
        .expect("net4 should exist");
    assert_eq!(net4.addresses.len(), 3);

    let dhcp4 = net4
        .addresses
        .iter()
        .find(|a| a.kind == "dhcp4")
        .expect("dhcp4 address should exist");
    assert_eq!(dhcp4.address, None); // DHCP doesn't need static address

    // Validate the configuration
    assert!(loader.validate().is_ok());
}

#[test]
fn test_to_system_state_conversion() {
    let mut loader = KdlConfigLoader::new();
    loader
        .load_file(Path::new("examples/minimal.kdl"))
        .expect("Should load minimal.kdl");

    let state = loader
        .to_system_state()
        .expect("Should convert to system state");

    // Check hostname in state
    let hostname = state.get("hostname").expect("Hostname should be in state");
    assert_eq!(hostname, serde_json::json!("minimal-host"));

    // Check nameservers in state
    let nameservers = state
        .get("nameservers")
        .expect("Nameservers should be in state");
    assert!(nameservers.is_array());
    assert_eq!(nameservers[0], "9.9.9.9");

    // Check interfaces in state
    let interfaces = state
        .get("interfaces")
        .expect("Interfaces should be in state");
    assert!(interfaces.is_array());
    assert_eq!(interfaces.as_array().unwrap().len(), 1);
}

#[test]
fn test_summary_generation() {
    let mut loader = KdlConfigLoader::new();
    loader
        .load_file(Path::new("examples/config.kdl"))
        .expect("Should load config.kdl");

    let summary = loader.summary();
    assert!(summary.contains("Hostname: example-host"));
    assert!(summary.contains("Nameservers:"));
    assert!(summary.contains("8.8.8.8"));
    assert!(summary.contains("Interfaces:"));
    assert!(summary.contains("eth0"));
    assert!(summary.contains("eth1"));
}

#[test]
fn test_validation_with_complex_config() {
    let mut loader = KdlConfigLoader::new();
    loader
        .load_file(Path::new("examples/full-system.kdl"))
        .expect("Should load full-system.kdl");

    // Should pass validation
    let validation_result = loader.validate();
    assert!(
        validation_result.is_ok(),
        "Validation failed: {:?}",
        validation_result
    );
}

#[test]
fn test_kdl_string_parsing() {
    let kdl = r#"
        sysconfig {
            hostname "test-host"
            nameserver "1.1.1.1"
            interface "test0" {
                address name="test" kind="dhcp4"
            }
        }
    "#;

    let mut loader = KdlConfigLoader::new();
    loader.load_string(kdl).expect("Should parse KDL string");

    let config = loader.get_config().expect("Config should be loaded");
    assert_eq!(config.hostname, Some("test-host".to_string()));
    assert_eq!(config.nameservers, vec!["1.1.1.1"]);
}

#[test]
fn test_interface_with_selector() {
    let kdl = r#"
        sysconfig {
            hostname "selector-test"
            interface "net0" selector="mac:aa:bb:cc:dd:ee:ff" {
                address name="v4" kind="static" "10.0.0.1/24"
            }
        }
    "#;

    let mut loader = KdlConfigLoader::new();
    loader.load_string(kdl).expect("Should parse KDL string");

    let config = loader.get_config().expect("Config should be loaded");
    let iface = &config.interfaces[0];
    assert_eq!(iface.name, "net0");
    assert_eq!(iface.selector, Some("mac:aa:bb:cc:dd:ee:ff".to_string()));
}

#[test]
fn test_multiple_addresses_per_interface() {
    let kdl = r#"
        sysconfig {
            hostname "multi-addr"
            interface "net0" {
                address name="v4-1" kind="static" "192.168.1.1/24"
                address name="v4-2" kind="static" "192.168.1.2/24"
                address name="v6" kind="static" "2001:db8::1/64"
                address name="dhcp" kind="dhcp4"
                address name="slaac" kind="addrconf"
            }
        }
    "#;

    let mut loader = KdlConfigLoader::new();
    loader.load_string(kdl).expect("Should parse KDL string");

    let config = loader.get_config().expect("Config should be loaded");
    assert_eq!(config.interfaces[0].addresses.len(), 5);

    // Check different address types
    let addrs = &config.interfaces[0].addresses;
    assert!(addrs
        .iter()
        .any(|a| a.kind == "static" && a.address.is_some()));
    assert!(addrs
        .iter()
        .any(|a| a.kind == "dhcp4" && a.address.is_none()));
    assert!(addrs
        .iter()
        .any(|a| a.kind == "addrconf" && a.address.is_none()));
}

#[test]
fn test_validation_errors() {
    // Test empty hostname
    let kdl = r#"
        sysconfig {
            hostname ""
        }
    "#;

    let mut loader = KdlConfigLoader::new();
    loader.load_string(kdl).expect("Should parse KDL string");
    assert!(loader.validate().is_err());

    // Test static address without value
    let kdl = r#"
        sysconfig {
            hostname "test"
            interface "net0" {
                address name="v4" kind="static"
            }
        }
    "#;

    let mut loader = KdlConfigLoader::new();
    loader.load_string(kdl).expect("Should parse KDL string");
    assert!(loader.validate().is_err());

    // Test empty interface name
    let kdl = r#"
        sysconfig {
            hostname "test"
            interface "" {
                address name="v4" kind="dhcp4"
            }
        }
    "#;

    let mut loader = KdlConfigLoader::new();
    loader.load_string(kdl).expect("Should parse KDL string");
    assert!(loader.validate().is_err());
}

#[test]
fn test_ipv6_nameservers() {
    let kdl = r#"
        sysconfig {
            hostname "ipv6-test"
            nameserver "2001:4860:4860::8888"
            nameserver "2001:4860:4860::8844"
            nameserver "2606:4700:4700::1111"
        }
    "#;

    let mut loader = KdlConfigLoader::new();
    loader.load_string(kdl).expect("Should parse KDL string");

    let config = loader.get_config().expect("Config should be loaded");
    assert_eq!(config.nameservers.len(), 3);
    assert!(config
        .nameservers
        .contains(&"2001:4860:4860::8888".to_string()));
    assert!(config
        .nameservers
        .contains(&"2606:4700:4700::1111".to_string()));
}

#[test]
fn test_to_sysconfig_conversion() {
    let kdl = r#"
        sysconfig {
            hostname "conversion-test"
            nameserver "8.8.8.8"
            interface "eth0" {
                address name="v4" kind="dhcp4"
                address name="v6" kind="dhcp6"
            }
        }
    "#;

    let config = KdlSysConfig::from_kdl_str(kdl).expect("Should parse KDL");

    let sysconfig = config.to_sysconfig();
    assert_eq!(sysconfig.hostname, "conversion-test");
    assert_eq!(sysconfig.nameservers, vec!["8.8.8.8"]);
    assert_eq!(sysconfig.interfaces.len(), 1);
    assert_eq!(sysconfig.interfaces[0].name, Some("eth0".to_string()));
}
