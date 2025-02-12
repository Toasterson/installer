use crate::machined::install_progress::Message;
use crate::machined::{InstallProgress, ProgressLevel};
use std::error::Error;
use tonic::Status;

pub fn report_install_info(msg: &str) -> Result<InstallProgress, Status> {
    Ok(InstallProgress {
        level: ProgressLevel::Info.into(),
        message: Some(Message::Info(msg.into())),
    })
}

pub fn report_install_warning(msg: &str) -> Result<InstallProgress, Status> {
    Ok(InstallProgress {
        level: ProgressLevel::Warning.into(),
        message: Some(Message::Error(msg.into())),
    })
}

pub fn report_install_error<E: Error>(err: E) -> Result<InstallProgress, Status> {
    Ok(InstallProgress {
        level: ProgressLevel::Error.into(),
        message: Some(Message::Error(err.to_string())),
    })
}

pub fn report_install_debug(msg: &str) -> Result<InstallProgress, Status> {
    Ok(InstallProgress {
        level: ProgressLevel::Debug.into(),
        message: Some(Message::Info(msg.into())),
    })
}
