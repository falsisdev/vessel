// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "VesselMac",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(
            name: "VesselMac",
            targets: ["VesselMac"]
        )
    ],
    targets: [
        .executableTarget(
            name: "VesselMac",
            path: "Sources/VesselMac"
        ),
        .testTarget(
            name: "VesselMacTests",
            dependencies: ["VesselMac"],
            path: "Tests/VesselMacTests"
        )
    ]
)
