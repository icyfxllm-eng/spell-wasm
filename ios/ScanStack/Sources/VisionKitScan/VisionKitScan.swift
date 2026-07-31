//  CC-SCAN-STACK module 1 — VisionKit: structure extraction.
//
//  Knows WHAT is in the photo and WHERE, per subject. Wraps Apple Vision:
//  instance masks (who), face landmarks (which features), saliency (what
//  matters). Output speaks the tracer's v7.4 language — containment-tree
//  parts and protected micro features — so the existing layout law
//  consumes it unchanged.
//
//  D2 (approved): instance masks require iOS 17 / macOS 14. The FEATURE
//  carries that floor; the app minimum does not move. Callers ask
//  `isAvailable` and get a clean refusal, never a crash.
//  D4 (approved): >= 8 landmark paths per face; a face whose shortest
//  side is under 60px is REFUSED with the audited reason, never rendered
//  as a smudge.
//  D5: Vision output can shift across OS versions, so masks are pinned by
//  IoU >= 0.98 against goldens, never by byte identity.

import CoreGraphics
import Foundation
import Vision

public struct FaceLandmarkPaths {
    /// Normalised (0...1, origin bottom-left as Vision reports) polylines.
    public let regions: [String: [CGPoint]]
    public let boundingBox: CGRect
    /// Shortest side in PIXELS of the source image.
    public let shortestSidePx: CGFloat

    /// D4: the protected set. These render even when they cannot host a
    /// word — the snowman-eyes rule, applied to faces.
    public static let protectedRegions = ["leftEye", "rightEye", "outerLips"]

    public var pathCount: Int { regions.count }
}

public struct InstanceMask {
    public let index: Int
    /// Row-major coverage, 0...1, at the analysed resolution.
    public let alpha: [Float]
    public let width: Int
    public let height: Int
}

public enum ScanVisionError: Error, Equatable {
    /// D2: the OS cannot do instance masks.
    case unsupportedOS
    /// D4: face too small to carry its protected features.
    case faceTooSmall(shortestSidePx: Int, minimum: Int)
    /// D4: landmarks resolved but below the path floor.
    case insufficientLandmarkPaths(found: Int, minimum: Int)
    case visionFailed(String)
}

public enum ScanVision {
    /// D4 constants, Eric-approved.
    public static let minLandmarkPaths = 8
    public static let minFaceShortestSidePx = 60

    /// D2 gate. Structure extraction is a FEATURE gate, not an app floor.
    public static var isAvailable: Bool {
        if #available(iOS 17.0, macOS 14.0, *) { return true }
        return false
    }

    /// Per-subject instance masks (who is in the photo).
    @available(iOS 17.0, macOS 14.0, *)
    public static func instanceMasks(_ image: CGImage) throws -> [InstanceMask] {
        let request = VNGenerateForegroundInstanceMaskRequest()
        let handler = VNImageRequestHandler(cgImage: image, options: [:])
        do {
            try handler.perform([request])
        } catch {
            throw ScanVisionError.visionFailed("\(error)")
        }
        guard let observation = request.results?.first else { return [] }
        var out: [InstanceMask] = []
        for (i, instance) in observation.allInstances.enumerated() {
            guard let buffer = try? observation.generateScaledMaskForImage(
                forInstances: IndexSet(integer: instance), from: handler) else { continue }
            out.append(Self.mask(from: buffer, index: i))
        }
        return out
    }

    /// Face landmarks -> the paths that become protected micro features.
    /// Refuses small faces per D4 rather than degrading them.
    public static func faceLandmarks(_ image: CGImage) throws -> [FaceLandmarkPaths] {
        let request = VNDetectFaceLandmarksRequest()
        let handler = VNImageRequestHandler(cgImage: image, options: [:])
        do {
            try handler.perform([request])
        } catch {
            throw ScanVisionError.visionFailed("\(error)")
        }
        let w = CGFloat(image.width), h = CGFloat(image.height)
        var faces: [FaceLandmarkPaths] = []
        for obs in request.results ?? [] {
            let box = obs.boundingBox
            let shortest = min(box.width * w, box.height * h)
            if shortest < CGFloat(minFaceShortestSidePx) {
                throw ScanVisionError.faceTooSmall(shortestSidePx: Int(shortest),
                                                   minimum: minFaceShortestSidePx)
            }
            var regions: [String: [CGPoint]] = [:]
            if let lm = obs.landmarks {
                let named: [(String, VNFaceLandmarkRegion2D?)] = [
                    ("faceContour", lm.faceContour), ("leftEye", lm.leftEye),
                    ("rightEye", lm.rightEye), ("leftEyebrow", lm.leftEyebrow),
                    ("rightEyebrow", lm.rightEyebrow), ("nose", lm.nose),
                    ("noseCrest", lm.noseCrest), ("outerLips", lm.outerLips),
                    ("innerLips", lm.innerLips), ("medianLine", lm.medianLine),
                ]
                for (name, region) in named {
                    guard let region, region.pointCount > 1 else { continue }
                    regions[name] = region.normalizedPoints.map {
                        CGPoint(x: box.origin.x + CGFloat($0.x) * box.width,
                                y: box.origin.y + CGFloat($0.y) * box.height)
                    }
                }
            }
            if regions.count < minLandmarkPaths {
                throw ScanVisionError.insufficientLandmarkPaths(found: regions.count,
                                                                minimum: minLandmarkPaths)
            }
            faces.append(FaceLandmarkPaths(regions: regions, boundingBox: box,
                                           shortestSidePx: shortest))
        }
        return faces
    }

    /// D5 pinning: masks compare by overlap, not bytes, because Vision
    /// models change between OS versions.
    public static func iou(_ a: [Float], _ b: [Float], threshold: Float = 0.5) -> Float {
        precondition(a.count == b.count)
        var inter = 0, union = 0
        for i in 0..<a.count {
            let x = a[i] >= threshold, y = b[i] >= threshold
            if x && y { inter += 1 }
            if x || y { union += 1 }
        }
        return union == 0 ? 1 : Float(inter) / Float(union)
    }

    private static func mask(from buffer: CVPixelBuffer, index: Int) -> InstanceMask {
        CVPixelBufferLockBaseAddress(buffer, .readOnly)
        defer { CVPixelBufferUnlockBaseAddress(buffer, .readOnly) }
        let w = CVPixelBufferGetWidth(buffer), h = CVPixelBufferGetHeight(buffer)
        var alpha = [Float](repeating: 0, count: w * h)
        if let base = CVPixelBufferGetBaseAddress(buffer) {
            let stride = CVPixelBufferGetBytesPerRow(buffer)
            let fmt = CVPixelBufferGetPixelFormatType(buffer)
            for y in 0..<h {
                let row = base.advanced(by: y * stride)
                for x in 0..<w {
                    if fmt == kCVPixelFormatType_OneComponent32Float {
                        alpha[y * w + x] = row.load(fromByteOffset: x * 4, as: Float.self)
                    } else {
                        alpha[y * w + x] = Float(row.load(fromByteOffset: x, as: UInt8.self)) / 255.0
                    }
                }
            }
        }
        return InstanceMask(index: index, alpha: alpha, width: w, height: h)
    }
}

// MARK: - Module 4: per-face detail guarantees
//
//  "The kid in the back row must not render as a smudge." Every detected
//  face gets a minimum landmark-driven path budget regardless of pixel
//  size, protected eye and mouth paths, and its own palette region so
//  each person keeps their own hair and skin tones.

public struct FaceBudget {
    public let faceIndex: Int
    /// Paths this face is guaranteed, regardless of how small it is.
    public let guaranteedPaths: Int
    /// Regions that render even when they cannot host a word (D4).
    public let protectedRegions: [String]
    /// Pixel rect in the source image — the face's own palette region.
    public let paletteRegionPx: CGRect
}

public enum FaceDetail {
    /// Allocate per-face budgets. Faces are sorted largest-first so the
    /// nearest subject is planned before the back row, but EVERY face
    /// clears the same floor — that is the point of the rule.
    public static func budgets(for faces: [FaceLandmarkPaths],
                               imageWidth: Int, imageHeight: Int) throws -> [FaceBudget] {
        let w = CGFloat(imageWidth), h = CGFloat(imageHeight)
        var out: [FaceBudget] = []
        for (i, f) in faces.enumerated() {
            if f.shortestSidePx < CGFloat(ScanVision.minFaceShortestSidePx) {
                throw ScanVisionError.faceTooSmall(shortestSidePx: Int(f.shortestSidePx),
                                                   minimum: ScanVision.minFaceShortestSidePx)
            }
            let present = FaceLandmarkPaths.protectedRegions.filter { f.regions[$0] != nil }
            if present.count < FaceLandmarkPaths.protectedRegions.count {
                throw ScanVisionError.insufficientLandmarkPaths(
                    found: present.count, minimum: FaceLandmarkPaths.protectedRegions.count)
            }
            let guaranteed = max(ScanVision.minLandmarkPaths, f.pathCount)
            // Vision reports normalised, origin bottom-left; convert to a
            // top-left pixel rect for the palette sampler.
            let box = f.boundingBox
            let rect = CGRect(x: box.origin.x * w,
                              y: (1 - box.origin.y - box.height) * h,
                              width: box.width * w, height: box.height * h)
            out.append(FaceBudget(faceIndex: i, guaranteedPaths: guaranteed,
                                  protectedRegions: present, paletteRegionPx: rect))
        }
        return out.sorted { $0.paletteRegionPx.width > $1.paletteRegionPx.width }
    }
}
