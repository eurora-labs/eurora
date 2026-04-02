use std::io::Result;

fn main() -> Result<()> {
    #[cfg(feature = "prost")]
    {
        use std::path::PathBuf;

        let proto_dir = PathBuf::from("../../../proto");
        let proto_file = proto_dir.join("agent_chain.proto");

        println!("cargo:rerun-if-changed={}", proto_file.display());

        let build_server = std::env::var("CARGO_FEATURE_SERVER").is_ok();

        #[cfg(target_os = "windows")]
        {
            let common_dir = PathBuf::from("C:\\protoc\\include");
            tonic_prost_build::configure()
                .build_server(build_server)
                .build_client(true)
                .protoc_arg("--experimental_allow_proto3_optional")
                .compile_protos(&[&proto_file], &[&proto_dir, &common_dir])?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            tonic_prost_build::configure()
                .build_server(build_server)
                .build_client(true)
                .protoc_arg("--experimental_allow_proto3_optional")
                .compile_protos(&[&proto_file], &[&proto_dir])?;
        }
    }
    Ok(())
}
