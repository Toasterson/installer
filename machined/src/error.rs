use crate::machined::InstallProgress;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug, Error)]
pub enum InstallationError {
    #[error(transparent)]
    SendError(#[from] SendError<InstallProgress>),
    #[error("failed to create zfs pool\n {0}")]
    ZpoolCreateFailed(String),
    #[error("failed to create rpool/ROOT dataset {0}")]
    BaseRootDSCreateFailed(String),
    #[error("requested installation image does not exist")]
    NoManifestFound,
    #[error("requested image does not support {0}")]
    NoManifestMatchesArch(String),
    #[error("cache base directory does not exist")]
    BaseDirDoesNotExist,
}
