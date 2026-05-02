fn main() {
    println!("cargo:rerun-if-changed=proto/omv/contract/versions/current/contract.proto");

    prost_build::Config::new()
        .compile_protos(
            &["proto/omv/contract/versions/current/contract.proto"],
            &["proto"],
        )
        .expect("failed to generate OMV protobuf contracts; install protoc for source builds");
}
