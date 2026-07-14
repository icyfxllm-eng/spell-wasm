import Foundation
import UIKit  // UITextChecker

/// On-device real-word validation via UITextChecker — no server, no bundled
/// dictionaries. Additional gate, never a replacement: the caller still runs
/// charset + profanity around it.
public enum WordChecker {
    /// `supported:false` when iOS has no dictionary for `lang` (caller skips this
    /// gate). Otherwise `isWord` is true iff the whole NFC word is spelled
    /// correctly. Checked in NFC so it agrees with the game's normalization for
    /// accented es words (á, ñ).
    public static func check(word: String, lang: String) -> WordCheckResult {
        guard let resolved = LocaleResolver.resolve(lang, from: UITextChecker.availableLanguages) else {
            return WordCheckResult(supported: false, isWord: false)
        }
        let nfc = word.precomposedStringWithCanonicalMapping
        let ns = nfc as NSString
        if ns.length == 0 { return WordCheckResult(supported: true, isWord: false) }
        let checker = UITextChecker()
        let miss = checker.rangeOfMisspelledWord(
            in: nfc,
            range: NSRange(location: 0, length: ns.length),
            startingAt: 0,
            wrap: false,
            language: resolved
        )
        return WordCheckResult(supported: true, isWord: miss.location == NSNotFound)
    }
}
