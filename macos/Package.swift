// swift-tools-version: 6.0
// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)

import PackageDescription

let package = Package(
    // Package name
    name: "MeedyaManager",

    // Require macOS 15 (Sequoia) or later
    platforms: [
        .macOS(.v15)
    ],

    // Executable product — builds the MeedyaManager app binary
    products: [
        .executable(
            name: "MeedyaManager",
            targets: ["MeedyaManager"]
        )
    ],

    // No external dependencies yet — add UniFFI, etc. here later
    dependencies: [],

    // Single target pulling sources from the MeedyaManager/ directory
    targets: [
        .executableTarget(
            name: "MeedyaManager",
            path: "MeedyaManager"
        )
    ]
)
