use std::io::Result;

fn main() -> Result<()> {
    tonic_build::compile_protos("proto/sysconfig.proto")?;
    Ok(())
}