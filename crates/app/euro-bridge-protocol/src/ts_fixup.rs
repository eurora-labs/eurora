//! Post-processors applied to TypeScript bindings files that include
//! the bridge protocol envelope.
//!
//! Some wire-protocol types have specta-renderings that don't survive
//! the round-trip through specta-typescript verbatim. The [`Payload`]
//! type — `Box<RawValue>` in Rust — is the canonical example: specta
//! has no native rendering for "any JSON value", so the type is emitted
//! as an empty named struct (`type Payload = [];`) which downstream
//! callers can't actually use. These helpers rewrite those placeholders
//! into the shapes the TypeScript consumers expect.
//!
//! Kept outside the `codegen` feature so every TS-emitting consumer
//! (`euro-browser`, `euro-office`, …) can call the helpers without
//! pulling in specta-swift and anyhow.
//!
//! [`Payload`]: crate::frame::Payload

use std::fs;
use std::io;
use std::path::Path;

/// Rewrite the empty `Payload` alias specta-typescript emits into
/// `type Payload = unknown;`.
///
/// The Rust side declares `Payload` as a `serde(transparent)` newtype
/// wrapping `Box<RawValue>` — i.e. "any JSON value". Specta has no
/// native rendering for this so the inner field is `specta(skip)`'d
/// and the wrapper renders as an empty alias (`type Payload = []`).
/// `unknown` is the honest TypeScript type for a Payload — anything
/// the wire can carry — so this rewrite restores the intended shape.
///
/// Idempotent: a second call on an already-fixed file is a no-op.
/// Returns a structured error if the file can't be read or rewritten.
pub fn rewrite_payload(path: &Path) -> io::Result<()> {
    const PLACEHOLDER: &str = "export type Payload = [];";
    const REPLACEMENT: &str = "export type Payload = unknown;";

    let content = fs::read_to_string(path)?;
    let Some(rewritten) = replace_once(&content, PLACEHOLDER, REPLACEMENT) else {
        // Already rewritten, or specta emitted a shape we don't know
        // about. The latter would be a regression worth catching, but
        // it's better caught by the caller's `git diff --exit-code`
        // guard than by a runtime panic here.
        return Ok(());
    };
    fs::write(path, rewritten)
}

fn replace_once(haystack: &str, needle: &str, replacement: &str) -> Option<String> {
    let idx = haystack.find(needle)?;
    let mut out = String::with_capacity(haystack.len() - needle.len() + replacement.len());
    out.push_str(&haystack[..idx]);
    out.push_str(replacement);
    out.push_str(&haystack[idx + needle.len()..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_once_substitutes_the_first_match() {
        assert_eq!(
            replace_once("a x b x c", "x", "y").as_deref(),
            Some("a y b x c"),
        );
    }

    #[test]
    fn replace_once_returns_none_when_needle_absent() {
        assert_eq!(replace_once("no match here", "x", "y"), None);
    }

    #[test]
    fn rewrite_payload_replaces_empty_alias() -> io::Result<()> {
        let path = scratch_path("rewrite_payload_replaces_empty_alias");
        fs::write(
            &path,
            "/** docs */\nexport type Payload = [];\n\nexport type Other = string;\n",
        )?;
        rewrite_payload(&path)?;
        let content = fs::read_to_string(&path)?;
        assert!(content.contains("export type Payload = unknown;"));
        assert!(content.contains("export type Other = string;"));
        Ok(())
    }

    #[test]
    fn rewrite_payload_is_idempotent() -> io::Result<()> {
        let path = scratch_path("rewrite_payload_is_idempotent");
        fs::write(&path, "export type Payload = unknown;\n")?;
        rewrite_payload(&path)?;
        let content = fs::read_to_string(&path)?;
        assert_eq!(content, "export type Payload = unknown;\n");
        Ok(())
    }

    fn scratch_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "euro-bridge-protocol-ts-fixup-{}-{name}.ts",
            std::process::id(),
        ));
        let _ = fs::remove_file(&path);
        path
    }
}
