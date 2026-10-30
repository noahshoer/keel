// swift-tools-version:5.9

import PackageDescription

let package = Package(
    name: "tauri-plugin-keel-ios",
    platforms: [.iOS("26.0")],
    products: [
        .library(name: "tauri-plugin-keel-ios", type: .static, targets: ["tauri-plugin-keel-ios"]),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api"),
    ],
    targets: [
        .target(
            name: "tauri-plugin-keel-ios",
            dependencies: [.byName(name: "Tauri")],
            path: "Sources"),
    ]
)
