import Foundation
import AVFoundation

/// The installed AVSpeech voices for a language, best quality first. The plugin
/// reports these; the web core picks (Decision D3) — the catalog never chooses.
public enum VoiceCatalog {
    /// Quality tier by raw value so we don't reference the iOS 16+ `.premium`
    /// case directly (default=1, enhanced=2, premium=3).
    static func qualityName(_ q: AVSpeechSynthesisVoiceQuality) -> String {
        if q.rawValue >= 3 { return "premium" }
        if q.rawValue == 2 { return "enhanced" }
        return "default"
    }

    /// Voices whose language subtag matches `lang` (so resolving es → es-ES still
    /// surfaces es-MX etc.), sorted highest-quality first, stable within a tier.
    public static func voices(lang: String) -> [VoiceInfo] {
        let all = AVSpeechSynthesisVoice.speechVoices()
        guard let resolved = LocaleResolver.resolve(lang, from: all.map { $0.language }) else { return [] }
        let sub = String(LocaleResolver.canon(resolved).split(separator: "-").first ?? "")
        return all
            .filter { LocaleResolver.canon($0.language).hasPrefix(sub) }
            .sorted { $0.quality.rawValue > $1.quality.rawValue }
            .map { VoiceInfo(id: $0.identifier, name: $0.name, quality: qualityName($0.quality)) }
    }
}
