// CC-MASTERPIECE-TONAL Feature 3 — Vision face landmarks, tool-side only
// (CC-SCAN-STACK Phase T0, greenlit by D3 for masterpiece authoring).
// Landmark output is SUGGESTION-ONLY: it seeds feature-path candidates
// that Eric approves/adjusts like any candidate. Never ships, never runs
// in the app target (the symbol scan enforces it).
//
// Build:  swiftc -O vision-landmarks.swift -o vision-landmarks
// Run:    ./vision-landmarks <image> > landmarks.json
import Foundation
import Vision
import AppKit

guard CommandLine.arguments.count > 1,
      let img = NSImage(contentsOfFile: CommandLine.arguments[1]),
      let cg = img.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    FileHandle.standardError.write("usage: vision-landmarks <image>\n".data(using: .utf8)!)
    exit(2)
}

let request = VNDetectFaceLandmarksRequest()
let handler = VNImageRequestHandler(cgImage: cg, options: [:])
try handler.perform([request])

guard let face = request.results?.first as? VNFaceObservation,
      let lm = face.landmarks else {
    print("{\"faces\": 0}")
    exit(0)
}

func pts(_ region: VNFaceLandmarkRegion2D?) -> [[Double]] {
    guard let r = region else { return [] }
    // normalizedPoints are in face bounding-box space; project to image
    // pixel space (origin top-left to match the authoring canvas).
    let bb = face.boundingBox
    let W = Double(cg.width), H = Double(cg.height)
    return r.normalizedPoints.map { p in
        let x = (Double(bb.origin.x) + Double(p.x) * Double(bb.width)) * W
        let yUp = (Double(bb.origin.y) + Double(p.y) * Double(bb.height)) * H
        return [x, H - yUp]
    }
}

var out: [String: Any] = ["faces": 1,
                          "imageW": cg.width, "imageH": cg.height]
out["leftEye"] = pts(lm.leftEye)
out["rightEye"] = pts(lm.rightEye)
out["leftBrow"] = pts(lm.leftEyebrow)
out["rightBrow"] = pts(lm.rightEyebrow)
out["nose"] = pts(lm.nose)
out["noseCrest"] = pts(lm.noseCrest)
out["mouth"] = pts(lm.outerLips)
out["innerLips"] = pts(lm.innerLips)
out["faceContour"] = pts(lm.faceContour)
out["medianLine"] = pts(lm.medianLine)

let data = try JSONSerialization.data(withJSONObject: out)
print(String(data: data, encoding: .utf8)!)
