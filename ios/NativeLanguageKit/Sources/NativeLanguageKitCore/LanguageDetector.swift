import Foundation
import NaturalLanguage

/// Language detection via NLLanguageRecognizer, for catching a Spanish word typed
/// into an English custom list. Single words give a weak signal, so the caller
/// sets a high confidence bar and only ever shows a non-blocking hint.
public enum LanguageDetector {
    /// `lang` is a bare ISO code ("es"); `confidence` in 0...1. `supported:false`
    /// when the recognizer produced no hypothesis at all.
    public static func detect(text: String) -> LanguageGuess {
        let nfc = text.precomposedStringWithCanonicalMapping
        let recognizer = NLLanguageRecognizer()
        recognizer.processString(nfc)
        guard let (lang, conf) = recognizer.languageHypotheses(withMaximum: 1).first else {
            return LanguageGuess(supported: false, lang: "", confidence: 0)
        }
        return LanguageGuess(supported: true, lang: lang.rawValue, confidence: conf)
    }
}
