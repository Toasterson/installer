use crate::config::MachinedConfig;
use crate::error::InstallationError;
use crate::machined::InstallProgress;
use crate::platform::illumos::image::{build_image_ref, fetch_image, install_image};
use crate::platform::illumos::zpool::{
    create_boot_environment, create_boot_environment_base_dataset, create_pool,
    mount_boot_environment,
};
use crate::util::{report_install_debug, report_install_error, report_install_info};
use machineconfig::MachineConfig;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;
use tonic::Status;

mod image;
mod sysconfig;
mod zpool;

const BEADM_BIN: &str = "/sbin/beadm";

const BOOTADM_BIN: &str = "/sbin/bootadm";

pub async fn install_system(
    mc: &MachineConfig,
    config: Arc<MachinedConfig>,
    tx: Sender<Result<InstallProgress, Status>>,
) -> Result<(), SendError<Result<InstallProgress, Status>>> {
    tx.send(report_install_debug("Starting installation"))
        .await?;
    for pool in &mc.pools {
        tx.send(report_install_debug(
            format!("Setting up pool {}", &pool.name).as_str(),
        ))
        .await?;
        match create_pool(&pool) {
            Ok(_) => {
                tx.send(report_install_debug(
                    format!("Pool {} created", &pool.name).as_str(),
                ))
                .await?;
            }
            Err(e) => {
                tx.send(report_install_error(&e)).await?;
                return Err(SendError(Err(Status::internal("Internal error"))));
            }
        }
    }

    match create_boot_environment_base_dataset() {
        Ok(_) => {
            tx.send(report_install_debug("base root Dataset created"))
                .await?;
        }
        Err(e) => {
            tx.send(report_install_error(&e)).await?;
            return Err(SendError(Err(Status::internal("Internal error"))));
        }
    }

    let be_path = match create_boot_environment(mc.boot_environment_name.clone()) {
        Ok(be_path) => {
            tx.send(report_install_debug("boot environment created"))
                .await?;
            be_path
        }
        Err(e) => {
            tx.send(report_install_error(&e)).await?;
            return Err(SendError(Err(Status::internal("Internal error"))));
        }
    };

    match mount_boot_environment(&be_path) {
        Ok(_) => {
            tx.send(report_install_debug("boot environment mounted to /a"))
                .await?;
        }
        Err(e) => {
            tx.send(report_install_error(&e)).await?;
            return Err(SendError(Err(Status::internal("Internal error"))));
        }
    }

    let image_ref = build_image_ref(&mc.image).map_err(|e| {
        SendError(Err(Status::internal(format!(
            "Parsing image reference failed: {}",
            e
        ))))
    })?;

    let image_config = match fetch_image(&image_ref, &config.default_oci_registry, tx.clone()).await
    {
        Ok(image_config) => {
            tx.send(report_install_debug("image fetched")).await?;
            image_config
        }
        Err(e) => {
            tx.send(report_install_error(e)).await?;
            return Err(SendError(Err(Status::internal("Internal error"))));
        }
    };

    install_image(&image_ref, image_config, &tx).await?;

    match make_be_bootable(&be_path) {
        Ok(_) => {
            tx.send(report_install_info("bootenvironment activated"))
                .await?;
        }
        Err(e) => {
            tx.send(report_install_error(e)).await?;
            return Err(SendError(Err(Status::internal("Internal error"))));
        }
    }

    Ok(())
}

pub fn make_be_bootable(be_name: &str) -> Result<(), InstallationError> {
    let pool_name = be_name
        .split("/")
        .next()
        .ok_or_else(|| InstallationError::InvalidBootEnvironmentName)?;
    let beadm_out = Command::new(BEADM_BIN)
        .arg("activate")
        .arg(be_name)
        .output()?;
    if !beadm_out.status.success() {
        return Err(InstallationError::BeadmFailed(String::from_utf8(
            beadm_out.stderr,
        )?));
    }

    let bootadm_install_out = Command::new(BOOTADM_BIN)
        .args([
            "install-bootloader",
            "-M",
            "-f",
            "-P",
            &pool_name,
            "-R",
            "/a",
        ])
        .output()?;
    if !bootadm_install_out.status.success() {
        return Err(InstallationError::InstallBootLoaderFailed(
            String::from_utf8(bootadm_install_out.stderr)?,
        ));
    }

    let bootadm_archive_out = Command::new(BOOTADM_BIN)
        .args(["update-archive", "-f", "-R", "/a"])
        .output()?;
    if !bootadm_archive_out.status.success() {
        return Err(InstallationError::InstallBootLoaderFailed(
            String::from_utf8(bootadm_archive_out.stderr)?,
        ));
    }

    Ok(())
}
