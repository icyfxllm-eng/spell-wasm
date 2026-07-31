import XCTest
@testable import TonalKit

// CC-SCAN-STACK Phase T0 — TonalKit laws. D5: seed + photo bytes fully
// determine bands, palette and flow, on every run and device class.

final class TonalKitTests: XCTestCase {

    // A deterministic synthetic photo: a horizontal luminance ramp with a
    // diagonal texture laid over it. Stands in for the grayscale-ramp
    // fixture in Done-when #3 until real photos are wired.
    private func rampWithTexture(_ w: Int = 96, _ h: Int = 64) -> Gray {
        var px = [UInt8](repeating: 0, count: w * h)
        for y in 0..<h {
            for x in 0..<w {
                let ramp = Double(x) / Double(w - 1) * 255.0
                let tex = sin(Double(x + y) * 0.7) * 18.0
                px[y * w + x] = UInt8(max(0, min(255, ramp + tex)))
            }
        }
        return Gray(width: w, height: h, px: px)
    }

    func testPosterizeIsDeterministic() {
        let g = rampWithTexture()
        let a = Tonal.posterize(g, count: 10)
        let b = Tonal.posterize(g, count: 10)
        XCTAssertEqual(a.band, b.band, "same photo must yield the same bands (D5)")
        XCTAssertEqual(a.bounds, b.bounds)
    }

    func testPosterizeUsesEveryBand() {
        // The failure this guards: equal-luminance cuts leave most bands
        // empty on a clustered photo, and the render loses its shading.
        let g = rampWithTexture()
        let m = Tonal.posterize(g, count: 10)
        let used = Set(m.band)
        XCTAssertEqual(used.count, 10, "every band carries pixels; got \(used.count)")
    }

    func testPosterizeIsMonotonicInLuminance() {
        let g = rampWithTexture()
        let m = Tonal.posterize(g, count: 8)
        for i in 0..<g.px.count {
            for j in 0..<g.px.count where g.px[i] < g.px[j] {
                XCTAssertLessThanOrEqual(m.band[i], m.band[j],
                                         "a darker pixel can never land in a lighter band")
                break
            }
        }
    }

    func testFlowFollowsTextureDirection() {
        // Vertical stripes -> edges run vertically -> flow angle ~pi/2.
        let w = 64, h = 64
        var px = [UInt8](repeating: 0, count: w * h)
        for y in 0..<h {
            for x in 0..<w { px[y * w + x] = (x / 4) % 2 == 0 ? 30 : 220 }
        }
        let f = Flow.field(Gray(width: w, height: h, px: px))
        var sum = 0.0, n = 0
        for y in 8..<(h - 8) {
            for x in 8..<(w - 8) where f.coherence[y * w + x] > 0.5 {
                sum += Double(f.angle[y * w + x]); n += 1
            }
        }
        XCTAssertGreaterThan(n, 100, "stripes must produce coherent flow")
        let mean = sum / Double(n)
        XCTAssertEqual(mean, .pi / 2, accuracy: 0.2, "flow runs along the stripes")
    }

    func testFlowIsDeterministic() {
        let g = rampWithTexture()
        let a = Flow.field(g), b = Flow.field(g)
        XCTAssertEqual(a.angle, b.angle)
        XCTAssertEqual(a.coherence, b.coherence)
    }

    func testPaletteIsSeedDeterministicAndSeedSensitive() {
        var rgb: [(UInt8, UInt8, UInt8)] = []
        for i in 0..<5000 {
            let v = UInt8(i % 256)
            rgb.append((v, UInt8((i * 7) % 256), UInt8((i * 13) % 256)))
        }
        let a = Palette.medianCut(rgb: rgb, count: 6, seed: 42)
        let b = Palette.medianCut(rgb: rgb, count: 6, seed: 42)
        XCTAssertEqual(a, b, "same seed, same palette (D5)")
        XCTAssertEqual(a.count, 6)
        XCTAssertTrue(zip(a, a.dropFirst()).allSatisfy { $0.luma <= $1.luma },
                      "palette is ordered dark -> light")
    }

    func testContrastFloorDropsInvisibleSwatchesButNeverAll() {
        let onPage = Swatch(r: 250, g: 250, b: 250, population: 10)  // invisible on white
        let ink = Swatch(r: 20, g: 20, b: 20, population: 10)
        let kept = Palette.enforceContrastFloor([onPage, ink], against: 1.0)
        XCTAssertEqual(kept, [ink], "a swatch the page swallows is not a colour we can spell in")
        let allInvisible = Palette.enforceContrastFloor([onPage], against: 1.0)
        XCTAssertEqual(allInvisible.count, 1, "never return an empty palette")
    }
}
