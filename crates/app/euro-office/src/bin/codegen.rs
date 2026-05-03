//! Generate the TypeScript bindings consumed by the Office add-in.
//!
//! Invoked from the workspace root:
//!
//! ```text
//! cargo run -p euro-office --features codegen -- --generate_specta
//! ```

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use euro_office::type_collection;
use specta_typescript::{BigIntExportBehavior, Typescript};

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
    let out = Path::new(TYPESCRIPT_OUT);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }

    let types = type_collection();

    Typescript::default()
        .bigint(BigIntExportBehavior::Fail)
        .export_to(out, &types)
        .context("exporting TypeScript bindings")?;

    Ok(())
}
