use crate::util::report_install_debug;
use crate::ProgressMessage;
use machineconfig::MachineConfig;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;

pub async fn install_system(mc: &MachineConfig, tx: Sender<ProgressMessage>) -> Result<(), SendError<ProgressMessage>> {
    tx.send(report_install_debug("test")).await?;
    Ok(())
}