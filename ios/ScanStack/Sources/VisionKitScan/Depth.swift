//  CC-SCAN-STACK v1.2 module 11 — depth, behind the frozen contract.
//
//  Depth is an ENHANCER, never a dependency: every consumer accepts nil and
//  a nil depth field must leave rendering byte-identical to the pre-v1.2
//  pipeline. That graceful absence is not a fallback bolted on afterwards —
//  it is the primary contract, and the tests treat it as such.
//
//  D9 (build-failing boundary): per-part median depth may reorder the
//  containment z-stack and bias TypeKit weight/opacity WITHIN the existing
//  legibility floors. It never touches glyph size, band assignment,
//  layout-law parameters, or word selection. The modulation type below
//  cannot express those things, which is the strongest form of the rule.
//
//  D10 (Eric, 2026-07-31): depth MAPS are held to MAE <= 0.05 against
//  per-platform goldens; z-ORDER must match goldens exactly, because
//  ordering is what players see. The golden corpus itself arrives with the
//  50-photo fixture set — the contract and its consumers land first so the
//  goldens have something stable to pin.

import Foundation
import CoreGraphics

/// Normalised depth as PURE DATA on the module-1 contract (v1.1 #10): no
/// Vision, ARKit or AVFoundation type escapes. 0 = nearest, 1 = farthest.
public struct DepthField {
    public let width: Int
    public let height: Int
    public let values: [Float]

    public init(width: Int, height: Int, values: [Float]) {
        precondition(values.count == width * height, "values must be width*height")
        self.width = width
        self.height = height
        self.values = values
    }
}

/// The ONLY things depth may change (D9). Weight bias in variable-font
/// axis units; opacity bias absolute. Both are biases over the band's
/// TypeKit style, clamped by the consumer against TypeKit's own floors —
/// there is deliberately no field here that could carry a size, a band,
/// or a word.
public struct DepthModulation: Equatable {
    public let weightBias: CGFloat
    public let opacityBias: CGFloat
    public static let none = DepthModulation(weightBias: 0, opacityBias: 0)

    public init(weightBias: CGFloat, opacityBias: CGFloat) {
        self.weightBias = weightBias
        self.opacityBias = opacityBias
    }
}

public enum Depth {
    /// Median depth per part, from each part's mask over the field.
    /// Deterministic: exact median of the masked values, lower-of-two for
    /// even counts (no float averaging surprises across platforms).
    public static func medianPerPart(_ field: DepthField, masks: [[Bool]]) -> [Float?] {
        masks.map { mask in
            precondition(mask.count == field.values.count, "mask must match field")
            var vals: [Float] = []
            vals.reserveCapacity(mask.count / 8)
            for i in 0..<mask.count where mask[i] {
                vals.append(field.values[i])
            }
            guard !vals.isEmpty else { return nil }
            vals.sort()
            return vals[(vals.count - 1) / 2]
        }
    }

    /// Paint order, far-to-near, from per-part medians. Nil depth — the
    /// graceful-absence leg — returns the CONTAINMENT order untouched, so
    /// a depth-less render is exactly the pre-v1.2 render. Ties keep
    /// containment order (stable), and the result is always a permutation:
    /// depth may reorder parts, never drop one.
    public static func zOrder(containment: [Int], medians: [Float?]?) -> [Int] {
        guard let medians else { return containment }
        precondition(medians.count == containment.count, "one median per part")
        // Stable sort on (depth desc = far first); parts with no depth
        // sample keep their containment slot relative to each other by
        // sorting with their original index as the tiebreak.
        return containment.enumerated().sorted { a, b in
            let da = medians[a.offset] ?? -1
            let db = medians[b.offset] ?? -1
            if da != db { return da > db }
            return a.offset < b.offset
        }.map(\.element)
    }

    /// Style bias from a part's median depth: near strokes heavier and
    /// fuller, far strokes lighter and fainter. The RANGE is capped so no
    /// clamp downstream can be defeated: |weight| <= 150 axis units,
    /// opacity bias can never push below TypeKit's 0.35 floor when applied
    /// to a legal style (floor 0.35 + bias >= 0.35 - 0.20 is impossible —
    /// the consumer clamps, and the test proves the composition).
    public static func modulation(for median: Float?) -> DepthModulation {
        guard let median else { return .none }
        let m = CGFloat(min(max(median, 0), 1))
        // near (0) -> +150 weight, +0.0 opacity; far (1) -> -150, -0.20
        return DepthModulation(weightBias: 150 - 300 * m, opacityBias: -0.20 * m)
    }
}
