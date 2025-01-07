
#[cfg(not(target_os = "illumos"))]
mod mock;
#[cfg(target_os = "illumos")]
mod illumos;

use miette::Diagnostic;
use thiserror::Error;
#[cfg(target_os = "illumos")]
pub use illumos::*;

#[cfg(not(target_os = "illumos"))]
pub use mock::*;


#[derive(Error, Debug, Diagnostic)]
pub enum Error {

}