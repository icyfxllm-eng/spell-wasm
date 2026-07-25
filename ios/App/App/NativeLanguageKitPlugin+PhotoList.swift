import Foundation
import Capacitor
import UIKit
import PhotosUI
import Vision

// Feature F1 "Photo-to-word-list" — on-device VisionKit OCR, kept as an
// extension so the core plugin file stays focused on the language capabilities.
// The photographed schoolwork is handed straight from the OS picker to Vision
// and back out as text lines — never written beyond the picker, never uploaded,
// no network. The two bits of async state (pendingCall, recognitionLanguages)
// live on the main NativeLanguageKitPlugin class (extensions can't hold stored
// properties).
extension NativeLanguageKitPlugin {
    @objc func recognizeWordList(_ call: CAPPluginCall) {
        if pendingCall != nil {
            call.reject("A recognition is already in progress")
            return
        }
        let lang = call.getString("lang") ?? "en-US"
        recognitionLanguages = [lang]
        let source = call.getString("source") ?? "auto"
        pendingCall = call
        DispatchQueue.main.async { [weak self] in
            self?.presentPicker(source: source)
        }
    }

    // MARK: - Capture

    fileprivate func presentPicker(source: String) {
        guard let vc = bridge?.viewController else {
            finish(reject: "No view controller to present from")
            return
        }
        // Camera only when explicitly asked and actually available (never on the
        // simulator); everything else uses the photo library (PHPicker).
        if source == "camera", UIImagePickerController.isSourceTypeAvailable(.camera) {
            let picker = UIImagePickerController()
            picker.sourceType = .camera
            picker.delegate = self
            vc.present(picker, animated: true)
        } else {
            var config = PHPickerConfiguration()
            config.filter = .images
            config.selectionLimit = 1
            let picker = PHPickerViewController(configuration: config)
            picker.delegate = self
            vc.present(picker, animated: true)
        }
    }

    // MARK: - Recognition (on-device Vision; no network)

    fileprivate func recognize(_ image: UIImage) {
        guard let cgImage = image.cgImage else {
            finish(reject: "Could not read the photo")
            return
        }
        let request = VNRecognizeTextRequest { [weak self] request, error in
            guard let self = self else { return }
            if let error = error {
                self.finish(reject: "Recognition failed: \(error.localizedDescription)")
                return
            }
            let observations = (request.results as? [VNRecognizedTextObservation]) ?? []
            let candidates = observations.compactMap { $0.topCandidates(1).first }
            // Merge on main: UITextChecker (used by the dictionary tiebreaker)
            // isn't documented thread-safe.
            DispatchQueue.main.async {
                let lang = self.recognitionLanguages.first
                let lines = candidates.map { Self.healSplitWords($0, language: lang) }
                self.finish(resolve: lines)
            }
        }
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = true
        // Read any script the OS recognizer knows, not just the seeded language —
        // the seeded recognitionLanguages below remain the tie-break hint.
        if #available(iOS 16.0, *) {
            request.automaticallyDetectsLanguage = true
        }
        if !recognitionLanguages.isEmpty {
            request.recognitionLanguages = recognitionLanguages
        }
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let orientation = Self.cgOrientation(from: image.imageOrientation)
            let handler = VNImageRequestHandler(cgImage: cgImage, orientation: orientation, options: [:])
            do {
                try handler.perform([request])
            } catch {
                self?.finish(reject: "Recognition failed")
            }
        }
    }

    // MARK: - Split-word healing

    /// Handwriting often leaves a small gap INSIDE a word, which the recognizer
    /// renders as a space — "software" comes back as "soft ware". Vision still
    /// knows the page geometry, so heal it here: merge two adjacent tokens when
    /// the pixel gap between them is far smaller than a real word space (measured
    /// against this line's own average character width), or when the gap is
    /// moderate but the joined text is a dictionary word (UITextChecker). Works
    /// in any language Vision reads; geometry needs no dictionary at all.
    fileprivate static func healSplitWords(_ candidate: VNRecognizedText, language: String?) -> String {
        let text = candidate.string
        // Whitespace-separated tokens with their ranges in the line.
        var tokens: [(text: String, range: Range<String.Index>)] = []
        var idx = text.startIndex
        while idx < text.endIndex {
            if text[idx].isWhitespace { idx = text.index(after: idx); continue }
            var end = idx
            while end < text.endIndex, !text[end].isWhitespace { end = text.index(after: end) }
            tokens.append((String(text[idx..<end]), idx..<end))
            idx = end
        }
        guard tokens.count > 1 else { return text }

        // Normalized bounding box per token; if geometry is unavailable for any
        // token, return the line untouched rather than guessing.
        var boxes: [CGRect] = []
        for t in tokens {
            guard let obs = (try? candidate.boundingBox(for: t.range)) ?? nil else { return text }
            boxes.append(obs.boundingBox)
        }
        let totalChars = tokens.reduce(0) { $0 + $1.text.count }
        let totalWidth = boxes.reduce(CGFloat(0)) { $0 + $1.width }
        guard totalChars > 0, totalWidth > 0 else { return text }
        let avgChar = totalWidth / CGFloat(totalChars)

        // Left-to-right greedy merge so "so ft ware" can chain into one word.
        var outTokens: [String] = [tokens[0].text]
        var prevBox = boxes[0]
        for i in 1..<tokens.count {
            let gap = boxes[i].minX - prevBox.maxX
            let tight = gap <= 0.45 * avgChar
            let moderate = gap <= 1.1 * avgChar
            let joined = outTokens[outTokens.count - 1] + tokens[i].text
            if tight || (moderate && isDictionaryWord(joined, language: language)) {
                outTokens[outTokens.count - 1] = joined
                prevBox = prevBox.union(boxes[i])
            } else {
                outTokens.append(tokens[i].text)
                prevBox = boxes[i]
            }
        }
        return outTokens.joined(separator: " ")
    }

    /// True when the OS spell-checker accepts `w` for the recognition language —
    /// the tiebreaker that turns a moderate handwriting gap ("soft ware") back
    /// into the word the writer meant.
    fileprivate static func isDictionaryWord(_ w: String, language: String?) -> Bool {
        guard w.count > 2, let language = language, !language.isEmpty else { return false }
        let checker = UITextChecker()
        let lang = language.replacingOccurrences(of: "-", with: "_")
        let range = NSRange(location: 0, length: (w as NSString).length)
        let miss = checker.rangeOfMisspelledWord(in: w, range: range, startingAt: 0,
                                                 wrap: false, language: lang)
        return miss.location == NSNotFound
    }

    // MARK: - Completion

    fileprivate func finish(resolve lines: [String]) {
        DispatchQueue.main.async { [weak self] in
            guard let self = self, let call = self.pendingCall else { return }
            call.resolve(["supported": true, "lines": lines])
            self.pendingCall = nil
        }
    }

    fileprivate func finish(reject message: String) {
        DispatchQueue.main.async { [weak self] in
            guard let self = self, let call = self.pendingCall else { return }
            call.reject(message)
            self.pendingCall = nil
        }
    }

    fileprivate static func cgOrientation(from orientation: UIImage.Orientation) -> CGImagePropertyOrientation {
        switch orientation {
        case .up: return .up
        case .upMirrored: return .upMirrored
        case .down: return .down
        case .downMirrored: return .downMirrored
        case .left: return .left
        case .leftMirrored: return .leftMirrored
        case .right: return .right
        case .rightMirrored: return .rightMirrored
        @unknown default: return .up
        }
    }
}

// MARK: - PHPickerViewControllerDelegate

extension NativeLanguageKitPlugin: PHPickerViewControllerDelegate {
    public func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
        picker.dismiss(animated: true)
        guard let provider = results.first?.itemProvider,
              provider.canLoadObject(ofClass: UIImage.self) else {
            finish(reject: "cancelled")
            return
        }
        provider.loadObject(ofClass: UIImage.self) { [weak self] object, _ in
            if let image = object as? UIImage {
                self?.recognize(image)
            } else {
                self?.finish(reject: "Could not load the photo")
            }
        }
    }
}

// MARK: - UIImagePickerControllerDelegate (camera)

extension NativeLanguageKitPlugin: UIImagePickerControllerDelegate, UINavigationControllerDelegate {
    public func imagePickerController(_ picker: UIImagePickerController,
                                      didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey: Any]) {
        picker.dismiss(animated: true)
        if let image = info[.originalImage] as? UIImage {
            recognize(image)
        } else {
            finish(reject: "Could not read the photo")
        }
    }

    public func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
        picker.dismiss(animated: true)
        finish(reject: "cancelled")
    }
}
