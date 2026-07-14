import Foundation
import Vision

/// On-device word-list recognizer for Feature F1 "Photo-to-word-list".
///
/// `recognizeLines` runs Apple's Vision `VNRecognizeTextRequest` in **accurate**
/// mode against a `CGImage` and returns the recognized text lines. Vision runs
/// entirely on-device — the image is never uploaded and this type makes no
/// network call. The Capacitor plugin shim hands the camera/library image
/// straight here.
///
/// `parseCandidates` is a Swift mirror of the Rust `native_lang::parse_candidates`
/// (src/native_lang.rs): it turns raw recognized lines into de-duplicated
/// candidate words by shape only. In production the authoritative parse (and the
/// charset + profanity trust gate) runs in the Rust/WASM core; this mirror
/// exists so the recognizer can be exercised end-to-end in `xcodebuild test`
/// without a WASM runtime.
public enum WordListRecognizer {

    /// One-letter tokens that are legitimate words in the app's Latin languages
    /// (Spanish "y/o/e/u/a", English "a"). Every other lone letter is dropped.
    private static let oneLetterWords: Set<Character> = ["a", "y", "o", "e", "u"]

    /// Recognize text lines in an image, entirely on-device.
    /// - Parameters:
    ///   - cgImage: the photographed handout.
    ///   - languages: BCP-47 recognition languages (e.g. `["en-US"]`). Empty ->
    ///     Vision's default.
    /// - Returns: the top candidate string for each recognized line, top to
    ///   bottom.
    public static func recognizeLines(in cgImage: CGImage, languages: [String] = []) throws -> [String] {
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = true
        if !languages.isEmpty {
            request.recognitionLanguages = languages
        }
        let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
        try handler.perform([request])
        let observations = request.results ?? []
        return observations.compactMap { $0.topCandidates(1).first?.string }
    }

    /// Shape-only cleanup mirroring the Rust parser: split lines on whitespace,
    /// NFC-normalize, strip list numbering / bullets / edge punctuation, drop
    /// digit-bearing tokens and stray single letters, de-dupe case-insensitively
    /// preserving first-seen order and casing.
    public static func parseCandidates(_ lines: [String]) -> [String] {
        var seen = Set<String>()
        var out: [String] = []
        let letters = CharacterSet.letters
        for line in lines {
            for rawToken in line.split(whereSeparator: { $0 == " " || $0 == "\t" || $0 == "\n" }) {
                // NFC first so a trailing combining mark composes onto its letter
                // instead of being trimmed as a non-letter.
                let composed = String(rawToken).precomposedStringWithCanonicalMapping
                let trimmed = composed.trimmingCharacters(in: letters.inverted)
                if trimmed.isEmpty { continue }
                if !trimmed.unicodeScalars.contains(where: { letters.contains($0) }) { continue }
                if trimmed.unicodeScalars.contains(where: { $0.value >= 48 && $0.value <= 57 }) { continue }
                if trimmed.count == 1, let only = trimmed.first {
                    let lower = Character(only.lowercased())
                    if !oneLetterWords.contains(lower) { continue }
                }
                let key = trimmed.lowercased()
                if seen.insert(key).inserted {
                    out.append(trimmed)
                    if out.count >= 2000 { return out }
                }
            }
        }
        return out
    }
}
