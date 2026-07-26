import Foundation

/// Traditional → Simplified Chinese conversion, entirely ON-DEVICE via the
/// OS's ICU transliterator (no table shipped, no network, nothing leaves the
/// phone). Lets a photographed page written in Traditional characters (a
/// Taiwan/Hong Kong workbook, a grandparent's handwriting) import straight
/// into practice against the app's Simplified word banks.
///
/// Direction matters: Hant→Hans is the deterministic direction (many
/// Traditional forms collapse onto one Simplified form). The reverse is
/// ambiguous and is deliberately NOT offered.
public enum ChineseScript {

    /// Convert any Traditional characters in `text` to Simplified. Text that
    /// is already Simplified (or not Chinese at all) passes through unchanged;
    /// on the rare transform failure the input is returned untouched.
    public static func toSimplified(_ text: String) -> String {
        text.applyingTransform(StringTransform(rawValue: "Hant-Hans"), reverse: false) ?? text
    }

    /// Is this a Chinese recognition/speak language tag ("zh-Hans", "zh-Hant",
    /// "cmn-CN", …)? Decides whether the photo pipeline normalizes script.
    public static func isChineseTag(_ tag: String) -> Bool {
        let lower = tag.lowercased()
        return lower.hasPrefix("zh") || lower.hasPrefix("cmn") || lower.hasPrefix("yue")
    }
}
