// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "tauri-plugin-apple-auth",
    platforms: [
        // iOS 15 is the floor for the Eurora mobile app — and Sign in
        // with Apple has been available since iOS 13, so the
        // `AuthenticationServices` symbols we use are well within range.
        // Pinning to the same floor as the AppAuth plugin keeps the
        // Swift concurrency back-deploy story consistent across plugins
        // (see that plugin's Package.swift for the EXC_BAD_ACCESS bug
        // that drove the iOS 14 → 15 bump).
        .iOS(.v15)
    ],
    products: [
        .library(
            name: "tauri-plugin-apple-auth",
            type: .static,
            targets: ["tauri-plugin-apple-auth"])
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api")
    ],
    targets: [
        .target(
            name: "tauri-plugin-apple-auth",
            dependencies: [
                .byName(name: "Tauri")
            ],
            path: "Sources")
    ]
)
