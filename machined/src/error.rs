use crate::machined::InstallProgress;
use jwt_simple::reexports::serde_json;
use oci_util::distribution::client::ClientError;
use std::io;
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
    #[error("cannot create image reference {0}")]
    CannotCreateImageReference(String),
    #[error(transparent)]
    XcClientError(#[from] ClientError),
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::error::Error),
    #[error(transparent)]
    StringConvert(#[from] std::string::FromUtf8Error),
    #[error("failed to download blob")]
    BlobDownloadFailed,
    #[error("Artifact manifests are not supported for download")]
    ArtifactManifestsNotSupported,
    #[error("tar return non-zero exit code")]
    TarReturnNonzeroExitCode,
    #[error("Send Failed")]
    SendFailed,
}
