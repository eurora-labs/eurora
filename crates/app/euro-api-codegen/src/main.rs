//! Emit TypeScript bindings for every Eurora HTTP service wire type.
//!
//! Invoked from the workspace root:
//!
//! ```text
//! pnpm specta:backend
//! # or
//! cargo run -p euro-api-codegen
//! ```
//!
//! Each `*-core` crate owns its own `type_collection()` function under the
//! `specta` feature; this binary orchestrates them so the workspace has a
//! single source of truth for the output directory and TypeScript settings.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use specta::TypeCollection;
use specta_typescript::{BigIntExportBehavior, Typescript};

const OUTPUT_DIR: &str = "packages/shared/src/lib/bindings";

struct Service {
    /// Filename stem; the emitted file is `<name>.ts`.
    name: &'static str,
    types: fn() -> TypeCollection,
}

const SERVICES: &[Service] = &[
    Service {
        name: "activity",
        types: activity_core::type_collection,
    },
    Service {
        name: "asset",
        types: asset_core::type_collection,
    },
    Service {
        name: "auth",
        types: auth_core::type_collection,
    },
    Service {
        name: "thread",
        types: thread_core::type_collection,
    },
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("euro-api-codegen failed: {err:?}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let out_dir = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating output directory {}", out_dir.display()))?;

    let exporter = Typescript::default().bigint(BigIntExportBehavior::BigInt);

    for service in SERVICES {
        let path = out_dir.join(format!("{}.ts", service.name));
        write_bindings(&exporter, &(service.types)(), &path)
            .with_context(|| format!("emitting {}", path.display()))?;
        println!("wrote {}", path.display());
    }

    Ok(())
}

fn write_bindings(exporter: &Typescript, types: &TypeCollection, path: &Path) -> Result<()> {
    exporter
        .export_to(path, types)
        .context("exporting TypeScript bindings")?;
    Ok(())
}
