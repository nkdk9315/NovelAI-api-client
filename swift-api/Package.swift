// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "NovelAIAPI",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    products: [
        .library(
            name: "NovelAIAPI",
            targets: ["NovelAIAPI"]
        ),
    ],
    dependencies: [
        .package(url: "https://github.com/weichsel/ZIPFoundation.git", from: "0.9.0"),
        .package(url: "https://github.com/fumoboy007/msgpack-swift.git", from: "2.0.0"),
    ],
    targets: [
        .target(
            name: "NovelAIAPI",
            dependencies: [
                "ZIPFoundation",
                .product(name: "DMMessagePack", package: "msgpack-swift"),
            ]
        ),
        .testTarget(
            name: "NovelAIAPITests",
            dependencies: ["NovelAIAPI"]
        ),
        .executableTarget(name: "ExampleGenerate", dependencies: ["NovelAIAPI"]),
        .executableTarget(name: "ExampleAugment", dependencies: ["NovelAIAPI"]),
        .executableTarget(name: "ExampleInfill", dependencies: ["NovelAIAPI"]),
        .executableTarget(name: "ExampleTokenizer", dependencies: ["NovelAIAPI"]),
        .executableTarget(name: "ExampleValidation", dependencies: ["NovelAIAPI"]),
    ]
)
