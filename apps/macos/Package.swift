// swift-tools-version:5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "EuroraMacOS",
    defaultLocalization: "en",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        // Shared library for Safari extension communication
        .library(
            name: "EuroraShared",
            targets: ["EuroraShared"]),
        // Container app library with gRPC client
        .library(
            name: "EuroraContainerApp",
            targets: ["EuroraContainerApp"])
    ],
    dependencies: [
        // gRPC Swift 2.x packages
        .package(url: "https://github.com/grpc/grpc-swift-2.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-nio-transport.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-protobuf.git", from: "2.0.0"),
        .package(url: "https://github.com/SimplyDanny/SwiftLintPlugins", from: "0.63.2"),
        // Swift Protobuf for generated message types
        .package(url: "https://github.com/apple/swift-protobuf.git", from: "1.25.0")
    ],
    targets: [
        // Shared target - used by both container app and extension
        // Only uses Network framework for local IPC, no gRPC dependencies
        .target(
            name: "EuroraShared",
            dependencies: [],
            path: "Shared",
            sources: ["NativeMessagingBridge.swift"],
            plugins: [.plugin(name: "SwiftLintBuildToolPlugin", package: "SwiftLintPlugins")]
        ),
        // Container app target - includes gRPC client
        .target(
            name: "EuroraContainerApp",
            dependencies: [
                .product(name: "GRPCCore", package: "grpc-swift-2"),
                .product(name: "GRPCNIOTransportHTTP2", package: "grpc-swift-nio-transport"),
                .product(name: "GRPCProtobuf", package: "grpc-swift-protobuf"),
                .product(name: "SwiftProtobuf", package: "swift-protobuf"),
                "EuroraShared",
                "BrowserBridgeProto"
            ],
            path: "macos",
            sources: [
                "BrowserBridgeClient.swift",
                "LocalBridgeServer.swift",
                "AppDelegate.swift",
                "ViewController.swift"
            ],
            plugins: [.plugin(name: "SwiftLintBuildToolPlugin", package: "SwiftLintPlugins")]
        ),
        // Generated protobuf target
        .target(
            name: "BrowserBridgeProto",
            dependencies: [
                .product(name: "GRPCCore", package: "grpc-swift-2"),
                .product(name: "GRPCProtobuf", package: "grpc-swift-protobuf"),
                .product(name: "SwiftProtobuf", package: "swift-protobuf")
            ],
            path: "Shared/Generated"
        )
    ]
)
