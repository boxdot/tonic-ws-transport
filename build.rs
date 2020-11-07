fn main() {
    tonic_build::compile_protos("examples/protos/helloworld.proto").unwrap();
}
