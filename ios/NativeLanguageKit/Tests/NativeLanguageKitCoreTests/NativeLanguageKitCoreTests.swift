import XCTest
import AVFoundation
import Speech
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

    // MARK: SyllablePlan (boundary → syllable mapping, Feature F7)

    func testSyllablePlanJoinsWithSeparatorAndOffsets() {
        let plan = SyllablePlan(syllables: ["ca", "sa"])
        XCTAssertEqual(plan.text, "ca sa")           // space-joined tokens
        XCTAssertEqual(plan.starts, [0, 3])           // "ca"=0..2, sep=2, "sa"=3
    }

    func testSyllablePlanMapsOffsetsToSyllableIndex() {
        let plan = SyllablePlan(syllables: ["trans", "por", "te"])
        XCTAssertEqual(plan.text, "trans por te")
        XCTAssertEqual(plan.starts, [0, 6, 10])
        // A boundary location lands on each token start.
        XCTAssertEqual(plan.syllableIndex(forUTF16Offset: 0), 0)
        XCTAssertEqual(plan.syllableIndex(forUTF16Offset: 6), 1)
        XCTAssertEqual(plan.syllableIndex(forUTF16Offset: 10), 2)
        // Interior offsets resolve to the owning syllable; past-the-end clamps.
        XCTAssertEqual(plan.syllableIndex(forUTF16Offset: 3), 0)
        XCTAssertEqual(plan.syllableIndex(forUTF16Offset: 8), 1)
        XCTAssertEqual(plan.syllableIndex(forUTF16Offset: 99), 2)
    }

    func testSyllablePlanEmptyIsSafe() {
        let plan = SyllablePlan(syllables: [])
        XCTAssertEqual(plan.text, "")
        XCTAssertEqual(plan.syllableIndex(forUTF16Offset: 0), 0)
    }

    func testSyllablePlanUTF16OffsetsForAccentedSyllables() {
        // Precomposed "í" is a single UTF-16 unit; offsets must stay consistent.
        let plan = SyllablePlan(syllables: ["dí", "a"])
        XCTAssertEqual(plan.text, "dí a")
        XCTAssertEqual(plan.starts, [0, 3])
        XCTAssertEqual(plan.syllableIndex(forUTF16Offset: 3), 1)
    }

    // MARK: SyllableSpeaker

    func testSyllableSpeakRejectsUnknownVoiceImmediately() {
        let speaker = SyllableSpeaker()
        let done = expectation(description: "completes")
        var result = true
        speaker.speak(syllables: ["ca", "sa"], voiceId: "not-a-real-voice-id", gameRate: 0.9,
                      onSyllable: { _ in }, onComplete: { ok in result = ok; done.fulfill() })
        wait(for: [done], timeout: 2)
        XCTAssertFalse(result, "an unknown voiceId must complete as failure, never pick a voice")
    }

    func testSyllableSpeakRejectsEmptyList() {
        let speaker = SyllableSpeaker()
        let done = expectation(description: "completes")
        var result = true
        speaker.speak(syllables: [], voiceId: "whatever", gameRate: 0.9,
                      onSyllable: { _ in }, onComplete: { ok in result = ok; done.fulfill() })
        wait(for: [done], timeout: 2)
        XCTAssertFalse(result, "an empty syllable list must complete as failure")
    }

    func testSyllableSpeakDrivesBoundariesAndCompletes() throws {
        guard let voice = VoiceCatalog.voices(lang: "en").first else {
            throw XCTSkip("no en voice on this simulator")
        }
        let speaker = SyllableSpeaker()
        let done = expectation(description: "finishes speaking")
        var finished = false
        var maxIndex = -1
        var reported: [Int] = []
        speaker.speak(syllables: ["ca", "sa"], voiceId: voice.id, gameRate: 0.9,
                      onSyllable: { idx in reported.append(idx); maxIndex = max(maxIndex, idx) },
                      onComplete: { ok in finished = ok; done.fulfill() })
        wait(for: [done], timeout: 20)
        XCTAssertTrue(finished, "syllable speak should resolve on completion")
        // The boundary callback must have fired for at least the first syllable,
        // and indices are non-decreasing (0-based, in order).
        XCTAssertGreaterThanOrEqual(maxIndex, 0, "willSpeakRange should report ≥1 syllable")
        XCTAssertEqual(reported, reported.sorted(), "syllable indices must arrive in order")
    }

    // MARK: SpeechCapabilities — locale resolution (Feature F2 "Say It")

    func testSpeechLocaleResolvesCanonicalFullLocale() {
        // en → en-US, es → es-ES (D4) from a recognizer-style supported set.
        XCTAssertEqual(
            SpeechCapabilities.resolveLocaleId(lang: "es", from: ["es-MX", "es-ES", "en-US"]),
            "es-ES")
        XCTAssertEqual(
            SpeechCapabilities.resolveLocaleId(lang: "en", from: ["en-GB", "en-US"]),
            "en-US")
    }

    func testSpeechLocaleFallsBackToSubtag() {
        XCTAssertEqual(
            SpeechCapabilities.resolveLocaleId(lang: "es", from: ["es-MX", "en-US"]),
            "es-MX")
    }

    func testSpeechLocaleNilWhenUnsupported() {
        XCTAssertNil(SpeechCapabilities.resolveLocaleId(lang: "th", from: ["en-US", "es-ES"]))
    }

    func testSpeechReportUnsupportedLocaleIsUnavailable() {
        // A language the recognizer offers no locale for → unavailable, and never
        // "available" without on-device support.
        let cap = SpeechCapabilities.report(lang: "zz")
        XCTAssertFalse(cap.available)
        XCTAssertFalse(cap.supportsOnDevice)
        XCTAssertEqual(cap.locale, "")
    }

    func testSpeechAvailableImpliesOnDevice() {
        // The privacy invariant, whatever this simulator reports: available is
        // NEVER true unless on-device recognition is supported. (On the simulator
        // `available` is typically false — that's the safe/fail-closed default.)
        let ids = SpeechCapabilities.supportedLocaleIds()
        XCTAssertFalse(ids.isEmpty, "recognizer should advertise some locales")
        for lang in ["en", "es"] {
            let cap = SpeechCapabilities.report(lang: lang)
            if cap.available {
                XCTAssertTrue(cap.supportsOnDevice,
                    "\(lang): available must imply on-device support")
            }
        }
    }

    // MARK: SpeechMatcher — exact token match after NFC + case fold (Decision D2)

    func testSpokenTokenPresentMatches() {
        XCTAssertTrue(SpeechMatcher.matches(transcript: "the elephant is big", target: "elephant"))
        XCTAssertTrue(SpeechMatcher.matches(transcript: "elephant", target: "elephant"))
    }

    func testSpokenCaseInsensitive() {
        XCTAssertTrue(SpeechMatcher.matches(transcript: "ELEPHANT", target: "elephant"))
    }

    func testSpokenTrimsEdgePunctuation() {
        XCTAssertTrue(SpeechMatcher.matches(transcript: "Elephant.", target: "elephant"))
        XCTAssertTrue(SpeechMatcher.matches(transcript: "is it a casa?", target: "casa"))
        XCTAssertTrue(SpeechMatcher.matches(transcript: "\u{a1}Hola!", target: "hola"))
    }

    func testSpokenIsAccentStrict() {
        XCTAssertTrue(SpeechMatcher.matches(transcript: "un caf\u{e9}", target: "caf\u{e9}"))
        XCTAssertFalse(SpeechMatcher.matches(transcript: "un cafe", target: "caf\u{e9}"))
    }

    func testSpokenNFDMatchesNFCTarget() {
        let nfd = "la aran\u{0303}a"           // araña, decomposed
        XCTAssertTrue(SpeechMatcher.matches(transcript: nfd, target: "ara\u{f1}a"))
    }

    func testSpokenNoSubstringOrPluralCredit() {
        XCTAssertFalse(SpeechMatcher.matches(transcript: "two elephants", target: "elephant"))
        XCTAssertFalse(SpeechMatcher.matches(transcript: "elephantine", target: "elephant"))
    }

    func testSpokenEmptyNeverMatches() {
        XCTAssertFalse(SpeechMatcher.matches(transcript: "", target: "elephant"))
        XCTAssertFalse(SpeechMatcher.matches(transcript: "anything", target: ""))
    }

    func testSpokenParityWithRustEdgeCases() {
        // Same cases asserted in Rust `norm::spoken_matches` tests, so the two
        // implementations can't silently diverge.
        XCTAssertTrue(SpeechMatcher.matches(transcript: "say don't now", target: "don't"))
        XCTAssertTrue(SpeechMatcher.matches(transcript: "it is mag-aral", target: "mag-aral"))
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
