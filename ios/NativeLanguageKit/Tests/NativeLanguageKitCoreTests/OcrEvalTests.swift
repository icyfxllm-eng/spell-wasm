import XCTest
import CoreGraphics
import ImageIO
@testable import NativeLanguageKitCore

/// CC-PHOTO-IMPORT Phase 7 — the OCR quality eval + merge gate.
///
/// Runs the on-device recognizer over the bundled fixture set (78 images: 13
/// launch languages × 3 printed fonts × 3 print-style handwriting fonts, each
/// with its ground-truth list from the app's own word banks) and scores
/// per-language precision/recall. The MERGE GATE: no language may fall more
/// than `tolerance` below the recorded baseline (`ocr-baseline.json`). When
/// recognition improves, re-record the baseline deliberately — the printed
/// JSON is the exact file content.
///
/// The eval mirrors production's registry profile: EnglishFallback languages
/// (fil, sw — consts::ocr_support) run the ENGLISH recognizer with correction
/// OFF; everything else runs its own language with correction on.
final class OcrEvalTests: XCTestCase {

    private struct Fixture: Codable {
        let file: String
        let lang: String
        let style: String
        let handwriting: Bool
        let words: [String]
    }

    private struct Score: Codable {
        var precision: Double
        var recall: Double
    }

    /// Mirror of the production registry (consts::ocr_support, G-B measured):
    /// recognition language + correction per app language.
    private static func profile(for lang: String) -> (languages: [String], correction: Bool) {
        switch lang {
        case "fil", "sw": return (["en-US"], false) // EnglishFallback
        case "zh": return (["zh-Hans"], true)
        case "ko": return (["ko-KR"], true)
        case "ja": return (["ja-JP"], true)
        case "ru": return (["ru-RU"], true)
        case "pt": return (["pt-BR"], true)
        default: return (["\(lang)-\(lang.uppercased())"], true)
        }
    }

    private func loadFixtures() throws -> [Fixture] {
        // .process("Resources") may flatten subdirectories — try both layouts.
        let url = try XCTUnwrap(
            Bundle.module.url(forResource: "manifest", withExtension: "json", subdirectory: "ocr-fixtures")
                ?? Bundle.module.url(forResource: "manifest", withExtension: "json"),
            "ocr-fixtures/manifest.json missing from test resources")
        return try JSONDecoder().decode([Fixture].self, from: Data(contentsOf: url))
    }

    private func loadImage(_ name: String) throws -> CGImage {
        let base = (name as NSString).deletingPathExtension
        let url = try XCTUnwrap(
            Bundle.module.url(forResource: base, withExtension: "png", subdirectory: "ocr-fixtures")
                ?? Bundle.module.url(forResource: base, withExtension: "png"),
            "fixture \(name) missing")
        let src = try XCTUnwrap(CGImageSourceCreateWithURL(url as CFURL, nil))
        return try XCTUnwrap(CGImageSourceCreateImageAtIndex(src, 0, nil))
    }

    private static func fold(_ s: String) -> String {
        s.precomposedStringWithCanonicalMapping.lowercased()
    }

    func testOcrPrecisionRecallAgainstBaseline() throws {
        let fixtures = try loadFixtures()
        XCTAssertFalse(fixtures.isEmpty)

        // Per-language micro counts across all 6 styles.
        var tp = [String: Int](), fp = [String: Int](), fnCount = [String: Int]()
        // Confidence stats for the LOW_CONFIDENCE calibration (Phase 3 open item).
        var handConfidences: [Float] = []
        var printConfidences: [Float] = []

        for fx in fixtures {
            let image = try loadImage(fx.file)
            let prof = Self.profile(for: fx.lang)
            let lines = try WordListRecognizer.recognizeLinesDetailed(
                in: image, languages: prof.languages, correction: prof.correction)
            if fx.handwriting {
                handConfidences.append(contentsOf: lines.map { $0.confidence })
            } else {
                printConfidences.append(contentsOf: lines.map { $0.confidence })
            }
            let got = Set(WordListRecognizer.parseCandidates(lines.map { $0.text }).map(Self.fold))
            let want = Set(fx.words.map(Self.fold))
            tp[fx.lang, default: 0] += got.intersection(want).count
            fp[fx.lang, default: 0] += got.subtracting(want).count
            fnCount[fx.lang, default: 0] += want.subtracting(got).count
        }

        var scores: [String: Score] = [:]
        for lang in tp.keys.sorted() {
            let t = Double(tp[lang] ?? 0)
            let p = t + Double(fp[lang] ?? 0) > 0 ? t / (t + Double(fp[lang] ?? 0)) : 0
            let r = t + Double(fnCount[lang] ?? 0) > 0 ? t / (t + Double(fnCount[lang] ?? 0)) : 0
            scores[lang] = Score(precision: (p * 1000).rounded() / 1000,
                                 recall: (r * 1000).rounded() / 1000)
            print(String(format: "OCR-EVAL: %@ precision=%.3f recall=%.3f", lang, p, r))
        }

        // Calibration data for photo_import::LOW_CONFIDENCE (provisional 0.4).
        func stats(_ v: [Float]) -> String {
            guard !v.isEmpty else { return "n=0" }
            let s = v.sorted()
            return String(format: "n=%d min=%.2f p10=%.2f median=%.2f",
                          s.count, s.first!, s[s.count / 10], s[s.count / 2])
        }
        print("OCR-EVAL: confidence printed[\(stats(printConfidences))] handwriting[\(stats(handConfidences))]")

        // The merge gate: compare against the recorded baseline.
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        let currentJSON = String(data: try enc.encode(scores), encoding: .utf8)!
        let baseCandidate = Bundle.module.url(forResource: "ocr-baseline", withExtension: "json",
                                              subdirectory: "ocr-fixtures")
            ?? Bundle.module.url(forResource: "ocr-baseline", withExtension: "json")
        guard let baseURL = baseCandidate else {
            // Bootstrap mode: no baseline recorded yet. Print it for check-in and
            // pass — the FIRST recorded run becomes the gate.
            print("OCR-EVAL: NO BASELINE — record this as Resources/ocr-fixtures/ocr-baseline.json:\n\(currentJSON)")
            return
        }
        let baseline = try JSONDecoder().decode([String: Score].self, from: Data(contentsOf: baseURL))
        let tolerance = 0.02
        for (lang, base) in baseline.sorted(by: { $0.key < $1.key }) {
            let cur = try XCTUnwrap(scores[lang], "language \(lang) vanished from the eval")
            XCTAssertGreaterThanOrEqual(cur.precision, base.precision - tolerance,
                "\(lang) precision regressed: \(cur.precision) < baseline \(base.precision) - \(tolerance)")
            XCTAssertGreaterThanOrEqual(cur.recall, base.recall - tolerance,
                "\(lang) recall regressed: \(cur.recall) < baseline \(base.recall) - \(tolerance)")
        }
        print("OCR-EVAL: current scores (re-record baseline if better):\n\(currentJSON)")
    }
}
