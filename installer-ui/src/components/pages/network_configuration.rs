//! Network configuration page component for the Illumos installer
//!
//! This page allows users to configure network interfaces, IP addresses,
//! DNS settings, and other network-related options for the installation.

use crate::state::{AddressKind, AddressObject, InstallerState, NetworkInterface};
use dioxus::prelude::*;

/// Network configuration page component
#[component]
pub fn NetworkConfiguration() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();
    let mut selected_interface_index = use_signal(|| 0usize);

    rsx! {
        div { class: "page network-page",
            div { class: "page-header",
                h2 { "Network Configuration" }
                p { class: "page-description",
                    "Configure network interfaces, IP addresses, and DNS settings for your system."
                }
            }

            div { class: "network-config",
                div { class: "network-sidebar",
                    div { class: "interface-list",
                        h3 { "Network Interfaces" }
                        for (index, interface) in state.read().interfaces.iter().enumerate() {
                            div {
                                class: if index == selected_interface_index() { "interface-item active" } else { "interface-item" },
                                onclick: move |_| selected_interface_index.set(index),

                                div { class: "interface-header",
                                    i { class: "fas fa-ethernet" }
                                    span { class: "interface-name", "{interface.name}" }
                                }
                                div { class: "interface-summary",
                                    "{interface.addresses.len()} address(es)"
                                }
                            }
                        }

                        button {
                            class: "add-interface-button",
                            onclick: move |_| {
                                let mut interfaces = state.read().interfaces.clone();
                                let new_index = interfaces.len();
                                interfaces.push(NetworkInterface {
                                    name: format!("net{}", new_index),
                                    selector: None,
                                    addresses: vec![],
                                });
                                state.write().interfaces = interfaces;
                                selected_interface_index.set(new_index);
                            },
                            i { class: "fas fa-plus" }
                            " Add Interface"
                        }
                    }

                    div { class: "dns-config",
                        h3 { "DNS Configuration" }
                        div { class: "form-group",
                            label { "Name Servers:" }
                            for (index, nameserver) in state.read().nameservers.iter().enumerate() {
                                div { class: "nameserver-item",
                                    input {
                                        r#type: "text",
                                        class: "form-control",
                                        value: "{nameserver}",
                                        oninput: move |evt| {
                                            let mut nameservers = state.read().nameservers.clone();
                                            if index < nameservers.len() {
                                                nameservers[index] = evt.value();
                                                state.write().nameservers = nameservers;
                                            }
                                        },
                                        placeholder: "8.8.8.8"
                                    }
                                    button {
                                        class: "remove-nameserver-button",
                                        onclick: move |_| {
                                            let mut nameservers = state.read().nameservers.clone();
                                            if index < nameservers.len() {
                                                nameservers.remove(index);
                                                state.write().nameservers = nameservers;
                                            }
                                        },
                                        i { class: "fas fa-times" }
                                    }
                                }
                            }

                            button {
                                class: "add-nameserver-button",
                                onclick: move |_| {
                                    let mut nameservers = state.read().nameservers.clone();
                                    nameservers.push("".to_string());
                                    state.write().nameservers = nameservers;
                                },
                                i { class: "fas fa-plus" }
                                " Add Name Server"
                            }
                        }
                    }

                    div { class: "hostname-config",
                        h3 { "System Configuration" }
                        div { class: "form-group",
                            label { "Hostname:" }
                            input {
                                r#type: "text",
                                class: "form-control",
                                value: state.read().hostname.clone(),
                                oninput: move |evt| {
                                    state.write().hostname = evt.value();
                                },
                                placeholder: "my-illumos-system"
                            }
                        }
                    }
                }

                div { class: "network-main",
                    if !state.read().interfaces.is_empty() && selected_interface_index() < state.read().interfaces.len() {
                        InterfaceEditor {
                            interface_index: selected_interface_index(),
                            on_interface_update: move |updated_interface: NetworkInterface| {
                                let mut interfaces = state.read().interfaces.clone();
                                if selected_interface_index() < interfaces.len() {
                                    interfaces[selected_interface_index()] = updated_interface;
                                    state.write().interfaces = interfaces;
                                }
                            }
                        }
                    } else {
                        div { class: "empty-state",
                            i { class: "fas fa-ethernet empty-icon" }
                            h3 { "No Network Interface Selected" }
                            p { "Select or create a network interface to configure its settings." }
                        }
                    }
                }
            }
        }
    }
}

/// Interface editor component for configuring a specific network interface
#[component]
pub fn InterfaceEditor(
    interface_index: usize,
    on_interface_update: EventHandler<NetworkInterface>,
) -> Element {
    let state = use_context::<Signal<InstallerState>>();
    let interface = &state.read().interfaces[interface_index];
    let mut interface_name = use_signal(|| interface.name.clone());
    let mut interface_selector = use_signal(|| interface.selector.clone());
    let mut addresses = use_signal(|| interface.addresses.clone());

    // Update signals when interface changes
    use_effect(move || {
        let current_interface = &state.read().interfaces[interface_index];
        interface_name.set(current_interface.name.clone());
        interface_selector.set(current_interface.selector.clone());
        addresses.set(current_interface.addresses.clone());
    });

    let update_interface = move || {
        let updated_interface = NetworkInterface {
            name: interface_name(),
            selector: interface_selector(),
            addresses: addresses(),
        };
        on_interface_update.call(updated_interface);
    };

    rsx! {
        div { class: "interface-editor",
            div { class: "interface-header-editor",
                h3 { "Configure Interface" }
                div { class: "interface-basic-config",
                    div { class: "form-group",
                        label { "Interface Name:" }
                        input {
                            r#type: "text",
                            class: "form-control",
                            value: "{interface_name}",
                            oninput: move |evt| {
                                interface_name.set(evt.value());
                                update_interface();
                            },
                            placeholder: "net0"
                        }
                    }

                    div { class: "form-group",
                        label { "Interface Selector (optional):" }
                        input {
                            r#type: "text",
                            class: "form-control",
                            value: interface_selector().as_deref().unwrap_or(""),
                            oninput: move |evt| {
                                let value = evt.value();
                                interface_selector.set(if value.is_empty() {
                                    None
                                } else {
                                    Some(value)
                                });
                                update_interface();
                            },
                            placeholder: "e1000g0 or auto"
                        }
                    }
                }
            }

            div { class: "address-section",
                div { class: "address-header",
                    h4 { "IP Address Configuration" }
                    button {
                        class: "add-address-button",
                        onclick: move |_| {
                            let mut current_addresses = addresses();
                            current_addresses.push(AddressObject {
                                name: format!("addr{}", current_addresses.len()),
                                kind: AddressKind::Dhcp4,
                                address: None,
                            });
                            addresses.set(current_addresses);
                            update_interface();
                        },
                        i { class: "fas fa-plus" }
                        " Add Address"
                    }
                }

                if addresses().is_empty() {
                    div { class: "address-empty-state",
                        i { class: "fas fa-plus-circle" }
                        p { "No IP addresses configured. Add an address configuration to get started." }
                    }
                } else {
                    div { class: "address-list",
                        for (addr_index, address) in addresses().iter().enumerate() {
                            AddressEditor {
                                address: address.clone(),
                                address_index: addr_index,
                                on_address_update: move |updated_address: AddressObject| {
                                    let mut current_addresses = addresses();
                                    if addr_index < current_addresses.len() {
                                        current_addresses[addr_index] = updated_address;
                                        addresses.set(current_addresses);
                                        update_interface();
                                    }
                                },
                                on_address_delete: move |_| {
                                    let mut current_addresses = addresses();
                                    if addr_index < current_addresses.len() {
                                        current_addresses.remove(addr_index);
                                        addresses.set(current_addresses);
                                        update_interface();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "interface-summary",
                h4 { "Interface Summary" }
                div { class: "summary-stats",
                    div { class: "stat-item",
                        span { class: "stat-label", "Interface Name:" }
                        span { class: "stat-value", "{interface_name}" }
                    }
                    div { class: "stat-item",
                        span { class: "stat-label", "Selector:" }
                        span { class: "stat-value",
                            if let Some(selector) = interface_selector() {
                                "{selector}"
                            } else {
                                "Auto-detect"
                            }
                        }
                    }
                    div { class: "stat-item",
                        span { class: "stat-label", "Addresses:" }
                        span { class: "stat-value", "{addresses().len()}" }
                    }
                }
            }
        }
    }
}

/// Address editor component for configuring IP addresses
#[component]
pub fn AddressEditor(
    address: AddressObject,
    address_index: usize,
    on_address_update: EventHandler<AddressObject>,
    on_address_delete: EventHandler<()>,
) -> Element {
    let mut address_name = use_signal(|| address.name.clone());
    let mut address_kind = use_signal(|| address.kind.clone());
    let mut address_value = use_signal(|| address.address.clone());
    let mut expanded = use_signal(|| false);

    let update_address = move || {
        let updated_address = AddressObject {
            name: address_name(),
            kind: address_kind(),
            address: address_value(),
        };
        on_address_update.call(updated_address);
    };

    rsx! {
        div { class: "address-editor",
            div { class: "address-header",
                button {
                    class: "address-toggle",
                    onclick: move |_| expanded.set(!expanded()),
                    i { class: if expanded() { "fas fa-chevron-down" } else { "fas fa-chevron-right" } }
                }

                h5 { "Address {address_index + 1}" }

                div { class: "address-type-selector",
                    select {
                        value: "{address_kind:?}",
                        onchange: move |evt| {
                            let new_kind = evt.value();
                            for kind in AddressKind::all() {
                                if format!("{:?}", kind) == new_kind {
                                    address_kind.set(kind.clone());
                                    // Clear address if switching to non-static
                                    if !kind.requires_address() {
                                        address_value.set(None);
                                    }
                                    update_address();
                                    break;
                                }
                            }
                        },

                        for kind in AddressKind::all() {
                            option {
                                value: "{kind:?}",
                                "{kind.display_name()}"
                            }
                        }
                    }
                }

                button {
                    class: "delete-address-button",
                    onclick: move |_| on_address_delete.call(()),
                    i { class: "fas fa-trash" }
                }
            }

            if expanded() {
                div { class: "address-content",
                    div { class: "form-group",
                        label { "Address Name:" }
                        input {
                            r#type: "text",
                            class: "form-control",
                            value: "{address_name}",
                            oninput: move |evt| {
                                address_name.set(evt.value());
                                update_address();
                            },
                            placeholder: "primary"
                        }
                    }

                    if address_kind().requires_address() {
                        div { class: "form-group",
                            label { "IP Address:" }
                            input {
                                r#type: "text",
                                class: "form-control",
                                value: address_value().as_deref().unwrap_or(""),
                                oninput: move |evt| {
                                    let value = evt.value();
                                    address_value.set(if value.is_empty() {
                                        None
                                    } else {
                                        Some(value)
                                    });
                                    update_address();
                                },
                                placeholder: "192.168.1.100/24"
                            }
                        }
                    } else {
                        div { class: "address-info",
                            p { "This address type is configured automatically." }
                        }
                    }

                    div { class: "address-validation",
                        if address_kind().requires_address() && address_value().is_none() {
                            div { class: "validation-warning",
                                i { class: "fas fa-exclamation-triangle" }
                                " IP address is required for static configuration."
                            }
                        } else {
                            div { class: "validation-success",
                                i { class: "fas fa-check-circle" }
                                " Address configuration is valid."
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Network validation summary component
#[component]
pub fn NetworkValidationSummary() -> Element {
    let state = use_context::<Signal<InstallerState>>();

    let total_interfaces = state.read().interfaces.len();
    let configured_interfaces = state
        .read()
        .interfaces
        .iter()
        .filter(|i| !i.addresses.is_empty())
        .count();
    let total_addresses = state
        .read()
        .interfaces
        .iter()
        .map(|i| i.addresses.len())
        .sum::<usize>();
    let hostname_configured = !state.read().hostname.trim().is_empty();

    rsx! {
        div { class: "network-validation",
            h4 { "Network Configuration Summary" }
            div { class: "validation-grid",
                div { class: "validation-item",
                    span { class: "validation-label", "Interfaces:" }
                    span { class: "validation-value", "{configured_interfaces}/{total_interfaces} configured" }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "IP Addresses:" }
                    span { class: "validation-value", "{total_addresses}" }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "Hostname:" }
                    span { class: "validation-value",
                        if hostname_configured { "Configured" }
                        else { "Not set" }
                    }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "DNS Servers:" }
                    span { class: "validation-value", "{state.read().nameservers.len()}" }
                }
            }
        }
    }
}
