#[cfg(test)]
mod tests {

    use miette::IntoDiagnostic;
    use std::fs;

    #[test]
    fn it_works() -> miette::Result<()> {
        let sample_string = fs::read_to_string("sample.kdl").into_diagnostic()?;
        let cfg = crate::parse_config("sample.kdl", &sample_string).unwrap();
        assert_eq!(cfg.sysconfig.hostname, "node01");
        assert_eq!(cfg.sysconfig.nameservers.len(), 2);
        assert_eq!(cfg.sysconfig.nameservers[0], String::from("9.9.9.9"));
        assert_eq!(cfg.pools[0].name, "rpool");
        assert_eq!(cfg.pools[0].options[0].name, "compression");
        assert_eq!(cfg.pools[0].options[0].value, "zstd");
        Ok(())
    }
}

use miette::Diagnostic;
use sysconfig::SysConfig;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    Knus(#[from] knus::Error),
}

pub fn parse_config(path: &str, content: &str) -> Result<MachineConfig, Error> {
    Ok(knus::parse(path, &content)?)
}

#[derive(Debug, knus::Decode, Default)]
pub struct MachineConfig {
    #[knus(children(name = "pool"))]
    pub pools: Vec<Pool>,

    #[knus(child, unwrap(argument))]
    pub image: String,
    
    #[knus(child)]
    pub sysconfig: SysConfig
}

#[derive(Debug, knus::Decode, Default)]
pub struct Pool {
    #[knus(argument)]
    pub name: String,

    #[knus(children(name = "vdev"))]
    pub vdevs: Vec<VDev>,

    #[knus(child, unwrap(children))]
    pub options: Vec<PoolOption>,
}

#[derive(Debug, knus::Decode, Default)]
pub struct PoolOption {
    #[knus(node_name)]
    name: String,
    #[knus(argument)]
    value: String,
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
