// swift-tools-version:5.9

import PackageDescription

let package = Package(
    name: "EuroraMacOS",
    defaultLocalization: "en",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "EuroraShared",
            targets: ["EuroraShared"]),
        .library(
            name: "EuroraContainerApp",
            targets: ["EuroraContainerApp"])
    ],
    dependencies: [
        .package(url: "https://github.com/grpc/grpc-swift-2.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-nio-transport.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-protobuf.git", from: "2.0.0"),
        .package(url: "https://github.com/SimplyDanny/SwiftLintPlugins", from: "0.63.2"),
        .package(url: "https://github.com/apple/swift-protobuf.git", from: "1.25.0")
    ],
    targets: [
        .target(
            name: "EuroraShared",
            dependencies: [],
            path: "Shared",
            sources: ["NativeMessagingBridge.swift"],
            plugins: [.plugin(name: "SwiftLintBuildToolPlugin", package: "SwiftLintPlugins")]
        ),
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
