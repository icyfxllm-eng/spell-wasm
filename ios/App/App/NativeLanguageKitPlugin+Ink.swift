import Capacitor
import Foundation
import Vision

// CC-CJK-INK F3 — recognize a hand-drawn CJK character, entirely on device.
//
// This is the SAME recognizer the photo-to-word-list feature already uses, with
// a different profile. The step-0 probe measured it on isolated characters
// before any of this was built: 10/10 and 9/10 on Chinese printed and degraded,
// 9/10 and 8/10 on Japanese. That is what made Vision the first path rather
// than a stroke-matching recognizer (D2).
//
// Invariant 1: ink never leaves the device. The PNG arrives as a data URI from
// the drawing pad, is decoded here, recognized here, and dropped. It is never
// written to disk, never cached, and no network call exists in this file.
//
// Invariant 3: one recognizer path. If a second appears, that is a build
// failure rather than a variation.
extension NativeLanguageKitPlugin {
    @objc func recognizeInk(_ call: CAPPluginCall) {
        guard let dataUri = call.getString("png"), let comma = dataUri.firstIndex(of: ",") else {
            call.reject("recognizeInk needs a png data URI")
            return
        }
        let b64 = String(dataUri[dataUri.index(after: comma)...])
        guard let data = Data(base64Encoded: b64),
              let provider = CGDataProvider(data: data as CFData),
              let cg = CGImage(pngDataProviderSource: provider, decode: nil,
                               shouldInterpolate: true, intent: .defaultIntent)
        else {
            call.reject("could not decode the ink image")
            return
        }
        // The study language decides the recognizer. Chinese seeds BOTH scripts
        // for the same reason the photo path does: a learner may write a
        // traditional form of a simplified character and it is still the
        // character they were asked for.
        let lang = call.getString("lang") ?? "zh"
        let langs: [String] = lang.hasPrefix("ja") ? ["ja"] : ["zh-Hans", "zh-Hant"]

        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        // Correction OFF, always. Language correction exists to turn text into
        // plausible WORDS, and a single character under test is exactly where
        // that would "helpfully" replace a learner's honest mistake with
        // something the model preferred.
        request.usesLanguageCorrection = false
        request.recognitionLanguages = langs
        // A lone glyph is not a text line. Without this Vision often finds no
        // region at all -- the probe needed it too.
        request.minimumTextHeight = 0.05

        DispatchQueue.global(qos: .userInitiated).async {
            let handler = VNImageRequestHandler(cgImage: cg, options: [:])
            do {
                try handler.perform([request])
            } catch {
                DispatchQueue.main.async { call.reject("recognition failed") }
                return
            }
            var out: [[String: Any]] = []
            for obs in (request.results ?? []) {
                for cand in obs.topCandidates(3) {
                    let t = cand.string.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !t.isEmpty {
                        out.append(["text": t, "confidence": cand.confidence])
                    }
                }
            }
            // Candidates are RETURNED, not chosen. A recognizer that silently
            // picks turns a wrong answer into a mystery, and the probe showed
            // correct-but-low-confidence reads often enough (0.30 on several
            // hits) that hiding the alternatives would read as a bug.
            DispatchQueue.main.async { call.resolve(["candidates": out]) }
        }
    }
}
