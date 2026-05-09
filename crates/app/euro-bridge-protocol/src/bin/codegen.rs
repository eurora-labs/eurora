//! Generate Swift bindings for the bridge wire types.
//!
//! Invoked from the workspace root:
//!
//! ```text
//! cargo run -p euro-bridge-protocol --features codegen -- --generate_specta
//! ```

use anyhow::{Context, Result};
use euro_bridge_protocol::type_collection;
use specta_swift::{NamingConvention, Swift};
use std::env;
use std::fs;
use std::process::ExitCode;

const SWIFT_OUT: &str = "apps/macos/Shared/BridgeProtocol.swift";

fn main() -> ExitCode {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "euro-bridge-protocol".into());

    match args.next().as_deref() {
        Some("--generate_specta") => match generate_bindings() {
            Ok(()) => {
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

    let swift = Swift::default()
        .naming(NamingConvention::PascalCase)
        .export(&types, specta_serde::Format)
        .context("exporting Swift bindings")?;

    fs::write(SWIFT_OUT, collapse_double_optional(&swift)).context("writing Swift bindings")?;

    Ok(())
}

/// Collapse `T??` → `T?` in field declarations.
///
/// specta-serde models `Option<T>` plus `#[serde(default)]` as a nested
/// optional (the field can be missing AND its value can be JSON null), and
/// specta-swift 0.0.3 renders both layers verbatim — `public let foo: T??`.
/// Swift's synthesized `Decodable` already treats a missing key on an
/// `Optional` property as `nil`, so the inner `?` is redundant and `T??`
/// is just an unidiomatic spelling of `T?`. specta-typescript handles the
/// same shape correctly (`foo?: T | null`); Swift does not, so we collapse
/// it here. Drop this once specta-swift learns the same trick.
///
/// `??` is the Swift nil-coalescing operator, but the Specta exporter only
/// emits type declarations, never expressions, so `??` in the generated
/// output is unambiguously the double-Optional bug.
fn collapse_double_optional(input: &str) -> String {
    input.replace("??", "?")
}

#[cfg(test)]
mod tests {
    use super::collapse_double_optional;

    #[test]
    fn collapses_double_optional_in_field_decl() {
        let input = "    public let payload: String??\n";
        assert_eq!(
            collapse_double_optional(input),
            "    public let payload: String?\n"
        );
    }

    #[test]
    fn collapses_double_optional_on_named_types() {
        let input = "    public let asset: ArticleAsset??\n";
        assert_eq!(
            collapse_double_optional(input),
            "    public let asset: ArticleAsset?\n"
        );
    }

    #[test]
    fn leaves_single_optional_alone() {
        let input = "    public let payload: String?\n";
        assert_eq!(collapse_double_optional(input), input);
    }
}
