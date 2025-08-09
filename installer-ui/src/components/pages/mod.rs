//! Page components for the Illumos installer
//!
//! This module contains all the individual page components that make up
//! the installer's user interface, one for each step in the installation process.

pub mod installation;
pub mod network_configuration;
pub mod review_configuration;
pub mod server_selection;
pub mod storage_configuration;
pub mod system_configuration;
pub mod welcome;

// Re-export all page components for easy access
pub use installation::*;
pub use network_configuration::*;
pub use review_configuration::*;
pub use server_selection::*;
pub use storage_configuration::*;
pub use system_configuration::*;
pub use welcome::*;
