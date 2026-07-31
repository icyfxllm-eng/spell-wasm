//  CC-SCAN-STACK module 3 — TypeKit: realism as typography.
//
//  Shading comes from stroke WEIGHT and OPACITY, never from shrinking
//  glyphs below the legibility floor. The v6/v8.1 solver remains the only
//  authority on size and placement; this module answers one question:
//  "given this tonal band, how heavy and how opaque?"
//
//  D3 (approved): Inter Variable (OFL) is the shipped face; SF is dev-only.
//  The font BINARY lands with app integration, which stays blocked on
//  P1+P2 — the mapping below is font-independent and testable now.

import CoreGraphics
import Foundation

public struct GlyphStyle: Equatable {
    /// Variable-font weight axis ('wght'), 100...900.
    public let weight: CGFloat
    /// Fill opacity, 0...1 — never below `minOpacity`.
    public let opacity: CGFloat
}

public enum TypeKit {
    /// D3: the shipped face. Dev builds may substitute SF; nothing else.
    public static let shippedFamily = "InterVariable"
    public static let devOnlyFallback = "SF Pro"

    public static let weightAxis: ClosedRange<CGFloat> = 100...900
    /// A word must stay readable even in the lightest band: shading is
    /// weight, and weight bottoms out here rather than fading to nothing.
    public static let minOpacity: CGFloat = 0.35

    /// Band -> style. `band` counts from 0 = DARKEST, matching TonalKit.
    /// Darkest band gets the heaviest, most opaque ink; lightest gets the
    /// thinnest — but never invisible and never smaller.
    public static func style(for band: Int, of count: Int) -> GlyphStyle {
        precondition(count >= 2, "need at least two bands")
        let b = max(0, min(count - 1, band))
        // t: 1 at the darkest band, 0 at the lightest
        let t = CGFloat(count - 1 - b) / CGFloat(count - 1)
        let weight = weightAxis.lowerBound + t * (weightAxis.upperBound - weightAxis.lowerBound)
        let opacity = minOpacity + t * (1.0 - minOpacity)
        return GlyphStyle(weight: weight.rounded(), opacity: opacity)
    }
}
