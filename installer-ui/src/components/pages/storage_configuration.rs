//! Storage configuration page component for the Illumos installer
//!
//! This page allows users to configure ZFS storage pools, virtual devices,
//! and disk allocation for the installation.

use crate::state::{InstallerState, Pool, VDev, VDevType};
use dioxus::prelude::*;
use std::collections::HashMap;

/// Storage configuration page component
#[component]
pub fn StorageConfiguration() -> Element {
    let mut state = use_context::<Signal<InstallerState>>();
    let mut selected_pool_index = use_signal(|| 0usize);

    rsx! {
        div { class: "page storage-page",
            div { class: "page-header",
                h2 { "Storage Configuration" }
                p { class: "page-description",
                    "Configure ZFS storage pools and virtual devices for your installation."
                }
            }

            div { class: "storage-config",
                div { class: "storage-sidebar",
                    div { class: "pool-list",
                        h3 { "Storage Pools" }
                        for (index, pool) in state.read().pools.iter().enumerate() {
                            div {
                                class: if index == selected_pool_index() { "pool-item active" } else { "pool-item" },
                                onclick: move |_| selected_pool_index.set(index),

                                div { class: "pool-header",
                                    i { class: "fas fa-hdd" }
                                    span { class: "pool-name", "{pool.name}" }
                                }
                                div { class: "pool-summary",
                                    "{pool.vdevs.len()} vdev(s)"
                                }
                            }
                        }

                        button {
                            class: "add-pool-button",
                            onclick: move |_| {
                                let mut pools = state.read().pools.clone();
                                let new_index = pools.len();
                                pools.push(Pool {
                                    name: format!("pool{}", new_index),
                                    vdevs: vec![],
                                    options: HashMap::new(),
                                });
                                state.write().pools = pools;
                                selected_pool_index.set(new_index);
                            },
                            i { class: "fas fa-plus" }
                            " Add Pool"
                        }
                    }

                    div { class: "image-config",
                        h3 { "System Image" }
                        div { class: "form-group",
                            label { "OCI Image:" }
                            input {
                                r#type: "text",
                                class: "form-control",
                                value: state.read().image.clone(),
                                oninput: move |evt| {
                                    state.write().image = evt.value();
                                },
                                placeholder: "oci://registry/image:tag"
                            }
                        }

                        div { class: "form-group",
                            label { "Boot Environment (optional):" }
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
                                placeholder: "boot-environment-name"
                            }
                        }
                    }
                }

                div { class: "storage-main",
                    if !state.read().pools.is_empty() && selected_pool_index() < state.read().pools.len() {
                        PoolEditor {
                            pool_index: selected_pool_index(),
                            on_pool_update: move |updated_pool: Pool| {
                                let mut pools = state.read().pools.clone();
                                if selected_pool_index() < pools.len() {
                                    pools[selected_pool_index()] = updated_pool;
                                    state.write().pools = pools;
                                }
                            }
                        }
                    } else {
                        div { class: "empty-state",
                            i { class: "fas fa-hdd empty-icon" }
                            h3 { "No Storage Pool Selected" }
                            p { "Select or create a storage pool to configure its virtual devices." }
                        }
                    }
                }
            }
        }
    }
}

/// Pool editor component for configuring a specific pool
#[component]
pub fn PoolEditor(pool_index: usize, on_pool_update: EventHandler<Pool>) -> Element {
    let state = use_context::<Signal<InstallerState>>();
    let pool = &state.read().pools[pool_index];
    let mut pool_name = use_signal(|| pool.name.clone());
    let mut vdevs = use_signal(|| pool.vdevs.clone());

    // Update signals when pool changes
    use_effect(move || {
        let current_pool = &state.read().pools[pool_index];
        pool_name.set(current_pool.name.clone());
        vdevs.set(current_pool.vdevs.clone());
    });

    let update_pool = move || {
        let updated_pool = Pool {
            name: pool_name(),
            vdevs: vdevs(),
            options: HashMap::new(),
        };
        on_pool_update.call(updated_pool);
    };

    rsx! {
        div { class: "pool-editor",
            div { class: "pool-header-editor",
                h3 { "Configure Pool" }
                div { class: "form-group inline",
                    label { "Pool Name:" }
                    input {
                        r#type: "text",
                        class: "form-control",
                        value: "{pool_name}",
                        oninput: move |evt| {
                            pool_name.set(evt.value());
                            update_pool();
                        }
                    }
                }
            }

            div { class: "vdev-section",
                div { class: "vdev-header",
                    h4 { "Virtual Devices" }
                    button {
                        class: "add-vdev-button",
                        onclick: move |_| {
                            let mut current_vdevs = vdevs();
                            current_vdevs.push(VDev {
                                kind: VDevType::Mirror,
                                disks: vec![],
                            });
                            vdevs.set(current_vdevs);
                            update_pool();
                        },
                        i { class: "fas fa-plus" }
                        " Add VDev"
                    }
                }

                if vdevs().is_empty() {
                    div { class: "vdev-empty-state",
                        i { class: "fas fa-plus-circle" }
                        p { "No virtual devices configured. Add a VDev to get started." }
                    }
                } else {
                    div { class: "vdev-list",
                        for (vdev_index, vdev) in vdevs().iter().enumerate() {
                            VDevEditor {
                                vdev: vdev.clone(),
                                vdev_index: vdev_index,
                                on_vdev_update: move |updated_vdev: VDev| {
                                    let mut current_vdevs = vdevs();
                                    if vdev_index < current_vdevs.len() {
                                        current_vdevs[vdev_index] = updated_vdev;
                                        vdevs.set(current_vdevs);
                                        update_pool();
                                    }
                                },
                                on_vdev_delete: move |_| {
                                    let mut current_vdevs = vdevs();
                                    if vdev_index < current_vdevs.len() {
                                        current_vdevs.remove(vdev_index);
                                        vdevs.set(current_vdevs);
                                        update_pool();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "pool-summary",
                h4 { "Pool Summary" }
                div { class: "summary-stats",
                    div { class: "stat-item",
                        span { class: "stat-label", "Pool Name:" }
                        span { class: "stat-value", "{pool_name}" }
                    }
                    div { class: "stat-item",
                        span { class: "stat-label", "Virtual Devices:" }
                        span { class: "stat-value", "{vdevs().len()}" }
                    }
                    div { class: "stat-item",
                        span { class: "stat-label", "Total Disks:" }
                        span { class: "stat-value", "{vdevs().iter().map(|v| v.disks.len()).sum::<usize>()}" }
                    }
                }
            }
        }
    }
}

/// VDev editor component for configuring virtual devices
#[component]
pub fn VDevEditor(
    vdev: VDev,
    vdev_index: usize,
    on_vdev_update: EventHandler<VDev>,
    on_vdev_delete: EventHandler<()>,
) -> Element {
    let mut vdev_type = use_signal(|| vdev.kind.clone());
    let mut disks = use_signal(|| vdev.disks.clone());
    let mut expanded = use_signal(|| false);

    let update_vdev = move || {
        let updated_vdev = VDev {
            kind: vdev_type(),
            disks: disks(),
        };
        on_vdev_update.call(updated_vdev);
    };

    rsx! {
        div { class: "vdev-editor",
            div { class: "vdev-header",
                button {
                    class: "vdev-toggle",
                    onclick: move |_| expanded.set(!expanded()),
                    i { class: if expanded() { "fas fa-chevron-down" } else { "fas fa-chevron-right" } }
                }

                h5 { "VDev {vdev_index + 1}" }

                div { class: "vdev-type-selector",
                    select {
                        value: "{vdev_type:?}",
                        onchange: move |evt| {
                            let new_type = evt.value();
                            for vdev_kind in VDevType::all() {
                                if format!("{:?}", vdev_kind) == new_type {
                                    vdev_type.set(vdev_kind);
                                    update_vdev();
                                    break;
                                }
                            }
                        },

                        for kind in VDevType::all() {
                            option {
                                value: "{kind:?}",
                                "{kind.display_name()}"
                            }
                        }
                    }
                }

                button {
                    class: "delete-vdev-button",
                    onclick: move |_| on_vdev_delete.call(()),
                    i { class: "fas fa-trash" }
                }
            }

            if expanded() {
                div { class: "vdev-content",
                    div { class: "vdev-info",
                        p { class: "vdev-description",
                            "Minimum disks required: {vdev_type().min_disks()}"
                        }
                    }

                    div { class: "disk-configuration",
                        h6 { "Disk Configuration" }
                        div { class: "disk-list",
                            for (disk_index, disk) in disks().iter().enumerate() {
                                div { class: "disk-item",
                                    input {
                                        r#type: "text",
                                        class: "disk-input",
                                        value: "{disk}",
                                        oninput: move |evt| {
                                            let mut current_disks = disks();
                                            if disk_index < current_disks.len() {
                                                current_disks[disk_index] = evt.value();
                                                disks.set(current_disks);
                                                update_vdev();
                                            }
                                        },
                                        placeholder: "Enter disk identifier (e.g., c0t0d0)"
                                    }
                                    button {
                                        class: "remove-disk-button",
                                        onclick: move |_| {
                                            let mut current_disks = disks();
                                            if disk_index < current_disks.len() {
                                                current_disks.remove(disk_index);
                                                disks.set(current_disks);
                                                update_vdev();
                                            }
                                        },
                                        i { class: "fas fa-times" }
                                    }
                                }
                            }

                            button {
                                class: "add-disk-button",
                                onclick: move |_| {
                                    let mut current_disks = disks();
                                    current_disks.push(format!("c0t{}d0", current_disks.len()));
                                    disks.set(current_disks);
                                    update_vdev();
                                },
                                i { class: "fas fa-plus" }
                                " Add Disk"
                            }
                        }

                        div { class: "disk-validation",
                            if disks().len() < vdev_type().min_disks() {
                                div { class: "validation-warning",
                                    i { class: "fas fa-exclamation-triangle" }
                                    " Insufficient disks. Need at least {vdev_type().min_disks()} disks for {vdev_type().display_name()}."
                                }
                            } else {
                                div { class: "validation-success",
                                    i { class: "fas fa-check-circle" }
                                    " Disk configuration is valid."
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Storage validation summary component
#[component]
pub fn StorageValidationSummary() -> Element {
    let state = use_context::<Signal<InstallerState>>();
    let pools = &state.read().pools;

    let total_pools = pools.len();
    let valid_pools = pools.iter().filter(|p| !p.vdevs.is_empty()).count();
    let total_vdevs = pools.iter().map(|p| p.vdevs.len()).sum::<usize>();
    let total_disks = pools
        .iter()
        .flat_map(|p| &p.vdevs)
        .map(|v| v.disks.len())
        .sum::<usize>();

    rsx! {
        div { class: "storage-validation",
            h4 { "Storage Configuration Summary" }
            div { class: "validation-grid",
                div { class: "validation-item",
                    span { class: "validation-label", "Pools:" }
                    span { class: "validation-value", "{valid_pools}/{total_pools} configured" }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "Virtual Devices:" }
                    span { class: "validation-value", "{total_vdevs}" }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "Total Disks:" }
                    span { class: "validation-value", "{total_disks}" }
                }
                div { class: "validation-item",
                    span { class: "validation-label", "System Image:" }
                    span { class: "validation-value",
                        if state.read().image.is_empty() { "Not specified" }
                        else { "Configured" }
                    }
                }
            }
        }
    }
}
