import XCTest
import Vision
@testable import NativeLanguageKitCore

/// CC-PHOTO-IMPORT G-B — the Vision language-support MEASUREMENT.
///
/// Not a pass/fail gate: this dumps `VNRecognizeTextRequest`'s supported
/// recognition languages on the running OS and classifies each of the app's 14
/// registry languages by primary subtag match. The printed matrix is the data
/// Phase 0 turns into `consts::ocr_support` (with Eric's sign-off); the values
/// are NEVER guessed. Run under `xcodebuild test` on an iOS simulator/device —
/// the list is a property of the OS release, not the hardware.
final class VisionLanguageMatrixTests: XCTestCase {

    /// The app registry's 14 languages (code, script-is-Latin) — mirrors
    /// `src/consts.rs::LANGS_BASE`. Latin matters because the plan's fallback
    /// class (`EnglishFallback`) is only sound for Latin-script languages.
    private static let registry: [(code: String, latin: Bool)] = [
        ("en", true), ("es", true), ("fr", true), ("de", true),
        ("pt", true), ("pl", true), ("vi", true), ("ko", false),
        ("ja", false), ("fil", true), ("zh", false), ("ru", false),
        ("ar", false), ("sw", true),
    ]

    func testDumpVisionLanguageMatrix() throws {
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        let supported = try request.supportedRecognitionLanguages()

        print("G-B-MATRIX: os=\(ProcessInfo.processInfo.operatingSystemVersionString)")
        print("G-B-MATRIX: supported=\(supported.joined(separator: ","))")

        // Primary-subtag index ("en-US" -> "en", "zh-Hans" -> "zh").
        let primaries = Set(supported.map { $0.split(separator: "-").first.map(String.init) ?? $0 })

        for lang in Self.registry {
            // Filipino may appear as "fil" or under "tl" (Tagalog).
            let hit = primaries.contains(lang.code) || (lang.code == "fil" && primaries.contains("tl"))
            let klass = hit ? "Native" : (lang.latin ? "EnglishFallback" : "Unsupported")
            print("G-B-MATRIX: \(lang.code) -> \(klass)")
        }

        XCTAssertFalse(supported.isEmpty, "Vision reported zero recognition languages")
        XCTAssertTrue(primaries.contains("en"), "English missing from Vision support — measurement is unusable")
    }
}
