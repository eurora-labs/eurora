const COMMANDS: &[&str] = &[
    "discover",
    "register",
    "authorize",
    "authorize_browser_only",
    "refresh",
    "end_session",
    "subscribe_events",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
