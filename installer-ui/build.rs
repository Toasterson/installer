fn main() {
    // Generate gRPC client from machined's proto
    let proto = "../machined/proto/machined.proto";
    println!("cargo:rerun-if-changed={}", proto);
    tonic_build::configure()
        .build_server(false)
        .compile(&[proto], &["../machined/proto"]) // include path is the proto dir
        .expect("failed to compile machined.proto");
}