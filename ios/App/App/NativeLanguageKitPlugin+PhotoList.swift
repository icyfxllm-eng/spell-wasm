import Foundation
import Capacitor
import UIKit
import PhotosUI
import Vision
import NativeLanguageKitCore

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
        // Phase 2 / registry-driven (consts::ocr_support in the core decides):
        // Native language -> correction ON; EnglishFallback -> the caller passes
        // the English recognizer + correction OFF so Vision can't "correct" a
        // Filipino/Swahili word into a lookalike English one.
        recognitionCorrection = call.getBool("correction") ?? true
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
        // Camera-first = the PLAIN camera. The VisionKit document scanner was
        // tried here (build 83) and REVERTED on device feedback: its capture
        // filter darkens pages in normal room light and its auto-capture hunts
        // focus ("super blurry", "pages are dark") with no API to tune either.
        // The plain camera + Vision recognition measured 0.9+ precision on the
        // same pages. Photo library (PHPicker) everywhere else (simulator).
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
        recognize(pages: [image])
    }

    /// Recognize one or more page images (the document scanner returns a page
    /// per scan) and aggregate every line IN PAGE ORDER, each with Vision's
    /// line confidence, before resolving once.
    fileprivate func recognize(pages: [UIImage]) {
        guard !pages.isEmpty else {
            finish(reject: "Could not read the photo")
            return
        }
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            var collected: [(VNRecognizedText, Float)] = []
            for image in pages {
                guard let cgImage = image.cgImage else { continue }
                let request = VNRecognizeTextRequest()
                request.recognitionLevel = .accurate
                request.usesLanguageCorrection = self.recognitionCorrection
                // Read any script the OS recognizer knows — but ONLY when
                // correction is on (a Native language). The EnglishFallback
                // profile wants the raw English model, no detour.
                if #available(iOS 16.0, *) {
                    request.automaticallyDetectsLanguage = self.recognitionCorrection
                }
                if !self.recognitionLanguages.isEmpty {
                    var langs = self.recognitionLanguages
                    // Chinese pages come in TWO scripts: seed both so a
                    // Traditional workbook is read as written (its text is
                    // normalized to Simplified after recognition, below).
                    if langs.contains(where: { ChineseScript.isChineseTag($0) }) {
                        for extra in ["zh-Hans", "zh-Hant"] where !langs.contains(extra) {
                            langs.append(extra)
                        }
                    }
                    request.recognitionLanguages = langs
                }
                let orientation = Self.cgOrientation(from: image.imageOrientation)
                let handler = VNImageRequestHandler(cgImage: cgImage, orientation: orientation, options: [:])
                do {
                    try handler.perform([request])
                } catch {
                    self.finish(reject: "Recognition failed")
                    return
                }
                let observations = (request.results as? [VNRecognizedTextObservation]) ?? []
                for obs in observations {
                    if let cand = obs.topCandidates(1).first {
                        collected.append((cand, cand.confidence))
                    }
                }
            }
            // Merge on main: UITextChecker (the dictionary tiebreaker inside
            // healSplitWords) isn't documented thread-safe.
            DispatchQueue.main.async {
                let lang = self.recognitionLanguages.first
                let chinese = lang.map { ChineseScript.isChineseTag($0) } ?? false
                let lines = collected.map { (cand, conf) -> RecognizedLine in
                    var text = Self.healSplitWords(cand, language: lang)
                    if chinese {
                        // Normalize script ON-DEVICE so a Traditional page
                        // practices against the app's Simplified banks.
                        text = ChineseScript.toSimplified(text)
                    }
                    // CC-SNAP-BOXES: geometry is measured against the RAW
                    // candidate, because healSplitWords and toSimplified both
                    // rewrite the string and the ranges would no longer point
                    // at the characters Vision measured. A line whose healed
                    // text differs from the raw text therefore ships no
                    // geometry -- the core's I-B5 would reject the mismatch
                    // anyway, and saying so here is cheaper than sending it.
                    let raw = cand.string
                    guard text == raw else { return RecognizedLine(text: text, confidence: conf) }
                    let geo = Self.geometry(for: cand, tokens: Self.tokenize(raw))
                    return RecognizedLine(text: text, confidence: conf,
                                          boxes: geo.boxes, glyph: geo.glyph)
                }
                self.finish(resolve: lines)
            }
        }
    }



    // MARK: - CC-SNAP-BOXES F4 — the gap thresholds, mirrored

    /// These four numbers MUST equal config/snap-geometry.json, which is the
    /// one source. Swift cannot read that file without adding it to the Xcode
    /// target, so the agreement is enforced instead:
    /// scripts/snap-geometry-check.mjs fails the build on any drift, and is
    /// wired into gate.sh and the pre-push hook.
    ///
    /// All four are multiples of the line's median glyph width. They were four
    /// unrelated literals in two languages until Phase B; the split threshold
    /// lives beside them now so the relationship is visible — the core splits
    /// at or above `splitGap`, and this file merges at or below `mergeTight`,
    /// which are NOT complements and never were.
    fileprivate static let splitGap: CGFloat = 0.35       // config: split_gap
    fileprivate static let mergeModerate: CGFloat = 1.1   // config: merge_moderate
    fileprivate static let mergeTight: CGFloat = 0.45     // config: merge_tight
    fileprivate static let mergeNoDict: CGFloat = 0.25    // config: merge_nodict

    // MARK: - CC-SNAP-BOXES — geometry for the core

    /// D-B7.
    fileprivate static let minProbeLen = 8
    fileprivate static let maxProbesPerLine = 120

    /// One token's geometry, in CC-SNAP-BOXES coordinates: normalized, origin
    /// TOP-left. Vision's origin is bottom-left; D-B2 flips it exactly once,
    /// here, so the core never has to remember a convention.
    struct TokenGeometry {
        let text: String
        let x0: CGFloat
        let x1: CGFloat
        /// (character index, gap width) for every probed interior boundary.
        var gaps: [(at: Int, w: CGFloat)]
    }

    /// F1 — per-token boxes plus the gaps measured INSIDE each token.
    ///
    /// The second half is the mechanism. Vision's token boxes are derived from
    /// the line string's own whitespace, so a word that arrived merged has one
    /// token and one box and no gap to find. But `boundingBox(for:)` accepts
    /// ANY range, including a single character, so the boundaries inside a
    /// token can be measured directly.
    ///
    /// Cost is bounded by D-B7: longest tokens first, at most
    /// `maxProbesPerLine` boundary queries. A line that exhausts the budget
    /// returns boxes with no interior gaps, which is exactly the state the app
    /// shipped in before this file -- degraded, never wrong.
    fileprivate static func geometry(for candidate: VNRecognizedText,
                                     tokens: [(text: String, range: Range<String.Index>)])
        -> (boxes: [TokenGeometry], glyph: CGFloat?) {
        var boxes: [TokenGeometry] = []
        var charWidths: [CGFloat] = []

        for t in tokens {
            guard let obs = (try? candidate.boundingBox(for: t.range)) ?? nil else {
                // One missing box makes the line's geometry untrustworthy as a
                // whole; the core would reject it anyway (I-B3), so say so by
                // returning nothing rather than a partial answer.
                return ([], nil)
            }
            let r = obs.boundingBox
            boxes.append(TokenGeometry(text: t.text, x0: r.minX, x1: r.maxX, gaps: []))
            let n = t.text.count
            if n > 0, r.width > 0 { charWidths.append(r.width / CGFloat(n)) }
        }

        // Probe budget: spend it on the tokens most likely to be merged words.
        var order = boxes.indices.filter { boxes[$0].text.count >= minProbeLen }
        order.sort { boxes[$0].text.count > boxes[$1].text.count }
        var budget = maxProbesPerLine

        for i in order {
            let token = tokens[i]
            let chars = Array(token.text.indices)
            guard chars.count > 1 else { continue }
            var perChar: [CGFloat] = []
            var rects: [CGRect] = []
            var ok = true
            for c in chars {
                if budget <= 0 { ok = false; break }
                budget -= 1
                let abs = token.range.lowerBound
                let lo = token.text.distance(from: token.text.startIndex, to: c)
                guard let s0 = candidate.string.index(abs, offsetBy: lo, limitedBy: candidate.string.endIndex),
                      let s1 = candidate.string.index(s0, offsetBy: 1, limitedBy: candidate.string.endIndex),
                      let obs = (try? candidate.boundingBox(for: s0..<s1)) ?? nil else { ok = false; break }
                rects.append(obs.boundingBox)
                if obs.boundingBox.width > 0 { perChar.append(obs.boundingBox.width) }
            }
            guard ok, rects.count == chars.count else { continue }
            var gaps: [(at: Int, w: CGFloat)] = []
            for k in 1..<rects.count {
                let w = rects[k].minX - rects[k - 1].maxX
                if w > 0 { gaps.append((at: k, w: w)) }
            }
            boxes[i].gaps = gaps
            charWidths.append(contentsOf: perChar)
        }

        // D-B3 — the median glyph width, measured where the per-character
        // boxes are rather than estimated from text length in the core.
        var glyph: CGFloat?
        if !charWidths.isEmpty {
            charWidths.sort()
            glyph = charWidths[charWidths.count / 2]
        }
        return (boxes, glyph)
    }

    /// The whitespace tokens of a candidate, with their ranges. One definition,
    /// shared by healSplitWords and by geometry(for:).
    fileprivate static func tokenize(_ text: String)
        -> [(text: String, range: Range<String.Index>)] {
        var tokens: [(text: String, range: Range<String.Index>)] = []
        var idx = text.startIndex
        while idx < text.endIndex {
            if text[idx].isWhitespace { idx = text.index(after: idx); continue }
            var end = idx
            while end < text.endIndex, !text[end].isWhitespace { end = text.index(after: end) }
            tokens.append((String(text[idx..<end]), idx..<end))
            idx = end
        }
        return tokens
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
        // One tokenizer, shared with geometry(for:) — I-B5 compares the boxes
        // against the line text, so two tokenizers would eventually disagree
        // and silently cost the page its geometry.
        let tokens = tokenize(text)
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
        //
        // OVER-MERGE GUARD (device report: five words written close together
        // imported as ONE entry): a tight gap alone no longer merges when BOTH
        // sides are real dictionary words — "cat dog" stays two words no matter
        // how cramped the handwriting, while "soft ware" (join is a word) and
        // "sof tware" (a side is a non-word fragment) still heal. For languages
        // the OS spell-checker can't judge, geometry stands alone but with a
        // much stricter gap so cramped neighbours don't fuse.
        let hasDict = hasDictionary(language)
        var outTokens: [String] = [tokens[0].text]
        var prevBox = boxes[0]
        for i in 1..<tokens.count {
            let gap = boxes[i].minX - prevBox.maxX
            let moderate = gap <= Self.mergeModerate * avgChar
            let left = outTokens[outTokens.count - 1]
            let right = tokens[i].text
            let joined = left + right
            let merge: Bool
            if hasDict {
                let tight = gap <= Self.mergeTight * avgChar
                let bothReal = isDictionaryWord(left, language: language)
                    && isDictionaryWord(right, language: language)
                merge = (moderate && isDictionaryWord(joined, language: language))
                    || (tight && !bothReal)
            } else {
                merge = gap <= Self.mergeNoDict * avgChar
            }
            if merge {
                outTokens[outTokens.count - 1] = joined
                prevBox = prevBox.union(boxes[i])
            } else {
                outTokens.append(right)
                prevBox = boxes[i]
            }
        }
        return outTokens.joined(separator: " ")
    }

    /// Does UITextChecker have a dictionary for this language at all? Decides
    /// whether dictionary evidence can veto geometry merges.
    fileprivate static func hasDictionary(_ language: String?) -> Bool {
        guard let language = language, !language.isEmpty else { return false }
        let primary = language.split(separator: "-").first.map(String.init)?.lowercased() ?? language
        return UITextChecker.availableLanguages.contains { $0.lowercased().hasPrefix(primary) }
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

    /// One recognized line on its way to the core. Geometry is optional and
    /// absent by default, so every path that cannot measure it stays correct.
    struct RecognizedLine {
        let text: String
        let confidence: Float
        var boxes: [TokenGeometry] = []
        var glyph: CGFloat?
    }


    fileprivate func finish(resolve lines: [RecognizedLine]) {
        DispatchQueue.main.async { [weak self] in
            guard let self = self, let call = self.pendingCall else { return }
            // Per-line confidence rides along (Phase 2) — the core decides what
            // counts as "low", the plugin just reports Vision's number.
            //
            // CC-SNAP-BOXES F2: boxes, gaps and glyph are added only when the
            // line has them. Their absence is the shape every build before
            // this one sent, and the core must behave identically (I-B1).
            let payload: [[String: Any]] = lines.map { l in
                var d: [String: Any] = ["text": l.text, "confidence": l.confidence]
                guard !l.boxes.isEmpty else { return d }
                d["boxes"] = l.boxes.map { ["text": $0.text, "x0": $0.x0, "x1": $0.x1] }
                var gaps: [[String: Any]] = []
                for (i, b) in l.boxes.enumerated() {
                    for g in b.gaps { gaps.append(["box": i, "at": g.at, "w": g.w]) }
                }
                if !gaps.isEmpty { d["gaps"] = gaps }
                if let glyph = l.glyph { d["glyph"] = glyph }
                return d
            }
            call.resolve(["supported": true, "lines": payload])
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
