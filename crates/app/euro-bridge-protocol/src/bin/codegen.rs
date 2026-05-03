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
const SWIFT_OUT: &str = "apps/macos/Shared/BridgeProtocol.swift";

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
/// 3. Externally-tagged enum `Codable` — when an enum variant carries a single
///    payload of an existing named type (e.g. `case request(RequestFrame)`),
///    specta-swift 0.0.1 emits `public enum X: Codable { … }` but no custom
///    `init(from:)`/`encode(to:)`. Swift's synthesized impl uses an
///    internally-tagged shape (`{"request":{"_0":…}}`) that does not match
///    Rust's externally-tagged JSON (`{"Request":{…}}`). Strip the `: Codable`
///    on those declarations and append a working extension that round-trips
///    against serde's externally-tagged form.
fn polish_swift(input: &str) -> String {
    let dropped: String = input
        .lines()
        .filter(|line| line.trim() != "import Codable")
        .map(|line| line.replace("String??", "String?"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    inject_externally_tagged_enums(&dropped)
}

#[derive(Debug)]
struct EnumVariant {
    /// Swift case name, e.g. `request`.
    case_name: String,
    /// Single tuple payload type, e.g. `RequestFrame`.
    payload: String,
}

#[derive(Debug)]
struct EnumDecl {
    name: String,
    variants: Vec<EnumVariant>,
}

/// Detect every `public enum <Name>: Codable { case <v>(<T>) … }` block whose
/// variants are all single-payload tuple cases referencing an existing named
/// type, drop the broken `: Codable` conformance, and append a working
/// externally-tagged extension at the end of the file.
fn inject_externally_tagged_enums(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut output: Vec<String> = Vec::with_capacity(lines.len());
    let mut decls: Vec<EnumDecl> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(name) = parse_codable_enum_header(line) {
            if let Some((variants, end)) = parse_enum_body(&lines, i + 1)
                && variants.iter().all(|v| !v.payload.is_empty())
            {
                output.push(line.replacen(": Codable", "", 1));
                for body_line in &lines[i + 1..=end] {
                    output.push((*body_line).to_string());
                }
                decls.push(EnumDecl { name, variants });
                i = end + 1;
                continue;
            }
        }
        output.push(line.to_string());
        i += 1;
    }

    let mut result = output.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    for decl in &decls {
        result.push('\n');
        result.push_str(&render_externally_tagged_extension(decl));
    }
    result
}

/// Match `public enum FrameKind: Codable {` (and the same with a trailing
/// space before `{`). Returns the enum name on success.
fn parse_codable_enum_header(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("public enum ")?;
    let (name, after) = rest.split_once(':')?;
    let after = after.trim_start();
    let after = after.strip_prefix("Codable")?;
    let after = after.trim_start();
    after.strip_prefix('{')?;
    Some(name.trim().to_string())
}

/// Walk the enum body starting at `start` (line just after the `{`). Returns
/// `(variants, end_line)` where `end_line` is the index of the closing `}`,
/// provided every non-blank line in the body is a single-payload tuple case
/// (`    case foo(Bar)`). Bails out otherwise.
fn parse_enum_body(lines: &[&str], start: usize) -> Option<(Vec<EnumVariant>, usize)> {
    let mut variants = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed == "}" {
            return Some((variants, i));
        }
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let case_body = trimmed.strip_prefix("case ")?;
        let (case_name, payload) = case_body.split_once('(')?;
        let payload = payload.strip_suffix(')')?;
        if payload.contains(',') {
            return None;
        }
        variants.push(EnumVariant {
            case_name: case_name.trim().to_string(),
            payload: payload.trim().to_string(),
        });
        i += 1;
    }
    None
}

/// Convert a Swift case name (`request`) to its serde externally-tagged
/// JSON key (`Request`). Specta uses Rust enum identifier as the variant
/// tag, which is PascalCase by convention, but downcases the first letter
/// for the Swift case name. Reverse that: uppercase the first character.
fn pascal_case(case_name: &str) -> String {
    let mut chars = case_name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn render_externally_tagged_extension(decl: &EnumDecl) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "// MARK: - {name} Codable Implementation\nextension {name}: Codable {{\n",
        name = decl.name,
    ));
    out.push_str("    private enum CodingKeys: String, CodingKey {\n");
    for v in &decl.variants {
        out.push_str(&format!(
            "        case {case} = \"{tag}\"\n",
            case = v.case_name,
            tag = pascal_case(&v.case_name),
        ));
    }
    out.push_str("    }\n\n");

    out.push_str("    public init(from decoder: Decoder) throws {\n");
    out.push_str("        let container = try decoder.container(keyedBy: CodingKeys.self)\n");
    out.push_str(
        "        guard container.allKeys.count == 1, let key = container.allKeys.first else {\n",
    );
    out.push_str("            throw DecodingError.dataCorrupted(\n");
    out.push_str("                DecodingError.Context(\n");
    out.push_str("                    codingPath: decoder.codingPath,\n");
    out.push_str("                    debugDescription: \"Expected exactly one key for externally-tagged enum\"\n");
    out.push_str("                )\n");
    out.push_str("            )\n");
    out.push_str("        }\n");
    out.push_str("        switch key {\n");
    for v in &decl.variants {
        out.push_str(&format!(
            "        case .{case}:\n            self = .{case}(try container.decode({payload}.self, forKey: .{case}))\n",
            case = v.case_name,
            payload = v.payload,
        ));
    }
    out.push_str("        }\n");
    out.push_str("    }\n\n");

    out.push_str("    public func encode(to encoder: Encoder) throws {\n");
    out.push_str("        var container = encoder.container(keyedBy: CodingKeys.self)\n");
    out.push_str("        switch self {\n");
    for v in &decl.variants {
        out.push_str(&format!(
            "        case .{case}(let value):\n            try container.encode(value, forKey: .{case})\n",
            case = v.case_name,
        ));
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::{
        inject_externally_tagged_enums, parse_codable_enum_header, parse_enum_body, polish_swift,
    };

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

    #[test]
    fn parses_codable_enum_header() {
        assert_eq!(
            parse_codable_enum_header("public enum FrameKind: Codable {").as_deref(),
            Some("FrameKind"),
        );
        assert_eq!(
            parse_codable_enum_header("public enum FrameKind : Codable {").as_deref(),
            Some("FrameKind"),
        );
        assert!(parse_codable_enum_header("public struct Frame: Codable {").is_none());
    }

    #[test]
    fn parses_enum_body_with_tuple_variants() {
        let lines = vec![
            "public enum FrameKind: Codable {",
            "    case request(RequestFrame)",
            "    case event(EventFrame)",
            "}",
        ];
        let (variants, end) = parse_enum_body(&lines, 1).expect("parse body");
        assert_eq!(end, 3);
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].case_name, "request");
        assert_eq!(variants[0].payload, "RequestFrame");
    }

    #[test]
    fn injects_externally_tagged_extension() {
        let input = "\
public enum FrameKind: Codable {
    case request(RequestFrame)
    case event(EventFrame)
}
";
        let polished = inject_externally_tagged_enums(input);

        // Stripped the broken synthesized Codable conformance.
        assert!(polished.contains("public enum FrameKind {\n"));
        assert!(!polished.contains("public enum FrameKind: Codable"));

        // Generated a working extension.
        assert!(polished.contains("extension FrameKind: Codable {"));
        assert!(polished.contains("case request = \"Request\""));
        assert!(polished.contains("case event = \"Event\""));
        assert!(polished.contains(
            "self = .request(try container.decode(RequestFrame.self, forKey: .request))"
        ));
        assert!(polished.contains("try container.encode(value, forKey: .request)"));
    }

    #[test]
    fn leaves_unrelated_enums_alone() {
        let input = "\
public enum HttpStatus: Codable {
    case ok
    case notFound
}
";
        // Unit variants (no payload) — bail out and leave as-is.
        let polished = inject_externally_tagged_enums(input);
        assert!(polished.contains("public enum HttpStatus: Codable {"));
        assert!(!polished.contains("extension HttpStatus: Codable"));
    }
}
