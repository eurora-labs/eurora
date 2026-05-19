//! Re-exports consumed by `eurora-tools-macros` emissions.
//!
//! Not part of the public API. The procedural macros emit
//! `::eurora_tools::__private::*` paths so adapter crates need only
//! `eurora-tools` in their dependency tree, with no transitive surface
//! to break under unrelated dep upgrades.

pub use futures;
pub use schemars;
pub use serde;
pub use serde_json;
pub use trait_variant;
