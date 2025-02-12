use crate::config::MachinedConfig;
use crate::machined::InstallProgress;
use crate::platform::illumos::image::{build_image_ref, fetch_image};
use crate::platform::illumos::zpool::{create_boot_environment_base_dataset, create_pool};
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

    let image_ref = build_image_ref(&mc.image)?;

    match fetch_image(&image_ref, &config.default_oci_registry, tx) {
        Ok(_) => {
            tx.send(report_install_debug("image fetched")).await?;
        }
        Err(e) => {
            tx.send(report_install_error(e)).await?;
            return Err(SendError(Err(Status::internal("Internal error"))));
        }
    }
    Ok(())
}
