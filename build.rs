use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);
    let protos = [
        PathBuf::from("proto/container.proto"),
        PathBuf::from("proto/images.proto"),
    ];
    let includes = [PathBuf::from("proto"), protoc_bin_vendored::include_path()?];
    tonic_prost_build::configure().compile_protos(&protos, &includes)?;
    Ok(())
}
