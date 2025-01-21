use crate::error::InstallationError;
use crate::machined::InstallProgress;
use crate::util::{report_install_debug, report_install_error};
use machineconfig::MachineConfig;
use tokio::sync::mpsc::Sender;

mod image;
mod sysconfig;
mod zpool;

pub fn install_system(
    mc: &MachineConfig,
    tx: Sender<InstallProgress>,
) -> Result<(), InstallationError> {
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
                tx.send(report_install_error(
                    format!("Pool {} could not be created: {}", &pool.name, e).as_str(),
                ))
                .await?;
                return Err(e);
            }
        }
    }
    Ok(())
}
