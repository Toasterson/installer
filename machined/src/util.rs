use crate::machined::install_progress::Message;
use crate::machined::{InstallProgress, ProgressLevel};
use crate::ProgressMessage;

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

pub fn report_install_error(msg: &str) -> ProgressMessage {
    Ok(InstallProgress {
        level: ProgressLevel::Error.into(),
        message: Some(Message::Error(msg.into())),
    })
}

pub fn report_install_debug(msg: &str) -> ProgressMessage {
    Ok(InstallProgress {
        level: ProgressLevel::Debug.into(),
        message: Some(Message::Info(msg.into())),
    })
}
