import Foundation
import UIKit  // UITextChecker

/// Assembles the answer to `capabilities(lang)` from the three real iOS sources.
/// This is the anti-silent-fallback mechanism: the web core asks per language,
/// per capability, before relying on anything.
public enum Capabilities {
    public static func report(lang: String) -> CapabilityReport {
        let voices = VoiceCatalog.voices(lang: lang)
        let spellSupported = LocaleResolver.resolve(lang, from: UITextChecker.availableLanguages) != nil
        return CapabilityReport(
            tts: TTSCapability(available: !voices.isEmpty, voices: voices),
            spellcheck: SimpleCapability(available: spellSupported),
            // NLLanguageRecognizer is available on every supported iOS version.
            langDetect: SimpleCapability(available: true)
        )
    }
}
