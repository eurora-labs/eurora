# focus-tracker-core

[![Crates.io](https://img.shields.io/crates/v/focus-tracker-core.svg)](https://crates.io/crates/focus-tracker-core)
[![Documentation](https://docs.rs/focus-tracker-core/badge.svg)](https://docs.rs/focus-tracker-core)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

A library of core types for [focus-tracker](https://github.com/eurora-labs/eurora/tree/main/crates/common/focus-tracker)
This core crate is supposed to be used in related crates that don't need the heavy dependencies of the full focus-tracker crate. The version of the core crate is pegged to the version of the full focus-tracker crate. The focus-tracker crate itself also re-exports all types from this crate.
