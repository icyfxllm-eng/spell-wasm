// CC-PHOTO-IMPORT Phase 7 — render the OCR fixture set.
//
// Reads scripts/ocr-fixtures/spec.json ({lang: [word,…]}), renders each
// language's list in 6 styles — 3 printed fonts + 3 handwriting-style fonts
// (synthetic print-style handwriting; photographed real handwriting can be
// added beside these later without changing the eval) — and writes PNGs +
// manifest.json into the SPM test resources
// (Tests/NativeLanguageKitCoreTests/Resources/ocr-fixtures/).
//
// Run:  swift scripts/ocr-fixtures/gen-ocr-fixtures.swift
import AppKit
import Foundation

let repo = FileManager.default.currentDirectoryPath
let specURL = URL(fileURLWithPath: "\(repo)/scripts/ocr-fixtures/spec.json")
let outDir = URL(fileURLWithPath:
    "\(repo)/ios/NativeLanguageKit/Tests/NativeLanguageKitCoreTests/Resources/ocr-fixtures")

let spec = try JSONDecoder().decode([String: [String]].self, from: Data(contentsOf: specURL))
try FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)

/// (style-slug, candidate font names — first resolvable wins, ".system" = system font)
let styles: [(String, [String])] = [
    ("print-helvetica", ["Helvetica"]),
    ("print-times", ["Times New Roman", "TimesNewRomanPSMT", "Georgia"]),
    ("print-menlo", ["Menlo-Regular", "Menlo", "Courier"]),
    ("hand-noteworthy", ["Noteworthy-Light", "Noteworthy"]),
    ("hand-markerfelt", ["MarkerFelt-Thin", "Marker Felt"]),
    ("hand-bradley", ["BradleyHandITCTT-Bold", "Bradley Hand", "ChalkboardSE-Regular", "Chalkboard SE"]),
]

func resolveFont(_ names: [String], size: CGFloat) -> NSFont {
    for n in names {
        if let f = NSFont(name: n, size: size) { return f }
    }
    return NSFont.systemFont(ofSize: size)
}

struct ManifestEntry: Codable {
    let file: String
    let lang: String
    let style: String
    let handwriting: Bool
    let words: [String]
}

var manifest: [ManifestEntry] = []

for (lang, words) in spec.sorted(by: { $0.key < $1.key }) {
    for (slug, fontNames) in styles {
        let font = resolveFont(fontNames, size: 56)
        let lineHeight: CGFloat = 96
        let width: CGFloat = 900
        let height = CGFloat(words.count) * lineHeight + 120
        let image = NSImage(size: NSSize(width: width, height: height))
        image.lockFocus()
        NSColor.white.setFill()
        NSRect(x: 0, y: 0, width: width, height: height).fill()
        let attrs: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: NSColor.black,
        ]
        for (i, word) in words.enumerated() {
            // Numbered like a real handout — the parser strips "1." numbering.
            let line = "\(i + 1). \(word)" as NSString
            let y = height - 90 - CGFloat(i) * lineHeight
            line.draw(at: NSPoint(x: 70, y: y), withAttributes: attrs)
        }
        image.unlockFocus()

        guard let tiff = image.tiffRepresentation,
              let rep = NSBitmapImageRep(data: tiff),
              let png = rep.representation(using: .png, properties: [:]) else {
            fatalError("render failed for \(lang)/\(slug)")
        }
        let file = "\(lang)-\(slug).png"
        try png.write(to: outDir.appendingPathComponent(file))
        manifest.append(ManifestEntry(
            file: file, lang: lang, style: slug,
            handwriting: slug.hasPrefix("hand-"), words: words))
        print("\(file)  font=\(font.fontName)")
    }
}

let enc = JSONEncoder()
enc.outputFormatting = [.prettyPrinted, .sortedKeys]
try enc.encode(manifest).write(to: outDir.appendingPathComponent("manifest.json"))
print("manifest.json — \(manifest.count) fixtures")
