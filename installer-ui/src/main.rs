use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(MainLayout)]
    #[route("/")]
    Welcome {},
    #[route("/server")]
    ServerSelection {},
    #[route("/storage")]
    StorageConfiguration {},
    #[route("/network")]
    NetworkConfiguration {},
    #[route("/system")]
    SystemConfiguration {},
    #[route("/review")]
    ReviewConfiguration {},
    #[route("/install")]
    Installation {},
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MachineServer {
    pub id: String,
    pub name: String,
    pub address: String,
    pub status: ServerStatus,
    pub specs: ServerSpecs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerStatus {
    Available,
    Busy,
    Offline,
    Installing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerSpecs {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub network_interfaces: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pool {
    pub name: String,
    pub vdevs: Vec<VDev>,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VDev {
    pub kind: VDevType,
    pub disks: Vec<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkInterface {
    pub name: String,
    pub selector: Option<String>,
    pub addresses: Vec<AddressObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddressObject {
    pub name: String,
    pub kind: AddressKind,
    pub address: Option<String>,
}

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

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(InstallerState::default()));

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn MainLayout() -> Element {
    let _state = use_context::<Signal<InstallerState>>();
    let current_route = use_route::<Route>();

    let step_info = match current_route {
        Route::Welcome {} => ("Welcome", 0),
        Route::ServerSelection {} => ("Server Selection", 1),
        Route::StorageConfiguration {} => ("Storage", 2),
        Route::NetworkConfiguration {} => ("Network", 3),
        Route::SystemConfiguration {} => ("System", 4),
        Route::ReviewConfiguration {} => ("Review", 5),
        Route::Installation {} => ("Installation", 6),
    };

    rsx! {
        div { class: "app-container",
            header { class: "app-header",
                h1 { "illumos Installer" }
                div { class: "progress-indicator",
                    div { class: "step-counter",
                        "Step {step_info.1 + 1} of 7: {step_info.0}"
                    }
                    div { class: "progress-bar",
                        div {
                            class: "progress-fill",
                            style: "width: {(step_info.1 as f32 / 6.0 * 100.0)}%"
                        }
                    }
                }
            }

            main { class: "app-main",
                Outlet::<Route> {}
            }

            footer { class: "app-footer",
                NavigationButtons { current_step: step_info.1 }
            }
        }
    }
}

#[component]
fn NavigationButtons(current_step: usize) -> Element {
    let navigator = use_navigator();

    let (prev_route, next_route) = match current_step {
        0 => (None, Some(Route::ServerSelection {})),
        1 => (
            Some(Route::Welcome {}),
            Some(Route::StorageConfiguration {}),
        ),
        2 => (
            Some(Route::ServerSelection {}),
            Some(Route::NetworkConfiguration {}),
        ),
        3 => (
            Some(Route::StorageConfiguration {}),
            Some(Route::SystemConfiguration {}),
        ),
        4 => (
            Some(Route::NetworkConfiguration {}),
            Some(Route::ReviewConfiguration {}),
        ),
        5 => (
            Some(Route::SystemConfiguration {}),
            Some(Route::Installation {}),
        ),
        6 => (Some(Route::ReviewConfiguration {}), None),
        _ => (None, None),
    };

    rsx! {
        div { class: "navigation-buttons",
            if let Some(prev) = prev_route {
                button {
                    class: "nav-button prev",
                    onclick: move |_| { navigator.push(prev.clone()); },
                    "← Previous"
                }
            } else {
                div {}
            }

            if let Some(next) = next_route {
                button {
                    class: "nav-button next",
                    onclick: move |_| { navigator.push(next.clone()); },
                    if current_step == 5 { "Start Installation →" } else { "Next →" }
                }
            }
        }
    }
}

#[component]
fn Welcome() -> Element {
    rsx! {
        div { class: "page welcome-page",
            div { class: "welcome-content",
                h2 { "Welcome to the illumos Installer" }
                p {
                    "This installer will guide you through the process of installing illumos on your target machine. "
                    "You'll be able to configure storage, networking, and system settings before starting the installation."
                }

                div { class: "feature-list",
                    h3 { "What you can configure:" }
                    ul {
                        li { "🖥️ Select and claim a target machine" }
                        li { "💾 Configure ZFS storage pools and datasets" }
                        li { "🌐 Set up network interfaces and addressing" }
                        li { "⚙️ Configure system settings like hostname and DNS" }
                        li { "📦 Choose the illumos image to install" }
                    }
                }

                div { class: "getting-started",
                    h3 { "Getting Started" }
                    p { "Click 'Next' to begin by selecting a machine to install on." }
                }
            }
        }
    }
}

#[component]
fn ServerSelection() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load servers on component mount
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            match load_available_servers().await {
                Ok(servers) => {
                    state.write().server_list = servers;
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load servers: {}", e)));
                }
            }
            loading.set(false);
        });
    });

    rsx! {
        div { class: "page server-selection-page",
            h2 { "Select Target Machine" }
            p { "Choose a machine to install illumos on. Only available machines are shown." }

            if loading() {
                div { class: "loading", "Loading available machines..." }
            } else if let Some(err) = error() {
                div { class: "error", "{err}" }
                button {
                    onclick: move |_| {
                        error.set(None);
                        // Retry loading
                        spawn(async move {
                            loading.set(true);
                            match load_available_servers().await {
                                Ok(servers) => {
                                    state.write().server_list = servers;
                                }
                                Err(e) => {
                                    error.set(Some(format!("Failed to load servers: {}", e)));
                                }
                            }
                            loading.set(false);
                        });
                    },
                    "Retry"
                }
            } else {
                div { class: "server-grid",
                    for server in state.read().server_list.iter() {
                        ServerCard {
                            server: server.clone(),
                            selected: state.read().selected_server.as_ref() == Some(&server.id),
                            on_select: move |server_id: String| {
                                state.write().selected_server = Some(server_id);
                            }
                        }
                    }
                }

                if state.read().server_list.is_empty() {
                    div { class: "empty-state",
                        "No available machines found. Please ensure machined servers are running and accessible."
                    }
                }
            }
        }
    }
}

#[component]
fn ServerCard(server: MachineServer, selected: bool, on_select: EventHandler<String>) -> Element {
    let status_class = match server.status {
        ServerStatus::Available => "available",
        ServerStatus::Busy => "busy",
        ServerStatus::Offline => "offline",
        ServerStatus::Installing => "installing",
    };

    let disabled = !matches!(server.status, ServerStatus::Available);

    let card_class = if selected {
        format!("server-card {} selected", status_class)
    } else {
        format!("server-card {}", status_class)
    };

    rsx! {
        div {
            class: "{card_class}",
            onclick: move |_| {
                if !disabled {
                    on_select.call(server.id.clone());
                }
            },

            div { class: "server-header",
                h3 { "{server.name}" }
                span { class: "server-status", "{server.status:?}" }
            }

            div { class: "server-details",
                p { "Address: {server.address}" }
                div { class: "server-specs",
                    span { "CPU: {server.specs.cpu_cores} cores" }
                    span { "RAM: {server.specs.memory_gb} GB" }
                    span { "Storage: {server.specs.storage_gb} GB" }
                    span { "NICs: {server.specs.network_interfaces}" }
                }
            }
        }
    }
}

#[component]
fn StorageConfiguration() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();

    rsx! {
        div { class: "page storage-page",
            h2 { "Storage Configuration" }
            p { "Configure ZFS storage pools for your installation." }

            div { class: "storage-config",
                h3 { "Root Pool Configuration" }

                // Simple pool configuration for now
                div { class: "form-group",
                    label { "Pool Name:" }
                    input {
                        r#type: "text",
                        value: state.read().pools[0].name.clone(),
                        oninput: move |evt| {
                            state.write().pools[0].name = evt.value();
                        }
                    }
                }

                h3 { "Image Configuration" }
                div { class: "form-group",
                    label { "OCI Image:" }
                    input {
                        r#type: "text",
                        value: state.read().image.clone(),
                        oninput: move |evt| {
                            state.write().image = evt.value();
                        }
                    }
                }

                div { class: "form-group",
                    label { "Boot Environment Name (optional):" }
                    input {
                        r#type: "text",
                        value: state.read().boot_environment_name.as_deref().unwrap_or(""),
                        oninput: move |evt| {
                            let value = evt.value();
                            state.write().boot_environment_name = if value.is_empty() {
                                None
                            } else {
                                Some(value)
                            };
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn NetworkConfiguration() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();

    rsx! {
        div { class: "page network-page",
            h2 { "Network Configuration" }
            p { "Configure network interfaces for your installation." }

            div { class: "network-config",
                p { "Interfaces: {state.read().interfaces.len()}" }

                button {
                    class: "add-button",
                    onclick: move |_| {
                        let len = state.read().interfaces.len();
                        state.write().interfaces.push(NetworkInterface {
                            name: format!("net{}", len),
                            selector: None,
                            addresses: vec![],
                        });
                    },
                    "Add Network Interface"
                }
            }
        }
    }
}

#[component]
fn SystemConfiguration() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();

    rsx! {
        div { class: "page system-page",
            h2 { "System Configuration" }
            p { "Configure basic system settings." }

            div { class: "system-config",
                div { class: "form-group",
                    label { "Hostname:" }
                    input {
                        r#type: "text",
                        value: state.read().hostname.clone(),
                        oninput: move |evt| {
                            state.write().hostname = evt.value();
                        }
                    }
                }

                h3 { "DNS Servers" }
                p { "DNS: {state.read().nameservers.join(\", \")}" }

                button {
                    class: "add-button",
                    onclick: move |_| {
                        state.write().nameservers.push("8.8.8.8".to_string());
                    },
                    "Add DNS Server"
                }
            }
        }
    }
}

#[component]
fn ReviewConfiguration() -> Element {
    let state = use_context::<Signal<InstallerState>>();

    rsx! {
        div { class: "page review-page",
            h2 { "Review Configuration" }
            p { "Please review your configuration before starting the installation." }

            div { class: "config-review",
                div { class: "review-section",
                    h3 { "Target Server" }
                    if let Some(server_id) = &state.read().selected_server {
                        if let Some(server) = state.read().server_list.iter().find(|s| &s.id == server_id) {
                            p { "Server: {server.name} ({server.address})" }
                        }
                    } else {
                        p { class: "error", "No server selected!" }
                    }
                }

                div { class: "review-section",
                    h3 { "Storage Configuration" }
                    p { "Pool: {state.read().pools[0].name.clone()}" }
                    p { "Image: {state.read().image.clone()}" }
                }

                div { class: "review-section",
                    h3 { "Network Configuration" }
                    p { "Interfaces: {state.read().interfaces.len()}" }
                }

                div { class: "review-section",
                    h3 { "System Configuration" }
                    p { "Hostname: {state.read().hostname.clone()}" }
                    p { "DNS Servers: {state.read().nameservers.join(\", \")}" }
                }
            }
        }
    }
}

#[component]
fn Installation() -> Element {
    let state = use_context::<Signal<InstallerState>>();
    let mut installing = use_signal(|| false);
    let mut completed = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div { class: "page installation-page",
            h2 { "Installation" }

            if !installing() && !completed() && error().is_none() {
                div { class: "installation-ready",
                    p { "Ready to start installation. This process cannot be undone." }
                    button {
                        class: "install-button",
                        onclick: move |_| {
                            let state_clone = state.read().clone();
                            spawn(async move {
                                installing.set(true);
                                match perform_installation(state_clone).await {
                                    Ok(_) => {
                                        completed.set(true);
                                    }
                                    Err(e) => {
                                        error.set(Some(format!("Installation failed: {}", e)));
                                    }
                                }
                                installing.set(false);
                            });
                        },
                        "Start Installation"
                    }
                }
            }

            if installing() {
                div { class: "installation-progress",
                    h3 { "Installing..." }
                    div { class: "progress-bar",
                        div {
                            class: "progress-fill",
                            style: "width: {state.read().installation_progress}%"
                        }
                    }
                    div { class: "installation-log",
                        p { "Log entries: {state.read().installation_log.len()}" }
                    }
                }
            }

            if completed() {
                div { class: "installation-complete",
                    h3 { "Installation Complete!" }
                    p { "Your illumos system has been successfully installed." }
                    p { "The system will be ready to boot after restarting." }
                }
            }

            if let Some(err) = error() {
                div { class: "installation-error",
                    h3 { "Installation Failed" }
                    p { "{err}" }
                    button {
                        onclick: move |_| {
                            error.set(None);
                        },
                        "Retry"
                    }
                }
            }
        }
    }
}

// Server functions for communicating with machined servers
#[server(LoadAvailableServers)]
async fn load_available_servers() -> Result<Vec<MachineServer>, ServerFnError> {
    // This would connect to machined discovery service
    // For now, return mock data
    Ok(vec![
        MachineServer {
            id: "server-001".to_string(),
            name: "Machine 001".to_string(),
            address: "192.168.1.100".to_string(),
            status: ServerStatus::Available,
            specs: ServerSpecs {
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
            status: ServerStatus::Busy,
            specs: ServerSpecs {
                cpu_cores: 16,
                memory_gb: 64,
                storage_gb: 1000,
                network_interfaces: 4,
            },
        },
    ])
}

#[server(ClaimServer)]
async fn claim_server(server_id: String) -> Result<(), ServerFnError> {
    // This would send a claim request to the specific machined server
    // Implementation would use the instcomd client to claim the server
    log::info!("Claiming server: {}", server_id);
    Ok(())
}

#[server(PerformInstallation)]
async fn perform_installation(config: InstallerState) -> Result<(), ServerFnError> {
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
