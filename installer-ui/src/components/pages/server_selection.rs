//! Server selection page component for the Illumos installer
//!
//! This page allows users to discover and select target machines for installation.
//! It displays available servers with their specifications and status.

use crate::server::load_available_servers;
use crate::state::{InstallerState, MachineServer, ServerStatus};
use dioxus::prelude::*;

/// Server selection page component
#[component]
pub fn ServerSelection() -> Element {
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

    let retry_loading = move |_| {
        error.set(None);
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
    };

    rsx! {
        div { class: "page server-selection-page",
            div { class: "page-header",
                h2 { "Select Target Machine" }
                p { class: "page-description",
                    "Choose a machine to install illumos on. Only available machines are shown below."
                }
            }

            if loading() {
                div { class: "loading-container",
                    div { class: "loading-spinner" }
                    p { "Discovering available machines..." }
                }
            } else if let Some(err) = error() {
                div { class: "error-container",
                    div { class: "error-icon",
                        i { class: "fas fa-exclamation-triangle" }
                    }
                    div { class: "error-content",
                        h3 { "Discovery Failed" }
                        p { "{err}" }
                        button {
                            class: "retry-button",
                            onclick: retry_loading,
                            i { class: "fas fa-redo" }
                            " Retry Discovery"
                        }
                    }
                }
            } else {
                div { class: "server-selection-content",
                    if !state.read().server_list.is_empty() {
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
                    } else {
                        EmptyServerState { on_retry: retry_loading }
                    }

                    if state.read().selected_server.is_some() {
                        div { class: "selection-summary",
                            i { class: "fas fa-check-circle" }
                            " Machine selected. Click 'Next' to configure storage."
                        }
                    }
                }
            }
        }
    }
}

/// Individual server card component
#[component]
pub fn ServerCard(
    server: MachineServer,
    selected: bool,
    on_select: EventHandler<String>,
) -> Element {
    let status_class = server.status.css_class();
    let disabled = !server.status.is_available();

    let card_class = if selected {
        format!("server-card {} selected", status_class)
    } else if disabled {
        format!("server-card {} disabled", status_class)
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
                h3 { class: "server-name", "{server.name}" }
                ServerStatusBadge { status: server.status.clone() }
            }

            div { class: "server-info",
                div { class: "server-address",
                    i { class: "fas fa-network-wired" }
                    span { "{server.address}" }
                }
                div { class: "server-id",
                    i { class: "fas fa-tag" }
                    span { "{server.id}" }
                }
            }

            div { class: "server-specs",
                div { class: "spec-item",
                    i { class: "fas fa-microchip" }
                    span { "{server.specs.cpu_cores} CPU cores" }
                }
                div { class: "spec-item",
                    i { class: "fas fa-memory" }
                    span { "{server.specs.memory_gb} GB RAM" }
                }
                div { class: "spec-item",
                    i { class: "fas fa-hdd" }
                    span { "{server.specs.storage_gb} GB Storage" }
                }
                div { class: "spec-item",
                    i { class: "fas fa-ethernet" }
                    span { "{server.specs.network_interfaces} Network interfaces" }
                }
            }

            if selected {
                div { class: "selection-indicator",
                    i { class: "fas fa-check-circle" }
                    span { "Selected" }
                }
            }

            if disabled {
                div { class: "disabled-overlay",
                    div { class: "disabled-message",
                        "Not available for installation"
                    }
                }
            }
        }
    }
}

/// Server status badge component
#[component]
pub fn ServerStatusBadge(status: ServerStatus) -> Element {
    let status_class = format!("status-badge {}", status.css_class());
    let status_text = status.as_str();
    let status_icon = match status {
        ServerStatus::Available => "fas fa-check-circle",
        ServerStatus::Busy => "fas fa-clock",
        ServerStatus::Offline => "fas fa-times-circle",
        ServerStatus::Installing => "fas fa-download",
    };

    rsx! {
        div { class: "{status_class}",
            i { class: "{status_icon}" }
            span { "{status_text}" }
        }
    }
}

/// Empty state component when no servers are found
#[component]
pub fn EmptyServerState(on_retry: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "empty-state",
            div { class: "empty-state-icon",
                i { class: "fas fa-server" }
            }
            div { class: "empty-state-content",
                h3 { "No Machines Found" }
                p {
                    "No available machines were discovered on your network. "
                    "Please ensure that:"
                }
                ul { class: "troubleshooting-list",
                    li { "Target machines are powered on and connected to the network" }
                    li { "machined service is running on target machines" }
                    li { "Network connectivity allows discovery packets" }
                    li { "Machines are not currently busy with other installations" }
                }
                button {
                    class: "retry-button primary",
                    onclick: on_retry,
                    i { class: "fas fa-search" }
                    " Search Again"
                }
            }
        }
    }
}

/// Server discovery help component
#[component]
pub fn ServerDiscoveryHelp() -> Element {
    rsx! {
        div { class: "discovery-help",
            details {
                summary { "Having trouble finding machines?" }
                div { class: "help-content",
                    h4 { "Troubleshooting Discovery" }
                    div { class: "help-section",
                        h5 { "Check Network Connectivity" }
                        p { "Ensure this installer and target machines are on the same network segment." }
                    }
                    div { class: "help-section",
                        h5 { "Verify machined Service" }
                        p { "Target machines must be running the machined discovery service." }
                        code { "svcs machined" }
                    }
                    div { class: "help-section",
                        h5 { "Firewall Configuration" }
                        p { "Check that discovery packets are not blocked by firewalls." }
                    }
                }
            }
        }
    }
}
