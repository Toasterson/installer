//! System configuration page component for the Illumos installer
//!
//! This page allows users to configure system-level settings such as
//! hostname, time zone, locale, and other OS-specific configurations.

use crate::state::InstallerState;
use dioxus::prelude::*;

/// System configuration page component
#[component]
pub fn SystemConfiguration() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();
    let mut selected_timezone = use_signal(|| "UTC".to_string());
    let mut selected_locale = use_signal(|| "en_US.UTF-8".to_string());
    let mut enable_ssh = use_signal(|| true);
    let mut root_password = use_signal(|| "".to_string());
    let mut confirm_password = use_signal(|| "".to_string());

    let password_match = root_password() == confirm_password() && !root_password().is_empty();
    let hostname_valid = !state.read().hostname.trim().is_empty();

    rsx! {
        div { class: "page system-page",
            div { class: "page-header",
                h2 { "System Configuration" }
                p { class: "page-description",
                    "Configure system settings including hostname, timezone, locale, and security options."
                }
            }

            div { class: "system-config",
                div { class: "config-section",
                    h3 { "Basic System Settings" }

                    div { class: "form-group",
                        label { "Hostname:" }
                        input {
                            r#type: "text",
                            class: if hostname_valid { "form-control" } else { "form-control invalid" },
                            value: state.read().hostname.clone(),
                            oninput: move |evt| {
                                state.write().hostname = evt.value();
                            },
                            placeholder: "my-illumos-system"
                        }
                        if !hostname_valid {
                            div { class: "validation-error",
                                i { class: "fas fa-exclamation-triangle" }
                                " Hostname is required"
                            }
                        }
                    }

                    div { class: "form-group",
                        label { "System Image:" }
                        input {
                            r#type: "text",
                            class: "form-control",
                            value: state.read().image.clone(),
                            oninput: move |evt| {
                                state.write().image = evt.value();
                            },
                            placeholder: "oci://registry/image:tag"
                        }
                        div { class: "form-help",
                            "Specify the OCI image to install. Default images are available from the illumos project."
                        }
                    }

                    div { class: "form-group",
                        label { "Boot Environment Name (optional):" }
                        input {
                            r#type: "text",
                            class: "form-control",
                            value: state.read().boot_environment_name.as_deref().unwrap_or(""),
                            oninput: move |evt| {
                                let value = evt.value();
                                state.write().boot_environment_name = if value.is_empty() {
                                    None
                                } else {
                                    Some(value)
                                };
                            },
                            placeholder: "initial-install"
                        }
                        div { class: "form-help",
                            "Optional name for the boot environment. If not specified, a default name will be used."
                        }
                    }
                }

                div { class: "config-section",
                    h3 { "Regional Settings" }

                    div { class: "form-group",
                        label { "Timezone:" }
                        select {
                            class: "form-control",
                            value: "{selected_timezone}",
                            onchange: move |evt| {
                                selected_timezone.set(evt.value());
                            },

                            option { value: "UTC", "UTC" }
                            option { value: "America/New_York", "America/New_York" }
                            option { value: "America/Chicago", "America/Chicago" }
                            option { value: "America/Denver", "America/Denver" }
                            option { value: "America/Los_Angeles", "America/Los_Angeles" }
                            option { value: "Europe/London", "Europe/London" }
                            option { value: "Europe/Berlin", "Europe/Berlin" }
                            option { value: "Asia/Tokyo", "Asia/Tokyo" }
                            option { value: "Asia/Shanghai", "Asia/Shanghai" }
                            option { value: "Australia/Sydney", "Australia/Sydney" }
                        }
                    }

                    div { class: "form-group",
                        label { "Locale:" }
                        select {
                            class: "form-control",
                            value: "{selected_locale}",
                            onchange: move |evt| {
                                selected_locale.set(evt.value());
                            },

                            option { value: "en_US.UTF-8", "English (US)" }
                            option { value: "en_GB.UTF-8", "English (UK)" }
                            option { value: "de_DE.UTF-8", "German" }
                            option { value: "fr_FR.UTF-8", "French" }
                            option { value: "es_ES.UTF-8", "Spanish" }
                            option { value: "ja_JP.UTF-8", "Japanese" }
                            option { value: "zh_CN.UTF-8", "Chinese (Simplified)" }
                        }
                    }
                }

                div { class: "config-section",
                    h3 { "Security Settings" }

                    div { class: "form-group checkbox-group",
                        input {
                            r#type: "checkbox",
                            id: "enable-ssh",
                            checked: enable_ssh(),
                            onchange: move |evt| {
                                enable_ssh.set(evt.checked());
                            }
                        }
                        label { r#for: "enable-ssh", "Enable SSH service" }
                        div { class: "form-help",
                            "Allow remote SSH access to the system after installation."
                        }
                    }

                    div { class: "form-group",
                        label { "Root Password:" }
                        input {
                            r#type: "password",
                            class: if password_match || root_password().is_empty() { "form-control" } else { "form-control invalid" },
                            value: root_password(),
                            oninput: move |evt| {
                                root_password.set(evt.value());
                            },
                            placeholder: "Enter root password"
                        }
                    }

                    div { class: "form-group",
                        label { "Confirm Password:" }
                        input {
                            r#type: "password",
                            class: if password_match || confirm_password().is_empty() { "form-control" } else { "form-control invalid" },
                            value: confirm_password(),
                            oninput: move |evt| {
                                confirm_password.set(evt.value());
                            },
                            placeholder: "Confirm root password"
                        }
                        if !root_password().is_empty() && !password_match {
                            div { class: "validation-error",
                                i { class: "fas fa-exclamation-triangle" }
                                " Passwords do not match"
                            }
                        }
                    }

                    if password_match && !root_password().is_empty() {
                        div { class: "validation-success",
                            i { class: "fas fa-check-circle" }
                            " Password confirmed"
                        }
                    }
                }

                div { class: "config-section",
                    h3 { "Additional Options" }

                    div { class: "form-group",
                        label { "Installation Notes:" }
                        textarea {
                            class: "form-control",
                            rows: "3",
                            placeholder: "Optional notes about this installation...",
                            // Note: In a real implementation, you might want to store this in state
                        }
                        div { class: "form-help",
                            "Optional notes that will be saved with the installation configuration."
                        }
                    }
                }

                SystemValidationSummary {
                    hostname_valid: hostname_valid,
                    image_configured: !state.read().image.is_empty(),
                    timezone: selected_timezone(),
                    locale: selected_locale(),
                    ssh_enabled: enable_ssh(),
                    password_configured: password_match && !root_password().is_empty()
                }
            }
        }
    }
}

/// System configuration validation summary component
#[component]
pub fn SystemValidationSummary(
    hostname_valid: bool,
    image_configured: bool,
    timezone: String,
    locale: String,
    ssh_enabled: bool,
    password_configured: bool,
) -> Element {
    let all_valid = hostname_valid && image_configured && password_configured;

    rsx! {
        div { class: "system-validation",
            h4 { "System Configuration Summary" }
            div { class: "validation-grid",
                div { class: "validation-item",
                    span { class: "validation-label", "Hostname:" }
                    span {
                        class: if hostname_valid { "validation-value valid" } else { "validation-value invalid" },
                        if hostname_valid { "Configured" } else { "Required" }
                    }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "System Image:" }
                    span {
                        class: if image_configured { "validation-value valid" } else { "validation-value invalid" },
                        if image_configured { "Configured" } else { "Required" }
                    }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "Timezone:" }
                    span { class: "validation-value", "{timezone}" }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "Locale:" }
                    span { class: "validation-value", "{locale}" }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "SSH Service:" }
                    span { class: "validation-value",
                        if ssh_enabled { "Enabled" } else { "Disabled" }
                    }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "Root Password:" }
                    span {
                        class: if password_configured { "validation-value valid" } else { "validation-value invalid" },
                        if password_configured { "Configured" } else { "Required" }
                    }
                }
            }

            if all_valid {
                div { class: "validation-success-banner",
                    i { class: "fas fa-check-circle" }
                    " System configuration is complete and valid"
                }
            } else {
                div { class: "validation-warning-banner",
                    i { class: "fas fa-exclamation-triangle" }
                    " Please complete all required system settings"
                }
            }
        }
    }
}

/// Timezone selector component with search capability
#[component]
pub fn TimezoneSelector(current_timezone: String, on_change: EventHandler<String>) -> Element {
    let mut search_term = use_signal(|| "".to_string());
    let mut show_dropdown = use_signal(|| false);

    let common_timezones = vec![
        ("UTC", "Coordinated Universal Time"),
        ("America/New_York", "Eastern Time (US)"),
        ("America/Chicago", "Central Time (US)"),
        ("America/Denver", "Mountain Time (US)"),
        ("America/Los_Angeles", "Pacific Time (US)"),
        ("Europe/London", "Greenwich Mean Time"),
        ("Europe/Berlin", "Central European Time"),
        ("Asia/Tokyo", "Japan Standard Time"),
        ("Asia/Shanghai", "China Standard Time"),
        ("Australia/Sydney", "Australian Eastern Time"),
    ];

    let filtered_timezones = if search_term().is_empty() {
        common_timezones.clone()
    } else {
        common_timezones
            .into_iter()
            .filter(|(tz, desc)| {
                tz.to_lowercase().contains(&search_term().to_lowercase())
                    || desc.to_lowercase().contains(&search_term().to_lowercase())
            })
            .collect()
    };

    rsx! {
        div { class: "timezone-selector",
            div { class: "timezone-input-group",
                input {
                    r#type: "text",
                    class: "form-control",
                    value: current_timezone,
                    oninput: move |evt| {
                        search_term.set(evt.value());
                        show_dropdown.set(true);
                    },
                    onfocus: move |_| show_dropdown.set(true),
                    placeholder: "Search timezones..."
                }
                button {
                    class: "timezone-dropdown-toggle",
                    onclick: move |_| show_dropdown.set(!show_dropdown()),
                    i { class: "fas fa-chevron-down" }
                }
            }

            if show_dropdown() {
                div { class: "timezone-dropdown",
                    for (timezone, description) in filtered_timezones {
                        div {
                            class: "timezone-option",
                            onclick: move |_| {
                                on_change.call(timezone.to_string());
                                show_dropdown.set(false);
                            },

                            div { class: "timezone-name", "{timezone}" }
                            div { class: "timezone-description", "{description}" }
                        }
                    }
                }
            }
        }
    }
}
