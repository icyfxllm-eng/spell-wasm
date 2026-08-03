import Foundation
import Capacitor
import AVFoundation
import Speech

/// BD-4 CC-FAMILY-VOICES V1 — Apple Personal Voice as orb voice.
///
/// TRUST ORDER (fixed by the readme): trust > voice quality > feature scope.
/// I1: nothing here talks to a server, sends bytes anywhere, or fits a
/// model — and the gate's V2 symbol scan polices this file's vocabulary
/// itself. Personal Voice never leaves the device (Apple's guarantee is
/// our guarantee). The web core owns ALL policy (approval, the readout
/// gate verdict, language honesty); this extension only reports facts and
/// renders audio to a local temp file for the calibration check.
extension NativeLanguageKitPlugin {

    /// Authorization state + the device's Personal Voices (empty until
    /// authorized — iOS hides them from `speechVoices()` before consent).
    @objc func pvStatus(_ call: CAPPluginCall) {
        guard #available(iOS 17.0, *) else {
            call.resolve(["status": "unsupported", "voices": []])
            return
        }
        let status: String
        switch AVSpeechSynthesizer.personalVoiceAuthorizationStatus {
        case .authorized: status = "authorized"
        case .denied: status = "denied"
        case .unsupported: status = "unsupported"
        default: status = "notDetermined"
        }
        let voices = AVSpeechSynthesisVoice.speechVoices()
            .filter { $0.voiceTraits.contains(.isPersonalVoice) }
            .map { ["id": $0.identifier, "name": $0.name, "lang": $0.language] }
        call.resolve(["status": status, "voices": voices])
    }

    /// The system consent prompt. One honest state on denial — the web core
    /// never re-prompts (acceptance #2).
    @objc func pvRequestAuth(_ call: CAPPluginCall) {
        guard #available(iOS 17.0, *) else {
            call.resolve(["status": "unsupported"])
            return
        }
        AVSpeechSynthesizer.requestPersonalVoiceAuthorization { status in
            let s: String
            switch status {
            case .authorized: s = "authorized"
            case .denied: s = "denied"
            case .unsupported: s = "unsupported"
            default: s = "notDetermined"
            }
            DispatchQueue.main.async { call.resolve(["status": s]) }
        }
    }

    /// The readout-gate loopback (I2), mic-free and deterministic: render
    /// each calibration word with the candidate voice to a local file, then
    /// transcribe that file with the ON-DEVICE recognizer. Returns
    /// [{word, heard}]; the web core judges against its threshold. Files
    /// are deleted before resolving; nothing persists, nothing transmits.
    @objc func pvCalibrate(_ call: CAPPluginCall) {
        guard #available(iOS 17.0, *) else {
            call.reject("unsupported", "UNSUPPORTED")
            return
        }
        guard let voiceId = call.getString("voiceId"),
              let lang = call.getString("lang"),
              let words = call.getArray("words", String.self),
              let voice = AVSpeechSynthesisVoice(identifier: voiceId) else {
            call.reject("voiceId, lang and words are required", "BAD_ARGS")
            return
        }
        guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: lang)),
              recognizer.supportsOnDeviceRecognition else {
            call.reject("no on-device recognizer for \(lang)", "UNAVAILABLE")
            return
        }
        let synth = AVSpeechSynthesizer()
        var results: [[String: String]] = []
        let queue = DispatchQueue(label: "pv.calibrate")

        func step(_ idx: Int) {
            if idx >= words.count {
                call.resolve(["results": results])
                return
            }
            let word = words[idx]
            let url = FileManager.default.temporaryDirectory
                .appendingPathComponent("pvcal-\(idx).caf")
            let utterance = AVSpeechUtterance(string: word.precomposedStringWithCanonicalMapping)
            utterance.voice = voice
            var file: AVAudioFile?
            synth.write(utterance) { buffer in
                guard let pcm = buffer as? AVAudioPCMBuffer, pcm.frameLength > 0 else {
                    // Zero-length buffer = end of utterance: transcribe.
                    guard file != nil else {
                        results.append(["word": word, "heard": ""])
                        queue.async { step(idx + 1) }
                        return
                    }
                    let req = SFSpeechURLRecognitionRequest(url: url)
                    req.requiresOnDeviceRecognition = true
                    recognizer.recognitionTask(with: req) { result, err in
                        guard err == nil, let r = result else {
                            if err != nil {
                                results.append(["word": word, "heard": ""])
                                try? FileManager.default.removeItem(at: url)
                                queue.async { step(idx + 1) }
                            }
                            return
                        }
                        if r.isFinal {
                            results.append(["word": word, "heard": r.bestTranscription.formattedString])
                            try? FileManager.default.removeItem(at: url)
                            queue.async { step(idx + 1) }
                        }
                    }
                    return
                }
                do {
                    if file == nil {
                        file = try AVAudioFile(forWriting: url, settings: pcm.format.settings)
                    }
                    try file?.write(from: pcm)
                } catch {
                    // A failed write leaves heard="" — the web core counts it
                    // as a miss; the gate stays honest under IO trouble.
                }
            }
        }
        step(0)
    }
}
