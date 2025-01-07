use std::sync::mpsc::SendError;
use machineconfig::MachineConfig;
use thiserror::Error;
use tokio::sync::mpsc::Sender;
use crate::machined::{InstallProgress, ProgressLevel};
use crate::machined::install_progress::Message;

mod zpool;
mod image;
mod sysconfig;

pub fn install_system(mc: &MachineConfig, tx: Sender<InstallProgress>) -> Result<(), SendError<InstallProgress>> {
    tx.send(InstallProgress{ level: ProgressLevel::Info.into(), message: Some(Message::Info("test".to_string())) })?;
    Ok(())
}