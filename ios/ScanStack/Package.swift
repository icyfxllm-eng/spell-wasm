// swift-tools-version:5.9
import PackageDescription

// CC-SCAN-STACK — Phase T0. Modules built and tested inside the offline
// authoring tool ONLY. No app-target integration: that stays blocked on
// CC-PHOTO-PICTURE's P1 + P2. TonalKit is pure math with no Apple-vision
// dependency, so it is the piece that needs none of D2/D3/D4 and can be
// written while those sign-offs are pending.
let package = Package(
    name: "ScanStack",
    platforms: [.macOS(.v13), .iOS(.v16)],
    products: [.library(name: "TonalKit", targets: ["TonalKit"])],
    targets: [
        .target(name: "TonalKit"),
        .testTarget(name: "TonalKitTests", dependencies: ["TonalKit"]),
    ]
)
