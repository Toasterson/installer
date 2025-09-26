use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    // Compile the sysconfig proto file
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile(
            &["../sysconfig/proto/sysconfig.proto"],
            &["../sysconfig/proto"],
        )?;

    Ok(())
}
