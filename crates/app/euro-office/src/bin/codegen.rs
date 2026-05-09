//! Generate the TypeScript bindings consumed by the Office add-in.
//!
//! Invoked from the workspace root:
//!
//! ```text
//! cargo run -p euro-office --features codegen -- --generate_specta
//! ```

use std::env;
use std::process::ExitCode;

use anyhow::{Context, Result};
use euro_office::type_collection;
use specta_typescript::Typescript;

const TYPESCRIPT_OUT: &str = "apps/office-addin/src/shared/bindings.ts";

fn main() -> ExitCode {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "euro-office".into());

    match args.next().as_deref() {
        Some("--generate_specta") => match generate_bindings() {
            Ok(()) => {
                println!("wrote {TYPESCRIPT_OUT}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("failed to generate bindings: {err:?}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage: {program} --generate_specta");
            ExitCode::FAILURE
        }
    }
}

fn generate_bindings() -> Result<()> {
    let types = type_collection();

    Typescript::default()
        .export_to(TYPESCRIPT_OUT, &types, specta_serde::Format)
        .context("exporting TypeScript bindings")?;

    Ok(())
}
