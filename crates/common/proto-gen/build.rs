use std::{
    io::Result,
    path::{Path, PathBuf},
};

fn main() -> Result<()> {
    let proto_dir = PathBuf::from("../../../proto");
    let proto_files = std::fs::read_dir(&proto_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "proto"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    watch_protos_recursively(&proto_dir);

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

fn watch_protos_recursively(dir: &Path) {
    fn walk(d: &Path) {
        let Ok(entries) = std::fs::read_dir(d) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("proto") {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }
    }
    walk(dir);
}
