use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    Knus(#[from] knus::Error),
}

pub fn parse_config(path: &str, content: &str) -> Result<SysConfig, Error> {
    Ok(knus::parse(path, &content)?)
}

#[derive(Debug, knus::Decode, Default)]
pub struct SysConfig {

}