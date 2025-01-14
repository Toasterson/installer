#[cfg(test)]
mod tests {

    use miette::IntoDiagnostic;
    use std::fs;

    #[test]
    fn it_works() -> miette::Result<()> {
        let sample_string = fs::read_to_string("sample.kdl").into_diagnostic()?;
        let cfg = crate::parse_config("sample.kdl", &sample_string).unwrap();
        assert_eq!(cfg.hostname, "node01");
        assert_eq!(cfg.nameservers.len(), 2);
        assert_eq!(cfg.nameservers[0], String::from("9.9.9.9"));
        assert_eq!(cfg.pools[0].name, "rpool");
        assert_eq!(cfg.pools[0].compression, "zstd");
        Ok(())
    }
}

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    Knus(#[from] knus::Error),
}

pub fn parse_config(path: &str, content: &str) -> Result<MachineConfig, knus::Error> {
    Ok(knus::parse(path, &content)?)
}

#[derive(Debug, knus::Decode, Default)]
pub struct MachineConfig {
    #[knus(children(name = "pool"))]
    pub pools: Vec<Pool>,

    #[knus(child, unwrap(argument))]
    pub image: String,

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

#[derive(Debug, knus::Decode, Default)]
pub struct Pool {
    #[knus(argument)]
    pub name: String,

    #[knus(property)]
    pub compression: String,

    #[knus(children)]
    pub vdevs: Vec<VDev>,
}

#[derive(Debug, knus::Decode, Default)]
pub struct VDev {
    #[knus(argument)]
    pub kind: VDevType,

    #[knus(child, unwrap(arguments))]
    pub disks: Vec<String>,
}

#[derive(knus::DecodeScalar, Debug, Default, strum::Display)]
pub enum VDevType {
    #[default]
    Mirror,
    RaidZ,
    RaidZ1,
    RaidZ2,
    RaidZ3,
    Spare,
    Log,
    Debup,
    Special,
    Cache,
}
