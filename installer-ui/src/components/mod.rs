//! UI Components for the Illumos installer
//!
//! This module contains all the UI components used throughout the installer,
//! organized by functionality and page structure.

pub mod layout;
pub mod pages;

// Re-export commonly used components
pub use layout::MainLayout;

// Re-export page components
pub use pages::*;
