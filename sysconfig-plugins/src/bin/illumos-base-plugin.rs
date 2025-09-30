use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(target_os = "illumos")]
use zone::{Adm, AttributeValue, Config, Zone};

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
#[clap(author, version, about = "Sysconfig base plugin: illumos", long_about = None)]
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
    if is_running_as_root() {
        "/var/run/sysconfig.sock".to_string()
    } else {
        // Use XDG_RUNTIME_DIR for non-root users
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/tmp/run-{}", unsafe { libc::geteuid() }));
        format!("{}/sysconfig.sock", runtime_dir)
    }
}

fn default_plugin_socket_path() -> String {
    if is_running_as_root() {
        "/var/run/sysconfig-illumos-base.sock".to_string()
    } else {
        // Use XDG_RUNTIME_DIR for non-root users
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/tmp/run-{}", unsafe { libc::geteuid() }));
        format!("{}/sysconfig-illumos-base.sock", runtime_dir)
    }
}

#[derive(Default)]
struct IllumosBasePlugin {
    inner: Arc<RwLock<PluginState>>,
}

#[derive(Default)]
struct PluginState {
    plugin_id: Option<String>,
    service_socket_path: Option<String>,
    auto_dry_run: bool,
}

/// Check if the process is running as root
fn is_running_as_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// Check if we should enable automatic dry-run mode
fn should_auto_enable_dry_run() -> bool {
    !is_running_as_root()
}

#[tonic::async_trait]
impl PluginService for IllumosBasePlugin {
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

        if auto_dry_run {
            warn!(
                "Running as non-root user (UID: {}). Auto-enabling dry-run mode for testing.",
                unsafe { libc::geteuid() }
            );
            info!("All operations will be simulated and no actual changes will be made to the system.");
            info!(
                "Using XDG_RUNTIME_DIR for socket paths: {}",
                std::env::var("XDG_RUNTIME_DIR")
                    .unwrap_or_else(|_| format!("/tmp/run-{}", unsafe { libc::geteuid() }))
            );
        }

        info!(plugin_id = %req.plugin_id, service = %req.service_socket_path, auto_dry_run = %auto_dry_run, "illumos base plugin initialized");
        Ok(Response::new(proto::InitializeResponse {
            success: true,
            error: String::new(),
        }))
    }

    async fn get_config(
        &self,
        _request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::GetConfigResponse>, Status> {
        let auto_dry_run = self.inner.read().await.auto_dry_run;
        let config = serde_json::json!({
            "name": "illumos-base",
            "os": "illumos",
            "tasks": ["storage", "users", "packages", "services", "firewall", "files", "network.links", "network.settings"],
            "auto_dry_run": auto_dry_run,
            "running_as_root": is_running_as_root(),
        })
        .to_string();
        Ok(Response::new(proto::GetConfigResponse { config }))
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

        info!("DEBUG: apply_state called with state: {}", req.state);
        info!("DEBUG: apply_state dry_run flag: {}", req.dry_run);

        // Check if we should force dry-run mode
        let auto_dry_run = self.inner.read().await.auto_dry_run;
        let effective_dry_run = req.dry_run || auto_dry_run;

        info!(
            "DEBUG: auto_dry_run: {}, effective_dry_run: {}",
            auto_dry_run, effective_dry_run
        );

        if auto_dry_run && !req.dry_run {
            info!("Auto-enabling dry-run mode since running as non-root user");
        }

        if effective_dry_run {
            info!("DRY-RUN MODE: Simulating state changes without applying them");
        }

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

            // Apply container configuration
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

            // Apply networking configuration
            if let Some(networking) = &unified_config.networking {
                if let Err(e) = self
                    .apply_networking_config(networking, effective_dry_run, &mut task_changes)
                    .await
                {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("failed to apply networking config: {}", e),
                        changes: vec![],
                    }));
                }
            }

            // Apply software configuration
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

            // Apply script configuration
            if let Some(scripts) = &unified_config.scripts {
                if let Err(e) = self
                    .apply_scripts_config(scripts, effective_dry_run, &mut task_changes)
                    .await
                {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("failed to apply scripts config: {}", e),
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
        } else {
            // Fall back to legacy JSON format
            debug!("Processing legacy JSON configuration format");

            // Legacy network settings handling
            if let Some(settings) = desired.get("network").and_then(|n| n.get("settings")) {
                if let Err(e) = NetworkSettings::validate_schema(settings) {
                    return Ok(Response::new(proto::PluginApplyStateResponse {
                        success: false,
                        error: format!("invalid network.settings schema: {}", e),
                        changes: vec![],
                    }));
                }

                if effective_dry_run {
                    info!("DRY-RUN: Would apply network settings: {}", settings);
                }

                match NetworkSettings::default().apply(settings, effective_dry_run) {
                    Ok(mut changes) => {
                        for change in &changes {
                            if effective_dry_run {
                                info!(
                                    "DRY-RUN: Would {} {} - new value: {:?}",
                                    change.change_type.as_str(),
                                    change.path,
                                    change.new_value
                                );
                            }
                        }
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

            // Legacy files handling
            if let Some(files) = desired.get("files") {
                if effective_dry_run {
                    info!("DRY-RUN: Would apply file changes: {}", files);
                }

                match Files::default().apply(files, effective_dry_run) {
                    Ok(mut changes) => {
                        for change in &changes {
                            if effective_dry_run {
                                info!(
                                    "DRY-RUN: Would {} {} - new value: {:?}",
                                    change.change_type.as_str(),
                                    change.path,
                                    change.new_value
                                );
                            }
                        }
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

        if effective_dry_run && !changes.is_empty() {
            info!(
                "DRY-RUN: Total of {} changes would be applied",
                changes.len()
            );
        }

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
            "illumos-base executed action '{}' with params '{}'",
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

impl IllumosBasePlugin {
    async fn apply_system_config(
        &self,
        system: &sysconfig_config_schema::SystemConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying system configuration");

        // Apply hostname
        if let Some(hostname) = &system.hostname {
            let hostname_json = serde_json::json!({
                "hostname": hostname
            });

            match NetworkSettings::default().apply(&hostname_json, dry_run) {
                Ok(mut changes) => {
                    task_changes.append(&mut changes);
                }
                Err(e) => return Err(format!("failed to set hostname: {}", e)),
            }
        }

        // Apply timezone
        if let Some(timezone) = &system.timezone {
            if dry_run {
                info!("DRY-RUN: Would set timezone to {}", timezone);
                task_changes.push(TaskChange {
                    change_type: TaskChangeType::Update,
                    path: "/etc/timezone".to_string(),
                    old_value: None,
                    new_value: Some(serde_json::Value::String(timezone.clone())),
                    verbose: false,
                });
            } else {
                // On illumos, timezone is set via SMF
                let output = std::process::Command::new("/usr/sbin/svccfg")
                    .args(&[
                        "-s",
                        "system/timezone",
                        "setprop",
                        "timezone/localtime",
                        "=",
                        timezone,
                    ])
                    .output()
                    .map_err(|e| format!("failed to execute svccfg: {}", e))?;

                if !output.status.success() {
                    return Err(format!(
                        "failed to set timezone: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }

                std::process::Command::new("/usr/sbin/svcadm")
                    .args(&["refresh", "system/timezone"])
                    .output()
                    .map_err(|e| format!("failed to refresh timezone service: {}", e))?;

                task_changes.push(TaskChange {
                    change_type: TaskChangeType::Update,
                    path: format!("smf:system/timezone:timezone/localtime"),
                    old_value: None,
                    new_value: Some(serde_json::Value::String(timezone.clone())),
                    verbose: false,
                });
            }
        }

        Ok(())
    }

    async fn apply_networking_config(
        &self,
        networking: &sysconfig_config_schema::NetworkingConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying networking configuration");

        // Convert unified networking config to legacy format for now
        let mut network_settings = serde_json::json!({});

        // DNS nameservers
        if !networking.nameservers.is_empty() {
            network_settings["nameservers"] = serde_json::Value::Array(
                networking
                    .nameservers
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }

        // DNS search domains
        if !networking.search_domains.is_empty() {
            network_settings["search_domains"] = serde_json::Value::Array(
                networking
                    .search_domains
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }

        // Apply DNS configuration if we have any
        if !network_settings.as_object().unwrap().is_empty() {
            match NetworkSettings::default().apply(&network_settings, dry_run) {
                Ok(mut changes) => {
                    task_changes.append(&mut changes);
                }
                Err(e) => return Err(format!("failed to apply DNS settings: {}", e)),
            }
        }

        // TODO: Handle network interfaces, routes, etc.
        // This would require extending the NetworkSettings task or creating new tasks

        Ok(())
    }

    async fn apply_software_config(
        &self,
        software: &sysconfig_config_schema::SoftwareConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying software configuration");

        // Convert to legacy format for package management
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

        if software.update_on_boot {
            packages_json["refresh"] = serde_json::Value::Bool(true);
        }

        if software.upgrade_on_boot {
            packages_json["update"] = serde_json::Value::Bool(true);
        }

        // Apply package configuration if we have any
        if !packages_json.as_object().unwrap().is_empty() {
            match Packages::default().apply(&packages_json, dry_run) {
                Ok(mut changes) => {
                    task_changes.append(&mut changes);
                }
                Err(e) => return Err(format!("failed to apply package configuration: {}", e)),
            }
        }

        // Handle IPS publishers if configured
        if let Some(repositories) = &software.repositories {
            if let Some(ips_config) = &repositories.ips {
                for publisher in &ips_config.publishers {
                    if dry_run {
                        info!("DRY-RUN: Would configure IPS publisher {}", publisher.name);
                    } else {
                        // Configure IPS publisher
                        let mut cmd = std::process::Command::new("/usr/bin/pkg");
                        cmd.args(&["set-publisher", "-O", &publisher.origin, &publisher.name]);

                        if !publisher.enabled {
                            cmd.arg("--disable");
                        }

                        let output = cmd
                            .output()
                            .map_err(|e| format!("failed to execute pkg command: {}", e))?;

                        if !output.status.success() {
                            return Err(format!(
                                "failed to configure publisher {}: {}",
                                publisher.name,
                                String::from_utf8_lossy(&output.stderr)
                            ));
                        }
                    }

                    task_changes.push(TaskChange {
                        change_type: TaskChangeType::Update,
                        path: format!("ips:publisher:{}", publisher.name),
                        old_value: None,
                        new_value: Some(serde_json::json!({
                            "origin": publisher.origin,
                            "enabled": publisher.enabled,
                            "preferred": publisher.preferred
                        })),
                        verbose: false,
                    });
                }
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

        if let Some(primary_group) = &user.primary_group {
            user_json["primary_group"] = serde_json::Value::String(primary_group.clone());
        }

        if let Some(home_directory) = &user.home_directory {
            user_json["home_directory"] = serde_json::Value::String(home_directory.clone());
        }

        if let Some(uid) = user.uid {
            user_json["uid"] = serde_json::Value::Number(uid.into());
        }

        // Handle SSH keys
        if !user.authentication.ssh_keys.is_empty() {
            user_json["ssh_keys"] = serde_json::Value::Array(
                user.authentication
                    .ssh_keys
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }

        // Handle password
        if let Some(password_config) = &user.authentication.password {
            user_json["password_hash"] = serde_json::Value::String(password_config.hash.clone());
            user_json["expire_on_first_login"] =
                serde_json::Value::Bool(password_config.expire_on_first_login);
        }

        // Handle sudo configuration
        if let Some(sudo_config) = &user.sudo {
            match sudo_config {
                sysconfig_config_schema::SudoConfig::Deny => {
                    user_json["sudo"] = serde_json::Value::Bool(false);
                }
                sysconfig_config_schema::SudoConfig::Unrestricted => {
                    user_json["sudo"] = serde_json::Value::Bool(true);
                }
                sysconfig_config_schema::SudoConfig::Custom(rules) => {
                    user_json["sudo_rules"] = serde_json::Value::Array(
                        rules
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    );
                }
            }
        }

        // Apply user configuration
        match Users::default().apply(&user_json, dry_run) {
            Ok(mut changes) => {
                task_changes.append(&mut changes);
            }
            Err(e) => return Err(format!("failed to apply user configuration: {}", e)),
        }

        Ok(())
    }

    async fn apply_scripts_config(
        &self,
        scripts: &sysconfig_config_schema::ScriptConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying scripts configuration");

        // Handle different script phases
        for script in &scripts.early_scripts {
            self.execute_script(script, "early", dry_run, task_changes)
                .await?;
        }

        for script in &scripts.main_scripts {
            self.execute_script(script, "main", dry_run, task_changes)
                .await?;
        }

        for script in &scripts.late_scripts {
            self.execute_script(script, "late", dry_run, task_changes)
                .await?;
        }

        Ok(())
    }

    async fn execute_script(
        &self,
        script: &sysconfig_config_schema::Script,
        phase: &str,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        if dry_run {
            info!("DRY-RUN: Would execute {} script: {}", phase, script.id);
            task_changes.push(TaskChange {
                change_type: TaskChangeType::Create,
                path: format!("script:{}:{}", phase, script.id),
                old_value: None,
                new_value: Some(serde_json::Value::String(script.content.clone())),
                verbose: false,
            });
            return Ok(());
        }

        // Create temporary script file
        let temp_dir = "/tmp";
        let script_path = format!("{}/sysconfig_script_{}.sh", temp_dir, script.id);

        std::fs::write(&script_path, &script.content)
            .map_err(|e| format!("failed to write script file: {}", e))?;

        // Make executable
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("failed to set script permissions: {}", e))?;

        // Execute script
        let mut cmd = std::process::Command::new(&script_path);

        if let Some(working_dir) = &script.working_directory {
            cmd.current_dir(working_dir);
        }

        for (key, value) in &script.environment {
            cmd.env(key, value);
        }

        let output = cmd
            .output()
            .map_err(|e| format!("failed to execute script: {}", e))?;

        // Clean up temporary file
        let _ = std::fs::remove_file(&script_path);

        if !output.status.success() {
            return Err(format!(
                "script {} failed with exit code {:?}: {}",
                script.id,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Log output if requested
        if let Some(output_file) = &script.output_file {
            if let Err(e) = std::fs::write(output_file, &output.stdout) {
                warn!("Failed to write script output to {}: {}", output_file, e);
            }
        }

        task_changes.push(TaskChange {
            change_type: TaskChangeType::Create,
            path: format!("script:{}:{}", phase, script.id),
            old_value: None,
            new_value: Some(serde_json::Value::String(
                String::from_utf8_lossy(&output.stdout).to_string(),
            )),
            verbose: true,
        });

        Ok(())
    }

    async fn apply_storage_config(
        &self,
        storage: &sysconfig_config_schema::StorageConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying storage configuration");

        // Handle ZFS pools with advanced topology
        for pool in &storage.pools {
            if let sysconfig_config_schema::StoragePoolType::ZfsPool = pool.pool_type {
                // Check if pool exists
                let pool_exists = if dry_run {
                    false
                } else {
                    std::process::Command::new("/usr/sbin/zpool")
                        .args(&["list", "-H", &pool.name])
                        .output()
                        .map(|output| output.status.success())
                        .unwrap_or(false)
                };

                if dry_run {
                    info!("DRY-RUN: Would create/configure ZFS pool {}", pool.name);
                } else if !pool_exists && (!pool.devices.is_empty() || pool.topology.is_some()) {
                    // Create the pool with topology
                    let mut cmd = std::process::Command::new("/usr/sbin/zpool");
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

                        // Add log vdevs
                        for vdev in &topology.log {
                            cmd.arg("log");
                            match vdev.vdev_type {
                                sysconfig_config_schema::ZfsVdevType::Mirror => {
                                    cmd.arg("mirror");
                                    cmd.args(&vdev.devices);
                                }
                                _ => {
                                    cmd.args(&vdev.devices);
                                }
                            }
                        }

                        // Add cache vdevs
                        for vdev in &topology.cache {
                            cmd.arg("cache");
                            cmd.args(&vdev.devices);
                        }

                        // Add spare vdevs
                        if !topology.spare.is_empty() {
                            cmd.arg("spare");
                            cmd.args(&topology.spare);
                        }
                    } else {
                        // Simple pool creation
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
                }

                // Set pool properties (both for new and existing pools)
                if !dry_run {
                    for (prop, value) in &pool.properties {
                        let output = std::process::Command::new("/usr/sbin/zpool")
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

        // Handle advanced ZFS datasets
        for dataset in &storage.zfs_datasets {
            self.create_zfs_dataset(dataset, dry_run, task_changes)
                .await?;
        }

        // Handle filesystems
        for filesystem in &storage.filesystems {
            if let sysconfig_config_schema::FilesystemType::Zfs = filesystem.fstype {
                if dry_run {
                    info!("DRY-RUN: Would create ZFS filesystem {}", filesystem.device);
                } else {
                    // Create ZFS dataset
                    let output = std::process::Command::new("/usr/sbin/zfs")
                        .args(&["create", &filesystem.device])
                        .output()
                        .map_err(|e| format!("failed to create ZFS filesystem: {}", e))?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if !stderr.contains("dataset already exists") {
                            return Err(format!(
                                "failed to create ZFS filesystem {}: {}",
                                filesystem.device, stderr
                            ));
                        }
                    }

                    // Set filesystem properties
                    for (prop, value) in &filesystem.options {
                        let output = std::process::Command::new("/usr/sbin/zfs")
                            .args(&["set", &format!("{}={}", prop, value), &filesystem.device])
                            .output()
                            .map_err(|e| format!("failed to set filesystem property: {}", e))?;

                        if !output.status.success() {
                            return Err(format!(
                                "failed to set property {}={} on filesystem {}: {}",
                                prop,
                                value,
                                filesystem.device,
                                String::from_utf8_lossy(&output.stderr)
                            ));
                        }
                    }
                }

                task_changes.push(TaskChange {
                    change_type: TaskChangeType::Create,
                    path: format!("zfs:{}", filesystem.device),
                    old_value: None,
                    new_value: Some(serde_json::json!({
                        "fstype": "zfs",
                        "options": filesystem.options
                    })),
                    verbose: false,
                });
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
                let mut cmd = std::process::Command::new("/usr/sbin/mount");

                if let Some(fstype) = &mount.fstype {
                    cmd.args(&["-F", fstype]);
                }

                if !mount.options.is_empty() {
                    cmd.args(&["-o", &mount.options.join(",")]);
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
            }

            task_changes.push(TaskChange {
                change_type: TaskChangeType::Create,
                path: format!("mount:{}:{}", mount.source, mount.target),
                old_value: None,
                new_value: Some(serde_json::json!({
                    "source": mount.source,
                    "target": mount.target,
                    "fstype": mount.fstype,
                    "options": mount.options,
                    "persistent": mount.persistent
                })),
                verbose: false,
            });
        }

        // Handle ZFS snapshots
        for snapshot in &storage.zfs_snapshots {
            self.create_zfs_snapshot(snapshot, dry_run, task_changes)
                .await?;
        }

        // Handle ZFS replication
        for replication in &storage.zfs_replication {
            self.setup_zfs_replication(replication, dry_run, task_changes)
                .await?;
        }

        Ok(())
    }

    async fn create_zfs_dataset(
        &self,
        dataset: &sysconfig_config_schema::ZfsDatasetConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        if dry_run {
            info!("DRY-RUN: Would create ZFS dataset {}", dataset.name);
        } else {
            // Create the dataset
            let mut cmd = std::process::Command::new("/usr/sbin/zfs");
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
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.contains("dataset already exists") {
                    return Err(format!(
                        "failed to create ZFS dataset {}: {}",
                        dataset.name, stderr
                    ));
                }
            }

            // Set properties
            for (prop, value) in &dataset.properties {
                let output = std::process::Command::new("/usr/sbin/zfs")
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

            // Set quota if specified
            if let Some(quota) = &dataset.quota {
                let output = std::process::Command::new("/usr/sbin/zfs")
                    .args(&["set", &format!("quota={}", quota), &dataset.name])
                    .output()
                    .map_err(|e| format!("failed to set quota: {}", e))?;

                if !output.status.success() {
                    return Err(format!(
                        "failed to set quota {} on dataset {}: {}",
                        quota,
                        dataset.name,
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
            }

            // Set reservation if specified
            if let Some(reservation) = &dataset.reservation {
                let output = std::process::Command::new("/usr/sbin/zfs")
                    .args(&[
                        "set",
                        &format!("reservation={}", reservation),
                        &dataset.name,
                    ])
                    .output()
                    .map_err(|e| format!("failed to set reservation: {}", e))?;

                if !output.status.success() {
                    return Err(format!(
                        "failed to set reservation {} on dataset {}: {}",
                        reservation,
                        dataset.name,
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
            }
        }

        // Recursively create child datasets
        for child in &dataset.children {
            Box::pin(self.create_zfs_dataset(child, dry_run, task_changes)).await?;
        }

        task_changes.push(TaskChange {
            change_type: TaskChangeType::Create,
            path: format!("zfs-dataset:{}", dataset.name),
            old_value: None,
            new_value: Some(serde_json::json!({
                "dataset_type": dataset.dataset_type,
                "properties": dataset.properties,
                "quota": dataset.quota,
                "reservation": dataset.reservation
            })),
            verbose: false,
        });

        Ok(())
    }

    async fn create_zfs_snapshot(
        &self,
        snapshot: &sysconfig_config_schema::ZfsSnapshotConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        let snapshot_name = format!("{}@{}", snapshot.dataset, snapshot.name);

        if dry_run {
            info!("DRY-RUN: Would create ZFS snapshot {}", snapshot_name);
        } else {
            let mut cmd = std::process::Command::new("/usr/sbin/zfs");
            cmd.args(&["snapshot"]);

            if snapshot.recursive {
                cmd.arg("-r");
            }

            cmd.arg(&snapshot_name);

            let output = cmd
                .output()
                .map_err(|e| format!("failed to create ZFS snapshot: {}", e))?;

            if !output.status.success() {
                return Err(format!(
                    "failed to create ZFS snapshot {}: {}",
                    snapshot_name,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            // Set snapshot properties
            for (prop, value) in &snapshot.properties {
                let output = std::process::Command::new("/usr/sbin/zfs")
                    .args(&["set", &format!("{}={}", prop, value), &snapshot_name])
                    .output()
                    .map_err(|e| format!("failed to set snapshot property: {}", e))?;

                if !output.status.success() {
                    return Err(format!(
                        "failed to set property {}={} on snapshot {}: {}",
                        prop,
                        value,
                        snapshot_name,
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
            }
        }

        task_changes.push(TaskChange {
            change_type: TaskChangeType::Create,
            path: format!("zfs-snapshot:{}", snapshot_name),
            old_value: None,
            new_value: Some(serde_json::json!({
                "dataset": snapshot.dataset,
                "name": snapshot.name,
                "recursive": snapshot.recursive,
                "properties": snapshot.properties
            })),
            verbose: false,
        });

        Ok(())
    }

    async fn setup_zfs_replication(
        &self,
        replication: &sysconfig_config_schema::ZfsReplicationConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        if dry_run {
            info!(
                "DRY-RUN: Would setup ZFS replication from {} to {}",
                replication.source_dataset, replication.target
            );
        } else {
            match replication.replication_type {
                sysconfig_config_schema::ZfsReplicationType::Send => {
                    // Setup basic ZFS send
                    let mut cmd = std::process::Command::new("/usr/sbin/zfs");
                    cmd.args(&["send", &replication.source_dataset]);

                    if let Some(ssh_config) = &replication.ssh_config {
                        // Pipe to SSH for remote replication
                        let ssh_cmd = format!(
                            "ssh {}@{} zfs recv {}",
                            ssh_config.user, ssh_config.host, replication.target
                        );
                        info!(
                            "Would execute: zfs send {} | {}",
                            replication.source_dataset, ssh_cmd
                        );
                    }
                }
                sysconfig_config_schema::ZfsReplicationType::Incremental => {
                    info!(
                        "Incremental replication setup for {}",
                        replication.source_dataset
                    );
                }
                sysconfig_config_schema::ZfsReplicationType::Full => {
                    info!("Full replication setup for {}", replication.source_dataset);
                }
            }
        }

        task_changes.push(TaskChange {
            change_type: TaskChangeType::Create,
            path: format!(
                "zfs-replication:{}:{}",
                replication.source_dataset, replication.target
            ),
            old_value: None,
            new_value: Some(serde_json::json!({
                "source_dataset": replication.source_dataset,
                "target": replication.target,
                "replication_type": replication.replication_type,
                "ssh_config": replication.ssh_config
            })),
            verbose: false,
        });

        Ok(())
    }

    async fn apply_container_config(
        &self,
        containers: &sysconfig_config_schema::ContainerConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        debug!("Applying container configuration");

        // Handle Solaris/illumos zones
        for zone in &containers.zones {
            self.apply_zone_config(zone, dry_run, task_changes).await?;
        }

        Ok(())
    }

    #[cfg(target_os = "illumos")]
    async fn apply_zone_config(
        &self,
        zone_config: &sysconfig_config_schema::ZoneConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        if dry_run {
            info!("DRY-RUN: Would configure zone {}", zone_config.name);
        } else {
            // Check if zone exists
            // Check if zone exists by trying to create a config for it
            let zone_exists = Config::new(&zone_config.name).is_ok();

            if !zone_exists {
                // Create new zone configuration
                let mut config = Config::create(&zone_config.name, &zone_config.zonepath)
                    .map_err(|e| format!("failed to create zone config: {}", e))?;

                // Set brand
                config
                    .set_brand(&zone_config.brand)
                    .map_err(|e| format!("failed to set brand: {}", e))?;

                // Add network configurations
                for network in &zone_config.networks {
                    let net_scope = config
                        .add_net()
                        .map_err(|e| format!("failed to add network: {}", e))?;

                    net_scope
                        .set_physical(&network.physical)
                        .map_err(|e| format!("failed to set physical interface: {}", e))?;

                    if let Some(address) = &network.address {
                        net_scope
                            .set_address(address)
                            .map_err(|e| format!("failed to set address: {}", e))?;
                    }

                    if let Some(defrouter) = &network.defrouter {
                        net_scope
                            .set_defrouter(defrouter)
                            .map_err(|e| format!("failed to set defrouter: {}", e))?;
                    }
                }

                // Add resource controls
                if let Some(resources) = &zone_config.resources {
                    if let Some(cpu_cap) = resources.cpu_cap {
                        let capped_cpu_scope = config
                            .add_capped_cpu()
                            .map_err(|e| format!("failed to add capped cpu: {}", e))?;
                        capped_cpu_scope
                            .set_ncpus(&cpu_cap.to_string())
                            .map_err(|e| format!("failed to set ncpus: {}", e))?;
                    }

                    if let Some(memory_cap) = &resources.physical_memory_cap {
                        let capped_memory_scope = config
                            .add_capped_memory()
                            .map_err(|e| format!("failed to add capped memory: {}", e))?;
                        capped_memory_scope
                            .set_physical(memory_cap)
                            .map_err(|e| format!("failed to set physical memory: {}", e))?;

                        if let Some(swap_cap) = &resources.swap_memory_cap {
                            capped_memory_scope
                                .set_swap(swap_cap)
                                .map_err(|e| format!("failed to set swap memory: {}", e))?;
                        }
                    }
                }

                // Add custom properties as attributes
                for (key, value) in &zone_config.properties {
                    let attr_scope = config
                        .add_attr()
                        .map_err(|e| format!("failed to add attribute: {}", e))?;
                    attr_scope
                        .set_name(key)
                        .map_err(|e| format!("failed to set attribute name: {}", e))?;
                    attr_scope
                        .set_value(&AttributeValue::String(value.clone()))
                        .map_err(|e| format!("failed to set attribute value: {}", e))?;
                }

                // Commit the zone configuration
                config
                    .commit()
                    .map_err(|e| format!("failed to commit zone config: {}", e))?;

                // Install the zone if needed
                match zone_config.state {
                    sysconfig_config_schema::ZoneState::Installed
                    | sysconfig_config_schema::ZoneState::Running => {
                        let mut adm = Adm::new(&zone_config.name);
                        adm.install(&[]).map_err(|e| {
                            format!("failed to install zone {}: {}", zone_config.name, e)
                        })?;
                    }
                    _ => {}
                }

                // Boot the zone if needed
                if let sysconfig_config_schema::ZoneState::Running = zone_config.state {
                    let mut adm = Adm::new(&zone_config.name);
                    adm.boot(&[])
                        .map_err(|e| format!("failed to boot zone {}: {}", zone_config.name, e))?;
                }
            } else {
                // Zone exists, check current state and handle transitions
                let mut adm = Adm::new(&zone_config.name);
                let zones = adm
                    .list()
                    .map_err(|e| format!("failed to list zones: {}", e))?;

                let current_zone = zones
                    .iter()
                    .find(|z| z.name() == zone_config.name)
                    .ok_or_else(|| format!("zone {} not found", zone_config.name))?;

                let current_state = current_zone.state();

                match (current_state, &zone_config.state) {
                    (zone::State::Configured, sysconfig_config_schema::ZoneState::Installed)
                    | (zone::State::Configured, sysconfig_config_schema::ZoneState::Running) => {
                        adm.install(&[]).map_err(|e| {
                            format!("failed to install zone {}: {}", zone_config.name, e)
                        })?;
                    }
                    (zone::State::Installed, sysconfig_config_schema::ZoneState::Running) => {
                        adm.boot(&[]).map_err(|e| {
                            format!("failed to boot zone {}: {}", zone_config.name, e)
                        })?;
                    }
                    _ => {} // No state change needed
                }
            }

            // Apply nested sysconfig if present
            if let Some(nested_config) = &zone_config.sysconfig {
                info!("Applying nested sysconfig to zone {}", zone_config.name);
                self.apply_nested_sysconfig(
                    &zone_config.name,
                    nested_config,
                    dry_run,
                    task_changes,
                )
                .await?;
            }
        }

        self.finish_zone_config(zone_config, task_changes).await
    }

    #[cfg(not(target_os = "illumos"))]
    async fn apply_zone_config(
        &self,
        zone_config: &sysconfig_config_schema::ZoneConfig,
        _dry_run: bool,
        _task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        Err(format!(
            "Zone management is only supported on illumos/Solaris, zone {} cannot be created",
            zone_config.name
        ))
    }

    #[cfg(target_os = "illumos")]
    async fn finish_zone_config(
        &self,
        zone_config: &sysconfig_config_schema::ZoneConfig,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        task_changes.push(TaskChange {
            change_type: TaskChangeType::Create,
            path: format!("zone:{}", zone_config.name),
            old_value: None,
            new_value: Some(serde_json::json!({
                "brand": zone_config.brand,
                "state": zone_config.state,
                "zonepath": zone_config.zonepath,
                "networks": zone_config.networks,
                "resources": zone_config.resources,
                "properties": zone_config.properties,
                "has_nested_config": zone_config.sysconfig.is_some()
            })),
            verbose: false,
        });

        Ok(())
    }

    #[cfg(target_os = "illumos")]
    async fn apply_nested_sysconfig(
        &self,
        zone_name: &str,
        config: &sysconfig_config_schema::UnifiedConfig,
        dry_run: bool,
        task_changes: &mut Vec<TaskChange>,
    ) -> Result<(), String> {
        if dry_run {
            info!(
                "DRY-RUN: Would apply nested sysconfig to zone {}",
                zone_name
            );
            return Ok(());
        }

        // Serialize the nested config to JSON
        let config_json = config
            .to_json()
            .map_err(|e| format!("failed to serialize nested config: {}", e))?;

        // Write the nested config directly to the zone's sysconfig directory
        let zone_config_dir = format!("/zones/{}/root/etc/sysconfig", zone_name);
        let zone_config_path = format!("{}/provisioning-config.json", zone_config_dir);

        // Ensure the sysconfig directory exists in the zone
        std::fs::create_dir_all(&zone_config_dir)
            .map_err(|e| format!("failed to create sysconfig directory in zone: {}", e))?;

        // Write the config file directly to the zone
        std::fs::write(&zone_config_path, config_json)
            .map_err(|e| format!("failed to write nested config to zone: {}", e))?;

        task_changes.push(TaskChange {
            change_type: TaskChangeType::Update,
            path: format!("zone:{}:nested-config", zone_name),
            old_value: None,
            new_value: Some(serde_json::json!({
                "config_file_placed": true,
                "config_path": "/etc/sysconfig/provisioning-config.json"
            })),
            verbose: false,
        });

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
        name: "illumos-base".to_string(),
        description: "Base plugin for illumos: storage, users, packages, services, firewall, files, network.links, network.settings".to_string(),
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
                info!(plugin_id = %plugin_id, "Registered illumos-base plugin with sysconfig service");
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

    let plugin = IllumosBasePlugin::default();

    // Display startup information about dry-run mode
    if should_auto_enable_dry_run() {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/tmp/run-{}", unsafe { libc::geteuid() }));
        warn!("====================================================================");
        warn!(
            "TESTING MODE ACTIVATED - Running as non-root user (UID: {})",
            unsafe { libc::geteuid() }
        );
        warn!("All operations will be simulated (dry-run) and logged.");
        warn!("No actual system changes will be made.");
        warn!("Socket paths using: {}", runtime_dir);
        warn!("  - Plugin socket: {}", plugin_socket);
        warn!("  - Service socket: {}", service_socket);
        warn!("To run with actual changes, execute this plugin as root.");
        warn!("====================================================================");
    } else {
        info!("Running as root user - operations will make actual system changes");
    }

    info!(socket = %plugin_socket, "Starting illumos-base plugin server");

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
