// swift-tools-version:5.9
//
// This package definition exists primarily so SwiftLint's build-tool
// plugin can lint the macOS launcher's Swift sources outside of Xcode.
// The actual app is built from `macos.xcodeproj`, which is the source
// of truth for build settings, code-signing, and entitlements.

import PackageDescription

let package = Package(
    name: "EuroraMacOS",
    defaultLocalization: "en",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(name: "EuroraMacOSLint", targets: ["EuroraMacOSLint"]),
    ],
    dependencies: [
        .package(url: "https://github.com/SimplyDanny/SwiftLintPlugins", from: "0.63.2"),
    ],
    targets: [
        .target(
            name: "EuroraMacOSLint",
            path: ".",
            exclude: [
                "Package.swift",
                "Package.resolved",
                "macos.xcodeproj",
                "macos/Assets.xcassets",
                "macos/Base.lproj",
                "macos/Info.plist",
                "macos/macos.entitlements",
                "macos Extension/Info.plist",
                "macos Extension/Resources",
            ],
            sources: [
                "Shared",
                "macos/AppDelegate.swift",
                "macos/BridgeWebSocketClient.swift",
                "macos/LocalBridgeServer.swift",
                "macos Extension/SafariWebExtensionHandler.swift",
            ],
            plugins: [.plugin(name: "SwiftLintBuildToolPlugin", package: "SwiftLintPlugins")]
        ),
    ]
)
