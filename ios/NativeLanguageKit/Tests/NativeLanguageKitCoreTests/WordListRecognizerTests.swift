import XCTest
import ImageIO
import CoreGraphics
@testable import NativeLanguageKitCore

final class WordListRecognizerTests: XCTestCase {

    /// End-to-end: run the on-device Vision recognizer over a bundled PNG of a
    /// printed spelling list and confirm the words come back. Proves the
    /// recognizer path works without any network or WASM runtime.
    func testRecognizesPrintedWordList() throws {
        let cg = try loadTestImage(named: "printed-words")
        let lines = try WordListRecognizer.recognizeLines(in: cg, languages: ["en-US"])
        XCTAssertFalse(lines.isEmpty, "Vision returned no lines for a clean printed list")

        let words = Set(WordListRecognizer.parseCandidates(lines).map { $0.lowercased() })
        for expected in ["apple", "banana", "orange", "purple", "yellow"] {
            XCTAssertTrue(words.contains(expected),
                          "expected '\(expected)' in recognized words, got \(words.sorted())")
        }
        // Numbering was stripped, so no bare digit tokens survive.
        XCTAssertFalse(words.contains("1"))
        XCTAssertFalse(words.contains("2"))
    }

    /// Pure parser coverage (no Vision) — mirrors the Rust `parse_candidates`
    /// tests so the two implementations can't silently drift.
    func testParseCandidatesShapeRules() {
        XCTAssertEqual(WordListRecognizer.parseCandidates(["1. apple", "2) banana", "• cherry,"]),
                       ["apple", "banana", "cherry"])
        // Internal apostrophes/hyphens survive.
        XCTAssertEqual(WordListRecognizer.parseCandidates(["don't", "co-op"]), ["don't", "co-op"])
        // Digits and digit-bearing tokens are dropped.
        XCTAssertEqual(WordListRecognizer.parseCandidates(["12", "H2O", "words"]), ["words"])
        // One-letter words: only the allowed es/en set survives.
        XCTAssertEqual(WordListRecognizer.parseCandidates(["a y o e u"]), ["a", "y", "o", "e", "u"])
        XCTAssertEqual(WordListRecognizer.parseCandidates(["b c x l"]), [])
        // Case-insensitive de-dup, first casing wins.
        XCTAssertEqual(WordListRecognizer.parseCandidates(["Cat", "cat", "CAT"]), ["Cat"])
    }

    // MARK: - helpers

    private func loadTestImage(named name: String) throws -> CGImage {
        guard let url = Bundle.module.url(forResource: name, withExtension: "png") else {
            throw XCTSkip("missing test resource \(name).png")
        }
        let data = try Data(contentsOf: url)
        guard let src = CGImageSourceCreateWithData(data as CFData, nil),
              let cg = CGImageSourceCreateImageAtIndex(src, 0, nil) else {
            throw NSError(domain: "test", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "could not decode \(name).png"])
        }
        return cg
    }
}
