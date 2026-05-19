//! Using `#[tool]` outside an `#[adapter]` trait should fail with a
//! clear, single-shot diagnostic that points the user to `#[adapter]`.

use eurora_tools::tool;

#[tool(timeout_ms = 100, source = "client_local")]
fn standalone() {}

fn main() {}
