fn main() {
    tonic_build::configure()
        .build_server(false)
        .compile(&["protos/helloworld.proto"], &["protos"])
        .unwrap();
}
