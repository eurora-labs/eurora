//! Client-side tool adapter implementations.
//!
//! Each submodule binds one adapter trait from `eurora-tools-*` to the
//! transport it actually uses on this client. The [`youtube`] module
//! routes the YouTube adapter through `euro-bridge`'s native-messaging
//! channel so the browser extension can satisfy each call.

pub mod youtube;
