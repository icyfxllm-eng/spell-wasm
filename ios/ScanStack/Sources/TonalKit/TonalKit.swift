//  CC-SCAN-STACK module 2 — TonalKit.
//
//  Deterministic tonal & flow math: photo + seed in, identical output
//  out, every run and every device class (D5). Nothing here invents a
//  pixel — every value traces back to sampled data.
//
//  Phase T0 scope: pure Swift, no Vision, no Metal, no font. Those carry
//  D2/D3 and stay unwritten until Eric signs off. vImage/Accelerate can
//  replace these loops later WITHOUT changing results — the fixtures in
//  TonalKitTests are what will hold that promise (module 5's CPU/GPU
//  equivalence check reuses them).

import Foundation

public struct Gray {
    public let width: Int
    public let height: Int
    /// Row-major luminance, 0...255.
    public let px: [UInt8]

    public init(width: Int, height: Int, px: [UInt8]) {
        precondition(px.count == width * height, "px must be width*height")
        self.width = width
        self.height = height
        self.px = px
    }

    @inlinable public func at(_ x: Int, _ y: Int) -> UInt8 {
        px[y * width + x]
    }
}

// MARK: - Posterization (CC-PHOTO-PICTURE v2 LR block: 8–12 bands)

public struct BandMap {
    public let width: Int
    public let height: Int
    /// Band index per pixel, 0 = darkest.
    public let band: [UInt8]
    public let count: Int
    /// Upper luminance bound of each band (inclusive), ascending.
    public let bounds: [UInt8]
}

public enum Tonal {
    /// Posterize into `count` bands by EQUAL-POPULATION quantiles, not
    /// equal luminance: a photo whose tones cluster (most portraits)
    /// otherwise collapses into two used bands and eight empty ones.
    /// Deterministic — the histogram fully determines the cuts.
    public static func posterize(_ g: Gray, count: Int) -> BandMap {
        precondition((2...16).contains(count), "band count out of range")
        var hist = [Int](repeating: 0, count: 256)
        for v in g.px { hist[Int(v)] += 1 }
        let total = g.px.count
        var bounds: [UInt8] = []
        var acc = 0
        var next = 1
        for v in 0..<256 {
            acc += hist[v]
            while next < count && acc * count >= next * total {
                bounds.append(UInt8(v))
                next += 1
            }
        }
        while bounds.count < count { bounds.append(255) }
        var band = [UInt8](repeating: 0, count: total)
        for i in 0..<total {
            let v = g.px[i]
            var b = 0
            while b < count - 1 && v > bounds[b] { b += 1 }
            band[i] = UInt8(b)
        }
        return BandMap(width: g.width, height: g.height, band: band,
                       count: count, bounds: bounds)
    }
}

// MARK: - Structure tensor flow field (stroke direction follows texture)

public struct FlowField {
    public let width: Int
    public let height: Int
    /// Dominant orientation per pixel in radians, 0..<pi (undirected).
    public let angle: [Float]
    /// Anisotropy 0...1 — how strongly that direction dominates.
    public let coherence: [Float]
}

public enum Flow {
    /// Sobel gradients -> structure tensor -> principal direction. The
    /// stroke path follows the texture, which is what makes fur read as
    /// fur rather than as hatching.
    public static func field(_ g: Gray, window: Int = 3) -> FlowField {
        let w = g.width, h = g.height
        var gx = [Float](repeating: 0, count: w * h)
        var gy = [Float](repeating: 0, count: w * h)
        for y in 1..<(h - 1) {
            for x in 1..<(w - 1) {
                func p(_ dx: Int, _ dy: Int) -> Float { Float(g.at(x + dx, y + dy)) }
                gx[y * w + x] = (p(1, -1) + 2 * p(1, 0) + p(1, 1)) - (p(-1, -1) + 2 * p(-1, 0) + p(-1, 1))
                gy[y * w + x] = (p(-1, 1) + 2 * p(0, 1) + p(1, 1)) - (p(-1, -1) + 2 * p(0, -1) + p(1, -1))
            }
        }
        var angle = [Float](repeating: 0, count: w * h)
        var coherence = [Float](repeating: 0, count: w * h)
        let r = max(1, window / 2)
        for y in 0..<h {
            for x in 0..<w {
                var jxx: Float = 0, jyy: Float = 0, jxy: Float = 0
                for dy in -r...r {
                    for dx in -r...r {
                        let nx = x + dx, ny = y + dy
                        guard nx >= 0, ny >= 0, nx < w, ny < h else { continue }
                        let a = gx[ny * w + nx], b = gy[ny * w + nx]
                        jxx += a * a; jyy += b * b; jxy += a * b
                    }
                }
                // principal eigenvector of [[jxx, jxy], [jxy, jyy]]
                let diff = jxx - jyy
                let theta = 0.5 * atan2(2 * jxy, diff)
                // gradient direction -> edge direction is perpendicular
                var e = theta + .pi / 2
                while e < 0 { e += .pi }
                while e >= .pi { e -= .pi }
                angle[y * w + x] = e
                let trace = jxx + jyy
                let disc = sqrt(diff * diff + 4 * jxy * jxy)
                coherence[y * w + x] = trace > 0 ? disc / trace : 0
            }
        }
        return FlowField(width: w, height: h, angle: angle, coherence: coherence)
    }
}

// MARK: - Seeded palette (median cut) with the contrast floor

public struct Swatch: Equatable {
    public let r: UInt8, g: UInt8, b: UInt8
    public let population: Int
    /// Relative luminance 0...1 (Rec. 709).
    public var luma: Double {
        (0.2126 * Double(r) + 0.7152 * Double(g) + 0.0722 * Double(b)) / 255.0
    }
}

public enum Palette {
    /// Median-cut over sampled pixels. `seed` fixes the sampling stride
    /// offset so the same photo + seed always yields the same palette
    /// (D5); the cut itself is deterministic given the sample.
    public static func medianCut(rgb: [(UInt8, UInt8, UInt8)], count: Int, seed: UInt64) -> [Swatch] {
        precondition(count > 0)
        guard !rgb.isEmpty else { return [] }
        var state = seed &+ 0x9E37_79B9_7F4A_7C15
        func next() -> UInt64 {
            state = state &+ 0x9E37_79B9_7F4A_7C15
            var z = state
            z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
            z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
            return z ^ (z >> 31)
        }
        let stride = max(1, rgb.count / 20_000)
        let offset = Int(next() % UInt64(stride))
        var box: [[(UInt8, UInt8, UInt8)]] = [Swift.stride(from: offset, to: rgb.count, by: stride).map { rgb[$0] }]
        while box.count < count {
            guard let i = box.enumerated().max(by: { $0.element.count < $1.element.count })?.offset,
                  box[i].count > 1 else { break }
            let b = box.remove(at: i)
            let rs = b.map { Int($0.0) }, gs = b.map { Int($0.1) }, bs = b.map { Int($0.2) }
            let spread = [rs.max()! - rs.min()!, gs.max()! - gs.min()!, bs.max()! - bs.min()!]
            let axis = spread.firstIndex(of: spread.max()!)!
            let sorted = b.sorted {
                axis == 0 ? $0.0 < $1.0 : (axis == 1 ? $0.1 < $1.1 : $0.2 < $1.2)
            }
            let mid = sorted.count / 2
            box.append(Array(sorted[..<mid]))
            box.append(Array(sorted[mid...]))
        }
        return box.filter { !$0.isEmpty }.map { b in
            let n = b.count
            return Swatch(r: UInt8(b.reduce(0) { $0 + Int($1.0) } / n),
                          g: UInt8(b.reduce(0) { $0 + Int($1.1) } / n),
                          b: UInt8(b.reduce(0) { $0 + Int($1.2) } / n),
                          population: n)
        }.sorted { $0.luma < $1.luma }
    }

    /// The contrast floor: drop swatches that cannot be told apart from
    /// the page. Returns the kept swatches, never fewer than one.
    public static func enforceContrastFloor(_ swatches: [Swatch], against background: Double,
                                            minDelta: Double = 0.18) -> [Swatch] {
        let kept = swatches.filter { abs($0.luma - background) >= minDelta }
        if kept.isEmpty, let farthest = swatches.max(by: {
            abs($0.luma - background) < abs($1.luma - background)
        }) {
            return [farthest]
        }
        return kept
    }
}
