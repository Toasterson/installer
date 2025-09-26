fn main() {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../sysconfig/proto/sysconfig.proto"],
            &["../sysconfig/proto"],
        )
        .expect("Failed to compile protobuf");
}
