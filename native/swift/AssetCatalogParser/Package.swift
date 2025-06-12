// swift-tools-version: 5.10

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
            name: "AssetCatalogParser",
            dependencies: ["ObjcSupport"],
            linkerSettings: [
              .unsafeFlags([
                "-F", "/System/Library/PrivateFrameworks",
                "-framework", "CoreUI",
              ])
            ]),
        .target(name: "ObjcSupport"),
        .testTarget(
          name: "AssetCatalogParserTests",
          dependencies: ["AssetCatalogParser"],
          resources: [
            .copy("Resources")
          ])
    ]
)
