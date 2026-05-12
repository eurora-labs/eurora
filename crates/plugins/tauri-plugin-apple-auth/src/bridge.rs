//! Platform-specific implementation of the [`AppleAuth`] handle.
//!
//! Only iOS has a real implementation. Android and desktop return
//! [`crate::AppleSignInOutcome::NativeUnavailable`] from the public
//! API (mapped from the underlying [`crate::Error::UnsupportedPlatform`]
//! at the call site in `euro-mobile`), letting the frontend fall back
//! to the browser flow without special-casing every non-iOS target.

cfg_select! {
    target_os = "ios" => {
        mod ios;
        pub use ios::AppleAuth;
        pub(crate) use ios::init;
    }
    _ => {
        mod stub;
        pub use stub::AppleAuth;
        pub(crate) use stub::init;
    }
}
