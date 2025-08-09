//! Review configuration page component for the Illumos installer
//!
//! This page displays a comprehensive summary of all configuration choices
//! made throughout the installation process, allowing users to review and
//! confirm their settings before starting the installation.

use crate::state::InstallerState;
use dioxus::prelude::*;

/// Review configuration page component
#[component]
pub fn ReviewConfiguration() -> Element {
    let state = use_context::<Signal<InstallerState>>();

    // Calculate validation status
    let server_selected = state.read().selected_server.is_some();
    let storage_configured =
        !state.read().pools.is_empty() && state.read().pools.iter().all(|p| !p.vdevs.is_empty());
    let network_configured = !state.read().hostname.trim().is_empty();
    let all_valid = server_selected && storage_configured && network_configured;

    rsx! {
        div { class: "page review-page",
            div { class: "page-header",
                h2 { "Review Configuration" }
                p { class: "page-description",
                    "Review all your configuration settings before starting the installation. "
                    "Once installation begins, these settings cannot be changed."
                }
            }

            div { class: "config-review",
                // Overall status banner
                div { class: "review-status-banner",
                    if all_valid {
                        div { class: "status-banner success",
                            i { class: "fas fa-check-circle" }
                            "Configuration is complete and ready for installation"
                        }
                    } else {
                        div { class: "status-banner warning",
                            i { class: "fas fa-exclamation-triangle" }
                            "Please complete all required configuration steps before proceeding"
                        }
                    }
                }

                // Server selection review
                div { class: "review-section",
                    div { class: "review-section-header",
                        h3 {
                            i { class: "fas fa-server" }
                            "Target Machine"
                        }
                        if server_selected {
                            div { class: "status-indicator success",
                                i { class: "fas fa-check" }
                            }
                        } else {
                            div { class: "status-indicator error",
                                i { class: "fas fa-times" }
                            }
                        }
                    }

                    div { class: "review-content",
                        if let Some(server_id) = &state.read().selected_server {
                            if let Some(server) = state.read().server_list.iter().find(|s| &s.id == server_id) {
                                div { class: "server-details",
                                    div { class: "detail-item",
                                        span { class: "detail-label", "Machine:" }
                                        span { class: "detail-value", "{server.name}" }
                                    }
                                    div { class: "detail-item",
                                        span { class: "detail-label", "Address:" }
                                        span { class: "detail-value", "{server.address}" }
                                    }
                                    div { class: "detail-item",
                                        span { class: "detail-label", "Status:" }
                                        span { class: "detail-value status-{server.status.css_class()}",
                                            "{server.status.as_str()}"
                                        }
                                    }
                                    div { class: "detail-item",
                                        span { class: "detail-label", "Specifications:" }
                                        span { class: "detail-value",
                                            "{server.specs.cpu_cores} cores, "
                                            "{server.specs.memory_gb} GB RAM, "
                                            "{server.specs.storage_gb} GB storage"
                                        }
                                    }
                                }
                            } else {
                                div { class: "error-message",
                                    "Selected server not found in server list"
                                }
                            }
                        } else {
                            div { class: "missing-config",
                                i { class: "fas fa-exclamation-triangle" }
                                "No target machine selected"
                            }
                        }
                    }
                }

                // Storage configuration review
                div { class: "review-section",
                    div { class: "review-section-header",
                        h3 {
                            i { class: "fas fa-hdd" }
                            "Storage Configuration"
                        }
                        if storage_configured {
                            div { class: "status-indicator success",
                                i { class: "fas fa-check" }
                            }
                        } else {
                            div { class: "status-indicator error",
                                i { class: "fas fa-times" }
                            }
                        }
                    }

                    div { class: "review-content",
                        if !state.read().pools.is_empty() {
                            for pool in &state.read().pools {
                                div { class: "pool-summary",
                                    div { class: "detail-item",
                                        span { class: "detail-label", "Pool Name:" }
                                        span { class: "detail-value", "{pool.name}" }
                                    }
                                    div { class: "detail-item",
                                        span { class: "detail-label", "Virtual Devices:" }
                                        span { class: "detail-value", "{pool.vdevs.len()}" }
                                    }
                                    if !pool.vdevs.is_empty() {
                                        div { class: "vdev-list",
                                            for (_i, vdev) in pool.vdevs.iter().enumerate() {
                                                div { class: "vdev-item",
                                                    span { class: "vdev-type", "{vdev.kind.display_name()}" }
                                                    span { class: "vdev-disks", "({vdev.disks.len()} disks)" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "missing-config",
                                i { class: "fas fa-exclamation-triangle" }
                                "No storage pools configured"
                            }
                        }

                        div { class: "image-config",
                            div { class: "detail-item",
                                span { class: "detail-label", "System Image:" }
                                span { class: "detail-value", "{state.read().image}" }
                            }
                            if let Some(be_name) = &state.read().boot_environment_name {
                                div { class: "detail-item",
                                    span { class: "detail-label", "Boot Environment:" }
                                    span { class: "detail-value", "{be_name}" }
                                }
                            }
                        }
                    }
                }

                // Network configuration review
                div { class: "review-section",
                    div { class: "review-section-header",
                        h3 {
                            i { class: "fas fa-network-wired" }
                            "Network Configuration"
                        }
                        if network_configured {
                            div { class: "status-indicator success",
                                i { class: "fas fa-check" }
                            }
                        } else {
                            div { class: "status-indicator error",
                                i { class: "fas fa-times" }
                            }
                        }
                    }

                    div { class: "review-content",
                        div { class: "detail-item",
                            span { class: "detail-label", "Hostname:" }
                            span { class: "detail-value",
                                if !state.read().hostname.trim().is_empty() {
                                    "{state.read().hostname}"
                                } else {
                                    span { class: "missing", "Not configured" }
                                }
                            }
                        }

                        div { class: "detail-item",
                            span { class: "detail-label", "DNS Servers:" }
                            span { class: "detail-value",
                                if !state.read().nameservers.is_empty() {
                                    "{state.read().nameservers.join(\", \")}"
                                } else {
                                    span { class: "missing", "None configured" }
                                }
                            }
                        }

                        div { class: "detail-item",
                            span { class: "detail-label", "Network Interfaces:" }
                            span { class: "detail-value", "{state.read().interfaces.len()}" }
                        }

                        if !state.read().interfaces.is_empty() {
                            div { class: "interface-list",
                                for interface in &state.read().interfaces {
                                    div { class: "interface-summary",
                                        div { class: "interface-name", "{interface.name}" }
                                        div { class: "interface-details",
                                            if let Some(selector) = &interface.selector {
                                                span { class: "interface-selector", "Selector: {selector}" }
                                            }
                                            span { class: "address-count",
                                                "{interface.addresses.len()} address(es)"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Configuration export/import section
                div { class: "review-section",
                    div { class: "review-section-header",
                        h3 {
                            i { class: "fas fa-file-export" }
                            "Configuration Export"
                        }
                    }

                    div { class: "review-content",
                        p { "You can export this configuration for backup or reuse:" }
                        div { class: "export-actions",
                            button { class: "export-button",
                                i { class: "fas fa-download" }
                                " Export as JSON"
                            }
                            button { class: "export-button",
                                i { class: "fas fa-file-code" }
                                " Export as KDL"
                            }
                        }
                    }
                }

                // Warning about installation
                div { class: "review-section warning-section",
                    div { class: "review-section-header",
                        h3 {
                            i { class: "fas fa-exclamation-triangle" }
                            "Important Notice"
                        }
                    }

                    div { class: "review-content",
                        div { class: "warning-box",
                            ul {
                                li { "This installation will completely replace the existing operating system on the target machine." }
                                li { "All data on the configured storage devices will be permanently lost." }
                                li { "The installation process cannot be undone once started." }
                                li { "Ensure you have backed up any important data before proceeding." }
                            }
                        }
                    }
                }

                // Final validation summary
                div { class: "review-section validation-section",
                    div { class: "review-section-header",
                        h3 {
                            i { class: "fas fa-clipboard-check" }
                            "Pre-Installation Checklist"
                        }
                    }

                    div { class: "review-content",
                        div { class: "checklist",
                            div { class: if server_selected { "checklist-item valid" } else { "checklist-item invalid" },
                                i { class: if server_selected { "fas fa-check-circle" } else { "fas fa-times-circle" } }
                                "Target machine selected and available"
                            }
                            div { class: if storage_configured { "checklist-item valid" } else { "checklist-item invalid" },
                                i { class: if storage_configured { "fas fa-check-circle" } else { "fas fa-times-circle" } }
                                "Storage pools and virtual devices configured"
                            }
                            div { class: if network_configured { "checklist-item valid" } else { "checklist-item invalid" },
                                i { class: if network_configured { "fas fa-check-circle" } else { "fas fa-times-circle" } }
                                "Network settings configured (hostname required)"
                            }
                            div { class: if !state.read().image.is_empty() { "checklist-item valid" } else { "checklist-item invalid" },
                                i { class: if !state.read().image.is_empty() { "fas fa-check-circle" } else { "fas fa-times-circle" } }
                                "System image specified"
                            }
                        }

                        if all_valid {
                            div { class: "ready-banner",
                                i { class: "fas fa-rocket" }
                                "Ready to begin installation! Click 'Start Installation' to proceed."
                            }
                        } else {
                            div { class: "not-ready-banner",
                                i { class: "fas fa-exclamation-circle" }
                                "Please complete all required configuration steps before starting installation."
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Configuration summary card component
#[component]
pub fn ConfigSummaryCard(title: String, icon: String, valid: bool, children: Element) -> Element {
    rsx! {
        div { class: "config-summary-card",
            div { class: "card-header",
                i { class: "{icon}" }
                h4 { "{title}" }
                div { class: if valid { "status-indicator success" } else { "status-indicator error" },
                    i { class: if valid { "fas fa-check" } else { "fas fa-times" } }
                }
            }
            div { class: "card-content",
                {children}
            }
        }
    }
}
