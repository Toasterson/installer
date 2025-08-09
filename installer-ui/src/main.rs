//! Illumos Installer UI - Main Application Entry Point
//!
//! A modern, responsive user interface for installing illumos on target machines.
//! Built with Dioxus for cross-platform compatibility and reactive state management.

use dioxus::prelude::*;

// Import all modules
mod components;
mod routes;
mod server;
mod state;

// Re-export commonly used items
use components::MainLayout;
use routes::Route;
use state::InstallerState;

// Application constants
const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    launch(App);
}

/// Main application component
#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(InstallerState::default()));

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}
