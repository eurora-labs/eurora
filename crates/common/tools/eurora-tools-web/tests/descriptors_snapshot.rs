//! Snapshot of the wire form of `WEB_DESCRIPTORS`.
//!
//! Pins the runtime contract — tool names, descriptions, schemas,
//! timeouts, sources, contexts, approval flags — that downstream
//! consumers (the server agent loop, the client `ChatBridge`) depend
//! on. Any change to the trait, its rustdoc, or the argument/return
//! types lands here as a reviewable diff.
//!
//! Regenerate with:
//!
//! ```sh
//! INSTA_UPDATE=auto cargo test -p eurora-tools-web --test descriptors_snapshot
//! ```

use eurora_tools_web::WEB_DESCRIPTORS;

#[test]
fn descriptor_table_snapshot() {
    let wire: Vec<_> = WEB_DESCRIPTORS.iter().map(|d| d.to_wire()).collect();
    insta::assert_debug_snapshot!(wire);
}
