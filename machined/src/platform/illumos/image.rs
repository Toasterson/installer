use crate::error::InstallationError;
use oci_util::distribution::client::Registry;
use oci_util::image_reference::ImageReference;

pub async fn fetch_image(image: &str, default_registry: &str) -> Result<(), InstallationError> {
    let image_ref = ImageReference::from_str(image)?;
    let registry = if let Some(hostname) = image_ref.hostname {
        Registry::new(format!("https://{}", hostname))
    } else {
        Registry::new(format!("https://{}", default_registry))
    };
    let session = registry.new_session(image_ref.name);
    let manifest = session.query_manifest(image_ref.tag.as_str()).await?;
    if let Some(manifest) = manifest {
    } else {
        return Err(InstallationError::NoManifestFound);
    }
    Ok(())
}
