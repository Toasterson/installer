use crate::machined::InstallProgress;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug, Error)]
pub enum InstallationError {
    #[error(transparent)]
    SendError(#[from] SendError<InstallProgress>),
}
