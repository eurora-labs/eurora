//! TypeScript bindings for the Office add-in, emitted into
//! `apps/office-addin/src/shared/bindings.ts`.
//!
//! Gated behind the `codegen` feature. The workspace-level
//! `euro-codegen` orchestrator is the only entry point — see the crate
//! root for context.

use std::path::Path;

use anyhow::{Context, Result};
use euro_bridge_protocol::ts_fixup;
use specta_typescript::Typescript;

use crate::type_collection;

const TYPESCRIPT_OUT: &str = "apps/office-addin/src/shared/bindings.ts";

/// Generate the TypeScript bindings and write them to [`TYPESCRIPT_OUT`].
pub fn run() -> Result<()> {
    let out = Path::new(TYPESCRIPT_OUT);
    Typescript::default()
        .export_to(out, &type_collection(), specta_serde::Format)
        .context("exporting TypeScript bindings")?;
    ts_fixup::rewrite_payload(out).context("post-processing Payload alias")?;
    println!("wrote {TYPESCRIPT_OUT}");

    Ok(())
}
