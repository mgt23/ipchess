fn main() {
    prost_build::Config::new()
        .out_dir("./src/protocol")
        .compile_protos(&["./src/protocol/ipchess.proto"], &["./src/protocol"])
        .expect("failed compiling protobuf files");
}
