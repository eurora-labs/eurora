use std::{io::Result, path::PathBuf};

fn main() -> Result<()> {
    let proto_dir = PathBuf::from("../../../proto");
    let proto_files = vec![PathBuf::from("assets_service.proto")];

    let build_server = std::env::var("CARGO_FEATURE_SERVER").is_ok();

    #[cfg(target_os = "windows")]
    {
        let common_dir = PathBuf::from("C:\\protoc\\include");
        tonic_prost_build::configure()
            .build_server(build_server)
            .build_client(true)
            .protoc_arg("--experimental_allow_proto3_optional")
            .compile_protos(&proto_files, &[proto_dir, common_dir])?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        tonic_prost_build::configure()
            .build_server(build_server)
            .build_client(true)
            .protoc_arg("--experimental_allow_proto3_optional")
            .compile_protos(&proto_files, &[proto_dir])?;
    }
    Ok(())
}
