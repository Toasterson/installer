use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use sysconfig_plugins::tasks::files::Files;
use sysconfig_plugins::tasks::network_settings::NetworkSettings;
use sysconfig_plugins::tasks::packages::Packages;
use sysconfig_plugins::tasks::users::Users;
use sysconfig_plugins::{TaskChange, TaskChangeType, TaskHandler};
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

// Local proto module generated from ../sysconfig/proto/sysconfig.proto
mod proto {
    tonic::include_proto!("sysconfig");
}

use proto::plugin_service_server::{PluginService, PluginServiceServer};
use proto::sys_config_service_client::SysConfigServiceClient;
use sysconfig_config_schema::UnifiedConfig;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Sysconfig base plugin: FreeBSD", long_about = None)]
struct Args {
    /// Path to the Unix socket this plugin listens on
    #[clap(long)]
    socket: Option<String>,

    /// Path to the sysconfig service Unix socket to register with
    #[clap(long)]
    service_socket: Option<String>,

    /// Do not register with the sysconfig service automatically
    #[clap(long)]
    no_register: bool,
}

fn default_sysconfig_socket_path() -> String {
    "/var/run/sysconfig.sock".to_string()
}

fn default_plugin_socket_path() -> String {
    "/var/run/sysconfig-freebsd-base.sock".to_string()
}

fn is_running_as_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn should_auto_enable_dry_run() -> bool {
    !is_running_as_root()
}

#[derive(Default)]
struct FreeBsdBasePlugin {
    inner: Arc<RwLock<PluginState>>,
}

#[derive(Default)]
struct PluginState {
    plugin_id: Option<String>,
    service_socket_path: Option<String>,
    auto_dry_run: bool,
}

#[tonic::async_trait]
impl PluginService for FreeBsdBasePlugin {
    async fn initialize(
        &self,
        request: Request<proto::InitializeRequest>,
    ) -> Result<Response<proto::InitializeResponse>, Status> {
        let req = request.into_inner();
        let auto_dry_run = should_auto_enable_dry_run();
        {
            let mut st = self.inner.write().await;
            st.plugin_id = Some(req.plugin_id.clone());
            st.service_socket_path = Some(req.service_socket_path.clone());
            st.auto_dry_run = auto_dry_run;
        }
        info!(plugin_id = %req.plugin_id, service = %req.service_socket_path, "FreeBSD base plugin initialized");
        Ok(Response::new(proto::InitializeResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn get_config(
        &self,
        _request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::GetConfigResponse>, Status> {
        let config = serde_json::json!({
            "plugin_type": "base",
            "platform": "freebsd",
            "capabilities": ["users", "packages", "network", "jails"]
        });

        Ok(Response::new(proto::GetConfigResponse {
            config: config.to_string(),
        }))
    }

    async fn diff_state(
        &self,
        request: Request<proto::DiffStateRequest>,
    ) -> Result<Response<proto::DiffStateResponse>, Status> {
        let req = request.into_inner();
        let current: serde_json::Value =
            serde_json::from_str(&req.current_state).unwrap_or(serde_json::Value::Null);
        let desired: serde_json::Value = match serde_json::from_str(&req.desired_state) {
            Ok(v) => v,
            Err(_) => {
                return Ok(Response::new(proto::DiffStateResponse {
                    different: false,
                    changes: vec![],
                }));
            }
        };

        let mut task_changes: Vec<sysconfig_plugins::TaskChange> = Vec::new();

        if let Some(settings) = desired.get("network").and_then(|n| n.get("settings")) {
            let cur_settings = current
                .get("network")
                .and_then(|n| n.get("settings"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if let Ok(mut ch) = NetworkSettings::default().diff(&cur_settings, settings) {
                task_changes.append(&mut ch);
            }
        }
        if let Some(files) = desired.get("files") {
            if let Ok(mut ch) = Files::default().diff(&serde_json::Value::Null, files) {
                task_changes.append(&mut ch);
            }
        }

        let changes: Vec<proto::StateChange> = task_changes
            .into_iter()
            .map(|c| proto::StateChange {
                r#type: match c.change_type {
                    sysconfig_plugins::TaskChangeType::Create => proto::ChangeType::Create as i32,
                    sysconfig_plugins::TaskChangeType::Update => proto::ChangeType::Update as i32,
                    sysconfig_plugins::TaskChangeType::Delete => proto::ChangeType::Delete as i32,
                },
                path: c.path,
                old_value: c.old_value.map(|v| v.to_string()).unwrap_or_default(),
                new_value: c.new_value.map(|v| v.to_string()).unwrap_or_default(),
                verbose: c.verbose,
            })
            .collect();

        Ok(Response::new(proto::DiffStateResponse {
            different: !changes.is_empty(),
            changes,
        }))
    }

    async fn apply_state(
        &self,
        request: Request<proto::PluginApplyStateRequest>,
    ) -> Result<Response<proto::PluginApplyStateResponse>, Status> {
        let req = request.into_inner();

        // Parse desired state - try unified config first, then fall back to legacy
        let desired: serde_json::Value = match serde_json::from_str(&req.state) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Response::new(proto::PluginApplyStateResponse {
                    success: false,
                    error: format!("invalid state JSON: {}", e),
                    changes: vec![],
                }));
            }
        };

        let mut task_changes: Vec<sysconfig_plugins::TaskChange> = Vec::new();

        // Check if we should force dry-run mode
        let auto_dry_run = false; // FreeBSD doesn't auto-enable dry-run like illumos
        let effective_dry_run = req.dry_run || auto_dry_run;

        // Try to parse as unified config first
        let unified_config_result = UnifiedConfig::from_json(&req.state);

        if let Ok(unified_config) = unified_config_result {
            debug!("Processing unified configuration schema");

            // Apply system configuration
            if let Some(system) = &unified_config.system {
                if let Err(e) = self
                    .apply_system_config(system, effective_dry_run, &mut task_changes)
                    .await
                {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("failed to apply system config: {}", e),
                        changes: vec![],
                    }));
                }
            }

            // Apply software configuration (especially PKG repositories)
            if let Some(software) = &unified_config.software {
                if let Err(e) = self
                    .apply_software_config(software, effective_dry_run, &mut task_changes)
                    .await
                {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("failed to apply software config: {}", e),
                        changes: vec![],
                    }));
                }
            }

            // Apply user configuration
            for user in &unified_config.users {
                if let Err(e) = self
                    .apply_user_config(user, effective_dry_run, &mut task_changes)
                    .await
                {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("failed to apply user config: {}", e),
                        changes: vec![],
                    }));
                }
            }

            // Apply storage configuration
            if let Some(storage) = &unified_config.storage {
                if let Err(e) = self
                    .apply_storage_config(storage, effective_dry_run, &mut task_changes)
                    .await
                {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("failed to apply storage config: {}", e),
                        changes: vec![],
                    }));
                }
            }

            // Apply container configuration (FreeBSD jails)
            if let Some(containers) = &unified_config.containers {
                if let Err(e) = self
                    .apply_container_config(containers, effective_dry_run, &mut task_changes)
                    .await
                {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("failed to apply container config: {}", e),
                        changes: vec![],
                    }));
                }
            }
        } else {
            // Fall back to legacy JSON format
            debug!("Processing legacy JSON configuration format");

            if let Some(settings) = desired.get("network").and_then(|n| n.get("settings")) {
                if let Err(e) = NetworkSettings::validate_schema(settings) {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("invalid network.settings schema: {}", e),
                        changes: vec![],
                    }));
                }
                match NetworkSettings::default().apply(settings, effective_dry_run) {
                    Ok(mut changes) => {
                        task_changes.append(&mut changes);
                    }
                    Err(e) => {
                        return Ok(Response::new(proto::PluginApplyStateResponse {
                            success: false,
                            error: e,
                            changes: vec![],
                        }));
                    }
                }
            }

            // Apply files task if present at top-level: files: [ { ... } ]
            if let Some(files) = desired.get("files") {
                match Files::default().apply(files, effective_dry_run) {
                    Ok(mut changes) => {
                        task_changes.append(&mut changes);
                    }
                    Err(e) => {
                        return Ok(Response::new(proto::PluginApplyStateResponse {
                            success: false,
                            error: e,
                            changes: vec![],
                        }));
                    }
                }
            }
        }

        let changes: Vec<proto::StateChange> = task_changes
            .into_iter()
            .map(|c| proto::StateChange {
                r#type: match c.change_type {
                    sysconfig_plugins::TaskChangeType::Create => proto::ChangeType::Create as i32,
                    sysconfig_plugins::TaskChangeType::Update => proto::ChangeType::Update as i32,
                    sysconfig_plugins::TaskChangeType::Delete => proto::ChangeType::Delete as i32,
                },
                path: c.path,
                old_value: c.old_value.map(|v| v.to_string()).unwrap_or_default(),
                new_value: c.new_value.map(|v| v.to_string()).unwrap_or_default(),
                verbose: c.verbose,
            })
            .collect();

        let resp = proto::PluginApplyStateResponse {
            success: true,
            error: String::new(),
            changes,
        };
        Ok(Response::new(resp))
    }

    async fn execute_action(
        &self,
        request: Request<proto::PluginExecuteActionRequest>,
    ) -> Result<Response<proto::PluginExecuteActionResponse>, Status> {
        let req = request.into_inner();
        let result = format!(
            "freebsd-base executed action '{}' with params '{}'",
            req.action, req.parameters
        );
        Ok(Response::new(proto::PluginExecuteActionResponse {
            success: true,
            error: String::new(),
            result,
        }))
    }

    async fn notify_state_change(
        &self,
        _request: Request<proto::NotifyStateChangeRequest>,
    ) -> Result<Response<proto::NotifyStateChangeResponse>, Status> {
        Ok(Response::new(proto::NotifyStateChangeResponse {
            success: true,
            error: String::new(),
        }))
    }
}

impl FreeBsdBasePlugin {
    async fn apply_system_config(
        &self,
        system: &sysconfig_config_schema::SystemConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying system configuration");

        // Apply hostname
        if let Some(hostname) = &system.hostname {
            if dry_run {
                info!("DRY-RUN: Would set hostname to {}", hostname);
            } else {
                // On FreeBSD, set hostname via sysctl
                let output = std::process::Command::new("/sbin/sysctl")
                    .args(&["kern.hostname", &format!("kern.hostname={}", hostname)])
                    .output()
                    .map_err(|e| format!("failed to execute sysctl: {}", e))?;

                if !output.status.success() {
                    return Err(format!(
                        "failed to set hostname: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
            }

            task_changes.push(TaskChange {
                change_type: TaskChangeType::Update,
                path: "kern.hostname".to_string(),
                old_value: None,
                new_value: Some(serde_json::Value::String(hostname.clone())),
                verbose: false,
            });
        }

        Ok(())
    }

    async fn apply_software_config(
        &self,
        software: &sysconfig_config_schema::SoftwareConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying software configuration");

        // Handle PKG repositories if configured
        if let Some(repositories) = &software.repositories {
            if let Some(pkg_config) = &repositories.pkg {
                // Configure PKG repositories
                for repo in &pkg_config.repositories {
                    let repo_conf_path = format!("/usr/local/etc/pkg/repos/{}.conf", repo.name);

                    let repo_content = format!(
                        "{}: {{\n    url: \"{}\"\n    enabled: {}\n}}\n",
                        repo.name,
                        repo.url,
                        if repo.enabled { "yes" } else { "no" }
                    );

                    if dry_run {
                        info!(
                            "DRY-RUN: Would configure PKG repository {} at {}",
                            repo.name, repo_conf_path
                        );
                    } else {
                        // Create directory if it doesn't exist
                        if let Some(parent) = std::path::Path::new(&repo_conf_path).parent() {
                            std::fs::create_dir_all(parent).map_err(|e| {
                                format!("failed to create repo config directory: {}", e)
                            })?;
                        }

                        // Write repository configuration
                        std::fs::write(&repo_conf_path, &repo_content)
                            .map_err(|e| format!("failed to write repo config: {}", e))?;
                    }

                    task_changes.push(TaskChange {
                        change_type: TaskChangeType::Update,
                        path: repo_conf_path,
                        old_value: None,
                        new_value: Some(serde_json::Value::String(repo_content)),
                        verbose: false,
                    });
                }

                // Set PKG configuration for proxy if specified
                if let Some(proxy) = &pkg_config.proxy {
                    let pkg_conf_path = "/usr/local/etc/pkg.conf";
                    let proxy_line = format!("PKG_ENV: {{ HTTP_PROXY: \"{}\" }}\n", proxy);

                    if dry_run {
                        info!("DRY-RUN: Would configure PKG proxy in {}", pkg_conf_path);
                    } else {
                        // Read existing config or create new
                        let existing_content =
                            std::fs::read_to_string(pkg_conf_path).unwrap_or_default();
                        let new_content = if existing_content.contains("PKG_ENV") {
                            // Replace existing PKG_ENV line (simplified approach)
                            existing_content
                        } else {
                            format!("{}{}", existing_content, proxy_line)
                        };

                        std::fs::write(pkg_conf_path, &new_content)
                            .map_err(|e| format!("failed to write pkg config: {}", e))?;
                    }

                    task_changes.push(TaskChange {
                        change_type: TaskChangeType::Update,
                        path: pkg_conf_path.to_string(),
                        old_value: None,
                        new_value: Some(serde_json::Value::String(proxy_line)),
                        verbose: false,
                    });
                }
            }
        }

        // Handle package installation/removal
        if !software.packages_to_install.is_empty() || !software.packages_to_remove.is_empty() {
            let mut packages_json = serde_json::json!({});

            if !software.packages_to_install.is_empty() {
                packages_json["install"] = serde_json::Value::Array(
                    software
                        .packages_to_install
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                );
            }

            if !software.packages_to_remove.is_empty() {
                packages_json["remove"] = serde_json::Value::Array(
                    software
                        .packages_to_remove
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                );
            }

            match Packages::default().apply(&packages_json, dry_run) {
                Ok(mut changes) => {
                    task_changes.append(&mut changes);
                }
                Err(e) => return Err(format!("failed to apply package configuration: {}", e)),
            }
        }

        Ok(())
    }

    async fn apply_user_config(
        &self,
        user: &sysconfig_config_schema::UserConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying user configuration for {}", user.name);

        // Convert to legacy format for user management
        let mut user_json = serde_json::json!({
            "name": user.name,
            "create_home": user.create_home,
            "system_user": user.system_user
        });

        if let Some(description) = &user.description {
            user_json["description"] = serde_json::Value::String(description.clone());
        }

        if let Some(shell) = &user.shell {
            user_json["shell"] = serde_json::Value::String(shell.clone());
        }

        if !user.groups.is_empty() {
            user_json["groups"] = serde_json::Value::Array(
                user.groups
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }

        if !user.authentication.ssh_keys.is_empty() {
            user_json["ssh_keys"] = serde_json::Value::Array(
                user.authentication
                    .ssh_keys
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }

        // Apply user configuration using the Users task handler
        match Users::default().apply(&user_json, dry_run) {
            Ok(mut changes) => {
                task_changes.append(&mut changes);
            }
            Err(e) => return Err(format!("failed to apply user configuration: {}", e)),
        }

        Ok(())
    }

    async fn apply_container_config(
        &self,
        containers: &sysconfig_config_schema::ContainerConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying container configuration (FreeBSD jails)");

        // Handle FreeBSD jails
        for jail in &containers.jails {
            self.apply_jail_config(jail, dry_run, task_changes).await?;
        }

        Ok(())
    }

    async fn apply_jail_config(
        &self,
        jail_config: &sysconfig_config_schema::JailConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        if dry_run {
            info!("DRY-RUN: Would configure jail {}", jail_config.name);
        } else {
            // Check if jail exists
            let jail_exists = std::process::Command::new("/usr/sbin/jls")
                .args(&["-j", &jail_config.name])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false);

            if !jail_exists {
                // Create jail configuration in /etc/jail.conf
                let jail_conf_path = "/etc/jail.conf";

                // Read existing jail.conf or create new
                let existing_content = std::fs::read_to_string(jail_conf_path).unwrap_or_default();

                // Generate jail configuration block
                let mut jail_config_block = format!("\n{} {{\n", jail_config.name);
                jail_config_block.push_str(&format!("    path = \"{}\";\n", jail_config.path));
                jail_config_block.push_str(&format!(
                    "    host.hostname = \"{}\";\n",
                    jail_config.hostname
                ));

                // Add IP addresses
                if !jail_config.ip_addresses.is_empty() {
                    jail_config_block.push_str(&format!(
                        "    ip4.addr = \"{}\";\n",
                        jail_config.ip_addresses.join(", ")
                    ));
                }

                // Add interfaces
                if !jail_config.interfaces.is_empty() {
                    jail_config_block.push_str(&format!(
                        "    interface = \"{}\";\n",
                        jail_config.interfaces.join(", ")
                    ));
                }

                // Add custom parameters
                for (key, value) in &jail_config.parameters {
                    jail_config_block.push_str(&format!("    {} = \"{}\";\n", key, value));
                }

                jail_config_block.push_str("}\n");

                // Write updated configuration
                let new_content = if existing_content.contains(&jail_config.name) {
                    // Replace existing jail configuration (simplified approach)
                    existing_content
                } else {
                    format!("{}{}", existing_content, jail_config_block)
                };

                std::fs::write(jail_conf_path, new_content)
                    .map_err(|e| format!("failed to write jail configuration: {}", e))?;

                // Create jail root directory
                std::fs::create_dir_all(&jail_config.path)
                    .map_err(|e| format!("failed to create jail directory: {}", e))?;

                // Start the jail if auto_start is enabled
                if jail_config.auto_start {
                    let output = std::process::Command::new("/usr/sbin/jail")
                        .args(&["-c", &jail_config.name])
                        .output()
                        .map_err(|e| format!("failed to start jail: {}", e))?;

                    if !output.status.success() {
                        return Err(format!(
                            "failed to start jail {}: {}",
                            jail_config.name,
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }
            }

            // Apply nested sysconfig if present
            if let Some(nested_config) = &jail_config.sysconfig {
                info!("Applying nested sysconfig to jail {}", jail_config.name);
                self.apply_nested_sysconfig(
                    &jail_config.name,
                    nested_config,
                    dry_run,
                    task_changes,
                )
                .await?;
            }
        }

        task_changes.push(TaskChange {
            change_type: TaskChangeType::Create,
            path: format!("jail:{}", jail_config.name),
            old_value: None,
            new_value: Some(serde_json::json!({
                "path": jail_config.path,
                "hostname": jail_config.hostname,
                "ip_addresses": jail_config.ip_addresses,
                "interfaces": jail_config.interfaces,
                "parameters": jail_config.parameters,
                "auto_start": jail_config.auto_start,
                "has_nested_config": jail_config.sysconfig.is_some()
            })),
            verbose: false,
        });

        Ok(())
    }

    async fn apply_nested_sysconfig(
        &self,
        jail_name: &str,
        config: &sysconfig_config_schema::UnifiedConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        if dry_run {
            info!(
                "DRY-RUN: Would apply nested sysconfig to jail {}",
                jail_name
            );
            return Ok(());
        }

        // Serialize the nested config to JSON
        let config_json = config
            .to_json()
            .map_err(|e| format!("failed to serialize nested config: {}", e))?;

        // Create a temporary file with the nested config
        let temp_file = format!("/tmp/jail_{}_nested_config.json", jail_name);
        std::fs::write(&temp_file, config_json)
            .map_err(|e| format!("failed to write nested config file: {}", e))?;

        // Copy the config file to the jail
        let jail_config_dir = format!("/usr/jails/{}/etc/sysconfig", jail_name);
        std::fs::create_dir_all(&jail_config_dir)
            .map_err(|e| format!("failed to create jail config directory: {}", e))?;

        let jail_config_path = format!("{}/nested-config.json", jail_config_dir);
        let output = std::process::Command::new("/bin/cp")
            .args(&[&temp_file, &jail_config_path])
            .output()
            .map_err(|e| format!("failed to copy config to jail: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "failed to copy nested config to jail {}: {}",
                jail_name,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Execute sysconfig provisioning inside the jail
        let output = std::process::Command::new("/usr/sbin/jexec")
            .args(&[
                jail_name,
                "/usr/local/bin/sysconfig",
                "provision",
                "--config-file",
                "/etc/sysconfig/nested-config.json",
                "--run-once",
            ])
            .output()
            .map_err(|e| format!("failed to execute nested provisioning: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "failed to apply nested config in jail {}: {}",
                jail_name,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Clean up temporary file
        let _ = std::fs::remove_file(&temp_file);

        task_changes.push(TaskChange {
            change_type: TaskChangeType::Update,
            path: format!("jail:{}:nested-config", jail_name),
            old_value: None,
            new_value: Some(serde_json::json!({
                "applied": true,
                "config_path": "/etc/sysconfig/nested-config.json"
            })),
            verbose: false,
        });

        Ok(())
    }

    async fn apply_storage_config(
        &self,
        storage: &sysconfig_config_schema::StorageConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying storage configuration (FreeBSD)");

        // Handle ZFS pools
        for pool in &storage.pools {
            if let sysconfig_config_schema::StoragePoolType::ZfsPool = pool.pool_type {
                // Check if pool exists
                let pool_exists = if dry_run {
                    false
                } else {
                    std::process::Command::new("/sbin/zpool")
                        .args(&["list", "-H", &pool.name])
                        .output()
                        .map(|output| output.status.success())
                        .unwrap_or(false)
                };

                if dry_run {
                    info!("DRY-RUN: Would create/configure ZFS pool {}", pool.name);
                } else if !pool_exists && (!pool.devices.is_empty() || pool.topology.is_some()) {
                    // Create the pool with topology
                    let mut cmd = std::process::Command::new("/sbin/zpool");
                    cmd.args(&["create", &pool.name]);

                    if let Some(topology) = &pool.topology {
                        // Add data vdevs
                        for vdev in &topology.data {
                            match vdev.vdev_type {
                                sysconfig_config_schema::ZfsVdevType::Stripe => {
                                    cmd.args(&vdev.devices);
                                }
                                sysconfig_config_schema::ZfsVdevType::Mirror => {
                                    cmd.arg("mirror");
                                    cmd.args(&vdev.devices);
                                }
                                sysconfig_config_schema::ZfsVdevType::Raidz => {
                                    cmd.arg("raidz");
                                    cmd.args(&vdev.devices);
                                }
                                sysconfig_config_schema::ZfsVdevType::Raidz2 => {
                                    cmd.arg("raidz2");
                                    cmd.args(&vdev.devices);
                                }
                                sysconfig_config_schema::ZfsVdevType::Raidz3 => {
                                    cmd.arg("raidz3");
                                    cmd.args(&vdev.devices);
                                }
                            }
                        }
                    } else {
                        // Simple pool with devices
                        cmd.args(&pool.devices);
                    }

                    let output = cmd
                        .output()
                        .map_err(|e| format!("failed to execute zpool create: {}", e))?;

                    if !output.status.success() {
                        return Err(format!(
                            "failed to create ZFS pool {}: {}",
                            pool.name,
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }

                    info!("Created ZFS pool: {}", pool.name);
                }

                // Set pool properties
                if !dry_run {
                    for (prop, value) in &pool.properties {
                        let output = std::process::Command::new("/sbin/zpool")
                            .args(&["set", &format!("{}={}", prop, value), &pool.name])
                            .output()
                            .map_err(|e| format!("failed to set pool property: {}", e))?;

                        if !output.status.success() {
                            return Err(format!(
                                "failed to set property {}={} on pool {}: {}",
                                prop,
                                value,
                                pool.name,
                                String::from_utf8_lossy(&output.stderr)
                            ));
                        }
                    }
                }

                task_changes.push(TaskChange {
                    change_type: if pool_exists {
                        TaskChangeType::Update
                    } else {
                        TaskChangeType::Create
                    },
                    path: format!("zpool:{}", pool.name),
                    old_value: None,
                    new_value: Some(serde_json::json!({
                        "devices": pool.devices,
                        "properties": pool.properties,
                        "topology": pool.topology
                    })),
                    verbose: false,
                });
            }
        }

        // Handle ZFS datasets
        for dataset in &storage.zfs_datasets {
            if dry_run {
                info!("DRY-RUN: Would create ZFS dataset {}", dataset.name);
            } else {
                // Create the dataset
                let mut cmd = std::process::Command::new("/sbin/zfs");
                cmd.args(&["create"]);

                match &dataset.dataset_type {
                    sysconfig_config_schema::ZfsDatasetType::Filesystem => {
                        // No additional arguments needed for filesystem
                    }
                    sysconfig_config_schema::ZfsDatasetType::Volume { size } => {
                        cmd.args(&["-V", size]);
                    }
                }

                cmd.arg(&dataset.name);

                let output = cmd
                    .output()
                    .map_err(|e| format!("failed to create ZFS dataset: {}", e))?;

                if !output.status.success() {
                    return Err(format!(
                        "failed to create ZFS dataset {}: {}",
                        dataset.name,
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }

                // Set properties
                for (prop, value) in &dataset.properties {
                    let output = std::process::Command::new("/sbin/zfs")
                        .args(&["set", &format!("{}={}", prop, value), &dataset.name])
                        .output()
                        .map_err(|e| format!("failed to set dataset property: {}", e))?;

                    if !output.status.success() {
                        return Err(format!(
                            "failed to set property {}={} on dataset {}: {}",
                            prop,
                            value,
                            dataset.name,
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }

                info!("Created ZFS dataset: {}", dataset.name);
            }

            task_changes.push(TaskChange {
                change_type: TaskChangeType::Create,
                path: format!("zfs-dataset:{}", dataset.name),
                old_value: None,
                new_value: Some(serde_json::json!({
                    "name": dataset.name,
                    "type": dataset.dataset_type,
                    "properties": dataset.properties
                })),
                verbose: false,
            });
        }

        // Handle filesystems (UFS, etc.)
        for filesystem in &storage.filesystems {
            match filesystem.fstype {
                sysconfig_config_schema::FilesystemType::Zfs => {
                    // Already handled above in zfs_datasets
                }
                sysconfig_config_schema::FilesystemType::Ufs => {
                    if dry_run {
                        info!(
                            "DRY-RUN: Would create UFS filesystem on {}",
                            filesystem.device
                        );
                    } else {
                        // Create UFS filesystem
                        let output = std::process::Command::new("/sbin/newfs")
                            .arg(&filesystem.device)
                            .output()
                            .map_err(|e| format!("failed to create UFS filesystem: {}", e))?;

                        if !output.status.success() {
                            return Err(format!(
                                "failed to create UFS filesystem on {}: {}",
                                filesystem.device,
                                String::from_utf8_lossy(&output.stderr)
                            ));
                        }

                        info!("Created UFS filesystem on: {}", filesystem.device);
                    }

                    task_changes.push(TaskChange {
                        change_type: TaskChangeType::Create,
                        path: format!("ufs:{}", filesystem.device),
                        old_value: None,
                        new_value: Some(serde_json::json!({
                            "device": filesystem.device,
                            "fstype": "ufs",
                            "options": filesystem.options
                        })),
                        verbose: false,
                    });
                }
                _ => {
                    // Other filesystem types not supported on FreeBSD
                    warn!(
                        "Unsupported filesystem type on FreeBSD: {:?}",
                        filesystem.fstype
                    );
                }
            }
        }

        // Handle mounts
        for mount in &storage.mounts {
            if dry_run {
                info!("DRY-RUN: Would mount {} at {}", mount.source, mount.target);
            } else {
                // Create mount point if it doesn't exist
                if let Err(e) = std::fs::create_dir_all(&mount.target) {
                    return Err(format!(
                        "failed to create mount point {}: {}",
                        mount.target, e
                    ));
                }

                // Mount the filesystem
                let mut cmd = std::process::Command::new("/sbin/mount");

                if !mount.options.is_empty() {
                    let options_str = mount.options.join(",");
                    cmd.args(&["-o", &options_str]);
                }

                cmd.args(&[&mount.source, &mount.target]);

                let output = cmd
                    .output()
                    .map_err(|e| format!("failed to mount filesystem: {}", e))?;

                if !output.status.success() {
                    return Err(format!(
                        "failed to mount {} at {}: {}",
                        mount.source,
                        mount.target,
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }

                info!("Mounted {} at {}", mount.source, mount.target);
            }

            task_changes.push(TaskChange {
                change_type: TaskChangeType::Create,
                path: format!("mount:{}", mount.target),
                old_value: None,
                new_value: Some(serde_json::json!({
                    "source": mount.source,
                    "target": mount.target,
                    "options": mount.options
                })),
                verbose: false,
            });
        }

        Ok(())
    }
}

async fn register_with_sysconfig(service_socket: String, plugin_socket: String) {
    use hyper_util::rt::TokioIo;
    use tokio::net::UnixStream;
    use tower::service_fn;

    let plugin_id = uuid::Uuid::new_v4().to_string();

    let endpoint = tonic::transport::Endpoint::from_static("http://[::]:50051");
    let sock_for_closure = service_socket.clone();
    let conn = endpoint
        .connect_with_connector(service_fn(move |_| {
            let path = sock_for_closure.clone();
            async move {
                let stream = UnixStream::connect(path).await?;
                Ok::<_, std::io::Error>(TokioIo::new(stream))
            }
        }))
        .await;

    let channel = match conn {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to connect to sysconfig service for registration");
            return;
        }
    };

    let mut client = SysConfigServiceClient::new(channel);
    let req = proto::RegisterPluginRequest {
        plugin_id: plugin_id.clone(),
        name: "freebsd-base".to_string(),
        description: "Base plugin for FreeBSD: storage, users, packages, services, firewall, files, network.links, network.settings".to_string(),
        socket_path: plugin_socket.to_string(),
        managed_paths: vec![
            "storage".to_string(),
            "users".to_string(),
            "packages".to_string(),
            "services".to_string(),
            "firewall".to_string(),
            "files".to_string(),
            "network.links".to_string(),
            "network.settings".to_string(),
        ],
    };
    match client.register_plugin(req).await {
        Ok(resp) => {
            let resp = resp.into_inner();
            if resp.success {
                info!(plugin_id = %plugin_id, "Registered freebsd-base plugin with sysconfig service");
            } else {
                error!(error = %resp.error, "Plugin registration rejected by sysconfig service");
            }
        }
        Err(status) => {
            error!(code = ?status.code(), msg = %status.message(), "Plugin registration RPC failed");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let plugin_socket = args.socket.unwrap_or_else(default_plugin_socket_path);
    let service_socket = args
        .service_socket
        .unwrap_or_else(default_sysconfig_socket_path);

    // Ensure no stale socket exists
    let _ = std::fs::remove_file(&plugin_socket);
    if let Some(parent) = PathBuf::from(&plugin_socket).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = tokio::net::UnixListener::bind(&plugin_socket)?;
    let incoming = UnixListenerStream::new(listener);

    let plugin = FreeBsdBasePlugin::default();

    info!(socket = %plugin_socket, "Starting freebsd-base plugin server");

    // Optionally register with the sysconfig service
    if !args.no_register {
        let service_socket_clone = service_socket.clone();
        let plugin_socket_clone = plugin_socket.clone();
        tokio::spawn(async move {
            // small delay to ensure listener is up
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            register_with_sysconfig(service_socket_clone, plugin_socket_clone).await;
        });
    }

    tonic::transport::Server::builder()
        .add_service(PluginServiceServer::new(plugin))
        .serve_with_incoming(incoming)
        .await?;

    Ok(())
}
