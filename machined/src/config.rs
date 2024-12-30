use config::File;
use miette::{miette, IntoDiagnostic, Result};
use passwords::PasswordGenerator;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default, Clone)]
pub struct MachinedConfig {
    pub server: Option<CommandServer>,
    pub listen: String,
    pub claim_key: Option<ClaimKey>,
    pub claim_password: String,
    pub wireguard: Option<WireguardConfig>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct WireguardConfig {
    pub server: String,
    pub private_key: String,
    pub server_public_key: String,
    pub peers: Vec<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct ClaimKey {
    pub private_key: String,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct CommandServer {
    pub url: String,
    pub public_key: String,
}

pub fn load_config() -> Result<MachinedConfig> {
    let pg = PasswordGenerator::new()
        .length(8)
        .lowercase_letters(true)
        .uppercase_letters(true)
        .exclude_similar_characters(true)
        .spaces(false)
        .numbers(true)
        .symbols(false)
        .strict(false);

    let claim_password = pg
        .generate_one()
        .map_err(|e| miette!("error generating password {e}"))?;

    let cfg = config::Config::builder()
        // In the installer the defaults get backed in under /etc so we read them first
        // we do not make them mandatory for local debugging
        .add_source(File::with_name("/etc/machined").required(false))
        // We assume that the first USB key gets mounted on /usb so we look for a machined config there
        .add_source(File::with_name("/usb/machined").required(false))
        .set_default("listen", "[::1]:50051").into_diagnostic()?
        .set_default("claim_password", claim_password).into_diagnostic()?
        .build().into_diagnostic()?;
    Ok(cfg.try_deserialize().into_diagnostic()?)
}