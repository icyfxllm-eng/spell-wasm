// swift-tools-version: 5.9
//
// NativeLanguageKit — the on-device linguistic capability core for SpellGame's
// iOS build. This is a STANDALONE package (deliberately NOT added to the
// CLI-managed CapApp-SPM/Package.swift) so its pure logic — capability
// discovery, UITextChecker word validation, AVSpeech voice catalog,
// NLLanguageRecognizer detection, speech-rate mapping — is unit-testable on the
// iOS simulator via `xcodebuild test` without dragging in Capacitor or the app.
//
// The thin CAPPlugin wrapper that exposes this to the Capacitor bridge lives in
// the App target (ios/App/App/NativeLanguageKitPlugin.swift) and calls into this
// library. Run the tests with:
//
//   xcodebuild test -scheme NativeLanguageKitCore \
//     -destination 'platform=iOS Simulator,name=iPhone 17' \
//     -workspace /dev/null 2>/dev/null || \
//   (cd ios/NativeLanguageKit && xcodebuild test \
//     -scheme NativeLanguageKitCore-Package \
//     -destination 'platform=iOS Simulator,name=iPhone 17')
//
import PackageDescription

let package = Package(
    name: "NativeLanguageKitCore",
    // iOS-only: UITextChecker (UIKit) is unavailable to host-side `swift test`,
    // so these tests must run on a simulator destination.
    platforms: [.iOS(.v15)],
    products: [
        .library(name: "NativeLanguageKitCore", targets: ["NativeLanguageKitCore"]),
    ],
    targets: [
        .target(name: "NativeLanguageKitCore"),
        .testTarget(
            name: "NativeLanguageKitCoreTests",
            dependencies: ["NativeLanguageKitCore"]
        ),
    ]
)
