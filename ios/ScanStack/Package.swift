// swift-tools-version:5.9
import PackageDescription

// CC-SCAN-STACK — Phase T0. Modules built and tested inside the offline
// authoring tool ONLY. No app-target integration: that stays blocked on
// CC-PHOTO-PICTURE's P1 + P2. TonalKit is pure math with no Apple-vision
// dependency, so it is the piece that needs none of D2/D3/D4 and can be
// written while those sign-offs are pending.
let package = Package(
    name: "ScanStack",
    // D2 (Eric approved): instance masks need iOS 17 / macOS 14. The
    // FEATURE is gated at that floor; the app minimum is untouched.
    platforms: [.macOS(.v14), .iOS(.v17)],
    products: [
        .library(name: "VisionKitScan", targets: ["VisionKitScan"]),
        .library(name: "TypeKitScan", targets: ["TypeKitScan"]),
    ],
    targets: [
        .target(name: "VisionKitScan"),
        .target(name: "TypeKitScan"),
        .testTarget(name: "VisionKitScanTests", dependencies: ["VisionKitScan"]),
        .testTarget(name: "TypeKitScanTests", dependencies: ["TypeKitScan"]),
    ]
)
