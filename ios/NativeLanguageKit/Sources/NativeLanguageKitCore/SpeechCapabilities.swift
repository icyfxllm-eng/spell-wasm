import Foundation
import Speech

/// Wire shape for `speechCapabilities(lang)` (Feature F2 "Say It").
///
/// `available` is the ONLY flag the caller should branch on to decide whether to
/// offer speaking — and it is defined so that **`available` is never true unless
/// on-device recognition is supported**. This is the privacy guarantee expressed
/// in the type: a child's voice never leaves the phone, so if iOS can't recognize
/// this locale entirely on-device, the feature is simply unavailable (it must
/// NEVER silently fall back to Apple's server-based recognition).
public struct SpeechCapability: Codable, Equatable {
    /// True iff the mode may be offered: an on-device recognizer exists, is
    /// available right now, and supports on-device recognition for the locale.
    public let available: Bool
    /// Whether `SFSpeechRecognizer.supportsOnDeviceRecognition` is true for the
    /// resolved locale. Reported separately for diagnostics; `available` already
    /// folds it in.
    public let supportsOnDevice: Bool
    /// The concrete locale id the recognizer would use ("en-US"), or "" if none.
    public let locale: String

    public init(available: Bool, supportsOnDevice: Bool, locale: String) {
        self.available = available
        self.supportsOnDevice = supportsOnDevice
        self.locale = locale
    }
}

/// Assembles the answer to `speechCapabilities(lang)` from SFSpeechRecognizer.
/// Doctrine (same as the rest of NativeLanguageKit): report what the platform
/// can do, decide nothing on the caller's behalf — and here, refuse to report
/// "available" for anything that would require the network.
public enum SpeechCapabilities {

    /// The set of locale ids the speech recognizer supports, as strings, so the
    /// shared `LocaleResolver` (which also serves UITextChecker/AVSpeech) can pick
    /// the best id for an app language. Injectable for unit tests.
    public static func supportedLocaleIds() -> [String] {
        SFSpeechRecognizer.supportedLocales().map { $0.identifier }
    }

    /// Pure locale resolution: which recognizer locale id (if any) serves `lang`,
    /// given the set the device offers. Extracted so it is unit-testable headless
    /// without touching the (permission-gated, device-only) recognizer itself.
    public static func resolveLocaleId(lang: String, from available: [String]) -> String? {
        LocaleResolver.resolve(lang, from: available)
    }

    /// Full capability report for `lang`. On-device support and live availability
    /// require a real recognizer instance and (on device) installed assets, so on
    /// the simulator `available` may be false even for a resolvable locale — that
    /// is correct and safe (fail closed). Never throws; never blocks.
    public static func report(lang: String) -> SpeechCapability {
        guard let localeId = resolveLocaleId(lang: lang, from: supportedLocaleIds()) else {
            return SpeechCapability(available: false, supportsOnDevice: false, locale: "")
        }
        guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: localeId)) else {
            return SpeechCapability(available: false, supportsOnDevice: false, locale: localeId)
        }
        let onDevice = recognizer.supportsOnDeviceRecognition
        // available REQUIRES on-device support: no on-device path => not offered.
        let available = recognizer.isAvailable && onDevice
        return SpeechCapability(available: available, supportsOnDevice: onDevice, locale: localeId)
    }
}
