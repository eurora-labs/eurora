//! Bridge-backed [`WebAdapter`](crate::adapter::WebAdapter) implementation.
//!
//! Phase 11 ships the feature gate with no runtime impl — the
//! [`WebBridgeImpl`] struct, action constants, and `WebAdapter` impl
//! land in Phase 12 alongside the corresponding `WEB_*` bridge actions
//! in `apps/browser/src/shared/background/native-messenger.ts`.
//!
//! Keeping the module declaration in place from day one means feature-
//! matrix builds (`cargo check -p eurora-tools-web --features bridge`)
//! are exercised by CI from the start, so the Phase 12 work can focus
//! on the body rather than the wiring.
