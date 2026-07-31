//  CC-SCAN-STACK module 5 — the acceleration boundary.
//
//  Metal is a PERFORMANCE path, never a correctness dependency: the same
//  math, moved to the GPU, behind the same interface. This file defines
//  that interface and keeps the CPU implementation as the reference, so
//  the equivalence check has something to compare against the day the
//  shaders land.

import Foundation

public protocol TonalBackend {
    var name: String { get }
    func posterize(_ g: Gray, count: Int) -> BandMap
    func flow(_ g: Gray, window: Int) -> FlowField
}

public struct CPUBackend: TonalBackend {
    public init() {}
    public var name: String { "cpu" }
    public func posterize(_ g: Gray, count: Int) -> BandMap { Tonal.posterize(g, count: count) }
    public func flow(_ g: Gray, window: Int) -> FlowField { Flow.field(g, window: window) }
}

public enum Backends {
    /// The GPU backend appears here when the shaders land. Until then the
    /// list holds one entry and the equivalence test still runs — it just
    /// compares the CPU path to itself, so the harness cannot rot.
    public static var all: [TonalBackend] { [CPUBackend()] }

    /// Module 5's contract: every backend must agree, exactly, on bands
    /// and flow. Returns the first disagreement it finds.
    public static func firstDisagreement(on g: Gray, bands: Int = 10) -> String? {
        let list = all
        guard let ref = list.first else { return nil }
        let refBands = ref.posterize(g, count: bands)
        let refFlow = ref.flow(g, window: 3)
        for b in list.dropFirst() {
            if b.posterize(g, count: bands).band != refBands.band {
                return "\(b.name) disagrees with \(ref.name) on bands"
            }
            let f = b.flow(g, window: 3)
            for i in 0..<f.angle.count where abs(f.angle[i] - refFlow.angle[i]) > 1e-5 {
                return "\(b.name) disagrees with \(ref.name) on flow at \(i)"
            }
        }
        return nil
    }
}
