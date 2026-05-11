//! Platform-specific implementation of the [`AppAuth`] handle.
//!
//! On mobile targets, the handle wraps a real `PluginHandle` and forwards
//! every call to AppAuth-iOS or AppAuth-Android via Tauri's async bridge.
//! On desktop targets, it is a zero-sized stub that returns
//! [`crate::Error::UnsupportedPlatform`] from every method.

cfg_select! {
    mobile => {
        mod mobile;
        pub use mobile::AppAuth;
        pub(crate) use mobile::init;
    }
    _ => {
        mod desktop;
        pub use desktop::AppAuth;
        pub(crate) use desktop::init;
    }
}
