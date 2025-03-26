use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let proto_dir = PathBuf::from("../../../proto");
    let proto_files = std::fs::read_dir(&proto_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "proto"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    println!("cargo:rerun-if-changed=../../../proto/");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/gen") // Output the generated files in a specific directory
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&proto_files, &[proto_dir])?;

    Ok(())
}
