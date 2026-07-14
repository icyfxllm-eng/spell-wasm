import Foundation

/// Wire shapes for the capability-discovery contract. These are `Codable` so the
/// CAPPlugin wrapper can hand them straight to the Capacitor bridge as JSON and
/// the web core sees exactly the documented shape. Doctrine: the plugin only
/// *reports* what the platform can do — it never chooses on the caller's behalf.

/// One installed TTS voice for a locale, with its quality tier so the web core
/// can prefer a downloaded enhanced/premium voice (Decision D3).
public struct VoiceInfo: Codable, Equatable {
    public let id: String       // AVSpeechSynthesisVoice.identifier (stable)
    public let name: String     // human-readable voice name
    public let quality: String  // "default" | "enhanced" | "premium"

    public init(id: String, name: String, quality: String) {
        self.id = id
        self.name = name
        self.quality = quality
    }
}

public struct TTSCapability: Codable, Equatable {
    public let available: Bool
    public let voices: [VoiceInfo]
    public init(available: Bool, voices: [VoiceInfo]) {
        self.available = available
        self.voices = voices
    }
}

public struct SimpleCapability: Codable, Equatable {
    public let available: Bool
    public init(available: Bool) { self.available = available }
}

/// The full answer to `capabilities(lang)`.
public struct CapabilityReport: Codable, Equatable {
    public let tts: TTSCapability
    public let spellcheck: SimpleCapability
    public let langDetect: SimpleCapability
    public init(tts: TTSCapability, spellcheck: SimpleCapability, langDetect: SimpleCapability) {
        self.tts = tts
        self.spellcheck = spellcheck
        self.langDetect = langDetect
    }
}

/// `checkWord` result. `supported:false` means iOS has no dictionary for the
/// locale — the web core then skips this gate entirely (no silent pass/fail).
public struct WordCheckResult: Codable, Equatable {
    public let supported: Bool
    public let isWord: Bool
    public init(supported: Bool, isWord: Bool) {
        self.supported = supported
        self.isWord = isWord
    }
}

/// `detectLanguage` result. `supported:false` when the recognizer produced no
/// usable hypothesis; `lang` is a bare ISO code ("es"), `confidence` in 0...1.
public struct LanguageGuess: Codable, Equatable {
    public let supported: Bool
    public let lang: String
    public let confidence: Double
    public init(supported: Bool, lang: String, confidence: Double) {
        self.supported = supported
        self.lang = lang
        self.confidence = confidence
    }
}
