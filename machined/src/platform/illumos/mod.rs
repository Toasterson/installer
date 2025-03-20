use crate::config::MachinedConfig;
use crate::machined::InstallProgress;
use crate::platform::illumos::image::{build_image_ref, fetch_image, install_image};
use crate::platform::illumos::zpool::{
    create_boot_environment, create_boot_environment_base_dataset, create_pool,
    mount_boot_environment,
};
use crate::util::{report_install_debug, report_install_error};
use machineconfig::MachineConfig;
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;
use tonic::Status;

mod image;
mod sysconfig;
mod zpool;

pub async fn install_system(
    mc: &MachineConfig,
    config: Arc<MachinedConfig>,
    tx: Sender<Result<InstallProgress, Status>>,
) -> Result<(), SendError<Result<InstallProgress, Status>>> {
    tx.send(report_install_debug("Starting installation"))
        .await?;
    for pool in mc.pools {
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

    match mount_boot_environment(be_path) {
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

    install_image(&image_ref, image_config, tx).await?;
    Ok(())
}
