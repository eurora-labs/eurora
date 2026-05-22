//! Workspace-level binding generator.
//!
//! Single entry point that emits every TypeScript and Swift bindings
//! file Eurora's frontends consume:
//!
//! - `apps/desktop/src/lib/bindings/specta.bindings.ts` — tauri-specta
//!   IPC surface (every desktop `#[tauri::command]` and event).
//! - `apps/browser/src/shared/content/bindings.ts` — bridge envelope
//!   types plus the browser-extension payload shapes.
//! - `apps/office-addin/src/shared/bindings.ts` — bridge envelope plus
//!   the Office add-in payload shapes.
//! - `apps/macos/Shared/BridgeProtocol.swift` — bridge envelope as
//!   Swift `Codable` types, consumed by the macOS Safari extension.
//! - `packages/shared/src/lib/bindings/{activity,asset,auth,settings,thread}.ts`
//!   — backend HTTP service wire types.
//!
//! Invoked from the workspace root via `cargo run -p euro-codegen`. CI
//! runs the binary then `git diff --exit-code` over the output
//! directories so a wire-type edit that doesn't regenerate fails the
//! build.
//!
//! Wire-type policy: every type that participates in any bindings file
//! must be serialize/deserialize-symmetric. `serde(default)` is fine,
//! but `skip_serializing_if`, `serde(with = ...)`, `serde(into = ...)`,
//! and `serde(from = ...)` are all directional and will make this
//! binary fail. That's the strictness we want — it keeps the bindings
//! unified (no `_Serialize`/`_Deserialize` phase pairs) and forces
//! optional fields to land as explicit `null`s on the wire instead of
//! being silently omitted.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use specta::Types;
use specta_typescript::Typescript;

/// Output directory for the backend HTTP service bindings, mirrored as
/// the `@eurora/shared` workspace package.
const BACKEND_OUT_DIR: &str = "packages/shared/src/lib/bindings";

/// Backend HTTP services whose wire types are emitted as
/// `{name}.ts` under [`BACKEND_OUT_DIR`].
struct BackendService {
    /// Filename stem; the emitted file is `<name>.ts`.
    name: &'static str,
    types: fn() -> Types,
}

const BACKEND_SERVICES: &[BackendService] = &[
    BackendService {
        name: "activity",
        types: activity_core::type_collection,
    },
    BackendService {
        name: "asset",
        types: asset_core::type_collection,
    },
    BackendService {
        name: "auth",
        types: auth_core::type_collection,
    },
    BackendService {
        name: "settings",
        types: settings_core::type_collection,
    },
    BackendService {
        name: "thread",
        types: thread_core::type_collection,
    },
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("euro-codegen failed: {err:?}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    export_backend_services().context("backend HTTP service bindings")?;
    euro_bridge_protocol::codegen::run().context("bridge protocol Swift bindings")?;
    euro_browser::codegen::run().context("browser extension TypeScript bindings")?;
    euro_office::codegen::run().context("Office add-in TypeScript bindings")?;
    euro_tauri::export_desktop_bindings(Path::new(euro_tauri::DESKTOP_BINDINGS_PATH))
        .context("desktop tauri-specta TypeScript bindings")?;
    Ok(())
}

fn export_backend_services() -> Result<()> {
    let out_dir = PathBuf::from(BACKEND_OUT_DIR);
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating output directory {}", out_dir.display()))?;

    let exporter = Typescript::default();

    for service in BACKEND_SERVICES {
        let path = out_dir.join(format!("{}.ts", service.name));
        exporter
            .export_to(&path, &(service.types)(), specta_serde::Format)
            .with_context(|| format!("emitting {}", path.display()))?;
        println!("wrote {}", path.display());
    }

    Ok(())
}
