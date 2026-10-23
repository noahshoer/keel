// swift-tools-version:5.9

import PackageDescription

let package = Package(
    name: "tauri-plugin-glass",
    platforms: [.iOS("26.0")],
    products: [
        .library(name: "tauri-plugin-glass", type: .static, targets: ["tauri-plugin-glass"]),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api"),
    ],
    targets: [
        .target(
            name: "tauri-plugin-glass",
            dependencies: [.byName(name: "Tauri")],
            path: "Sources"),
    ]
)
