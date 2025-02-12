use crate::error::InstallationError;
use crate::error::InstallationError::CannotCreateImageReference;
use crate::machined::InstallProgress;
use crate::util::report_install_debug;
use jwt_simple::reexports::serde_json;
use oci_util::digest::OciDigest;
use oci_util::distribution::client::{Registry, Session};
use oci_util::image_reference::ImageReference;
use oci_util::models::ManifestVariant::{Artifact, List, Manifest};
use oci_util::models::{AnyOciConfig, ArtifactManifest, ImageManifest, ImageManifestList};
use std::env;
use std::fs::{create_dir_all, File};
use std::path::Path;
use std::str::FromStr;
use tokio::sync::mpsc::Sender;
use tonic::Status;

pub const OCI_BASE_CACHE_DIR: &str = "/var/tmp/";

pub fn build_image_ref(image: &str) -> Result<ImageReference, InstallationError> {
    Ok(ImageReference::from_str(image)
        .map_err(|e| Err(CannotCreateImageReference(e.to_string())))?)
}

pub async fn fetch_image(
    image_ref: &ImageReference,
    default_registry: &str,
    tx: Sender<Result<InstallProgress, Status>>,
) -> Result<(), InstallationError> {
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
            Artifact(artifact_manifest) => {
                fetch_artifact(artifact_manifest, session, tx, image_path.as_path()).await
            }
        }
    } else {
        Err(InstallationError::NoManifestFound)
    }
}

async fn fetch_artifact(
    artifact_manifest: ArtifactManifest,
    session: Session,
    tx: Sender<Result<InstallProgress, Status>>,
    local_image_path: &Path,
) -> Result<(), InstallationError> {
    fetch_blobs(
        artifact_manifest
            .blobs
            .into_iter()
            .map(|desc| desc.digest)
            .collect(),
        session,
        tx,
        local_image_path,
    )
    .await?;
    Ok(())
}

async fn select_correct_manifest(
    list: ImageManifestList,
    mut session: Session,
    tx: Sender<Result<InstallProgress, Status>>,
    local_image_path: &Path,
) -> Result<(), InstallationError> {
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
) -> Result<(), InstallationError> {
    let resp = session
        .fetch_blob_as::<AnyOciConfig>(&manifest.config.digest)
        .await?;
    let manifest = resp.ok_or(InstallationError::NoManifestFound)?;
    fetch_blobs(manifest.layers(), session, tx, local_image_path).await?;
    let c_file = File::create(local_image_path.join("config.json"))?;
    serde_json::to_writer(c_file, &manifest)?;
    Ok(())
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
        let digest_as_path = blob.as_str().replace(":", "/");
        let local_path = local_image_path.join(&digest_as_path);
        let local_dir = local_path.parent().unwrap();
        if !local_dir.exists() {
            create_dir_all(local_dir)?;
        }
        session.download_blob(&blob, &local_path, true).await?;
    }
    Ok(())
}

fn install_image(
    image_ref: &ImageReference,
    tx: Sender<Result<InstallProgress, Status>>,
) -> Result<(), InstallationError> {
    let base_path = Path::new(OCI_BASE_CACHE_DIR);
    let image_path = base_path.join(image_ref.name.clone());
    Ok(())
}
