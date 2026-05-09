//! Standalone exporter for the tauri-specta TypeScript bindings.
//!
//! Run with `cargo run -p euro-tauri --bin gen-bindings`. Writes to
//! `apps/desktop/src/lib/bindings/specta.bindings.ts`. Equivalent to
//! the debug-only export in `main.rs`, but doesn't require launching
//! the Tauri runtime.

use specta_typescript::Typescript;

fn main() {
    let bindings_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../apps/desktop/src/lib/bindings/specta.bindings.ts");

    euro_tauri::build_specta()
        .export(Typescript::default(), &bindings_path)
        .expect("Failed to export tauri-specta bindings");

    println!("Wrote {}", bindings_path.display());
}
