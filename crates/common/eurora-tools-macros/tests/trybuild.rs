//! Compile-pass / compile-fail cases for the adapter macro.
//!
//! `trybuild` invokes `rustc` on every `.rs` file under each directory
//! and either expects success (`pass/`) or matches the produced
//! `stderr` against the sibling `.stderr` file (`fail/`).
//!
//! Regenerate the `.stderr` files after intentional message changes with
//! `TRYBUILD=overwrite cargo test -p eurora-tools-macros --test trybuild`.

#[test]
fn compile_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/pass/*.rs");
}

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
}
