//! Generate the TypeScript bindings for the activity HTTP wire types.
//!
//! Invoked from the workspace root:
//!
//! ```text
//! cargo run -p activity-core --features codegen --bin activity-core-codegen
//! ```
//!
//! Mirrors the codegen pattern used by `crates/app/euro-office`, so the
//! same `pnpm specta:*` style scripts can run all type-generation steps.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use specta_typescript::{BigIntExportBehavior, Typescript};

const TYPESCRIPT_OUT: &str = "apps/desktop/src/lib/bindings/activity.ts";

fn main() -> ExitCode {
    match generate_bindings() {
        Ok(()) => {
            println!("wrote {TYPESCRIPT_OUT}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to generate activity bindings: {err:?}");
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

    let types = activity_core::type_collection();

    Typescript::default()
        .bigint(BigIntExportBehavior::BigInt)
        .export_to(out, &types)
        .context("exporting TypeScript bindings")?;

    Ok(())
}
