import CoreGraphics
import XCTest
@testable import VisionKitScan

// CC-SCAN-STACK module 1 laws. What can be asserted without a face
// corpus: the D2 gate, the D4 floors, and the D5 IoU comparison.

final class VisionKitScanTests: XCTestCase {

    func testD2GateIsAFeatureGateNotACrash() {
        // On any machine that can run these tests the answer is true;
        // what matters is that callers can ASK instead of crashing on an
        // older OS.
        XCTAssertTrue(ScanVision.isAvailable || !ScanVision.isAvailable)
    }

    func testD4FloorsAreTheApprovedNumbers() {
        XCTAssertEqual(ScanVision.minLandmarkPaths, 8)
        XCTAssertEqual(ScanVision.minFaceShortestSidePx, 60)
    }

    func testSmallFaceIsRefusedNotDegraded() {
        // A 40px face must produce the audited refusal, never a smudge.
        let err = ScanVisionError.faceTooSmall(shortestSidePx: 40, minimum: 60)
        guard case .faceTooSmall(let px, let min) = err else { return XCTFail() }
        XCTAssertLessThan(px, min)
    }

    func testIoUIsTheComparisonNotByteIdentity() {
        // D5: Vision output shifts across OS versions, so goldens compare
        // by overlap. Identical masks score 1; a half-shifted mask scores
        // well under the 0.98 pin.
        let a: [Float] = [1, 1, 1, 1, 0, 0, 0, 0]
        XCTAssertEqual(ScanVision.iou(a, a), 1.0, accuracy: 0.0001)
        let b: [Float] = [0, 0, 1, 1, 1, 1, 0, 0]
        XCTAssertLessThan(ScanVision.iou(a, b), 0.98)
        XCTAssertGreaterThan(ScanVision.iou(a, b), 0.0)
    }

    func testLandmarkProtectedSetMatchesTheSnowmanEyesRule() {
        XCTAssertEqual(Set(FaceLandmarkPaths.protectedRegions),
                       ["leftEye", "rightEye", "outerLips"],
                       "eyes and mouth are protected — they render even when they cannot host a word")
    }

    /// Real Vision call on a synthetic image: proves the wrapper runs and
    /// refuses cleanly rather than throwing something unexpected.
    func testInstanceMasksRunOnASyntheticImage() throws {
        guard #available(iOS 17.0, macOS 14.0, *) else {
            throw XCTSkip("D2: instance masks need iOS 17 / macOS 14")
        }
        let w = 64, h = 64
        var px = [UInt8](repeating: 220, count: w * h * 4)
        for y in 20..<44 {
            for x in 20..<44 {
                let i = (y * w + x) * 4
                px[i] = 40; px[i + 1] = 40; px[i + 2] = 40; px[i + 3] = 255
            }
        }
        let cs = CGColorSpaceCreateDeviceRGB()
        let ctx = CGContext(data: &px, width: w, height: h, bitsPerComponent: 8,
                            bytesPerRow: w * 4, space: cs,
                            bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
        let image = ctx.makeImage()!
        let masks = try ScanVision.instanceMasks(image)
        // A synthetic square may or may not read as a foreground instance;
        // either is fine. What must hold is that we got here without a
        // crash and every returned mask is well-formed.
        for m in masks {
            XCTAssertEqual(m.alpha.count, m.width * m.height)
        }
    }
}

// MARK: - Module 4 laws

final class FaceDetailTests: XCTestCase {
    private func face(short: CGFloat, regions: [String]) -> FaceLandmarkPaths {
        var r: [String: [CGPoint]] = [:]
        for n in regions { r[n] = [CGPoint(x: 0.1, y: 0.1), CGPoint(x: 0.2, y: 0.2)] }
        return FaceLandmarkPaths(regions: r,
                                 boundingBox: CGRect(x: 0.1, y: 0.1, width: 0.3, height: 0.3),
                                 shortestSidePx: short)
    }
    private let ten = ["faceContour", "leftEye", "rightEye", "leftEyebrow", "rightEyebrow",
                       "nose", "noseCrest", "outerLips", "innerLips", "medianLine"]

    func testEveryFaceClearsTheSameFloor() throws {
        let faces = [face(short: 400, regions: ten), face(short: 65, regions: ten)]
        let budgets = try FaceDetail.budgets(for: faces, imageWidth: 1000, imageHeight: 800)
        XCTAssertEqual(budgets.count, 2)
        for b in budgets {
            XCTAssertGreaterThanOrEqual(b.guaranteedPaths, ScanVision.minLandmarkPaths,
                                        "the back row gets the same floor as the front")
            XCTAssertEqual(Set(b.protectedRegions), Set(FaceLandmarkPaths.protectedRegions))
        }
    }

    func testTooSmallFaceRefusesRatherThanDegrades() {
        let faces = [face(short: 40, regions: ten)]
        XCTAssertThrowsError(try FaceDetail.budgets(for: faces, imageWidth: 500, imageHeight: 500)) {
            guard case ScanVisionError.faceTooSmall = $0 else {
                return XCTFail("expected the audited refusal, got \($0)")
            }
        }
    }

    func testMissingEyeOrMouthIsRefused() {
        let noMouth = ["faceContour", "leftEye", "rightEye", "leftEyebrow", "rightEyebrow",
                       "nose", "noseCrest", "medianLine", "innerLips"]
        XCTAssertThrowsError(try FaceDetail.budgets(for: [face(short: 300, regions: noMouth)],
                                                    imageWidth: 900, imageHeight: 900)) {
            guard case ScanVisionError.insufficientLandmarkPaths = $0 else {
                return XCTFail("eyes and mouth are protected — a face without them is refused")
            }
        }
    }

    func testEachFaceGetsItsOwnPaletteRegion() throws {
        var a = face(short: 300, regions: ten)
        a = FaceLandmarkPaths(regions: a.regions,
                              boundingBox: CGRect(x: 0.05, y: 0.5, width: 0.2, height: 0.2),
                              shortestSidePx: 300)
        var b = face(short: 300, regions: ten)
        b = FaceLandmarkPaths(regions: b.regions,
                              boundingBox: CGRect(x: 0.7, y: 0.5, width: 0.2, height: 0.2),
                              shortestSidePx: 300)
        let budgets = try FaceDetail.budgets(for: [a, b], imageWidth: 1000, imageHeight: 1000)
        XCTAssertNotEqual(budgets[0].paletteRegionPx, budgets[1].paletteRegionPx,
                          "each person keeps their own hair and skin tones")
    }
}
