import Foundation
import AVFoundation
import Capacitor
import NativeLanguageKitCore
import WidgetKit

/// Thin Capacitor bridge over NativeLanguageKitCore. All real logic lives in the
/// (unit-tested) package; this only marshals CAPPluginCall <-> the core and owns
/// the one long-lived Speaker + the audio session. Doctrine: capability provider,
/// never a decision-maker — every method reports or executes an explicit request.
@objc(NativeLanguageKitPlugin)
public class NativeLanguageKitPlugin: CAPPlugin, CAPBridgedPlugin {
    public let identifier = "NativeLanguageKitPlugin"
    public let jsName = "NativeLanguageKit"
    public let pluginMethods: [CAPPluginMethod] = [
        CAPPluginMethod(name: "capabilities", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "speak", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "stop", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "checkWord", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "detectLanguage", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "syncWidgetState", returnType: CAPPluginReturnPromise),
    ]

    private let speaker = Speaker()

    @objc func capabilities(_ call: CAPPluginCall) {
        let report = Capabilities.report(lang: call.getString("lang") ?? "")
        call.resolve(report.asDictionary())
    }

    @objc func speak(_ call: CAPPluginCall) {
        // voiceId is REQUIRED — the plugin never picks a voice (Decision D3).
        guard let text = call.getString("text"), let voiceId = call.getString("voiceId") else {
            call.reject("text and voiceId are required", "BAD_ARGS")
            return
        }
        let rate = Float(call.getDouble("rate") ?? Double(SpeechRate.gameNormal))
        // Match the cached-audio path: .playback so it respects the silent switch
        // exactly like audio-native.js configure({focus:true}) does. No regression
        // where the native path suddenly ignores the mute switch.
        try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
        try? AVAudioSession.sharedInstance().setActive(true)
        DispatchQueue.main.async {
            self.speaker.speak(text: text, voiceId: voiceId, gameRate: rate) { ok in
                if ok {
                    call.resolve()
                } else {
                    // Unknown voice, or superseded by a newer speak/stop.
                    call.reject("speech did not complete", "SPEAK_INCOMPLETE")
                }
            }
        }
    }

    @objc func stop(_ call: CAPPluginCall) {
        DispatchQueue.main.async {
            self.speaker.stop()
            call.resolve()
        }
    }

    @objc func checkWord(_ call: CAPPluginCall) {
        let r = WordChecker.check(word: call.getString("word") ?? "", lang: call.getString("lang") ?? "")
        call.resolve(["supported": r.supported, "isWord": r.isWord])
    }

    @objc func detectLanguage(_ call: CAPPluginCall) {
        let g = LanguageDetector.detect(text: call.getString("text") ?? "")
        call.resolve(["supported": g.supported, "lang": g.lang, "confidence": g.confidence])
    }

    /// F3 — persist the widget-state JSON (verbatim) into the App Group container
    /// the WidgetKit extension + F4 App Intents read, then nudge WidgetCenter so
    /// the home-screen widgets refresh promptly. Best-effort: a missing App Group
    /// (misprovisioned build) just no-ops rather than disturbing game flow.
    @objc func syncWidgetState(_ call: CAPPluginCall) {
        guard let state = call.getString("state") else {
            call.reject("state is required", "BAD_ARGS")
            return
        }
        WidgetShared.write(rawJSON: state)
        WidgetCenter.shared.reloadAllTimelines()
        call.resolve()
    }
}

private extension Encodable {
    /// Encode a Codable capability struct to the [String: Any] Capacitor returns.
    func asDictionary() -> [String: Any] {
        guard let data = try? JSONEncoder().encode(self),
              let dict = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return [:] }
        return dict
    }
}
