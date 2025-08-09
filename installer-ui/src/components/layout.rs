//! Layout components for the Illumos installer UI
//!
//! This module contains the main layout components including the header,
//! navigation, and overall application structure.

use crate::routes::Route;
use crate::state::InstallerState;
use dioxus::prelude::*;

/// Main layout component that wraps all pages
#[component]
pub fn MainLayout() -> Element {
    let _state = use_context::<Signal<InstallerState>>();
    let current_route = use_route::<Route>();

    let step_number = current_route.step_number();
    let step_name = current_route.display_name();
    let total_steps = Route::total_steps();

    rsx! {
        div { class: "app-container",
            AppHeader {
                step_name: step_name.to_string(),
                step_number: step_number,
                total_steps: total_steps
            }

            main { class: "app-main",
                Outlet::<Route> {}
            }

            footer { class: "app-footer",
                NavigationButtons { current_route: current_route.clone() }
            }
        }
    }
}

/// Application header with title and progress indicator
#[component]
fn AppHeader(step_name: String, step_number: usize, total_steps: usize) -> Element {
    let progress_percentage = if total_steps > 1 {
        (step_number - 1) as f32 / (total_steps - 1) as f32 * 100.0
    } else {
        0.0
    };

    rsx! {
        header { class: "app-header",
            h1 { "illumos Installer" }
            div { class: "progress-indicator",
                div { class: "step-counter",
                    "{step_number}/{total_steps}: {step_name}"
                }
                div { class: "progress-bar",
                    div {
                        class: "progress-fill",
                        style: "width: {progress_percentage}%"
                    }
                }
            }
        }
    }
}

/// Navigation buttons for moving between installation steps
#[component]
pub fn NavigationButtons(current_route: Route) -> Element {
    let navigator = use_navigator();
    let state = use_context::<Signal<InstallerState>>();

    let prev_route = current_route.previous();
    let next_route = current_route.next();
    let can_go_back = current_route.can_go_back();
    let can_go_forward =
        current_route.can_go_forward() && is_step_valid(&current_route, &state.read());

    rsx! {
        div { class: "navigation-buttons",
            if let Some(prev) = prev_route {
                if can_go_back {
                    button {
                        class: "nav-button prev",
                        onclick: move |_| { navigator.push(prev.clone()); },
                        i { class: "fas fa-arrow-left" }
                        " Previous"
                    }
                } else {
                    div { class: "nav-spacer" }
                }
            } else {
                div { class: "nav-spacer" }
            }

            if let Some(next) = next_route {
                if can_go_forward {
                    button {
                        class: "nav-button next",
                        onclick: move |_| { navigator.push(next.clone()); },
                        if matches!(current_route, Route::ReviewConfiguration {}) {
                            "Start Installation "
                        } else {
                            "Next "
                        }
                        i { class: "fas fa-arrow-right" }
                    }
                } else {
                    button {
                        class: "nav-button next disabled",
                        disabled: true,
                        if matches!(current_route, Route::ReviewConfiguration {}) {
                            "Start Installation "
                        } else {
                            "Next "
                        }
                        i { class: "fas fa-arrow-right" }
                    }
                }
            }
        }
    }
}

/// Progress breadcrumb component showing all installation steps
#[component]
pub fn ProgressBreadcrumb() -> Element {
    let current_route = use_route::<Route>();
    let current_step = current_route.step_number();

    let all_routes = vec![
        Route::Welcome {},
        Route::ServerSelection {},
        Route::StorageConfiguration {},
        Route::NetworkConfiguration {},
        Route::SystemConfiguration {},
        Route::ReviewConfiguration {},
        Route::Installation {},
    ];

    rsx! {
        nav { class: "breadcrumb",
            ol { class: "breadcrumb-list",
                for (index, route) in all_routes.iter().enumerate() {
                    li {
                        class: format!("{}", if index + 1 == current_step { "breadcrumb-item active" }
                               else if index + 1 < current_step { "breadcrumb-item completed" }
                               else { "breadcrumb-item" }),

                        i { class: "{route.icon_class()}" }
                        span { class: "breadcrumb-text", "{route.display_name()}" }

                        if index < all_routes.len() - 1 {
                            i { class: "fas fa-chevron-right breadcrumb-separator" }
                        }
                    }
                }
            }
        }
    }
}

/// Validates if the current step has all required information filled
fn is_step_valid(route: &Route, state: &InstallerState) -> bool {
    match route {
        Route::Welcome {} => true,
        Route::ServerSelection {} => state.selected_server.is_some(),
        Route::StorageConfiguration {} => {
            !state.pools.is_empty() && state.pools.iter().all(|pool| !pool.vdevs.is_empty())
        }
        Route::NetworkConfiguration {} => !state.hostname.trim().is_empty(),
        Route::SystemConfiguration {} => !state.hostname.trim().is_empty(),
        Route::ReviewConfiguration {} => {
            state.selected_server.is_some()
                && !state.pools.is_empty()
                && !state.hostname.trim().is_empty()
        }
        Route::Installation {} => true,
    }
}

/// Status indicator component for showing current step status
#[component]
pub fn StepStatus(is_valid: bool, is_current: bool) -> Element {
    let (class, icon) = if is_current {
        if is_valid {
            ("step-status current valid", "fas fa-check-circle")
        } else {
            ("step-status current invalid", "fas fa-exclamation-circle")
        }
    } else if is_valid {
        ("step-status completed", "fas fa-check")
    } else {
        ("step-status pending", "fas fa-circle")
    };

    rsx! {
        div { class: class,
            i { class: icon }
        }
    }
}
