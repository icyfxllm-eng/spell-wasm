import Foundation

/// Pure, unit-testable plan for speaking a word as a sequence of syllables in a
/// SINGLE utterance while recovering which syllable AVSpeech is currently
/// speaking. `willSpeakRangeOfSpeechString` reports the UTF-16 character range
/// of the token being spoken; `syllableIndex(forUTF16Offset:)` maps that range's
/// start back to a syllable index so the web core can highlight the revealed
/// spelling in sync. No AVSpeech here — this is the seam that lets the boundary
/// logic be tested without audio (Feature F7).
///
/// Syllables are joined with a single space so the synthesizer voices them as
/// distinct tokens (a gentle pause between each — exactly the "hear it slowly,
/// piece by piece" pedagogy) and fires a word boundary at each syllable start.
public struct SyllablePlan: Equatable {
    /// The syllable tokens, in order (e.g. ["ca", "sa"]).
    public let syllables: [String]
    /// The single string handed to AVSpeech (syllables joined by `separator`).
    public let text: String
    /// UTF-16 offset at which each syllable STARTS in `text` (NSRange is UTF-16).
    public let starts: [Int]

    public static let separator = " "

    public init(syllables: [String]) {
        self.syllables = syllables
        var joined = ""
        var offs: [Int] = []
        for (i, s) in syllables.enumerated() {
            if i > 0 { joined += SyllablePlan.separator }
            offs.append(joined.utf16.count)
            joined += s
        }
        self.text = joined
        self.starts = offs
    }

    /// Which syllable owns UTF-16 character index `i` in `text`: the last
    /// syllable whose start is ≤ `i`. Returns 0 for an empty plan. Boundaries
    /// fire at token starts, so `i` lands on (or just after) one of `starts`.
    public func syllableIndex(forUTF16Offset i: Int) -> Int {
        guard !syllables.isEmpty else { return 0 }
        var idx = 0
        for (k, start) in starts.enumerated() {
            if i >= start { idx = k } else { break }
        }
        return idx
    }
}
