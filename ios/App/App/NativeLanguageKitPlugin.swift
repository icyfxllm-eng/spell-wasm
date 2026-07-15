import Foundation
import AVFoundation
import Speech
import Capacitor
import NativeLanguageKitCore

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
        // Feature F2 "Say It" — on-device pronunciation practice.
        CAPPluginMethod(name: "speechCapabilities", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "startListening", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "stopListening", returnType: CAPPluginReturnPromise),
        // "Spell It Out Loud" — on-device LETTER capture (second recognition
        // profile over the SAME mic): raw tokens stream back as plugin events.
        CAPPluginMethod(name: "startLetterCapture", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "stopLetterCapture", returnType: CAPPluginReturnPromise),
    ]

    private let speaker = Speaker()
    private let listener = SpeechListener()

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

    // MARK: Say It (Feature F2) — on-device speech recognition.

    /// Report whether `lang` can be recognized ON-DEVICE. `available` is false
    /// unless on-device recognition is supported (see SpeechCapabilities) — the
    /// web layer treats `available:false` as "mode UNAVAILABLE", never as a cue to
    /// use server recognition.
    @objc func speechCapabilities(_ call: CAPPluginCall) {
        let cap = SpeechCapabilities.report(lang: call.getString("lang") ?? "")
        call.resolve(cap.asDictionary())
    }

    /// Start ON-DEVICE listening (requiresOnDeviceRecognition = true). Resolves
    /// `{ transcription }` with the final on-device transcription. Rejects with a
    /// specific code the web maps to a state — "UNAVAILABLE" (no on-device path,
    /// never falls back to a server), "PERMISSION_DENIED" (→ needs-mic state),
    /// "BUSY", "AUDIO_ERROR", or "NO_SPEECH". The child's voice is streamed only
    /// to the on-device recognizer; it is never written to disk or sent anywhere.
    @objc func startListening(_ call: CAPPluginCall) {
        let lang = call.getString("lang") ?? ""
        DispatchQueue.main.async {
            self.listener.start(lang: lang) { result in
                switch result {
                case .success(let text):
                    call.resolve(["transcription": text])
                case .failure(let err):
                    call.reject(err.rawValue, err.rawValue)
                }
            }
        }
    }

    /// Stop listening and finalize; the in-flight startListening resolves with
    /// whatever on-device transcription was captured.
    @objc func stopListening(_ call: CAPPluginCall) {
        DispatchQueue.main.async {
            self.listener.stop()
            call.resolve()
        }
    }

    // MARK: Spell It Out Loud — on-device letter capture.

    /// Start ON-DEVICE letter capture. The recognizer is biased with the
    /// `contextualStrings` the caller passes (the language's spoken letter names,
    /// sourced from the Rust lexicon — never hardcoded here) and streams RAW
    /// transcript tokens (partials included) back as plugin events: `letterToken`
    /// per partial, `letterFinal` once, `letterError` with a code. The plugin does
    /// ZERO parsing — the Rust `spell_aloud` parser owns all linguistic knowledge.
    /// Resolves immediately as an ack; the transcript never leaves the phone.
    @objc func startLetterCapture(_ call: CAPPluginCall) {
        let lang = call.getString("lang") ?? ""
        let contextual = (call.getArray("contextualStrings")?.compactMap { $0 as? String }) ?? []
        DispatchQueue.main.async {
            self.listener.startLetters(
                lang: lang,
                contextualStrings: contextual,
                onPartial: { [weak self] text in
                    self?.notifyListeners("letterToken", data: ["token": text])
                },
                onFinal: { [weak self] text in
                    self?.notifyListeners("letterFinal", data: ["token": text])
                },
                onError: { [weak self] err in
                    self?.notifyListeners("letterError", data: ["code": err.rawValue])
                }
            )
            call.resolve()
        }
    }

    /// Stop letter capture and finalize; the recognizer emits its final result,
    /// which is delivered via the `letterFinal` event.
    @objc func stopLetterCapture(_ call: CAPPluginCall) {
        DispatchQueue.main.async {
            self.listener.stop()
            call.resolve()
        }
    }
}

/// Live on-device speech capture for Say-It. HARD RULE: on-device only —
/// `requiresOnDeviceRecognition = true`, and we refuse to start unless
/// `SpeechCapabilities.report(...).available` is true for the locale (which is
/// itself gated on `supportsOnDeviceRecognition`). The mic buffer is streamed
/// straight to the on-device recognizer; nothing is persisted or transmitted. A
/// child's voice never leaves the phone.
///
/// Not unit-testable headless (AVAudioEngine + SFSpeechRecognizer need a device
/// and permission grants) — the deterministic pieces it relies on
/// (SpeechCapabilities locale resolution, the SpeechMatcher rule) are XCTested in
/// NativeLanguageKitCore instead.
final class SpeechListener {
    enum ListenError: String, Error {
        case unavailable = "UNAVAILABLE"
        case permissionDenied = "PERMISSION_DENIED"
        case busy = "BUSY"
        case audio = "AUDIO_ERROR"
        case noSpeech = "NO_SPEECH"
    }

    private let audioEngine = AVAudioEngine()
    private var request: SFSpeechAudioBufferRecognitionRequest?
    private var task: SFSpeechRecognitionTask?
    private var recognizer: SFSpeechRecognizer?
    private var completion: ((Result<String, ListenError>) -> Void)?
    private var finished = false
    private var best = ""
    // Letter-capture profile only: live partial-token stream + recognizer biasing.
    // nil for the Say-It (whole-word) profile, so that path is byte-for-byte as it
    // was — ONE capture engine, two profiles, never a second microphone.
    private var partialHandler: ((String) -> Void)?
    private var contextualStrings: [String] = []

    var isListening: Bool { task != nil }

    func start(lang: String, completion: @escaping (Result<String, ListenError>) -> Void) {
        // Say-It (whole-word) profile: no biasing, no partial streaming.
        begin(lang: lang, contextualStrings: [], onPartial: nil, completion: completion)
    }

    /// Spell It Out Loud (letter) profile: SAME on-device recognizer, biased with
    /// `contextualStrings` (the language's letter names) and streaming raw partials
    /// live via `onPartial`. Reuses the exact engine/session path as Say-It.
    func startLetters(
        lang: String,
        contextualStrings: [String],
        onPartial: @escaping (String) -> Void,
        onFinal: @escaping (String) -> Void,
        onError: @escaping (ListenError) -> Void
    ) {
        begin(lang: lang, contextualStrings: contextualStrings, onPartial: onPartial) { result in
            switch result {
            case .success(let text): onFinal(text)
            case .failure(let err): onError(err)
            }
        }
    }

    /// Shared entry for both profiles. The ONLY differences are `contextualStrings`
    /// (recognizer biasing) and `onPartial` (live token streaming); everything else
    /// — auth, session, engine, on-device enforcement — is identical.
    private func begin(
        lang: String,
        contextualStrings: [String],
        onPartial: ((String) -> Void)?,
        completion: @escaping (Result<String, ListenError>) -> Void
    ) {
        if isListening { completion(.failure(.busy)); return }
        // Fail closed: only proceed when on-device recognition is truly available.
        let cap = SpeechCapabilities.report(lang: lang)
        guard cap.available, let rec = SFSpeechRecognizer(locale: Locale(identifier: cap.locale)) else {
            completion(.failure(.unavailable)); return
        }
        recognizer = rec
        self.completion = completion
        self.partialHandler = onPartial
        self.contextualStrings = contextualStrings
        finished = false
        best = ""
        // OS permission prompts appear HERE, at first use — the web layer shows a
        // plain-language pre-prompt before this call.
        ensureAuthorized { [weak self] granted in
            guard let self = self else { return }
            guard granted else { self.finish(.failure(.permissionDenied)); return }
            self.beginCapture()
        }
    }

    /// Stop capture; the recognition task then emits its final result and resolves
    /// the pending `start` completion.
    func stop() {
        guard isListening else { return }
        request?.endAudio()
        if audioEngine.isRunning {
            audioEngine.stop()
            audioEngine.inputNode.removeTap(onBus: 0)
        }
        // Safety net: if the task doesn't finalize promptly, resolve with what we
        // have so the UI never hangs.
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in
            guard let self = self, !self.finished else { return }
            self.finish(self.best.isEmpty ? .failure(.noSpeech) : .success(self.best))
        }
    }

    private func ensureAuthorized(_ done: @escaping (Bool) -> Void) {
        SFSpeechRecognizer.requestAuthorization { status in
            guard status == .authorized else { DispatchQueue.main.async { done(false) }; return }
            AVAudioSession.sharedInstance().requestRecordPermission { granted in
                DispatchQueue.main.async { done(granted) }
            }
        }
    }

    private func beginCapture() {
        guard let recognizer = recognizer else { finish(.failure(.unavailable)); return }
        let req = SFSpeechAudioBufferRecognitionRequest()
        req.requiresOnDeviceRecognition = true   // HARD on-device — never the server.
        req.shouldReportPartialResults = true
        // Letter profile: bias the recognizer toward the language's spoken letter
        // names (passed from Rust/JS, never hardcoded). Empty for the Say-It profile.
        if !contextualStrings.isEmpty {
            req.contextualStrings = contextualStrings
        }
        request = req

        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playAndRecord, mode: .measurement, options: [.duckOthers, .defaultToSpeaker])
            try session.setActive(true, options: .notifyOthersOnDeactivation)
        } catch { finish(.failure(.audio)); return }

        let input = audioEngine.inputNode
        let format = input.outputFormat(forBus: 0)
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            self?.request?.append(buffer)
        }
        audioEngine.prepare()
        do { try audioEngine.start() } catch { finish(.failure(.audio)); return }

        task = recognizer.recognitionTask(with: req) { [weak self] result, error in
            guard let self = self else { return }
            if let result = result {
                self.best = result.bestTranscription.formattedString
                // Letter profile streams every partial (the growing transcript) so
                // the Rust parser can echo "C… CA… CAT" live.
                self.partialHandler?(self.best)
                if result.isFinal { self.finish(.success(self.best)) }
            }
            if error != nil {
                self.finish(self.best.isEmpty ? .failure(.noSpeech) : .success(self.best))
            }
        }
    }

    private func finish(_ result: Result<String, ListenError>) {
        if finished { return }
        finished = true
        audioEngine.inputNode.removeTap(onBus: 0)
        if audioEngine.isRunning { audioEngine.stop() }
        task?.cancel()
        task = nil
        request = nil
        partialHandler = nil
        contextualStrings = []
        // Restore the game's normal .playback session so word audio keeps working.
        try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
        try? AVAudioSession.sharedInstance().setActive(true)
        let cb = completion
        completion = nil
        cb?(result)
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
