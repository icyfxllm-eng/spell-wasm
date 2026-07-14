import Foundation

/// The Say-It match rule (Feature F2, Decision D2), on the native side.
///
/// A target word counts as *said* iff, after NFC + case folding, it equals one of
/// the whitespace-separated tokens of the recognizer's transcription (edge
/// punctuation trimmed). This is an **exact token match after folding** — never
/// fuzzy, phonetic, or confidence-scored.
///
/// This mirrors `norm::spoken_matches` in the Rust core, which is the rule the
/// game actually scores with; the two are kept byte-for-byte equivalent and both
/// are unit-tested. Having it here lets the deterministic rule be exercised in
/// XCTest alongside the capability/locale logic (live mic recognition itself
/// can't be unit-tested headless).
public enum SpeechMatcher {

    /// NFC + lowercase fold (Swift's `lowercased()` is full Unicode case folding),
    /// matching the Rust `fold_strict`. Accent-strict: diacritics are preserved.
    static func fold(_ s: String) -> String {
        s.precomposedStringWithCanonicalMapping
            .lowercased()
            .components(separatedBy: .whitespacesAndNewlines)
            .joined()
    }

    /// Trim leading/trailing punctuation the recognizer appends, keeping internal
    /// apostrophes/hyphens (don't, mag-aral).
    static func trimEdgePunct(_ tok: String) -> String {
        let keep: (Character) -> Bool = { ch in
            ch.isLetter || ch.isNumber || ch == "'" || ch == "\u{2019}" || ch == "-"
        }
        var chars = Array(tok)
        while let f = chars.first, !keep(f) { chars.removeFirst() }
        while let l = chars.last, !keep(l) { chars.removeLast() }
        return String(chars)
    }

    public static func matches(transcript: String, target: String) -> Bool {
        let want = fold(target)
        if want.isEmpty { return false }
        return transcript
            .split(whereSeparator: { $0.isWhitespace })
            .contains { fold(trimEdgePunct(String($0))) == want }
    }
}
