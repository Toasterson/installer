use config::File;
use serde::Deserialize;
use miette::{IntoDiagnostic, Result};

#[derive(Deserialize)]
pub struct MachinedConfig {
    pub command_server: String,
    pub listen: String,
    pub public_key: String,
}

pub fn load_config() -> Result<MachinedConfig> {
    let cfg = config::Config::builder()
        // In the installer the defaults get backed in under /etc so we read them first
        // we do not make them mandatory for local debugging
        .add_source(File::with_name("/etc/machined").required(false))
        // We assume that the first USB key gets mounted on /usb so we look for a machined config there
        .add_source(File::with_name("/usb/machined").required(false))
        .set_default("listen", "[::1]:50051").into_diagnostic()?
        .build().into_diagnostic()?;
    Ok(cfg.try_deserialize().into_diagnostic()?)
}