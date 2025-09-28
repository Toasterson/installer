//! Configuration converter for transforming cloud-init and other formats
//! to the unified provisioning schema.

use serde_json::{Value, json};
use sysconfig_config_schema::*;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Convert raw configuration data to unified schema
pub fn convert_to_unified_schema(config: Value) -> Result<UnifiedConfig, Box<dyn std::error::Error>> {
    let mut unified = UnifiedConfig::new();

    // Convert system configuration
    if let Some(system) = convert_system_config(&config)? {
        unified.system = Some(system);
    }

    // Convert user configuration
    unified.users = convert_users(&config)?;

    // Convert networking configuration
    if let Some(networking) = convert_networking(&config)? {
        unified.networking = Some(networking);
    }

    // Convert software configuration
    if let Some(software) = convert_software(&config)? {
        unified.software = Some(software);
    }

    // Convert scripts
    if let Some(scripts) = convert_scripts(&config)? {
        unified.scripts = Some(scripts);
    }

    // Convert storage configuration
    if let Some(storage) = convert_storage(&config)? {
        unified.storage = Some(storage);
    }

    // Convert power state
    if let Some(power_state) = convert_power_state(&config)? {
        unified.power_state = Some(power_state);
    }

    Ok(unified)
}

fn convert_system_config(config: &Value) -> Result<Option<SystemConfig>, Box<dyn std::error::Error>> {
    let mut system = SystemConfig {
        hostname: None,
        fqdn: None,
        timezone: None,
        locale: None,
        environment: HashMap::new(),
    };

    let mut has_config = false;

    // Extract hostname from various sources
    if let Some(hostname) = extract_string_field(config, &[
        "hostname",
        "meta_data.local-hostname",
        "gcp.hostname",
        "user_data.hostname",
        "ec2.local_hostname"
    ]) {
        system.hostname = Some(hostname);
        has_config = true;
    }

    // Extract FQDN
    if let Some(fqdn) = extract_string_field(config, &[
        "fqdn",
        "user_data.fqdn",
        "meta_data.fqdn"
    ]) {
        system.fqdn = Some(fqdn);
        has_config = true;
    }

    // Extract timezone
    if let Some(timezone) = extract_string_field(config, &[
        "timezone",
        "user_data.timezone",
        "meta_data.timezone"
    ]) {
        system.timezone = Some(timezone);
        has_config = true;
    }

    // Extract locale
    if let Some(locale) = extract_string_field(config, &[
        "locale",
        "user_data.locale",
        "meta_data.locale"
    ]) {
        system.locale = Some(locale);
        has_config = true;
    }

    if has_config {
        Ok(Some(system))
    } else {
        Ok(None)
    }
}

fn convert_users(config: &Value) -> Result<Vec<UserConfig>, Box<dyn std::error::Error>> {
    let mut users = Vec::new();

    // Handle cloud-init users format
    if let Some(user_data) = config.get("user_data") {
        if let Some(cloud_users) = user_data.get("users") {
            if let Some(user_array) = cloud_users.as_array() {
                for user in user_array {
                    if let Some(converted_user) = convert_cloud_init_user(user)? {
                        users.push(converted_user);
                    }
                }
            }
        }

        // Handle default user
        if let Some(default_user) = user_data.get("user") {
            if let Some(converted_user) = convert_cloud_init_user(default_user)? {
                users.push(converted_user);
            }
        }
    }

    // Handle SSH keys for default user (often root or default cloud user)
    if let Some(user_data) = config.get("user_data") {
        if let Some(ssh_keys) = user_data.get("ssh_authorized_keys") {
            if let Some(key_array) = ssh_keys.as_array() {
                let mut default_user = UserConfig {
                    name: "root".to_string(),
                    description: None,
                    shell: None,
                    groups: vec![],
                    primary_group: None,
                    system_user: false,
                    home_directory: None,
                    uid: None,
                    create_home: false,
                    sudo: None,
                    authentication: AuthenticationConfig {
                        password: None,
                        ssh_keys: key_array.iter().filter_map(|k| k.as_str().map(|s| s.to_string())).collect(),
                        ssh_import_ids: vec![],
                    },
                };

                // Check if we already have a root user
                if !users.iter().any(|u| u.name == "root") {
                    users.push(default_user);
                } else {
                    // Merge SSH keys into existing root user
                    if let Some(root_user) = users.iter_mut().find(|u| u.name == "root") {
                        for key in key_array.iter().filter_map(|k| k.as_str()) {
                            if !root_user.authentication.ssh_keys.contains(&key.to_string()) {
                                root_user.authentication.ssh_keys.push(key.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(users)
}

fn convert_cloud_init_user(user: &Value) -> Result<Option<UserConfig>, Box<dyn std::error::Error>> {
    // Extract user name - required field
    let name = match user.get("name").and_then(|n| n.as_str()) {
        Some(n) => n.to_string(),
        None => return Ok(None), // Skip users without names
    };

    let mut converted = UserConfig {
        name,
        description: user.get("gecos").and_then(|g| g.as_str()).map(|s| s.to_string()),
        shell: user.get("shell").and_then(|s| s.as_str()).map(|s| s.to_string()),
        groups: extract_string_array(user, "groups").unwrap_or_default(),
        primary_group: user.get("primary_group")
            .or(user.get("primary-group"))
            .and_then(|g| g.as_str())
            .map(|s| s.to_string()),
        system_user: user.get("system").and_then(|s| s.as_bool()).unwrap_or(false),
        home_directory: user.get("homedir")
            .or(user.get("home"))
            .and_then(|h| h.as_str())
            .map(|s| s.to_string()),
        uid: user.get("uid").and_then(|u| u.as_u64()).map(|u| u as u32),
        create_home: user.get("create_home")
            .or(user.get("create-home"))
            .and_then(|c| c.as_bool())
            .unwrap_or(true),
        sudo: convert_sudo_config(user.get("sudo"))?,
        authentication: AuthenticationConfig {
            password: convert_password_config(user)?,
            ssh_keys: extract_string_array(user, "ssh_authorized_keys")
                .or_else(|| extract_string_array(user, "ssh-authorized-keys"))
                .unwrap_or_default(),
            ssh_import_ids: extract_string_array(user, "ssh_import_id")
                .or_else(|| extract_string_array(user, "ssh-import-id"))
                .unwrap_or_default(),
        },
    };

    Ok(Some(converted))
}

fn convert_sudo_config(sudo_value: Option<&Value>) -> Result<Option<SudoConfig>, Box<dyn std::error::Error>> {
    match sudo_value {
        Some(Value::Bool(true)) => Ok(Some(SudoConfig::Unrestricted)),
        Some(Value::Bool(false)) => Ok(Some(SudoConfig::Deny)),
        Some(Value::String(s)) => {
            match s.as_str() {
                "ALL=(ALL) NOPASSWD:ALL" => Ok(Some(SudoConfig::Unrestricted)),
                "false" | "deny" => Ok(Some(SudoConfig::Deny)),
                _ => Ok(Some(SudoConfig::Custom(vec![s.clone()]))),
            }
        }
        Some(Value::Array(rules)) => {
            let rule_strings: Vec<String> = rules
                .iter()
                .filter_map(|r| r.as_str().map(|s| s.to_string()))
                .collect();
            if rule_strings.is_empty() {
                Ok(None)
            } else {
                Ok(Some(SudoConfig::Custom(rule_strings)))
            }
        }
        _ => Ok(None),
    }
}

fn convert_password_config(user: &Value) -> Result<Option<PasswordConfig>, Box<dyn std::error::Error>> {
    // Look for password hash in various fields
    if let Some(passwd) = user.get("passwd").and_then(|p| p.as_str()) {
        if passwd != "*" && !passwd.is_empty() {
            return Ok(Some(PasswordConfig {
                hash: passwd.to_string(),
                expire_on_first_login: user.get("expire").and_then(|e| e.as_bool()).unwrap_or(false),
            }));
        }
    }

    // Check for hashed_passwd field
    if let Some(hashed_passwd) = user.get("hashed_passwd").and_then(|p| p.as_str()) {
        if !hashed_passwd.is_empty() {
            return Ok(Some(PasswordConfig {
                hash: hashed_passwd.to_string(),
                expire_on_first_login: user.get("expire").and_then(|e| e.as_bool()).unwrap_or(false),
            }));
        }
    }

    Ok(None)
}

fn convert_networking(config: &Value) -> Result<Option<NetworkingConfig>, Box<dyn std::error::Error>> {
    let mut networking = NetworkingConfig {
        interfaces: vec![],
        nameservers: vec![],
        search_domains: vec![],
        routes: vec![],
        ntp_servers: vec![],
    };

    let mut has_config = false;

    // Handle cloud-init network config v1 and v2
    if let Some(network_config) = config.get("network_config") {
        if let Some(version) = network_config.get("version").and_then(|v| v.as_u64()) {
            match version {
                1 => {
                    networking.interfaces = convert_network_v1_interfaces(network_config)?;
                    if !networking.interfaces.is_empty() {
                        has_config = true;
                    }
                }
                2 => {
                    networking = convert_network_v2_config(network_config)?;
                    has_config = true;
                }
                _ => {
                    warn!("Unsupported network config version: {}", version);
                }
            }
        }
    }

    // Handle legacy network configuration from user_data
    if let Some(user_data) = config.get("user_data") {
        // DNS servers
        if let Some(dns) = extract_string_array(user_data, "dns_nameservers") {
            networking.nameservers = dns;
            has_config = true;
        }

        // Search domains
        if let Some(search) = extract_string_array(user_data, "dns_search") {
            networking.search_domains = search;
            has_config = true;
        }

        // NTP servers
        if let Some(ntp) = extract_string_array(user_data, "ntp_servers") {
            networking.ntp_servers = ntp;
            has_config = true;
        }
    }

    if has_config {
        Ok(Some(networking))
    } else {
        Ok(None)
    }
}

fn convert_network_v1_interfaces(network_config: &Value) -> Result<Vec<NetworkInterfaceConfig>, Box<dyn std::error::Error>> {
    let mut interfaces = Vec::new();

    if let Some(config_array) = network_config.get("config").and_then(|c| c.as_array()) {
        for item in config_array {
            if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                if item_type == "physical" {
                    if let Some(interface) = convert_v1_physical_interface(item)? {
                        interfaces.push(interface);
                    }
                }
            }
        }
    }

    Ok(interfaces)
}

fn convert_v1_physical_interface(interface: &Value) -> Result<Option<NetworkInterfaceConfig>, Box<dyn std::error::Error>> {
    let name = match interface.get("name").and_then(|n| n.as_str()) {
        Some(n) => n.to_string(),
        None => return Ok(None),
    };

    let mut net_interface = NetworkInterfaceConfig {
        name,
        mac_address: interface.get("mac_address").and_then(|m| m.as_str()).map(|s| s.to_string()),
        addresses: vec![],
        gateway: None,
        mtu: interface.get("mtu").and_then(|m| m.as_u64()).map(|m| m as u16),
        description: None,
        vlan: None,
    };

    // Convert subnets to addresses
    if let Some(subnets) = interface.get("subnets").and_then(|s| s.as_array()) {
        for (index, subnet) in subnets.iter().enumerate() {
            if let Some(subnet_type) = subnet.get("type").and_then(|t| t.as_str()) {
                let address_name = format!("addr_{}", index);

                match subnet_type {
                    "dhcp" | "dhcp4" => {
                        net_interface.addresses.push(AddressConfig {
                            name: address_name,
                            kind: AddressKind::Dhcp4,
                        });
                    }
                    "dhcp6" => {
                        net_interface.addresses.push(AddressConfig {
                            name: address_name,
                            kind: AddressKind::Dhcp6,
                        });
                    }
                    "static" => {
                        if let Some(address) = subnet.get("address").and_then(|a| a.as_str()) {
                            net_interface.addresses.push(AddressConfig {
                                name: address_name,
                                kind: AddressKind::Static(address.to_string()),
                            });
                        }

                        // Extract gateway
                        if let Some(gateway) = subnet.get("gateway").and_then(|g| g.as_str()) {
                            net_interface.gateway = Some(gateway.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(Some(net_interface))
}

fn convert_network_v2_config(network_config: &Value) -> Result<NetworkingConfig, Box<dyn std::error::Error>> {
    let mut networking = NetworkingConfig {
        interfaces: vec![],
        nameservers: vec![],
        search_domains: vec![],
        routes: vec![],
        ntp_servers: vec![],
    };

    // Handle ethernets section
    if let Some(ethernets) = network_config.get("ethernets") {
        if let Some(eth_obj) = ethernets.as_object() {
            for (name, config) in eth_obj {
                if let Some(interface) = convert_v2_ethernet_interface(name, config)? {
                    networking.interfaces.push(interface);
                }
            }
        }
    }

    // Handle nameservers
    if let Some(nameservers) = network_config.get("nameservers") {
        if let Some(addresses) = extract_string_array(nameservers, "addresses") {
            networking.nameservers = addresses;
        }
        if let Some(search) = extract_string_array(nameservers, "search") {
            networking.search_domains = search;
        }
    }

    Ok(networking)
}

fn convert_v2_ethernet_interface(name: &str, config: &Value) -> Result<Option<NetworkInterfaceConfig>, Box<dyn std::error::Error>> {
    let mut interface = NetworkInterfaceConfig {
        name: name.to_string(),
        mac_address: config.get("match")
            .and_then(|m| m.get("macaddress"))
            .and_then(|mac| mac.as_str())
            .map(|s| s.to_string()),
        addresses: vec![],
        gateway: None,
        mtu: config.get("mtu").and_then(|m| m.as_u64()).map(|m| m as u16),
        description: None,
        vlan: None,
    };

    // Handle DHCP configuration
    if let Some(dhcp4) = config.get("dhcp4").and_then(|d| d.as_bool()) {
        if dhcp4 {
            interface.addresses.push(AddressConfig {
                name: "dhcp4".to_string(),
                kind: AddressKind::Dhcp4,
            });
        }
    }

    if let Some(dhcp6) = config.get("dhcp6").and_then(|d| d.as_bool()) {
        if dhcp6 {
            interface.addresses.push(AddressConfig {
                name: "dhcp6".to_string(),
                kind: AddressKind::Dhcp6,
            });
        }
    }

    // Handle static addresses
    if let Some(addresses) = config.get("addresses").and_then(|a| a.as_array()) {
        for (index, addr) in addresses.iter().enumerate() {
            if let Some(addr_str) = addr.as_str() {
                interface.addresses.push(AddressConfig {
                    name: format!("static_{}", index),
                    kind: AddressKind::Static(addr_str.to_string()),
                });
            }
        }
    }

    // Handle gateway
    if let Some(gateway4) = config.get("gateway4").and_then(|g| g.as_str()) {
        interface.gateway = Some(gateway4.to_string());
    }

    Ok(Some(interface))
}

fn convert_software(config: &Value) -> Result<Option<SoftwareConfig>, Box<dyn std::error::Error>> {
    let mut software = SoftwareConfig {
        update_on_boot: false,
        upgrade_on_boot: false,
        packages_to_install: vec![],
        packages_to_remove: vec![],
        repositories: None,
    };

    let mut has_config = false;

    // Handle cloud-init package configuration
    if let Some(user_data) = config.get("user_data") {
        // Package lists
        if let Some(packages) = extract_string_array(user_data, "packages") {
            software.packages_to_install = packages;
            has_config = true;
        }

        // Package update/upgrade flags
        if let Some(update) = user_data.get("package_update").and_then(|u| u.as_bool()) {
            software.update_on_boot = update;
            has_config = true;
        }

        if let Some(upgrade) = user_data.get("package_upgrade").and_then(|u| u.as_bool()) {
            software.upgrade_on_boot = upgrade;
            has_config = true;
        }

        // Repository configuration
        if let Some(repos) = convert_repositories(user_data)? {
            software.repositories = Some(repos);
            has_config = true;
        }
    }

    if has_config {
        Ok(Some(software))
    } else {
        Ok(None)
    }
}

fn convert_repositories(user_data: &Value) -> Result<Option<RepositoryConfig>, Box<dyn std::error::Error>> {
    let mut repo_config = RepositoryConfig {
        apt: None,
        yum: None,
        apk: None,
        ips: None,
        pkg: None,
    };

    let mut has_repo_config = false;

    // Handle APT configuration
    if let Some(apt_sources) = user_data.get("apt") {
        let mut apt_config = AptRepositoryConfig {
            proxy: None,
            ppas: vec![],
            sources: vec![],
            preferences: HashMap::new(),
        };

        // Extract proxy
        if let Some(proxy) = apt_sources.get("proxy").and_then(|p| p.as_str()) {
            apt_config.proxy = Some(proxy.to_string());
        }

        // Extract sources
        if let Some(sources) = apt_sources.get("sources") {
            if let Some(sources_obj) = sources.as_object() {
                for (name, source) in sources_obj {
                    if let Some(apt_source) = convert_apt_source(name, source)? {
                        apt_config.sources.push(apt_source);
                    }
                }
            }
        }

        repo_config.apt = Some(apt_config);
        has_repo_config = true;
    }

    if has_repo_config {
        Ok(Some(repo_config))
    } else {
        Ok(None)
    }
}

fn convert_apt_source(name: &str, source: &Value) -> Result<Option<AptSource>, Box<dyn std::error::Error>> {
    let source_str = source.get("source").and_then(|s| s.as_str());
    if source_str.is_none() {
        return Ok(None);
    }

    let source_str = source_str.unwrap();

    // Parse the source string (e.g., "deb http://archive.ubuntu.com/ubuntu focal main")
    let parts: Vec<&str> = source_str.split_whitespace().collect();
    if parts.len() < 3 {
        return Ok(None);
    }

    let uri = parts[1].to_string();
    let suite = parts[2].to_string();
    let components = if parts.len() > 3 {
        parts[3..].iter().map(|s| s.to_string()).collect()
    } else {
        vec!["main".to_string()]
    };

    let apt_source = AptSource {
        name: name.to_string(),
        uri,
        suites: vec![suite],
        components,
        key_id: source.get("keyid").and_then(|k| k.as_str()).map(|s| s.to_string()),
        key_server: source.get("keyserver").and_then(|k| k.as_str()).map(|s| s.to_string()),
        key_content: source.get("key").and_then(|k| k.as_str()).map(|s| s.to_string()),
    };

    Ok(Some(apt_source))
}

fn convert_scripts(config: &Value) -> Result<Option<ScriptConfig>, Box<dyn std::error::Error>> {
    let mut script_config = ScriptConfig {
        early_scripts: vec![],
        main_scripts: vec![],
        late_scripts: vec![],
        always_scripts: vec![],
    };

    let mut has_scripts = false;

    if let Some(user_data) = config.get("user_data") {
        // Handle runcmd (main scripts)
        if let Some(runcmd) = user_data.get("runcmd").and_then(|r| r.as_array()) {
            for (index, cmd) in runcmd.iter().enumerate() {
                let content = match cmd {
                    Value::String(s) => s.clone(),
                    Value::Array(arr) => {
                        // Command array - join with spaces
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                    _ => continue,
                };

                let script = Script {
                    id: format!("runcmd_{}", index),
                    content,
                    interpreter: Some("/bin/bash".to_string()),
                    working_directory: None,
                    environment: HashMap::new(),
                    run_once: true,
                    output_file: None,
                    timeout: None,
                };

                script_config.main_scripts.push(script);
                has_scripts = true;
            }
        }

        // Handle bootcmd (early scripts)
        if let Some(bootcmd) = user_data.get("bootcmd").and_then(|b| b.as_array()) {
            for (index, cmd) in bootcmd.iter().enumerate() {
                if let Some(content) = cmd.as_str() {
                    let script = Script {
                        id: format!("bootcmd_{}", index),
                        content: content.to_string(),
                        interpreter: Some("/bin/bash".to_string()),
                        working_directory: None,
                        environment: HashMap::new(),
                        run_once: true,
                        output_file: None,
                        timeout: None,
                    };

                    script_config.early_scripts.push(script);
                    has_scripts = true;
                }
            }
        }
    }

    // Handle raw script content from various sources
    for script_field in &["user_data_raw", "startup_script", "custom_data"] {
        if let Some(script_content) = config.get(script_field).and_then(|s| s.as_str()) {
            if script_content.starts_with("#!/") {
                let script = Script {
                    id: format!("{}_script", script_field),
                    content: script_content.to_string(),
                    interpreter: None, // Use shebang
                    working_directory: None,
                    environment: HashMap::new(),
                    run_once: true,
                    output_file: Some(format!("/var/log/{}.log", script_field)),
                    timeout: Some(600), // 10 minutes timeout
                };

                script_config.main_scripts.push(script);
                has_scripts = true;
            }
        }
    }

    if has_scripts {
        Ok(Some(script_config))
    } else {
        Ok(None)
    }
}

fn convert_storage(_config: &Value) -> Result<Option<StorageConfig>, Box<dyn std::error::Error>> {
    // TODO: Implement storage configuration conversion
    // This would handle disk_setup, mounts, etc. from cloud-init
    Ok(None)
}

fn convert_power_state(config: &Value) -> Result<Option<PowerStateConfig>, Box<dyn std::error::Error>> {
    if let Some(user_data) = config.get("user_data") {
        if let Some(power_state) = user_data.get("power_state") {
            let mode = match power_state.get("mode").and_then(|m| m.as_str()) {
                Some("halt") => PowerStateMode::Halt,
                Some("poweroff") => PowerStateMode::Poweroff,
                Some("reboot") => PowerStateMode::Reboot,
                _ => PowerStateMode::Noop,
            };

            let delay = power_state.get("delay")
                .and_then(|d| d.as_u64());

            let message = power_state.get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            return Ok(Some(PowerStateConfig {
                mode,
                delay,
                message,
            }));
        }
    }

    Ok(None)
}

// Helper functions

fn extract_string_field(config: &Value, paths: &[&str]) -> Option<String> {
    for path in paths {
        let pointer_path = format!("/{}", path.replace('.', "/"));
        if let Some(value) = config.pointer(&pointer_path) {
            if let Some(s) = value.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn extract_string_array(value: &Value, key: &str) -> Option<Vec<String>> {
    value.get(key)?.as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>()
        .into()
}
