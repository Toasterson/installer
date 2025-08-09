//! Route definitions for the Illumos installer UI
//!
//! This module defines all the routes used in the installer application,
//! including the main layout and navigation structure.

use crate::components::{pages::*, MainLayout};
use dioxus::prelude::*;

/// Main route enum for the installer application
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
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

impl Route {
    /// Returns the display name for the route
    pub fn display_name(&self) -> &'static str {
        match self {
            Route::Welcome {} => "Welcome",
            Route::ServerSelection {} => "Server Selection",
            Route::StorageConfiguration {} => "Storage Configuration",
            Route::NetworkConfiguration {} => "Network Configuration",
            Route::SystemConfiguration {} => "System Configuration",
            Route::ReviewConfiguration {} => "Review Configuration",
            Route::Installation {} => "Installation",
        }
    }

    /// Returns the step number for progress indication
    pub fn step_number(&self) -> usize {
        match self {
            Route::Welcome {} => 1,
            Route::ServerSelection {} => 2,
            Route::StorageConfiguration {} => 3,
            Route::NetworkConfiguration {} => 4,
            Route::SystemConfiguration {} => 5,
            Route::ReviewConfiguration {} => 6,
            Route::Installation {} => 7,
        }
    }

    /// Returns the total number of steps
    pub fn total_steps() -> usize {
        7
    }

    /// Returns the next route in the installation flow
    pub fn next(&self) -> Option<Route> {
        match self {
            Route::Welcome {} => Some(Route::ServerSelection {}),
            Route::ServerSelection {} => Some(Route::StorageConfiguration {}),
            Route::StorageConfiguration {} => Some(Route::NetworkConfiguration {}),
            Route::NetworkConfiguration {} => Some(Route::SystemConfiguration {}),
            Route::SystemConfiguration {} => Some(Route::ReviewConfiguration {}),
            Route::ReviewConfiguration {} => Some(Route::Installation {}),
            Route::Installation {} => None,
        }
    }

    /// Returns the previous route in the installation flow
    pub fn previous(&self) -> Option<Route> {
        match self {
            Route::Welcome {} => None,
            Route::ServerSelection {} => Some(Route::Welcome {}),
            Route::StorageConfiguration {} => Some(Route::ServerSelection {}),
            Route::NetworkConfiguration {} => Some(Route::StorageConfiguration {}),
            Route::SystemConfiguration {} => Some(Route::NetworkConfiguration {}),
            Route::ReviewConfiguration {} => Some(Route::SystemConfiguration {}),
            Route::Installation {} => Some(Route::ReviewConfiguration {}),
        }
    }

    /// Returns true if this route allows going back
    pub fn can_go_back(&self) -> bool {
        !matches!(self, Route::Welcome {} | Route::Installation {})
    }

    /// Returns true if this route allows going forward
    pub fn can_go_forward(&self) -> bool {
        !matches!(self, Route::Installation {})
    }

    /// Returns the CSS class for the route's icon
    pub fn icon_class(&self) -> &'static str {
        match self {
            Route::Welcome {} => "fas fa-home",
            Route::ServerSelection {} => "fas fa-server",
            Route::StorageConfiguration {} => "fas fa-hdd",
            Route::NetworkConfiguration {} => "fas fa-network-wired",
            Route::SystemConfiguration {} => "fas fa-cog",
            Route::ReviewConfiguration {} => "fas fa-check-circle",
            Route::Installation {} => "fas fa-download",
        }
    }
}
