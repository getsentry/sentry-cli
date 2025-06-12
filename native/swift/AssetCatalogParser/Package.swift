// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "AssetCatalogParser",
    platforms: [
        .macOS(.v11),
    ],
    products: [
        .library(
            name: "AssetCatalogParser",
            targets: ["AssetCatalogParser"]),
    ],
    targets: [
        .target(
            name: "AssetCatalogParser", dependencies: ["ObjcSupport"]),
        .target(name: "ObjcSupport"),
    ]
)