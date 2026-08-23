// CC-CJK-INK step 0 — can Vision read a hand-drawn CJK character?
//
// The app already runs VNRecognizeTextRequest on device for photo-to-word-list,
// so if it can read a drawn character the handwriting pad is mostly plumbing
// onto parts that exist. If it cannot, the recogniser has to be stroke-based
// and that is a different build. Nothing else about the design can be decided
// before this is measured.
import Foundation
import Vision
import AppKit

let args = CommandLine.arguments
guard args.count > 1 else { print("usage: probe <dir>"); exit(1) }
let dir = args[1]
let files = (try! FileManager.default.contentsOfDirectory(atPath: dir))
    .filter { $0.hasSuffix(".png") }.sorted()

func recognize(_ path: String, langs: [String]) -> [(String, Float)] {
    guard let img = NSImage(contentsOfFile: path),
          let cg = img.cgImage(forProposedRect: nil, context: nil, hints: nil)
    else { return [] }
    let req = VNRecognizeTextRequest()
    req.recognitionLevel = .accurate
    req.usesLanguageCorrection = false
    req.recognitionLanguages = langs
    // A single glyph is not a text line, so let Vision consider small regions.
    req.minimumTextHeight = 0.05
    let handler = VNImageRequestHandler(cgImage: cg, options: [:])
    do { try handler.perform([req]) } catch { return [] }
    var out: [(String, Float)] = []
    for obs in (req.results ?? []) {
        for cand in obs.topCandidates(3) { out.append((cand.string, cand.confidence)) }
    }
    return out
}

var tally: [String: (hit: Int, n: Int)] = [:]
for f in files {
    let parts = f.replacingOccurrences(of: ".png", with: "").components(separatedBy: "__")
    guard parts.count == 3 else { continue }
    let (lang, cond, want) = (parts[0], parts[1], parts[2])
    let langs = lang == "zh" ? ["zh-Hans", "zh-Hant"] : ["ja"]
    let got = recognize("\(dir)/\(f)", langs: langs)
    let best = got.first.map { "\($0.0) \(String(format: "%.2f", $0.1))" } ?? "—"
    let hit = got.contains { $0.0.contains(want) }
    let key = "\(lang)/\(cond)"
    tally[key, default: (0,0)].n += 1
    if hit { tally[key]!.hit += 1 }
    print("  \(lang) \(cond.padding(toLength: 6, withPad: " ", startingAt: 0)) want \(want)  got \(best)  \(hit ? "HIT" : "miss")")
}
print("")
for (k, v) in tally.sorted(by: { $0.key < $1.key }) {
    print("  \(k.padding(toLength: 10, withPad: " ", startingAt: 0)) \(v.hit)/\(v.n)")
}
