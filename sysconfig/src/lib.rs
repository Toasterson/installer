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
    #[knus(child, unwrap(argument))]
    pub hostname: String,

    #[knus(children(name = "nameserver"), unwrap(argument))]
    pub nameservers: Vec<String>,

    #[knus(children(name = "interface"))]
    pub interfaces: Vec<Interface>,
}

#[derive(Debug, knus::Decode, Default)]
pub struct Interface {
    #[knus(argument)]
    pub name: Option<String>,

    #[knus(property)]
    pub selector: Option<String>,
}