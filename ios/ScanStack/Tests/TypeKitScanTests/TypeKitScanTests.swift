import XCTest
@testable import TypeKitScan

// CC-SCAN-STACK module 3 laws. Shading is weight and opacity; size is
// the solver's business and this module must never touch it.

final class TypeKitScanTests: XCTestCase {

    func testDarkerBandsAreHeavierAndMoreOpaque() {
        let n = 10
        var last = TypeKit.style(for: 0, of: n)   // darkest
        for b in 1..<n {
            let s = TypeKit.style(for: b, of: n)
            XCTAssertLessThan(s.weight, last.weight, "band \(b) must be lighter than \(b - 1)")
            XCTAssertLessThan(s.opacity, last.opacity)
            last = s
        }
    }

    func testWeightStaysOnTheAxisAndOpacityNeverVanishes() {
        for n in [8, 10, 12] {
            for b in 0..<n {
                let s = TypeKit.style(for: b, of: n)
                XCTAssertTrue(TypeKit.weightAxis.contains(s.weight), "weight off-axis: \(s.weight)")
                XCTAssertGreaterThanOrEqual(s.opacity, TypeKit.minOpacity,
                                            "the lightest band must still be readable")
                XCTAssertLessThanOrEqual(s.opacity, 1.0)
            }
        }
    }

    func testStyleExposesNoSizeControl() {
        // The v6/v8.1 solver owns size. If a size ever appears on
        // GlyphStyle, this stack has started competing with the layout
        // law — that is the bug this test exists to catch.
        let mirror = Mirror(reflecting: TypeKit.style(for: 3, of: 10))
        let names = mirror.children.compactMap { $0.label }
        XCTAssertEqual(Set(names), ["weight", "opacity"],
                       "GlyphStyle may only carry weight and opacity; got \(names)")
    }

    func testShippedFamilyIsTheApprovedOne() {
        XCTAssertEqual(TypeKit.shippedFamily, "InterVariable", "D3: Inter Variable ships")
    }

    func testDeterministic() {
        XCTAssertEqual(TypeKit.style(for: 4, of: 12), TypeKit.style(for: 4, of: 12))
    }
}
