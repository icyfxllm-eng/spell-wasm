import Foundation
import Speech

/// Wire shape for `speechCapabilities(lang)` (Say-It + Spell It voice input).
///
/// `available` is the ONLY flag the caller should branch on to decide whether
/// capture can start RIGHT NOW — and it is defined so that **`available` is
/// never true unless on-device recognition is supported**. This is the privacy
/// guarantee expressed in the type: a child's voice never leaves the phone, so
/// if iOS can't recognize this locale entirely on-device, capture is simply
/// unavailable (it must NEVER silently fall back to Apple's server-based
/// recognition).
///
/// v2 (mic-everywhere): `state` widens the answer without weakening the
/// guarantee — on iOS 26+ the new Speech framework can DOWNLOAD an on-device
/// model for many locales, so "not installed" is no longer "never":
///   * "installed"    — capture can start now (`available == true`)
///   * "downloadable" — an on-device model exists for this locale but its
///                      assets aren't installed yet; `downloadSpeechAssets`
///                      fetches them, after which state becomes "installed".
///                      Recognition still happens 100% on-device.
///   * "unavailable"  — no on-device path exists for this locale on this
///                      device/OS. Never a cue to use a server.
public struct SpeechCapability: Codable, Equatable {
    /// True iff capture may start right now: an on-device recognizer exists,
    /// is available, and its model/assets are installed.
    public let available: Bool
    /// Whether the resolved engine supports on-device recognition. Reported
    /// separately for diagnostics; `available` already folds it in.
    public let supportsOnDevice: Bool
    /// The concrete locale id the recognizer would use ("en-US"), or "" if none.
    public let locale: String
    /// "installed" | "downloadable" | "unavailable" (see type doc).
    public let state: String
    /// Which capture engine serves the locale: "legacy" (SFSpeechRecognizer),
    /// "analyzer" (SpeechTranscriber), "dictation" (DictationTranscriber), or "".
    public let engine: String

    public init(available: Bool, supportsOnDevice: Bool, locale: String,
                state: String = "unavailable", engine: String = "") {
        self.available = available
        self.supportsOnDevice = supportsOnDevice
        self.locale = locale
        self.state = state
        self.engine = engine
    }
}

/// Assembles the answer to `speechCapabilities(lang)`.
/// Doctrine (same as the rest of NativeLanguageKit): report what the platform
/// can do, decide nothing on the caller's behalf — and here, refuse to report
/// "available" for anything that would require the network.
public enum SpeechCapabilities {

    /// The set of locale ids the legacy speech recognizer supports, as strings,
    /// so the shared `LocaleResolver` (which also serves UITextChecker/AVSpeech)
    /// can pick the best id for an app language. Injectable for unit tests.
    public static func supportedLocaleIds() -> [String] {
        SFSpeechRecognizer.supportedLocales().map { $0.identifier }
    }

    /// Pure locale resolution: which recognizer locale id (if any) serves `lang`,
    /// given the set the device offers. Extracted so it is unit-testable headless
    /// without touching the (permission-gated, device-only) recognizer itself.
    public static func resolveLocaleId(lang: String, from available: [String]) -> String? {
        LocaleResolver.resolve(lang, from: available)
    }

    /// Legacy (pre-iOS-26) capability report: SFSpeechRecognizer only, available
    /// iff its on-device model happens to be installed (in practice: the locales
    /// the user has enabled as dictation keyboards). Never throws; never blocks.
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
        return SpeechCapability(available: available, supportsOnDevice: onDevice,
                                locale: localeId,
                                state: available ? "installed" : "unavailable",
                                engine: available ? "legacy" : "")
    }

    /// Full capability ladder. Order:
    ///  1. legacy SFSpeechRecognizer with an installed on-device model — zero
    ///     new moving parts, exactly the shipped behavior;
    ///  2. iOS 26 `SpeechTranscriber` (best models, downloadable assets);
    ///  3. iOS 26 `DictationTranscriber` (much wider locale set, downloadable).
    /// Anything else is honestly unavailable. All three paths are on-device.
    public static func fullReport(lang: String) async -> SpeechCapability {
        let legacy = report(lang: lang)
        if legacy.available {
            return legacy
        }
        if #available(iOS 26.0, *) {
            let st = await SpeechTranscriber.supportedLocales.map { $0.identifier }
            if let id = resolveLocaleId(lang: lang, from: st) {
                let module = SpeechTranscriber(locale: Locale(identifier: id),
                                               preset: .progressiveTranscription)
                if let cap = await capability(for: module, locale: id, engine: "analyzer") {
                    return cap
                }
            }
            let dt = await DictationTranscriber.supportedLocales.map { $0.identifier }
            if let id = resolveLocaleId(lang: lang, from: dt) {
                let module = DictationTranscriber(locale: Locale(identifier: id),
                                                  preset: .shortDictation)
                if let cap = await capability(for: module, locale: id, engine: "dictation") {
                    return cap
                }
            }
        }
        return legacy // unavailable, with whatever locale diagnostics legacy found
    }

    @available(iOS 26.0, *)
    private static func capability(for module: any SpeechModule, locale: String,
                                   engine: String) async -> SpeechCapability? {
        switch await AssetInventory.status(forModules: [module]) {
        case .installed:
            return SpeechCapability(available: true, supportsOnDevice: true,
                                    locale: locale, state: "installed", engine: engine)
        case .supported, .downloading:
            return SpeechCapability(available: false, supportsOnDevice: true,
                                    locale: locale, state: "downloadable", engine: engine)
        default:
            return nil
        }
    }
}
