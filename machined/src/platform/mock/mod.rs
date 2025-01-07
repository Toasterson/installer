use crate::machined::InstallProgress;
use crate::util::report_install_debug;
use machineconfig::MachineConfig;
use std::sync::mpsc::SendError;
use tokio::sync::mpsc::Sender;

pub fn install_system(mc: &MachineConfig, tx: Sender<InstallProgress>) -> Result<(), SendError<InstallProgress>> {
    tx.send(report_install_debug("test"))?;
    Ok(())
}