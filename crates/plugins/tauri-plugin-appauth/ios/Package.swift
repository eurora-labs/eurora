// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "tauri-plugin-appauth",
    platforms: [
        // iOS 15 is the floor: Swift Concurrency is part of the OS from
        // 15.0 onwards. Targeting 14 forces the toolchain to link
        // `libswiftCompatibilityConcurrency.a` (the back-deploy runtime),
        // and Xcode 26's compiler emits async-thunk code that crashes
        // inside that library at `withCheckedThrowingContinuation +0x4`
        // — `EXC_BAD_ACCESS` on the first sign-in tap. Pinning to 15
        // drops the back-deploy lib entirely; native concurrency from
        // the OS has the bug fixed.
        .iOS(.v15)
    ],
    products: [
        .library(
            name: "tauri-plugin-appauth",
            type: .static,
            targets: ["tauri-plugin-appauth"])
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api"),
        .package(url: "https://github.com/openid/AppAuth-iOS", .exact("1.7.6"))
    ],
    targets: [
        .target(
            name: "tauri-plugin-appauth",
            dependencies: [
                .byName(name: "Tauri"),
                .product(name: "AppAuth", package: "AppAuth-iOS")
            ],
            path: "Sources"),
        .testTarget(
            name: "AppAuthPluginTests",
            dependencies: [
                "tauri-plugin-appauth",
                .product(name: "AppAuth", package: "AppAuth-iOS")
            ],
            path: "Tests/AppAuthPluginTests")
    ]
)
