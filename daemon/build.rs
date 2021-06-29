fn main() {
    prost_build::Config::new()
        .out_dir("./src/protocol")
        .compile_protos(&["./src/protocol/ipchess.proto"], &["./src/protocol"])
        .expect("failed compiling protobuf files");

    println!("cargo:rerun-if-changed=src/protocol/ipchess.proto");
}
