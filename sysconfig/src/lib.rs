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

    #[knus(children(name = "address"))]
    pub addresses: Vec<AddressObject>,
}

#[derive(Debug, knus::Decode, Default)]
pub struct AddressObject {
    #[knus(property)]
    pub name: String,
    #[knus(property)]
    pub kind: AddressKind,
    #[knus(argument)]
    pub address: Option<String>,
}

#[derive(knus::DecodeScalar, Debug, Default, strum::Display)]
pub enum AddressKind {
    #[default]
    Dhcp4,
    Dhcp6,
    Addrconf,
    Static,
}