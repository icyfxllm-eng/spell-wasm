import Foundation
import Capacitor
import Photos
import UIKit

// CC-FINALE feature 3 — "Save to Photos". The finished picture lands in the
// camera roll like any art the player made.
//
// ADD-ONLY authorization (.addOnly), never .readWrite. The app has no reason
// to read the library and asking for it would be a much larger permission
// prompt for a much smaller purpose; the spec is explicit about this.
//
// The PNG arrives as base64 from the Rust export path, already rendered at
// export resolution. Nothing here re-encodes or re-scales it, so what lands
// in Photos is byte-for-byte what the export renderer produced.
extension NativeLanguageKitPlugin {
    @objc func savePicture(_ call: CAPPluginCall) {
        guard let b64 = call.getString("data"),
              let bytes = Data(base64Encoded: b64),
              let image = UIImage(data: bytes) else {
            call.reject("bad-image")
            return
        }

        // .addOnly is the whole point: we contribute one asset and can never
        // enumerate what else is there.
        let status = PHPhotoLibrary.authorizationStatus(for: .addOnly)
        switch status {
        case .authorized, .limited:
            Self.write(image, call)
        case .notDetermined:
            PHPhotoLibrary.requestAuthorization(for: .addOnly) { granted in
                if granted == .authorized || granted == .limited {
                    Self.write(image, call)
                } else {
                    // A refusal, reported honestly. The caller shows an audited
                    // string and keeps Share working -- sharing needs no
                    // Photos access at all.
                    call.reject("denied")
                }
            }
        default:
            call.reject("denied")
        }
    }

    private static func write(_ image: UIImage, _ call: CAPPluginCall) {
        PHPhotoLibrary.shared().performChanges {
            PHAssetChangeRequest.creationRequestForAsset(from: image)
        } completionHandler: { ok, err in
            if ok {
                call.resolve(["saved": true])
            } else {
                call.reject(err?.localizedDescription ?? "save-failed")
            }
        }
    }
}
