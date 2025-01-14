#[cfg(target_os = "illumos")]
mod illumos;
#[cfg(not(target_os = "illumos"))]
mod mock;

#[cfg(target_os = "illumos")]
pub use illumos::*;
use miette::Diagnostic;
use thiserror::Error;

#[cfg(not(target_os = "illumos"))]
pub use mock::*;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {}
