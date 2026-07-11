// DigitalInkPlugin — Capacitor bridge to ML Kit Digital Ink Recognition (iOS).
//
// STAGED, NOT YET IN THE BUILD. To activate: add this file + DigitalInkPlugin.m
// to the App target, resolve the ML Kit dependency, and flip
// consts::DRAW_MLKIT_READY. See ../README.md for the exact steps.
//
// Signatures verified against the current ML Kit iOS Digital Ink docs
// (developers.google.com/ml-kit/vision/digital-ink-recognition/ios). Per the
// VERIFY-FIRST rule, re-confirm them against the docs at integration time — ML
// Kit APIs shift between releases.
//
// NOTE: iOS ML Kit candidates expose `.text` but NO numeric score (unlike
// Android). We synthesize a rank-derived score (1.0 for rank 0, descending) so
// the core scorer's top-N logic is uniform across platforms.

import Foundation
import Capacitor
import MLKitDigitalInkRecognition

@objc(DigitalInkPlugin)
public class DigitalInkPlugin: CAPPlugin {

    private let workQueue = DispatchQueue(label: "net.spellgame.digitalink", qos: .userInitiated)

    // MARK: - Model management

    private func model(forTag tag: String) -> DigitalInkRecognitionModel? {
        // Full BCP-47 tags only: "zh-Hani-CN", "ja-JP", "ko-KR", "th-TH".
        guard let identifier = DigitalInkRecognitionModelIdentifier(forLanguageTag: tag) else {
            return nil
        }
        return DigitalInkRecognitionModel(modelIdentifier: identifier)
    }

    @objc func downloadModel(_ call: CAPPluginCall) {
        guard let tag = call.getString("languageTag"), let model = model(forTag: tag) else {
            call.reject("unknown or unsupported languageTag", "BAD_TAG")
            return
        }
        let manager = ModelManager.modelManager()
        if manager.isModelDownloaded(model) {
            call.resolve(["status": "already"])
            return
        }
        // Tens of MB; allow cellular per the user's network (Capacitor call is
        // user-initiated on first language selection).
        let conditions = ModelDownloadConditions(allowsCellularAccess: true, allowsBackgroundDownloading: true)
        let progress = manager.download(model, conditions: conditions)
        // Poll the Progress to completion off the main thread; resolve once.
        workQueue.async {
            while !progress.isFinished && !progress.isCancelled {
                Thread.sleep(forTimeInterval: 0.2)
            }
            if progress.isCancelled {
                call.reject("model download cancelled", "DOWNLOAD_CANCELLED")
            } else if manager.isModelDownloaded(model) {
                call.resolve(["status": "downloaded"])
            } else {
                call.reject("model download failed", "DOWNLOAD_FAILED")
            }
        }
    }

    @objc func isModelDownloaded(_ call: CAPPluginCall) {
        guard let tag = call.getString("languageTag"), let model = model(forTag: tag) else {
            call.reject("unknown or unsupported languageTag", "BAD_TAG")
            return
        }
        call.resolve(["downloaded": ModelManager.modelManager().isModelDownloaded(model)])
    }

    @objc func deleteModel(_ call: CAPPluginCall) {
        guard let tag = call.getString("languageTag"), let model = model(forTag: tag) else {
            call.reject("unknown or unsupported languageTag", "BAD_TAG")
            return
        }
        ModelManager.modelManager().deleteDownloadedModel(model) { error in
            if let error = error {
                call.reject("delete failed: \(error.localizedDescription)", "DELETE_FAILED")
            } else {
                call.resolve()
            }
        }
    }

    // MARK: - Recognition

    @objc func recognize(_ call: CAPPluginCall) {
        guard let tag = call.getString("languageTag"), let model = model(forTag: tag) else {
            call.reject("unknown or unsupported languageTag", "BAD_TAG")
            return
        }
        let manager = ModelManager.modelManager()
        guard manager.isModelDownloaded(model) else {
            call.reject("model not downloaded", "MODEL_UNAVAILABLE")
            return
        }
        // strokes: [{ points: [{ x, y, t }] }] — CSS-pixel space, t in ms.
        guard let strokesRaw = call.getArray("strokes") as? [[String: Any]] else {
            call.reject("missing strokes", "BAD_INPUT")
            return
        }
        let width = call.getInt("writingArea.width") ?? (call.getObject("writingArea")?["width"] as? Int ?? 0)
        let height = call.getInt("writingArea.height") ?? (call.getObject("writingArea")?["height"] as? Int ?? 0)
        let preContext = call.getString("preContext") ?? ""

        var strokes: [Stroke] = []
        for strokeDict in strokesRaw {
            guard let pts = strokeDict["points"] as? [[String: Any]] else { continue }
            var points: [StrokePoint] = []
            for p in pts {
                let x = Float((p["x"] as? Double) ?? (p["x"] as? Int).map(Double.init) ?? 0)
                let y = Float((p["y"] as? Double) ?? (p["y"] as? Int).map(Double.init) ?? 0)
                let t = Int((p["t"] as? Int) ?? (p["t"] as? Double).map(Int.init) ?? 0)
                points.append(StrokePoint(x: x, y: y, t: t))
            }
            if !points.isEmpty { strokes.append(Stroke(points: points)) }
        }
        guard !strokes.isEmpty else {
            call.reject("no strokes", "BAD_INPUT")
            return
        }
        let ink = Ink(strokes: strokes)

        let options = DigitalInkRecognizerOptions(model: model)
        let recognizer = DigitalInkRecognizer.digitalInkRecognizer(options: options)
        let context = DigitalInkRecognitionContext(
            preContext: preContext,
            writingArea: WritingArea(width: width, height: height)
        )

        workQueue.async {
            recognizer.recognizeHandwriting(from: ink, context: context) { result, error in
                if let error = error {
                    call.reject("recognize failed: \(error.localizedDescription)", "RECOGNIZE_FAILED")
                    return
                }
                let cands = result?.candidates ?? []
                // iOS gives no score → synthesize a descending rank score.
                let n = max(cands.count, 1)
                let mapped: [[String: Any]] = cands.enumerated().map { (i, c) in
                    ["text": c.text, "score": Double(n - i) / Double(n)]
                }
                call.resolve(["candidates": mapped])
            }
        }
    }
}
