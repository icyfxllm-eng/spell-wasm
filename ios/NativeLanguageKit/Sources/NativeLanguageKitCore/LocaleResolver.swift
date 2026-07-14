import Foundation

/// Maps an app language code ("en", "es") onto a concrete locale id that a given
/// iOS API actually offers, never assuming one exists. UITextChecker reports
/// underscore ids ("en_US"), AVSpeech reports hyphen ids ("en-US"); this
/// normalizes separators so one matcher serves both, and returns the ORIGINAL id
/// so callers hand the API back exactly what it gave.
public enum LocaleResolver {
    /// The full locale each app language prefers. Spanish targets **es-ES** to
    /// match the Spanish TTS/content pipeline (Spanish audit D1 = es-ES); English
    /// defaults to US. (Decision D4.)
    static let preferred: [String: String] = ["en": "en-US", "es": "es-ES"]

    static func canon(_ s: String) -> String {
        s.replacingOccurrences(of: "_", with: "-").lowercased()
    }

    /// Best available id for `lang`, preferring the app's canonical full locale,
    /// then the bare language, then any locale sharing the language subtag.
    /// Returns nil when the platform offers nothing for the language.
    public static func resolve(_ lang: String, from available: [String]) -> String? {
        let want = canon(lang)                              // "es"
        let sub = String(want.split(separator: "-").first ?? Substring(want))
        if let pref = preferred[sub].map(canon),
           let hit = available.first(where: { canon($0) == pref }) { return hit }   // es-ES
        if let hit = available.first(where: { canon($0) == sub }) { return hit }     // es
        if let hit = available.first(where: { canon($0).hasPrefix(sub + "-") }) { return hit } // es-MX
        return nil
    }
}
