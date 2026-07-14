import XCTest
import AVFoundation
@testable import NativeLanguageKitCore

/// These run on the iOS simulator (UITextChecker/AVSpeech/NaturalLanguage are
/// iOS frameworks). Known-answer checks for the capability contract.
final class NativeLanguageKitCoreTests: XCTestCase {

    // MARK: LocaleResolver

    func testResolvePrefersCanonicalFullLocale() {
        // es should resolve to es-ES (D4) when offered, over es-MX.
        XCTAssertEqual(LocaleResolver.resolve("es", from: ["es_MX", "es_ES", "en_US"]), "es_ES")
        XCTAssertEqual(LocaleResolver.resolve("en", from: ["en_GB", "en_US"]), "en_US")
    }

    func testResolveFallsBackToLanguageSubtag() {
        XCTAssertEqual(LocaleResolver.resolve("es", from: ["es_MX", "en_US"]), "es_MX")
        XCTAssertEqual(LocaleResolver.resolve("es", from: ["es"]), "es")
    }

    func testResolveReturnsNilWhenUnavailable() {
        XCTAssertNil(LocaleResolver.resolve("th", from: ["en_US", "es_ES"]))
    }

    // MARK: WordChecker (UITextChecker)

    func testRealEnglishWordIsWord() {
        let r = WordChecker.check(word: "elephant", lang: "en")
        XCTAssertTrue(r.supported)
        XCTAssertTrue(r.isWord, "‘elephant’ should validate as an English word")
    }

    func testGarbageIsNotAWord() {
        let r = WordChecker.check(word: "asdfghjkl", lang: "en")
        XCTAssertTrue(r.supported)
        XCTAssertFalse(r.isWord, "‘asdfghjkl’ must not validate")
    }

    func testRealSpanishWordIsWord() {
        // Accented Spanish word, checked in NFC — verifies UITextChecker agrees
        // with the game's NFC normalization.
        let r = WordChecker.check(word: "araña", lang: "es")
        if r.supported {
            XCTAssertTrue(r.isWord, "‘araña’ should validate as a Spanish word")
        }
    }

    func testNFCEquivalenceForAccentedWord() {
        // Decomposed (n + combining tilde) vs precomposed ñ must give the same verdict.
        let decomposed = "aran\u{0303}a"      // araña, NFD
        let precomposed = "ara\u{00F1}a"      // araña, NFC
        // Swift String == is canonical-equivalence, so compare raw scalars to
        // prove the inputs really differ before normalization.
        XCTAssertNotEqual(Array(decomposed.unicodeScalars), Array(precomposed.unicodeScalars))
        XCTAssertEqual(
            WordChecker.check(word: decomposed, lang: "es").isWord,
            WordChecker.check(word: precomposed, lang: "es").isWord
        )
    }

    func testUnsupportedLocaleReportsUnsupported() {
        // A language iOS has no dictionary for → supported:false (caller skips gate).
        let r = WordChecker.check(word: "whatever", lang: "zz")
        XCTAssertFalse(r.supported)
    }

    // MARK: VoiceCatalog / Capabilities shapes

    func testEnglishHasVoicesAndCapabilityShape() {
        let report = Capabilities.report(lang: "en")
        // Every simulator ships at least one en voice.
        XCTAssertTrue(report.tts.available)
        XCTAssertFalse(report.tts.voices.isEmpty)
        for v in report.tts.voices {
            XCTAssertTrue(["default", "enhanced", "premium"].contains(v.quality))
            XCTAssertFalse(v.id.isEmpty)
        }
        XCTAssertTrue(report.langDetect.available)
    }

    func testVoicesSortedBestQualityFirst() {
        let voices = VoiceCatalog.voices(lang: "en")
        let ranks = voices.map { ["default": 1, "enhanced": 2, "premium": 3][$0.quality] ?? 0 }
        XCTAssertEqual(ranks, ranks.sorted(by: >), "voices must be highest-quality first")
    }

    // MARK: LanguageDetector

    func testDetectsSpanishOverEnglish() {
        let g = LanguageDetector.detect(text: "el niño come una manzana en la casa")
        XCTAssertTrue(g.supported)
        XCTAssertEqual(g.lang, "es")
        XCTAssertGreaterThan(g.confidence, 0.5)
    }

    func testDetectsEnglish() {
        let g = LanguageDetector.detect(text: "the quick brown fox jumps over the lazy dog")
        XCTAssertEqual(g.lang, "en")
    }

    // MARK: Speaker

    func testSpeakRejectsUnknownVoiceImmediately() {
        let speaker = Speaker()
        let done = expectation(description: "completes")
        var result = true
        speaker.speak(text: "hola", voiceId: "not-a-real-voice-id", gameRate: 0.9) { ok in
            result = ok; done.fulfill()
        }
        wait(for: [done], timeout: 2)
        XCTAssertFalse(result, "an unknown voiceId must complete as failure, never pick a voice")
    }

    func testSpeakCompletesWithARealVoice() throws {
        // Use the first available en voice from the catalog and confirm the
        // utterance drives to a natural finish on the simulator.
        guard let voice = VoiceCatalog.voices(lang: "en").first else {
            throw XCTSkip("no en voice on this simulator")
        }
        let speaker = Speaker()
        let done = expectation(description: "finishes speaking")
        var finished = false
        speaker.speak(text: "hi", voiceId: voice.id, gameRate: 0.9) { ok in
            finished = ok; done.fulfill()
        }
        wait(for: [done], timeout: 15)
        XCTAssertTrue(finished, "speak() should resolve on completion")
    }

    // MARK: SpeechRate mapping

    func testRateMapping() {
        // Game "normal" (0.9) → AVSpeech default; slower game rate → slower AVSpeech.
        XCTAssertEqual(SpeechRate.avRate(fromGameRate: 0.9), AVSpeechUtteranceDefaultSpeechRate, accuracy: 0.0001)
        XCTAssertLessThan(SpeechRate.avRate(fromGameRate: 0.7), AVSpeechUtteranceDefaultSpeechRate)
        // Clamped to AVSpeech's range.
        XCTAssertGreaterThanOrEqual(SpeechRate.avRate(fromGameRate: 0.01), AVSpeechUtteranceMinimumSpeechRate)
        XCTAssertLessThanOrEqual(SpeechRate.avRate(fromGameRate: 9.0), AVSpeechUtteranceMaximumSpeechRate)
    }
}
