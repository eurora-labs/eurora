//! Generate TypeScript and Swift bindings for the bridge wire types.
//!
//! Invoked from the workspace root:
//!
//! ```text
//! cargo run -p euro-bridge-protocol --features codegen -- --generate_specta
//! ```

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use euro_bridge_protocol::type_collection;
use specta_swift::{NamingConvention, SerdeMode, Swift};
use specta_typescript::{BigIntExportBehavior, Typescript};

const TYPESCRIPT_OUT: &str = "apps/browser/src/shared/content/bridge-protocol.ts";
const SWIFT_OUT: &str = "apps/macos/macos/BridgeProtocol.swift";

fn main() -> ExitCode {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "euro-bridge-protocol".into());

    match args.next().as_deref() {
        Some("--generate_specta") => match generate_bindings() {
            Ok(()) => {
                println!("wrote {TYPESCRIPT_OUT}");
                println!("wrote {SWIFT_OUT}");
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
        .bigint(BigIntExportBehavior::Fail)
        .export_to(Path::new(TYPESCRIPT_OUT), &types)
        .context("exporting TypeScript bindings")?;

    let swift = Swift::default()
        .with_serde(SerdeMode::Both)
        .naming(NamingConvention::PascalCase)
        .export(&types)
        .context("exporting Swift bindings")?;
    fs::write(SWIFT_OUT, polish_swift(&swift)).context("writing Swift bindings")?;

    Ok(())
}

/// Patch over a couple of known specta-swift 0.0.1 quirks so the generated
/// file is valid Swift. Drop these once we can move to specta-swift 0.0.2+
/// (which requires bumping the workspace specta to 2.0.0-rc.24).
///
/// 1. `import Codable` — `Codable` is a `Foundation` typealias, not a module;
///    the line must go or `swiftc` rejects the file.
/// 2. `String??` — fields typed `Option<String>` plus `#[serde(default)]` come
///    out as a double-Optional. Collapse to a single `?`.
fn polish_swift(input: &str) -> String {
    input
        .lines()
        .filter(|line| line.trim() != "import Codable")
        .map(|line| line.replace("String??", "String?"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::polish_swift;

    #[test]
    fn drops_bogus_codable_import() {
        let input = "import Foundation\nimport Codable\n\nstruct X {}\n";
        assert_eq!(polish_swift(input), "import Foundation\n\nstruct X {}\n");
    }

    #[test]
    fn collapses_double_optional_string() {
        let input = "    public let payload: String??\n";
        assert_eq!(polish_swift(input), "    public let payload: String?\n");
    }
}
