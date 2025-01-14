use crate::machined::install_progress::Message;
use crate::machined::{InstallProgress, ProgressLevel};
use machineconfig::MachineConfig;
use std::sync::mpsc::SendError;
use tokio::sync::mpsc::Sender;

mod image;
mod sysconfig;
mod zpool;

pub fn install_system(
    mc: &MachineConfig,
    tx: Sender<InstallProgress>,
) -> Result<(), SendError<InstallProgress>> {
    tx.send(InstallProgress {
        level: ProgressLevel::Info.into(),
        message: Some(Message::Info("test".to_string())),
    })?;
    Ok(())
}
