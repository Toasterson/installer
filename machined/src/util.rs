use crate::machined::install_progress::Message;
use crate::machined::{InstallProgress, ProgressLevel};
use crate::ProgressMessage;
use std::error::Error;

pub fn report_install_info(msg: &str) -> ProgressMessage {
    Ok(InstallProgress {
        level: ProgressLevel::Info.into(),
        message: Some(Message::Info(msg.into())),
    })
}

pub fn report_install_warning(msg: &str) -> ProgressMessage {
    Ok(InstallProgress {
        level: ProgressLevel::Warning.into(),
        message: Some(Message::Error(msg.into())),
    })
}

pub fn report_install_error<E: Error>(err: E) -> ProgressMessage {
    Ok(InstallProgress {
        level: ProgressLevel::Error.into(),
        message: Some(Message::Error(err.to_string())),
    })
}

pub fn report_install_debug(msg: &str) -> ProgressMessage {
    Ok(InstallProgress {
        level: ProgressLevel::Debug.into(),
        message: Some(Message::Info(msg.into())),
    })
}
