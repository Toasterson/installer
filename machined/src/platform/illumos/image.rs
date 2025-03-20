use crate::error::InstallationError;
use crate::error::InstallationError::CannotCreateImageReference;
use crate::machined::InstallProgress;
use crate::util::{report_install_debug, report_install_error, report_install_info};
use oci_util::digest::OciDigest;
use oci_util::distribution::client::{Registry, Session};
use oci_util::image_reference::ImageReference;
use oci_util::models::ManifestVariant::{Artifact, List, Manifest};
use oci_util::models::{AnyOciConfig, ImageManifest, ImageManifestList};
use std::env;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;
use tonic::Status;

pub const OCI_BASE_CACHE_DIR: &str = "/var/tmp/";

pub fn build_image_ref(image: &str) -> Result<ImageReference, InstallationError> {
    ImageReference::from_str(image).map_err(|e| CannotCreateImageReference(e.to_string()))
}

pub async fn fetch_image(
    image_ref: &ImageReference,
    default_registry: &str,
    tx: Sender<Result<InstallProgress, Status>>,
) -> Result<AnyOciConfig, InstallationError> {
    let base_path = Path::new(OCI_BASE_CACHE_DIR);
    if !base_path.exists() {
        return Err(InstallationError::BaseDirDoesNotExist);
    }
    let registry = if let Some(hostname) = image_ref.hostname.clone() {
        Registry::new(format!("https://{}", hostname), None)
    } else {
        Registry::new(format!("https://{}", default_registry), None)
    };
    let mut session = registry.new_session(image_ref.name.clone());
    let manifest = session.query_manifest(image_ref.tag.as_str()).await?;
    let image_path = base_path.join(image_ref.name.clone());
    if !image_path.exists() {
        create_dir_all(image_path.as_path())?;
    }
    if let Some(manifest) = manifest {
        match manifest {
            Manifest(manifest) => fetch_manifest(manifest, session, tx, image_path.as_path()).await,
            List(manifest_list) => {
                select_correct_manifest(manifest_list, session, tx, image_path.as_path()).await
            }
            Artifact(_) => Err(InstallationError::ArtifactManifestsNotSupported),
        }
    } else {
        Err(InstallationError::NoManifestFound)
    }
}

async fn select_correct_manifest(
    list: ImageManifestList,
    mut session: Session,
    tx: Sender<Result<InstallProgress, Status>>,
    local_image_path: &Path,
) -> Result<AnyOciConfig, InstallationError> {
    let cur_os_arch = format!("{}/{}", env::consts::OS, std::env::consts::ARCH);
    for manifest in list {
        let plat = manifest.platform;
        let m_os_arch = format!("{}/{}", plat.os, plat.architecture);
        if m_os_arch == cur_os_arch {
            tx.send(report_install_debug(
                format!("selecting {} to install", manifest.digest.as_str()).as_str(),
            ))
            .await?;
            let resp = session
                .fetch_blob_as::<ImageManifest>(&manifest.digest)
                .await?;
            let manifest = resp.ok_or(InstallationError::NoManifestFound)?;
            return fetch_manifest(manifest, session, tx, local_image_path).await;
        }
    }
    Err(InstallationError::NoManifestMatchesArch(cur_os_arch))
}

async fn fetch_manifest(
    manifest: ImageManifest,
    mut session: Session,
    tx: Sender<Result<InstallProgress, Status>>,
    local_image_path: &Path,
) -> Result<AnyOciConfig, InstallationError> {
    let resp = session
        .fetch_blob_as::<AnyOciConfig>(&manifest.config.digest)
        .await?;
    let manifest = resp.ok_or(InstallationError::NoManifestFound)?;
    fetch_blobs(manifest.layers(), session, tx, local_image_path).await?;
    Ok(manifest)
}

async fn fetch_blobs(
    blobs: Vec<OciDigest>,
    mut session: Session,
    tx: Sender<Result<InstallProgress, Status>>,
    local_image_path: &Path,
) -> Result<(), InstallationError> {
    for blob in blobs {
        tx.send(report_install_debug(
            format!("downloading blob {}", &blob.as_str()).as_str(),
        ))
        .await
        .map_err(|e| Err(InstallationError::BlobDownloadFailed))?;
        let local_path = build_local_image_path(local_image_path, &blob);
        let local_dir = local_path.parent().unwrap();
        if !local_dir.exists() {
            create_dir_all(local_dir)?;
        }
        session.download_blob(&blob, &local_path, true).await?;
    }
    Ok(())
}

fn build_local_image_path(local_image_path: &Path, blob: &OciDigest) -> PathBuf {
    let digest_as_path = blob.as_str().replace(":", "/");
    let local_path = local_image_path.join(&digest_as_path);
    local_path
}

pub async fn install_image(
    image_ref: &ImageReference,
    image_config: AnyOciConfig,
    tx: Sender<Result<InstallProgress, Status>>,
) -> Result<(), SendError<Result<InstallProgress, Status>>> {
    let base_path = Path::new(OCI_BASE_CACHE_DIR);
    let image_path = base_path.join(image_ref.name.clone());
    tx.send(report_install_info("installing image to root dataset"))
        .await?;
    for layer in image_config.layers() {
        tx.send(report_install_info(
            format!("unpacking layer {}", layer.as_str()).as_str(),
        ))
        .await?;

        let layer_file_path = build_local_image_path(image_path.as_path(), &layer)
            .to_string_lossy()
            .to_string();

        let mut tar_cmd = match Command::new("gtar")
            .arg("-xaf")
            .arg(layer_file_path.as_str())
            .arg("-C")
            .arg("/a")
            .spawn()
        {
            Ok(t) => t,
            Err(e) => {
                tx.send(report_install_error(e)).await?;
                return Err(SendError(Err(Status::internal(
                    "could not spawn tar process",
                ))));
            }
        };

        match tar_cmd
            .wait()
            .map_err(|e| SendError(Err(Status::internal("could not wait for tar process"))))?
            .code()
        {
            Some(ec) if ec != 0 => {
                tx.send(report_install_error(Err("tar return non-zero exit code")))
                    .await?;
                return Err(SendError(Err(Status::internal(
                    "tar returned non-zero exit code",
                ))));
            }
            _ => {}
        }
    }
    Ok(())
}
