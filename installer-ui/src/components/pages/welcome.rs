//! Welcome page component for the Illumos installer
//!
//! This is the first page users see when starting the installer.
//! It provides an overview of the installation process and steps.

use dioxus::prelude::*;

/// Welcome page component that introduces the installer
#[component]
pub fn Welcome() -> Element {
    rsx! {
        div { class: "page welcome-page",
            div { class: "welcome-content",
                div { class: "welcome-header",
                    h2 { "Welcome to the illumos Installer" }
                    p { class: "welcome-subtitle",
                        "This installer will guide you through installing illumos on your selected machine."
                    }
                }

                div { class: "installation-overview",
                    h3 { "Installation Process Overview" }
                    p {
                        "The installation process consists of several configuration steps that will prepare "
                        "your system for illumos deployment. Each step builds upon the previous one to create "
                        "a complete system configuration."
                    }
                }

                div { class: "feature-list",
                    h3 { "Configuration Steps:" }
                    ul { class: "step-list",
                        li { class: "step-item",
                            div { class: "step-icon", i { class: "fas fa-server" } }
                            div { class: "step-content",
                                strong { "Select Target Machine" }
                                p { "Choose from available machines discovered on your network" }
                            }
                        }
                        li { class: "step-item",
                            div { class: "step-icon", i { class: "fas fa-hdd" } }
                            div { class: "step-content",
                                strong { "Configure ZFS Storage" }
                                p { "Set up storage pools, vdevs, and disk configuration" }
                            }
                        }
                        li { class: "step-item",
                            div { class: "step-icon", i { class: "fas fa-network-wired" } }
                            div { class: "step-content",
                                strong { "Network Configuration" }
                                p { "Configure network interfaces, IP addresses, and DNS settings" }
                            }
                        }
                        li { class: "step-item",
                            div { class: "step-icon", i { class: "fas fa-cog" } }
                            div { class: "step-content",
                                strong { "System Settings" }
                                p { "Set hostname, choose OS image, and configure boot environment" }
                            }
                        }
                        li { class: "step-item",
                            div { class: "step-icon", i { class: "fas fa-check-circle" } }
                            div { class: "step-content",
                                strong { "Review & Install" }
                                p { "Review your configuration and start the installation process" }
                            }
                        }
                    }
                }

                div { class: "requirements-section",
                    h3 { "Before You Begin" }
                    div { class: "requirements-grid",
                        div { class: "requirement-card",
                            i { class: "fas fa-exclamation-triangle warning-icon" }
                            div { class: "requirement-content",
                                strong { "Important Notice" }
                                p { "This installation will completely replace the existing operating system on the target machine." }
                            }
                        }
                        div { class: "requirement-card",
                            i { class: "fas fa-network-wired info-icon" }
                            div { class: "requirement-content",
                                strong { "Network Access" }
                                p { "Ensure your target machines are discoverable on the network and running machined." }
                            }
                        }
                        div { class: "requirement-card",
                            i { class: "fas fa-save info-icon" }
                            div { class: "requirement-content",
                                strong { "Data Backup" }
                                p { "Back up any important data before proceeding with the installation." }
                            }
                        }
                    }
                }

                div { class: "getting-started",
                    div { class: "getting-started-content",
                        h3 { "Ready to Get Started?" }
                        p {
                            "Click 'Next' to begin by selecting a target machine from the available "
                            "systems on your network."
                        }
                    }
                    div { class: "start-button-container",
                        div { class: "start-hint",
                            i { class: "fas fa-arrow-right" }
                            " Use the Next button below to continue"
                        }
                    }
                }
            }
        }
    }
}

/// Info card component for displaying helpful information
#[component]
pub fn InfoCard(title: String, content: String, icon: String) -> Element {
    rsx! {
        div { class: "info-card",
            div { class: "info-icon",
                i { class: "{icon}" }
            }
            div { class: "info-content",
                h4 { "{title}" }
                p { "{content}" }
            }
        }
    }
}
