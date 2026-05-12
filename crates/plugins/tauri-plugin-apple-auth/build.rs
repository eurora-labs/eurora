const COMMANDS: &[&str] = &["sign_in_with_apple"];

fn main() {
    // No `.android_path(...)`: Sign in with Apple has no Android SDK.
    // The Rust bridge's non-iOS branch (`src/bridge/stub.rs`) returns
    // `AppleSignInOutcome::NativeUnavailable` without invoking any
    // native layer, so there is nothing for the Tauri Android build
    // step to bundle. Omitting the path avoids shipping an empty
    // Kotlin stub purely to satisfy the build system.
    tauri_plugin::Builder::new(COMMANDS).ios_path("ios").build();
}
