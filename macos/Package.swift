// swift-tools-version: 6.0
// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Swift Package Manifest
//
// Builds the macOS SwiftUI application.  The mm-ffi Rust crate is built
// separately by the CI workflow and delivered as an XCFramework binary at:
//   macos/Frameworks/MeedyaManagerFFI.xcframework
//
// Platform requirement: macOS 15 (Sequoia) minimum.
//   - macOS 26+ enables Liquid Glass visual effect (runtime check in app).
//   - CI workflow: ci-macos.yml (builds mm-ffi → XCFramework, then this package)

import PackageDescription

let package = Package(
    name: "MeedyaManager",

    // macOS 15 minimum; Liquid Glass checked at runtime via #available(macOS 26, *)
    platforms: [
        .macOS(.v15)
    ],

    products: [
        // The main macOS desktop application
        .executable(
            name: "MeedyaManager",
            targets: ["MeedyaManager"]
        )
    ],

    dependencies: [
        // No remote Swift dependencies — UniFFI bindings are provided as a
        // local binary XCFramework built from the mm-ffi Rust crate.
        // See: macos/Frameworks/MeedyaManagerFFI.xcframework (CI artifact)
    ],

    targets: [
        // Main application target — SwiftUI views, models, and bindings
        .executableTarget(
            name: "MeedyaManager",
            path: "MeedyaManager",
            resources: [
                .copy("Resources/AppIcon.icns"),
                .copy("Resources/AppIcon.svg"),
            ],
            // The FFI framework is linked when available (set by CI)
            // In development without the XCFramework, the app uses stub implementations
            swiftSettings: [
                // Enable Liquid Glass compile-time flag for macOS 26+ code paths
                .define("LIQUID_GLASS_AVAILABLE", .when(platforms: [.macOS])),
                // Enable FFI when the XCFramework is present in CI/release builds
                // .define("MM_FFI_AVAILABLE") — uncomment when XCFramework is linked
            ]
        ),

        // Unit test target — tests model logic without the SwiftUI runtime.
        // Sources in MeedyaManagerTests/ are compiled independently; the test
        // models are copied here rather than @testable-importing the executable
        // (SPM does not support @testable import from .executableTarget).
        .testTarget(
            name: "MeedyaManagerTests",
            dependencies: [],
            path: "MeedyaManagerTests"
        )
    ]
)
