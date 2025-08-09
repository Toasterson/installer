//! Installation page component for the Illumos installer
//!
//! This page manages the actual installation process, including progress monitoring,
//! log display, error handling, and completion status.

use crate::server::perform_installation;
use crate::state::InstallerState;
use dioxus::prelude::*;

/// Installation page component
#[component]
pub fn Installation() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();
    let mut installing = use_signal(|| false);
    let mut completed = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut installation_stage = use_signal(|| "Preparing".to_string());

    // Simulate progress updates during installation
    let mut update_progress = move |stage: &str, progress: f32| {
        installation_stage.set(stage.to_string());
        state.write().installation_progress = progress;
    };

    rsx! {
        div { class: "page installation-page",
            div { class: "page-header",
                h2 { "Installation" }
                p { class: "page-description",
                    if !installing() && !completed() && error().is_none() {
                        "Ready to begin the installation process. Once started, this cannot be undone."
                    } else if installing() {
                        "Installing illumos on your selected machine..."
                    } else if completed() {
                        "Installation completed successfully!"
                    } else {
                        "Installation encountered an error."
                    }
                }
            }

            div { class: "installation-content",
                // Pre-installation state
                if !installing() && !completed() && error().is_none() {
                    div { class: "installation-ready",
                        div { class: "ready-summary",
                            h3 { "Ready to Install" }
                            p {
                                "All configuration steps have been completed. The installation will now begin "
                                "on your selected target machine."
                            }
                        }

                        div { class: "installation-warning",
                            div { class: "warning-box critical",
                                i { class: "fas fa-exclamation-triangle" }
                                div { class: "warning-content",
                                    h4 { "Final Warning" }
                                    ul {
                                        li { "This will completely erase the target machine's current operating system" }
                                        li { "All data on the configured storage devices will be permanently lost" }
                                        li { "The installation process cannot be stopped once started" }
                                        li { "Network connectivity must remain stable throughout the process" }
                                    }
                                }
                            }
                        }

                        div { class: "installation-actions",
                            button {
                                class: "install-button primary large",
                                onclick: move |_| {
                                    let state_clone = state.read().clone();
                                    spawn(async move {
                                        installing.set(true);
                                        error.set(None);

                                        // Simulate installation stages
                                        update_progress("Claiming target machine", 5.0);
                                        // In real implementation, this would be actual server calls

                                        match perform_installation(state_clone).await {
                                            Ok(_) => {
                                                update_progress("Installation complete", 100.0);
                                                completed.set(true);
                                            }
                                            Err(e) => {
                                                error.set(Some(format!("Installation failed: {}", e)));
                                            }
                                        }
                                        installing.set(false);
                                    });
                                },
                                i { class: "fas fa-rocket" }
                                " Begin Installation"
                            }
                        }
                    }
                }

                // Installation in progress
                if installing() {
                    div { class: "installation-progress",
                        div { class: "progress-header",
                            h3 { "Installing illumos..." }
                            div { class: "installation-stage",
                                "{installation_stage()}"
                            }
                        }

                        div { class: "progress-container",
                            div { class: "progress-bar",
                                div {
                                    class: "progress-fill",
                                    style: "width: {state.read().installation_progress}%"
                                }
                                div { class: "progress-text",
                                    "{state.read().installation_progress:.1}%"
                                }
                            }
                        }

                        div { class: "installation-stages",
                            InstallationStages { current_progress: state.read().installation_progress }
                        }

                        div { class: "installation-log",
                            h4 { "Installation Log" }
                            div { class: "log-container",
                                if state.read().installation_log.is_empty() {
                                    div { class: "log-empty",
                                        "Waiting for installation to begin..."
                                    }
                                } else {
                                    for entry in &state.read().installation_log {
                                        div { class: "log-entry",
                                            span { class: "log-timestamp", "[{entry}]" }
                                            span { class: "log-message", "{entry}" }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "installation-warning active",
                            i { class: "fas fa-info-circle" }
                            "Do not close this window or disconnect from the network during installation."
                        }
                    }
                }

                // Installation completed successfully
                if completed() {
                    div { class: "installation-complete",
                        div { class: "success-icon",
                            i { class: "fas fa-check-circle" }
                        }

                        div { class: "completion-content",
                            h3 { "Installation Complete!" }
                            p {
                                "Your illumos system has been successfully installed and is ready to use. "
                                "The target machine will automatically reboot into the new system."
                            }

                            div { class: "completion-details",
                                h4 { "What happens next:" }
                                ul {
                                    li { "The target machine will reboot automatically" }
                                    li { "Initial system services will start" }
                                    li { "Network configuration will be applied" }
                                    li { "You can access the system via SSH or console" }
                                }
                            }

                            div { class: "post-installation-info",
                                h4 { "Access Information:" }
                                div { class: "info-grid",
                                    div { class: "info-item",
                                        span { class: "info-label", "Hostname:" }
                                        span { class: "info-value", "{state.read().hostname}" }
                                    }
                                    if let Some(server_id) = &state.read().selected_server {
                                        if let Some(server) = state.read().server_list.iter().find(|s| &s.id == server_id) {
                                            div { class: "info-item",
                                                span { class: "info-label", "IP Address:" }
                                                span { class: "info-value", "{server.address}" }
                                            }
                                        }
                                    }
                                    div { class: "info-item",
                                        span { class: "info-label", "SSH Access:" }
                                        span { class: "info-value", "ssh root@{state.read().hostname}" }
                                    }
                                }
                            }
                        }

                        div { class: "completion-actions",
                            button {
                                class: "action-button primary",
                                onclick: move |_| {
                                    // In a real implementation, this might navigate to a management interface
                                    // or close the installer
                                },
                                i { class: "fas fa-external-link-alt" }
                                " Access System"
                            }
                            button {
                                class: "action-button secondary",
                                onclick: move |_| {
                                    // Reset for another installation
                                    *state.write() = InstallerState::default();
                                    installing.set(false);
                                    completed.set(false);
                                    error.set(None);
                                },
                                i { class: "fas fa-plus" }
                                " Install Another System"
                            }
                        }
                    }
                }

                // Installation error state
                if let Some(err) = error() {
                    div { class: "installation-error",
                        div { class: "error-icon",
                            i { class: "fas fa-exclamation-circle" }
                        }

                        div { class: "error-content",
                            h3 { "Installation Failed" }
                            div { class: "error-message",
                                "{err}"
                            }

                            div { class: "error-details",
                                h4 { "Possible causes:" }
                                ul {
                                    li { "Network connection was lost during installation" }
                                    li { "Target machine became unavailable" }
                                    li { "Insufficient storage space or disk errors" }
                                    li { "Invalid configuration parameters" }
                                }
                            }

                            div { class: "error-suggestions",
                                h4 { "Next steps:" }
                                ul {
                                    li { "Check network connectivity to the target machine" }
                                    li { "Verify the target machine is still running and accessible" }
                                    li { "Review the installation log for specific error details" }
                                    li { "Retry the installation or modify configuration as needed" }
                                }
                            }
                        }

                        div { class: "error-actions",
                            button {
                                class: "action-button primary",
                                onclick: move |_| {
                                    error.set(None);
                                    // Reset progress
                                    state.write().installation_progress = 0.0;
                                    state.write().installation_log.clear();
                                },
                                i { class: "fas fa-redo" }
                                " Retry Installation"
                            }
                            button {
                                class: "action-button secondary",
                                onclick: move |_| {
                                    // Navigate back to review to modify configuration
                                    error.set(None);
                                    let navigator = use_navigator();
                                    navigator.push(crate::routes::Route::ReviewConfiguration {});
                                },
                                i { class: "fas fa-edit" }
                                " Modify Configuration"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Installation stages progress indicator
#[component]
pub fn InstallationStages(current_progress: f32) -> Element {
    let stages = vec![
        ("Claiming Machine", 0.0, 10.0),
        ("Preparing Storage", 10.0, 25.0),
        ("Installing Base System", 25.0, 60.0),
        ("Configuring Network", 60.0, 75.0),
        ("Setting up Services", 75.0, 90.0),
        ("Finalizing Installation", 90.0, 100.0),
    ];

    rsx! {
        div { class: "installation-stages",
            for (stage_name, start_progress, end_progress) in stages {
                div {
                    class: format!("{}", if current_progress >= end_progress {
                        "stage-item completed"
                    } else if current_progress >= start_progress {
                        "stage-item active"
                    } else {
                        "stage-item pending"
                    }),

                    div { class: "stage-icon",
                        if current_progress >= end_progress {
                            i { class: "fas fa-check-circle" }
                        } else if current_progress >= start_progress {
                            i { class: "fas fa-spinner fa-spin" }
                        } else {
                            i { class: "fas fa-circle" }
                        }
                    }
                    div { class: "stage-info",
                        h5 { "{stage_name}" }
                        div { class: "stage-progress",
                            if current_progress >= end_progress {
                                "Completed"
                            } else if current_progress >= start_progress {
                                "In Progress..."
                            } else {
                                "Pending"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Live log viewer component
#[component]
pub fn LiveLogViewer() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();
    let mut auto_scroll = use_signal(|| true);

    rsx! {
        div { class: "live-log-viewer",
            div { class: "log-header",
                h4 { "Installation Log" }
                div { class: "log-controls",
                    label {
                        input {
                            r#type: "checkbox",
                            checked: auto_scroll(),
                            onchange: move |evt| auto_scroll.set(evt.checked())
                        }
                        " Auto-scroll"
                    }
                    button {
                        class: "log-control-button",
                        onclick: move |_| {
                            state.write().installation_log.clear();
                        },
                        i { class: "fas fa-trash" }
                        " Clear"
                    }
                }
            }

            div {
                class: "log-content",
                id: "log-content",

                if state.read().installation_log.is_empty() {
                    div { class: "log-placeholder",
                        "Installation log will appear here..."
                    }
                } else {
                    for (index, entry) in state.read().installation_log.iter().enumerate() {
                        div {
                            class: "log-line",
                            key: "{index}",
                            span { class: "log-index", "[{index + 1:03}]" }
                            span { class: "log-text", "{entry}" }
                        }
                    }
                }
            }
        }
    }
}
