// swift-tools-version: 6.3
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
                // Apple Privacy Manifest — required for App Store / TestFlight
                .copy("PrivacyInfo.xcprivacy"),
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

        // Unit test target. The project's chosen design is "standalone tests":
        // each test file replicates the small enum/struct it tests rather than
        // @testable-importing the executable. Reasons (preserved deliberately):
        //   - SPM has historical pain around @testable for executableTarget.
        //   - Adding `dependencies: ["MeedyaManager"]` causes type-shadowing
        //     when the test target has a local copy AND the imported one
        //     (Swift picks the local — silently wrong test).
        // If a test references a new property on a replicated type, the
        // replica must be updated in sync. CI catches drift only when the
        // referenced property is missing; semantic drift (different default,
        // different label string) is not caught.
        .testTarget(
            name: "MeedyaManagerTests",
            dependencies: [],
            path: "MeedyaManagerTests"
        )
    ]
)
