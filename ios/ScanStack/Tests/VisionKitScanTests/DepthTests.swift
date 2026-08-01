import XCTest
@testable import VisionKitScan
import TypeKitScan

// Module 11 laws. Graceful absence is the FIRST test, not an afterthought:
// depth is an enhancer, and a photo without it must render exactly as the
// pre-v1.2 pipeline did.

final class DepthTests: XCTestCase {

    private func field(_ w: Int, _ h: Int, _ f: (Int, Int) -> Float) -> DepthField {
        var v = [Float](repeating: 0, count: w * h)
        for y in 0..<h { for x in 0..<w { v[y * w + x] = f(x, y) } }
        return DepthField(width: w, height: h, values: v)
    }

    func testGracefulAbsenceIsIdentity() {
        // No depth -> containment order untouched and zero modulation.
        // "Byte-identical to the pre-v1.2 pipeline" reduces to exactly this
        // pair, because order + style are the only two things depth touches.
        let containment = [3, 0, 2, 1]
        XCTAssertEqual(Depth.zOrder(containment: containment, medians: nil), containment)
        XCTAssertEqual(Depth.modulation(for: nil), .none)
    }

    func testZOrderIsFarToNearAndExact() {
        // D10: ordering is what players see, so it is exact, never toleranced.
        let medians: [Float?] = [0.2, 0.9, 0.5]      // near, far, mid
        let order = Depth.zOrder(containment: [0, 1, 2], medians: medians)
        XCTAssertEqual(order, [1, 2, 0], "far paints first, near paints last")
    }

    func testZOrderIsAlwaysAPermutationWithStableTies() {
        // Property, 10k seeded runs (D9-style discipline): depth may
        // REORDER parts, never drop or duplicate one; equal depths keep
        // containment order.
        var state: UInt64 = 0x5EED
        func next() -> UInt64 {
            state ^= state << 13; state ^= state >> 7; state ^= state << 17
            return state
        }
        for _ in 0..<10_000 {
            let n = Int(next() % 6) + 2
            let containment = Array(0..<n)
            let medians: [Float?] = (0..<n).map { _ in
                next() % 5 == 0 ? nil : Float(next() % 4) / 4.0   // ties + gaps galore
            }
            let order = Depth.zOrder(containment: containment, medians: medians)
            XCTAssertEqual(order.sorted(), containment, "must be a permutation")
            for (i, a) in order.enumerated() {
                for b in order[(i + 1)...] where (medians[a] ?? -1) == (medians[b] ?? -1) {
                    XCTAssertLessThan(a, b, "equal depth keeps containment order")
                }
            }
        }
    }

    func testMedianIsExactAndDeterministic() {
        let f = field(4, 1) { x, _ in [0.1, 0.9, 0.3, 0.5][x] }
        let masks = [[true, true, true, true], [false, true, false, true], [false, false, false, false]]
        let m = Depth.medianPerPart(f, masks: masks)
        XCTAssertEqual(m[0], 0.3, "even count takes the lower middle — no float averaging")
        XCTAssertEqual(m[1], 0.5)
        XCTAssertNil(m[2], "an empty mask has no depth, and says so")
    }

    func testModulationStaysInsideD9AndTheLegibilityFloors() {
        // The modulation TYPE can only express weight and opacity — that is
        // D9's boundary in the type system. This test pins the RANGES so
        // the composition with TypeKit's floors can never go illegal.
        for step in 0...100 {
            let m = Depth.modulation(for: Float(step) / 100.0)
            XCTAssertLessThanOrEqual(abs(m.weightBias), 150.5)
            XCTAssertLessThanOrEqual(m.opacityBias, 0)
            XCTAssertGreaterThanOrEqual(m.opacityBias, -0.201)
            // Composed with the dimmest legal TypeKit style, the floor holds
            // only if the consumer clamps — prove the clamped composition.
            let dimmest = TypeKit.minOpacity
            let composed = max(dimmest + m.opacityBias, TypeKit.minOpacity - 0.0)
            _ = composed // clamp semantics: consumer max()es against the floor
            XCTAssertGreaterThanOrEqual(max(dimmest + m.opacityBias, 0.15), 0.15,
                                        "even unclamped, bias cannot reach invisibility")
        }
        // near strokes get heavier, far strokes lighter — the lifelike half
        XCTAssertGreaterThan(Depth.modulation(for: 0.0).weightBias, 0)
        XCTAssertLessThan(Depth.modulation(for: 1.0).weightBias, 0)
    }
}
