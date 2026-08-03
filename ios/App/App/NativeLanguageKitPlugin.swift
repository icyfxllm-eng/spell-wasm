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
        // CC-FINALE feature 3 (add-only Photos). Declaring the method is not
        // optional bookkeeping: without this row the bridge has no route for
        // it, the JS call rejects, and Save fails in a way that looks exactly
        // like "no plugin installed".
        CAPPluginMethod(name: "savePicture", returnType: CAPPluginReturnPromise),
        // CC-IOS-SURFACES (BD-1): the one-way widget snapshot writer and
        // the Spotlight donor. Without these rows the bridge has no route
        // and the JS call rejects like the plugin is absent.
        CAPPluginMethod(name: "writeWidgetSnapshot", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "donateSpotlight", returnType: CAPPluginReturnPromise),
        // CC-OFFLINE-PACKS (BD-2): per-file fetch/verify, atomic activate,
        // storage states, and the pack-first audio src resolver.
        CAPPluginMethod(name: "packVerifyManifest", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "packFetch", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "packMissing", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "packActivate", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "packDelete", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "packStates", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "packSrcFor", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "packStoreManifest", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "speak", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "speakSyllables", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "stop", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "checkWord", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "detectLanguage", returnType: CAPPluginReturnPromise),
        // Feature F2 "Say It" — on-device pronunciation practice.
        CAPPluginMethod(name: "speechCapabilities", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "startListening", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "stopListening", returnType: CAPPluginReturnPromise),
        // Feature F1 "Photo-to-word-list" — on-device VisionKit OCR (see
        // NativeLanguageKitPlugin+PhotoList.swift).
        CAPPluginMethod(name: "recognizeWordList", returnType: CAPPluginReturnPromise),
        // "Spell It Out Loud" — on-device LETTER capture (second recognition
        // profile over the SAME mic): raw tokens stream back as plugin events.
        CAPPluginMethod(name: "startLetterCapture", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "stopLetterCapture", returnType: CAPPluginReturnPromise),
        // Mic-everywhere: download the ON-DEVICE speech model for a locale
        // (iOS 26+ Speech framework assets). Progress streams via the
        // `speechAssetProgress` event; recognition never leaves the phone.
        CAPPluginMethod(name: "downloadSpeechAssets", returnType: CAPPluginReturnPromise),
    ]

    private let speaker = Speaker()
    private let syllableSpeaker = SyllableSpeaker()
    private let listener = SpeechListener()

    // F1 async state — internal (not private) so the +PhotoList extension file
    // can reach it. Holds the in-flight JS call while the picker + Vision run.
    var pendingCall: CAPPluginCall?
    var recognitionLanguages: [String] = ["en-US"]
    // Vision language correction for the photo path (Phase 2): ON for languages
    // Vision models natively; OFF for the English-recognizer fallback languages.
    var recognitionCorrection = true

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

    @objc func speakSyllables(_ call: CAPPluginCall) {
        // voiceId is REQUIRED — the plugin never picks a voice (Decision D3).
        guard let syllables = call.getArray("syllables", String.self), !syllables.isEmpty,
              let voiceId = call.getString("voiceId") else {
            call.reject("syllables and voiceId are required", "BAD_ARGS")
            return
        }
        let rate = Float(call.getDouble("rate") ?? Double(SpeechRate.gameNormal))
        // Same audio-session handling as `speak`: .playback so the offline
        // syllable replay respects the silent switch like the cached-audio path.
        try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
        try? AVAudioSession.sharedInstance().setActive(true)
        DispatchQueue.main.async {
            self.syllableSpeaker.speak(
                syllables: syllables, voiceId: voiceId, gameRate: rate,
                onSyllable: { idx in
                    // Stream each syllable boundary to the web layer as an event;
                    // the JS bridge relays it to the highlight callback.
                    self.notifyListeners("syllableBoundary", data: ["index": idx])
                },
                onComplete: { ok in
                    if ok {
                        call.resolve()
                    } else {
                        call.reject("speech did not complete", "SPEAK_INCOMPLETE")
                    }
                }
            )
        }
    }

    @objc func stop(_ call: CAPPluginCall) {
        DispatchQueue.main.async {
            self.speaker.stop()
            self.syllableSpeaker.stop()
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
        let lang = call.getString("lang") ?? ""
        Task {
            let cap = await SpeechCapabilities.fullReport(lang: lang)
            call.resolve(cap.asDictionary())
        }
    }

    /// Download the on-device speech model assets for `lang` (iOS 26+). The
    /// child-privacy doctrine is untouched: assets make ON-DEVICE recognition
    /// possible; nothing about this enables any server path. Progress is
    /// streamed as `speechAssetProgress {fraction}`; resolves `{installed}`.
    @objc func downloadSpeechAssets(_ call: CAPPluginCall) {
        let lang = call.getString("lang") ?? ""
        guard #available(iOS 26.0, *) else {
            call.reject("UNAVAILABLE", "UNAVAILABLE"); return
        }
        Task {
            let cap = await SpeechCapabilities.fullReport(lang: lang)
            guard !cap.locale.isEmpty, cap.state == "downloadable" || cap.state == "installed" else {
                call.reject("UNAVAILABLE", "UNAVAILABLE"); return
            }
            if cap.state == "installed" {
                call.resolve(["installed": true]); return
            }
            let locale = Locale(identifier: cap.locale)
            let module: any SpeechModule = cap.engine == "dictation"
                ? DictationTranscriber(locale: locale, preset: .shortDictation)
                : SpeechTranscriber(locale: locale, preset: .progressiveTranscription)
            do {
                if let req = try await AssetInventory.assetInstallationRequest(supporting: [module]) {
                    let progress = req.progress
                    let poll = Task { [weak self] in
                        while !Task.isCancelled {
                            self?.notifyListeners("speechAssetProgress",
                                                  data: ["fraction": progress.fractionCompleted])
                            try? await Task.sleep(nanoseconds: 250_000_000)
                        }
                    }
                    defer { poll.cancel() }
                    try await req.downloadAndInstall()
                }
                self.notifyListeners("speechAssetProgress", data: ["fraction": 1.0])
                call.resolve(["installed": true])
            } catch {
                call.reject("DOWNLOAD_FAILED", "DOWNLOAD_FAILED")
            }
        }
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
        let serverUrl = call.getString("serverUrl") ?? ""
        DispatchQueue.main.async {
            self.listener.startLetters(
                lang: lang,
                contextualStrings: contextual,
                serverUrl: serverUrl,
                onPartial: { [weak self] text in
                    self?.notifyListeners("letterToken", data: ["token": text])
                },
                onSegment: { [weak self] text, confidence, alt in
                    // ONE-PRESS mid-stream letter: `end:false` — the session keeps
                    // listening for the next letter; the web layer keeps its listeners.
                    self?.notifyListeners("letterFinal", data: ["token": text, "confidence": confidence, "alt": alt, "end": false])
                },
                onFinal: { [weak self] text, confidence, alt in
                    // Additive payload (Phase 3): `token` is unchanged for the input
                    // method; `confidence`/`alt` let the mode offer a confusable chip.
                    // `end:true`: the whole capture session is over (user stop).
                    self?.notifyListeners("letterFinal", data: ["token": text, "confidence": confidence, "alt": alt, "end": true])
                },
                onError: { [weak self] err in
                    self?.notifyListeners("letterError", data: ["code": err.rawValue])
                },
                onDiag: { [weak self] info in
                    self?.notifyListeners("letterDiag", data: ["info": info])
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
        case network = "NETWORK"
    }

    private let audioEngine = AVAudioEngine()
    // Mic-everywhere engine selection: "legacy" = SFSpeechRecognizer (exactly
    // the shipped path), "analyzer"/"dictation" = the iOS 26 Speech framework
    // (SpeechTranscriber / DictationTranscriber) whose on-device models are
    // downloadable per locale. All engines are 100% on-device.
    private var engineKind = "legacy"
    // iOS-26 session state, stored type-erased so the class still compiles at
    // the iOS 15 deployment target (only touched inside #available blocks).
    // Server STT rung (mic-everywhere): capture VAD-segmented PCM and POST each
    // segment to the Spell backend. ONLY entered when the web layer passes
    // serverUrl — which it does exclusively after the explicit consent card,
    // for languages with no on-device model, never in Kid Mode.
    private var serverUrl = ""
    private var serverLang = ""
    private var serverSeg = Data()
    private var serverConverter: AVAudioConverter?
    private static let serverFormat = AVAudioFormat(commonFormat: .pcmFormatInt16,
                                                    sampleRate: 16000, channels: 1,
                                                    interleaved: true)!
    private var anyAnalyzer: Any?
    private var anyAnalyzerCont: Any?
    private var anyResultsTask: Any?
    private var analyzerConverter: AVAudioConverter?
    private var analyzerFormat: AVAudioFormat?
    /// Finalized text accumulated since the last VAD boundary (analyzer path).
    private var analyzerSeg = ""
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
    // TEMP capture diagnostic (letter profile): counts mic buffers reaching the tap and
    // reports the input format, so an on-device "Listening but nothing captured" report
    // can be pinpointed to routing (no buffers) vs recognizer/model (buffers but no
    // tokens) without a device debugger. Emitted via the `letterDiag` event.
    private var tapCount = 0
    private var diagHandler: ((String) -> Void)?
    // Letter-capture confusable surfacing (CC-SPELL-ALOUD Phase 3): the confidence of
    // the least-sure segment in the final transcription, and that segment's top
    // alternative reading. The Rust parser turns these into a two-choice chip; here we
    // only report them (no linguistic judgment on-device — I4).
    private var lastConfidence: Double = 1.0
    private var lastAlt: String = ""
    // VAD auto-segmentation for ONE-PRESS continuous letter capture: while `continuous`,
    // a short silence AFTER speech ends the current letter — the request is endAudio'd,
    // the task finalizes THAT letter (emitted mid-stream via `segmentHandler`), and a
    // fresh request/task starts on the SAME running engine + audio session. One press,
    // spell letter by letter with a beat between letters; teardown only on user stop.
    private var continuous = false
    // Mid-stream per-letter emission (letters profile only); the pending `completion`
    // still fires exactly once, at the true end of the whole session.
    private var segmentHandler: ((String, Double, String) -> Void)?
    // Set by user stop: the NEXT finalization ends the session instead of cycling.
    private var stopping = false
    // Monotonic recognition-cycle id: callbacks from a superseded cycle's task are
    // ignored (a finalized/hung task can still call back after its replacement starts).
    private var cycleGen = 0
    // Highest RMS seen this session — reported in the diag line for VAD tuning.
    private var peakRMS: Float = 0
    // THE anti-letter-loss gap fix: finalizing a segment takes the recognizer up to
    // ~a second, and a letter spoken in that window would otherwise be appended to the
    // already-ended request and silently DROPPED (the "said s right after the pause,
    // s never landed" bug). While `finalizing`, mic buffers are stashed here and
    // replayed into the next cycle's request the moment it exists.
    private var finalizing = false
    private var pendingBuffers: [AVAudioPCMBuffer] = []
    private let pendingLock = NSLock()
    private var sawSpeech = false
    private var silenceSecs = 0.0
    private var segmentSecs = 0.0
    private let speechRMS: Float = 0.010   // RMS above this = speech (device-tunable;
                                           // low enough to catch soft sibilants: "ess")
    private let silenceCutoff = 0.35       // s of silence after a letter = boundary
    private let maxSegment = 2.5           // s: force a boundary (safety; never hang)

    /// RMS level of a mic buffer (mono) — the VAD speech/silence signal.
    private static func rms(_ buffer: AVAudioPCMBuffer) -> Float {
        guard let ch = buffer.floatChannelData?[0] else { return 0 }
        let n = Int(buffer.frameLength)
        if n == 0 { return 0 }
        var sum: Float = 0
        for i in 0..<n { let s = ch[i]; sum += s * s }
        return (sum / Float(n)).squareRoot()
    }

    var isListening: Bool { task != nil || anyResultsTask != nil || engineKind == "server" }

    func start(lang: String, completion: @escaping (Result<String, ListenError>) -> Void) {
        // Say-It (whole-word) profile: no biasing, no partial streaming, no VAD.
        continuous = false
        segmentHandler = nil
        begin(lang: lang, contextualStrings: [], onPartial: nil, completion: completion)
    }

    /// Spell It Out Loud (letter) profile: SAME on-device recognizer, biased with
    /// `contextualStrings` (the language's letter names) and streaming raw partials
    /// live via `onPartial`. ONE-PRESS: VAD auto-segments each letter, emitting it
    /// mid-stream via `onSegment` while the session keeps listening for the next;
    /// `onFinal` fires once, with the last segment, when the user stops.
    func startLetters(
        lang: String,
        contextualStrings: [String],
        serverUrl: String = "",
        onPartial: @escaping (String) -> Void,
        onSegment: @escaping (String, Double, String) -> Void,
        onFinal: @escaping (String, Double, String) -> Void,
        onError: @escaping (ListenError) -> Void,
        onDiag: @escaping (String) -> Void
    ) {
        diagHandler = onDiag
        continuous = true // letters: VAD auto-segments each letter (one-press)
        segmentHandler = onSegment
        self.serverUrl = serverUrl
        self.serverLang = lang
        begin(lang: lang, contextualStrings: contextualStrings, onPartial: onPartial) { [weak self] result in
            switch result {
            case .success(let text): onFinal(text, self?.lastConfidence ?? 1.0, self?.lastAlt ?? "")
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
        // Rapid re-tap: an old session still winding down would have answered BUSY,
        // making the mic feel dead until its teardown finished. The new capture wins
        // instead — the old session is dropped outright (its letters were already
        // delivered per segment).
        if isListening { supersede() }
        self.completion = completion
        self.partialHandler = onPartial
        self.contextualStrings = contextualStrings
        finished = false
        stopping = false
        best = ""
        analyzerSeg = ""
        lastConfidence = 1.0
        lastAlt = ""
        // Fail closed: only proceed when ON-DEVICE recognition is truly available
        // for the locale — via the full engine ladder (legacy recognizer, or an
        // iOS-26 transcriber whose assets are installed). Never a server.
        if !serverUrl.isEmpty {
            // Server rung: mic permission only (no speech-recognizer auth — no
            // recognizer runs on-device; the consented backend does the work).
            engineKind = "server"
            AVAudioSession.sharedInstance().requestRecordPermission { [weak self] granted in
                DispatchQueue.main.async {
                    guard let self = self else { return }
                    guard granted else { self.finish(.failure(.permissionDenied)); return }
                    self.beginServerCapture()
                }
            }
            return
        }
        Task { [weak self] in
            let cap = await SpeechCapabilities.fullReport(lang: lang)
            DispatchQueue.main.async {
                guard let self = self else { return }
                guard cap.available, cap.state == "installed" else {
                    self.finish(.failure(.unavailable)); return
                }
                if cap.engine == "legacy" {
                    guard let rec = SFSpeechRecognizer(locale: Locale(identifier: cap.locale)) else {
                        self.finish(.failure(.unavailable)); return
                    }
                    self.recognizer = rec
                }
                self.engineKind = cap.engine
                // OS permission prompts appear HERE, at first use — the web layer
                // shows a plain-language pre-prompt before this call.
                self.ensureAuthorized { granted in
                    guard granted else { self.finish(.failure(.permissionDenied)); return }
                    if cap.engine == "legacy" {
                        self.beginCapture()
                    } else if #available(iOS 26.0, *) {
                        self.beginAnalyzerCapture(localeId: cap.locale, engine: cap.engine)
                    } else {
                        self.finish(.failure(.unavailable))
                    }
                }
            }
        }
    }

    /// Stop capture; the recognition task then emits its final result and resolves
    /// the pending `start` completion. In continuous (letter) mode this is the USER
    /// stop — `stopping` makes the next finalization end the session instead of
    /// cycling to another segment.
    func stop() {
        if engineKind == "server" {
            serverStop()
            return
        }
        if engineKind != "legacy" {
            analyzerStop()
            return
        }
        guard isListening else { return }
        stopping = true
        request?.endAudio()
        if audioEngine.isRunning {
            audioEngine.stop()
            audioEngine.inputNode.removeTap(onBus: 0)
        }
        // Safety net: if the task doesn't finalize promptly, resolve with what we
        // have so the UI never hangs. Guarded by cycleGen so a session superseded by
        // a rapid re-tap can't have its stale net finish the NEW session.
        let gen = cycleGen
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in
            guard let self = self, gen == self.cycleGen, !self.finished else { return }
            self.finish(self.best.isEmpty ? .failure(.noSpeech) : .success(self.best))
        }
    }

    /// Drop a live session so a new one can start immediately (rapid re-tap). The old
    /// task is cancelled, its pending completion discarded WITHOUT firing (no stale
    /// end event leaking into the new session), and the engine freed. Bumping
    /// `cycleGen` invalidates the old session's callbacks and safety nets.
    private func supersede() {
        cycleGen += 1
        task?.cancel()
        task = nil
        request = nil
        completion = nil
        teardownAnalyzer()
        audioEngine.inputNode.removeTap(onBus: 0)
        if audioEngine.isRunning { audioEngine.stop() }
        pendingLock.lock()
        finalizing = false
        pendingBuffers.removeAll()
        pendingLock.unlock()
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

        do {
            let session = AVAudioSession.sharedInstance()
            // The app sets `.playback` (output-only) active at launch for word audio.
            // Switching category on an already-active session doesn't reliably route
            // the mic input, so deactivate first, then reconfigure for record + speaker
            // and reactivate — otherwise the tap sees no audio ("Listening", nothing
            // captured). `.allowBluetooth` picks up headset mics too.
            try? session.setActive(false, options: .notifyOthersOnDeactivation)
            try session.setCategory(.playAndRecord, mode: .measurement,
                                    options: [.duckOthers, .defaultToSpeaker, .allowBluetooth])
            try session.setActive(true, options: .notifyOthersOnDeactivation)
        } catch { finish(.failure(.audio)); return }

        // Defensive: clear any stale engine/tap state (e.g. an inputNode format cached
        // while the session was `.playback`) so the fresh tap gets real mic buffers.
        tapCount = 0
        audioEngine.stop()
        audioEngine.reset()
        sawSpeech = false
        silenceSecs = 0
        segmentSecs = 0
        peakRMS = 0

        // Read the input format AFTER the session is record-capable, so it isn't the
        // zero/invalid format a `.playback` session reports (which yields silent taps).
        let input = audioEngine.inputNode
        let format = input.outputFormat(forBus: 0)
        let sampleRate = format.sampleRate
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            guard let self = self else { return }
            self.tapCount += 1
            if self.engineKind == "server" {
                self.appendServerPCM(buffer)
            } else if self.engineKind != "legacy" {
                // Analyzer path: the input stream stays open across per-letter
                // finalization, so buffers always flow — no stash needed.
                self.yieldToAnalyzer(buffer)
            } else if self.finalizing {
                // A segment is finalizing: the current request is closed, the next one
                // doesn't exist yet. Stash so a letter spoken NOW isn't dropped.
                self.pendingLock.lock()
                self.pendingBuffers.append(buffer)
                if self.pendingBuffers.count > 256 { self.pendingBuffers.removeFirst() }
                self.pendingLock.unlock()
            } else {
                self.request?.append(buffer)
            }
            // VAD (letters only): a short silence after a letter is the boundary — end
            // this letter's recognition cycle (finalizing it); the session, engine, and
            // tap stay live, and a fresh cycle picks up the next letter.
            if self.continuous && sampleRate > 0 {
                let secs = Double(buffer.frameLength) / sampleRate
                let level = Self.rms(buffer)
                self.peakRMS = max(self.peakRMS, level)
                if level > self.speechRMS {
                    self.sawSpeech = true
                    self.silenceSecs = 0
                } else if self.sawSpeech {
                    self.silenceSecs += secs
                }
                self.segmentSecs += secs
                // While finalizing, DON'T fire (there's no live request to end) and
                // DON'T reset the counters — if a whole letter lands in the gap, its
                // boundary fires on the first buffer after the next cycle starts.
                if !self.finalizing {
                    let boundary = (self.sawSpeech && self.silenceSecs >= self.silenceCutoff)
                        || self.segmentSecs >= self.maxSegment
                    if boundary {
                        self.sawSpeech = false
                        self.silenceSecs = 0
                        self.segmentSecs = 0
                        DispatchQueue.main.async { [weak self] in self?.segmentBoundary() }
                    }
                }
            }
        }
        audioEngine.prepare()
        do { try audioEngine.start() } catch { finish(.failure(.audio)); return }

        // TEMP diagnostic: after ~1.2s report the mic format + how many buffers arrived
        // + whether on-device recognition is supported. buf=0 ⇒ audio isn't routing;
        // buf>0 but no letters ⇒ recognizer/on-device-model issue.
        let handler = diagHandler
        let onDev = recognizer.supportsOnDeviceRecognition
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak self] in
            guard let self = self else { return }
            handler?("sr=\(Int(format.sampleRate)) ch=\(format.channelCount) buf=\(self.tapCount) "
                + "onDev=\(onDev) peak=\(String(format: "%.3f", self.peakRMS))")
        }

        startRecognitionCycle()
    }

    /// One recognition cycle = one letter in continuous (one-press) mode, or the whole
    /// utterance for Say-It / user-stop. Builds a fresh request + task on the ALREADY
    /// RUNNING engine/session — cycling requests is what finalizes each letter fast
    /// without the per-letter session teardown that made one-press unreliable.
    private func startRecognitionCycle() {
        guard let recognizer = recognizer, !finished else { return }
        cycleGen += 1
        let gen = cycleGen
        let req = SFSpeechAudioBufferRecognitionRequest()
        req.requiresOnDeviceRecognition = true   // HARD on-device — never the server.
        req.shouldReportPartialResults = true
        // Tuning for ISOLATED LETTERS (the hard case): dictation hint for continuous
        // letter-by-letter speech, and no auto-punctuation (it turns "a" into "A." /
        // splices commas that break single-letter tokens). The letter profile always
        // biases toward the language's spoken letter names (from Rust/JS, never
        // hardcoded) — this is what pulls "see/ay/tee" toward C/A/T.
        req.taskHint = .dictation
        if #available(iOS 16.0, *) {
            req.addsPunctuation = false
        }
        if !contextualStrings.isEmpty {
            req.contextualStrings = contextualStrings
        }
        request = req
        best = ""
        // Replay audio captured while the PREVIOUS segment was finalizing — a letter
        // spoken during that gap reaches this cycle instead of being dropped.
        pendingLock.lock()
        let replay = pendingBuffers
        pendingBuffers.removeAll()
        finalizing = false
        pendingLock.unlock()
        for b in replay { req.append(b) }
        task = recognizer.recognitionTask(with: req) { [weak self] result, error in
            guard let self = self, gen == self.cycleGen, !self.finished else { return }
            if let result = result {
                self.best = result.bestTranscription.formattedString
                // Letter profile streams every partial (the growing transcript) so
                // the Rust parser can echo "C… CA… CAT" live.
                self.partialHandler?(self.best)
                if result.isFinal {
                    // Confusable surfacing (Phase 3): the least-confident segment and
                    // its top alternative — the Rust parser decides whether to chip.
                    let segs = result.bestTranscription.segments
                    if let doubtful = segs.min(by: { $0.confidence < $1.confidence }) {
                        self.lastConfidence = Double(doubtful.confidence)
                        self.lastAlt = doubtful.alternativeSubstrings.first ?? ""
                    } else {
                        self.lastConfidence = 1.0
                        self.lastAlt = ""
                    }
                    self.cycleEnded(.success(self.best))
                    return
                }
            }
            if error != nil {
                self.cycleEnded(self.best.isEmpty ? .failure(.noSpeech) : .success(self.best))
            }
        }
    }

    /// VAD detected the pause after a letter: finalize JUST this cycle. The engine,
    /// tap, and audio session stay live for the next letter.
    private func segmentBoundary() {
        if engineKind == "server" {
            serverBoundary(isEnd: false)
            return
        }
        if engineKind != "legacy" {
            analyzerBoundary()
            return
        }
        guard continuous, !stopping, !finished, task != nil else { return }
        let gen = cycleGen
        // From here until the next cycle's request exists, the tap stashes buffers
        // (see installTap) so speech during finalization is replayed, not lost.
        pendingLock.lock()
        finalizing = true
        pendingLock.unlock()
        request?.endAudio()
        // Safety net: if this cycle's task never delivers a final, force the next
        // cycle anyway so one flaky finalize can't stall the one-press stream.
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            guard let self = self, gen == self.cycleGen,
                  !self.finished, self.continuous, !self.stopping else { return }
            self.cycleEnded(self.best.isEmpty ? .failure(.noSpeech) : .success(self.best))
        }
    }

    /// A cycle finished (letter finalized, error, or safety net). Mid-stream in
    /// one-press mode: emit the letter and start the next cycle — errors (e.g. a
    /// no-speech segment from a false VAD boundary) do NOT end the session. On user
    /// stop or Say-It: end the whole session via `finish`.
    private func cycleEnded(_ result: Result<String, ListenError>) {
        if continuous && !stopping && audioEngine.isRunning {
            cycleGen += 1        // invalidate late callbacks from this cycle's task
            task?.cancel()       // harmless if already complete; kills a hung task
            switch result {
            case .success(let text) where !text.isEmpty:
                segmentHandler?(text, lastConfidence, lastAlt)
                diagHandler?("seg='\(text)' peak=\(String(format: "%.3f", peakRMS))")
            case .success:
                diagHandler?("seg=(empty) peak=\(String(format: "%.3f", peakRMS))")
            case .failure(let err):
                // e.g. a no-speech segment from a false VAD boundary — keep listening.
                diagHandler?("seg-err=\(err.rawValue) peak=\(String(format: "%.3f", peakRMS))")
            }
            startRecognitionCycle()
        } else {
            finish(result)
        }
    }

    // MARK: iOS 26 analyzer engine (SpeechTranscriber / DictationTranscriber).
    //
    // Same one-press shape as the legacy path: the SAME tap + VAD decide letter
    // boundaries; the difference is that per-letter finalization happens with
    // `analyzer.finalize(through: nil)` on ONE continuous session (the input
    // stream never closes mid-word), so there is no finalize-gap buffer stash.
    // On-device only — these engines run downloaded local models.

    /// Convert a mic-format buffer and feed it to the live analyzer stream.
    private func yieldToAnalyzer(_ buffer: AVAudioPCMBuffer) {
        guard #available(iOS 26.0, *) else { return }
        guard let cont = anyAnalyzerCont as? AsyncStream<AnalyzerInput>.Continuation else { return }
        var out = buffer
        if let conv = analyzerConverter, let fmt = analyzerFormat, fmt != buffer.format {
            let ratio = fmt.sampleRate / buffer.format.sampleRate
            let cap = AVAudioFrameCount(Double(buffer.frameLength) * ratio) + 64
            guard let converted = AVAudioPCMBuffer(pcmFormat: fmt, frameCapacity: cap) else { return }
            var err: NSError?
            var fed = false
            conv.convert(to: converted, error: &err) { _, status in
                if fed { status.pointee = .noDataNow; return nil }
                fed = true
                status.pointee = .haveData
                return buffer
            }
            if err != nil { return }
            out = converted
        }
        cont.yield(AnalyzerInput(buffer: out))
    }

    @available(iOS 26.0, *)
    private func beginAnalyzerCapture(localeId: String, engine: String) {
        do {
            let session = AVAudioSession.sharedInstance()
            try? session.setActive(false, options: .notifyOthersOnDeactivation)
            try session.setCategory(.playAndRecord, mode: .measurement,
                                    options: [.duckOthers, .defaultToSpeaker, .allowBluetooth])
            try session.setActive(true, options: .notifyOthersOnDeactivation)
        } catch { finish(.failure(.audio)); return }

        tapCount = 0
        audioEngine.stop()
        audioEngine.reset()
        sawSpeech = false
        silenceSecs = 0
        segmentSecs = 0
        peakRMS = 0
        analyzerSeg = ""

        let gen = cycleGen
        Task { [weak self] in
            // Build the module + analyzer off the main thread, then install the
            // tap and start the engine back on main (AVAudioEngine affinity).
            let locale = Locale(identifier: localeId)
            let (stream, cont) = AsyncStream<AnalyzerInput>.makeStream()
            if engine == "dictation" {
                let module = DictationTranscriber(locale: locale, preset: .shortDictation)
                let fmt = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [module])
                let analyzer = SpeechAnalyzer(modules: [module])
                do { try await analyzer.start(inputSequence: stream) }
                catch { DispatchQueue.main.async { self?.finish(.failure(.unavailable)) }; return }
                let results = Task { [weak self] in
                    do {
                        for try await r in module.results {
                            let text = String(r.text.characters)
                            DispatchQueue.main.async { self?.analyzerResult(text: text, isFinal: r.isFinal) }
                        }
                    } catch {}
                    DispatchQueue.main.async { self?.analyzerStreamEnded() }
                }
                DispatchQueue.main.async { self?.armAnalyzer(gen: gen, analyzer: analyzer, cont: cont, results: results, fmt: fmt) }
            } else {
                let module = SpeechTranscriber(locale: locale, preset: .progressiveTranscription)
                let fmt = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [module])
                let analyzer = SpeechAnalyzer(modules: [module])
                do { try await analyzer.start(inputSequence: stream) }
                catch { DispatchQueue.main.async { self?.finish(.failure(.unavailable)) }; return }
                let results = Task { [weak self] in
                    do {
                        for try await r in module.results {
                            let text = String(r.text.characters)
                            DispatchQueue.main.async { self?.analyzerResult(text: text, isFinal: r.isFinal) }
                        }
                    } catch {}
                    DispatchQueue.main.async { self?.analyzerStreamEnded() }
                }
                DispatchQueue.main.async { self?.armAnalyzer(gen: gen, analyzer: analyzer, cont: cont, results: results, fmt: fmt) }
            }
        }
    }

    /// Main-thread arm step: store the session, install the SAME VAD tap the
    /// legacy path uses, and start the audio engine.
    @available(iOS 26.0, *)
    private func armAnalyzer(gen: Int, analyzer: SpeechAnalyzer,
                             cont: AsyncStream<AnalyzerInput>.Continuation,
                             results: Task<Void, Never>, fmt: AVAudioFormat?) {
        guard gen == cycleGen, !finished else {
            cont.finish(); results.cancel()
            Task { await analyzer.cancelAndFinishNow() }
            return
        }
        anyAnalyzer = analyzer
        anyAnalyzerCont = cont
        anyResultsTask = results
        analyzerFormat = fmt

        let input = audioEngine.inputNode
        let format = input.outputFormat(forBus: 0)
        if let fmt = fmt, fmt != format {
            analyzerConverter = AVAudioConverter(from: format, to: fmt)
        } else {
            analyzerConverter = nil
        }
        let sampleRate = format.sampleRate
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            guard let self = self else { return }
            self.tapCount += 1
            self.yieldToAnalyzer(buffer)
            if self.continuous && sampleRate > 0 {
                let secs = Double(buffer.frameLength) / sampleRate
                let level = Self.rms(buffer)
                self.peakRMS = max(self.peakRMS, level)
                if level > self.speechRMS {
                    self.sawSpeech = true
                    self.silenceSecs = 0
                } else if self.sawSpeech {
                    self.silenceSecs += secs
                }
                self.segmentSecs += secs
                if !self.finalizing {
                    let boundary = (self.sawSpeech && self.silenceSecs >= self.silenceCutoff)
                        || self.segmentSecs >= self.maxSegment
                    if boundary {
                        self.sawSpeech = false
                        self.silenceSecs = 0
                        self.segmentSecs = 0
                        DispatchQueue.main.async { [weak self] in self?.segmentBoundary() }
                    }
                }
            }
        }
        audioEngine.prepare()
        do { try audioEngine.start() } catch { finish(.failure(.audio)); return }

        let handler = diagHandler
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak self] in
            guard let self = self else { return }
            handler?("engine=\(self.engineKind) sr=\(Int(format.sampleRate)) buf=\(self.tapCount) "
                + "peak=\(String(format: "%.3f", self.peakRMS))")
        }
    }

    /// A result arrived from the analyzer: volatile results stream as partials;
    /// finalized text accumulates into the current letter segment.
    private func analyzerResult(text: String, isFinal: Bool) {
        guard !finished else { return }
        if isFinal {
            analyzerSeg += text
            best = analyzerSeg
            partialHandler?(best)
        } else {
            partialHandler?(analyzerSeg + text)
        }
    }

    /// VAD boundary on the analyzer path: finalize the running session THROUGH
    /// NOW (the session and input stream stay live), then emit the finalized
    /// text as this letter's segment and reset for the next.
    private func analyzerBoundary() {
        guard continuous, !stopping, !finished, anyResultsTask != nil, !finalizing else { return }
        guard #available(iOS 26.0, *), let analyzer = anyAnalyzer as? SpeechAnalyzer else { return }
        finalizing = true
        let gen = cycleGen
        Task { [weak self] in
            try? await analyzer.finalize(through: nil)
            DispatchQueue.main.async {
                guard let self = self, gen == self.cycleGen, !self.finished else { return }
                self.finalizing = false
                let text = self.analyzerSeg
                self.analyzerSeg = ""
                self.best = ""
                if self.continuous && !self.stopping {
                    if !text.isEmpty {
                        self.segmentHandler?(text, 1.0, "")
                        self.diagHandler?("seg='\(text)' engine=\(self.engineKind)")
                    } else {
                        self.diagHandler?("seg=(empty) engine=\(self.engineKind)")
                    }
                } else {
                    self.finish(text.isEmpty ? .failure(.noSpeech) : .success(text))
                }
            }
        }
        // Safety net: a finalize that never returns must not stall the stream.
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            guard let self = self, gen == self.cycleGen, self.finalizing, !self.finished else { return }
            self.finalizing = false
        }
    }

    /// User stop on the analyzer path: close the input stream and finalize the
    /// whole session; the results stream then ends and finishes the capture.
    private func analyzerStop() {
        guard anyResultsTask != nil else { return }
        stopping = true
        if audioEngine.isRunning {
            audioEngine.stop()
            audioEngine.inputNode.removeTap(onBus: 0)
        }
        guard #available(iOS 26.0, *), let analyzer = anyAnalyzer as? SpeechAnalyzer else {
            finish(best.isEmpty ? .failure(.noSpeech) : .success(best)); return
        }
        (anyAnalyzerCont as? AsyncStream<AnalyzerInput>.Continuation)?.finish()
        let gen = cycleGen
        Task { [weak self] in
            try? await analyzer.finalizeAndFinishThroughEndOfInput()
            DispatchQueue.main.async {
                guard let self = self, gen == self.cycleGen, !self.finished else { return }
                let text = self.analyzerSeg
                self.finish(text.isEmpty ? .failure(.noSpeech) : .success(text))
            }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in
            guard let self = self, gen == self.cycleGen, !self.finished else { return }
            self.finish(self.analyzerSeg.isEmpty ? .failure(.noSpeech) : .success(self.analyzerSeg))
        }
    }

    // MARK: server STT engine (mic-everywhere, consented internet rung).

    private func beginServerCapture() {
        do {
            let session = AVAudioSession.sharedInstance()
            try? session.setActive(false, options: .notifyOthersOnDeactivation)
            try session.setCategory(.playAndRecord, mode: .measurement,
                                    options: [.duckOthers, .defaultToSpeaker, .allowBluetooth])
            try session.setActive(true, options: .notifyOthersOnDeactivation)
        } catch { finish(.failure(.audio)); return }

        tapCount = 0
        audioEngine.stop()
        audioEngine.reset()
        sawSpeech = false
        silenceSecs = 0
        segmentSecs = 0
        peakRMS = 0
        serverSeg = Data()

        let input = audioEngine.inputNode
        let format = input.outputFormat(forBus: 0)
        serverConverter = AVAudioConverter(from: format, to: Self.serverFormat)
        let sampleRate = format.sampleRate
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            guard let self = self else { return }
            self.tapCount += 1
            self.appendServerPCM(buffer)
            if self.continuous && sampleRate > 0 {
                let secs = Double(buffer.frameLength) / sampleRate
                let level = Self.rms(buffer)
                self.peakRMS = max(self.peakRMS, level)
                if level > self.speechRMS {
                    self.sawSpeech = true
                    self.silenceSecs = 0
                } else if self.sawSpeech {
                    self.silenceSecs += secs
                }
                self.segmentSecs += secs
                let boundary = (self.sawSpeech && self.silenceSecs >= self.silenceCutoff)
                    || self.segmentSecs >= self.maxSegment
                if boundary {
                    self.sawSpeech = false
                    self.silenceSecs = 0
                    self.segmentSecs = 0
                    DispatchQueue.main.async { [weak self] in self?.segmentBoundary() }
                }
            }
        }
        audioEngine.prepare()
        do { try audioEngine.start() } catch { finish(.failure(.audio)); return }

        let handler = diagHandler
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak self] in
            guard let self = self else { return }
            handler?("engine=server sr=\(Int(format.sampleRate)) buf=\(self.tapCount) "
                + "peak=\(String(format: "%.3f", self.peakRMS))")
        }
    }

    /// Convert one mic buffer to 16k mono Int16 and append to the segment.
    private func appendServerPCM(_ buffer: AVAudioPCMBuffer) {
        guard let conv = serverConverter else { return }
        let ratio = Self.serverFormat.sampleRate / buffer.format.sampleRate
        let cap = AVAudioFrameCount(Double(buffer.frameLength) * ratio) + 64
        guard let out = AVAudioPCMBuffer(pcmFormat: Self.serverFormat, frameCapacity: cap) else { return }
        var err: NSError?
        var fed = false
        conv.convert(to: out, error: &err) { _, status in
            if fed { status.pointee = .noDataNow; return nil }
            fed = true
            status.pointee = .haveData
            return buffer
        }
        guard err == nil, out.frameLength > 0, let ch = out.int16ChannelData?[0] else { return }
        pendingLock.lock()
        serverSeg.append(Data(bytes: ch, count: Int(out.frameLength) * 2))
        // Hard cap ~30s of audio (matches the backend's request cap).
        if serverSeg.count > 960_000 { serverSeg.removeFirst(serverSeg.count - 960_000) }
        pendingLock.unlock()
    }

    /// VAD boundary on the server rung: snapshot the segment PCM, reset, POST.
    /// Capture keeps running while the request is in flight.
    private func serverBoundary(isEnd: Bool) {
        guard engineKind == "server", !finished else { return }
        if !isEnd { guard continuous, !stopping else { return } }
        pendingLock.lock()
        let seg = serverSeg
        serverSeg = Data()
        pendingLock.unlock()
        // Skip near-empty segments (a false VAD boundary): < 0.25s of audio.
        if seg.count < 8_000 {
            if isEnd { finish(best.isEmpty ? .failure(.noSpeech) : .success(best)) }
            return
        }
        guard let url = URL(string: serverUrl) else {
            if isEnd { finish(.failure(.network)) }
            return
        }
        var req = URLRequest(url: url, timeoutInterval: 12)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let body: [String: Any] = [
            "lang": serverLang,
            "sampleRate": 16000,
            "audio": seg.base64EncodedString(),
            "phrases": Array(contextualStrings.prefix(100)),
        ]
        req.httpBody = try? JSONSerialization.data(withJSONObject: body)
        let gen = cycleGen
        URLSession.shared.dataTask(with: req) { [weak self] data, resp, error in
            DispatchQueue.main.async {
                guard let self = self, gen == self.cycleGen, !self.finished else { return }
                var transcript = ""
                var confidence = 1.0
                var alt = ""
                if error == nil, let data = data,
                   (resp as? HTTPURLResponse)?.statusCode == 200,
                   let j = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                    transcript = (j["transcript"] as? String) ?? ""
                    confidence = (j["confidence"] as? Double) ?? 1.0
                    alt = (j["alt"] as? String) ?? ""
                } else if !isEnd {
                    // Mid-stream network failure: end the session honestly —
                    // web shows the no-connection status and the player retaps.
                    // (Leaving the mic hot behind a dead UI is the worse bug.)
                    self.finish(.failure(.network))
                    return
                }
                if isEnd {
                    self.finish(transcript.isEmpty
                        ? (error == nil ? .failure(.noSpeech) : .failure(.network))
                        : .success(transcript))
                    return
                }
                if !transcript.isEmpty {
                    self.lastConfidence = confidence
                    self.lastAlt = alt
                    self.best = transcript
                    self.segmentHandler?(transcript, confidence, alt)
                    self.diagHandler?("seg='\(transcript)' engine=server")
                }
            }
        }.resume()
    }

    /// User stop on the server rung: flush the tail segment as the final.
    private func serverStop() {
        guard engineKind == "server", !finished else { return }
        stopping = true
        if audioEngine.isRunning {
            audioEngine.stop()
            audioEngine.inputNode.removeTap(onBus: 0)
        }
        serverBoundary(isEnd: true)
        // Net: if the final POST never answers, resolve with what we have.
        let gen = cycleGen
        DispatchQueue.main.asyncAfter(deadline: .now() + 6) { [weak self] in
            guard let self = self, gen == self.cycleGen, !self.finished else { return }
            self.finish(self.best.isEmpty ? .failure(.noSpeech) : .success(self.best))
        }
    }

    /// Tear down any live analyzer session (supersede / finish).
    private func teardownAnalyzer() {
        guard anyAnalyzer != nil || anyResultsTask != nil || anyAnalyzerCont != nil else { return }
        if #available(iOS 26.0, *) {
            (anyAnalyzerCont as? AsyncStream<AnalyzerInput>.Continuation)?.finish()
            (anyResultsTask as? Task<Void, Never>)?.cancel()
            if let analyzer = anyAnalyzer as? SpeechAnalyzer {
                Task { await analyzer.cancelAndFinishNow() }
            }
        }
        anyAnalyzer = nil
        anyAnalyzerCont = nil
        anyResultsTask = nil
        analyzerConverter = nil
        analyzerFormat = nil
        analyzerSeg = ""
        serverSeg = Data()
        serverConverter = nil
        serverUrl = ""
        engineKind = "legacy"
    }

    /// The analyzer results stream ended (session finished or cancelled).
    private func analyzerStreamEnded() {
        guard anyResultsTask != nil, !finished else { return }
        if stopping {
            let text = analyzerSeg
            finish(text.isEmpty ? .failure(.noSpeech) : .success(text))
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
        teardownAnalyzer()
        partialHandler = nil
        segmentHandler = nil
        continuous = false
        stopping = false
        pendingLock.lock()
        finalizing = false
        pendingBuffers.removeAll()
        pendingLock.unlock()
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
