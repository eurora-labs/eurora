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
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use specta::Types;
use specta_typescript::Typescript;

const OUTPUT_DIR: &str = "packages/shared/src/lib/bindings";

struct Service {
    /// Filename stem; the emitted file is `<name>.ts`.
    name: &'static str,
    types: fn() -> Types,
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

    let exporter = Typescript::default();

    // Wire types are serialize/deserialize-symmetric by policy: `serde(default)`
    // is fine, but `skip_serializing_if`, `serde(with = ...)`, `serde(into = ...)`,
    // and `serde(from = ...)` are all directional and will make this binary
    // fail. That's the strictness we want — it keeps the bindings unified
    // (no `_Serialize`/`_Deserialize` phase pairs) and forces optional fields
    // to land as explicit `null`s on the wire instead of being silently omitted.
    for service in SERVICES {
        let path = out_dir.join(format!("{}.ts", service.name));
        exporter
            .export_to(&path, &(service.types)(), specta_serde::Format)
            .with_context(|| format!("emitting {}", path.display()))?;
        println!("wrote {}", path.display());
    }

    Ok(())
}
