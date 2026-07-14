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
            let lines = observations.compactMap { $0.topCandidates(1).first?.string }
            self.finish(resolve: lines)
        }
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = true
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
