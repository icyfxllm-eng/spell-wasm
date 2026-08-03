// CC-IOS-SURFACES (BD-1) — the one-way App Group snapshot (D2: the app
// writes on session end; widgets ONLY read). No networking symbols may
// enter this target — CI scans the binary (I-scan).
import Foundation

struct WidgetSnapshot: Codable {
    var schema: Int
    var streak: Int
    var dailyDone: Bool
    /// Endonym of today's date-seeded challenge language ("Español") —
    /// an audited-pool value written by the app; never composed here.
    var dailyLang: String
    var shieldsEarned: Int
    var shieldsTotal: Int
    var entitledLangs: [String]

    static let empty = WidgetSnapshot(
        schema: 1, streak: 0, dailyDone: false, dailyLang: "",
        shieldsEarned: 0, shieldsTotal: 5, entitledLangs: ["en"])

    static let groupId = "group.net.spellgame.app"
    static let fileName = "widget-snapshot.json"

    static func load() -> WidgetSnapshot {
        guard
            let dir = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: groupId),
            let data = try? Data(contentsOf: dir.appendingPathComponent(fileName)),
            let snap = try? JSONDecoder().decode(WidgetSnapshot.self, from: data),
            snap.schema == 1
        else { return .empty }
        return snap
    }
}

// Fixture states — the four the acceptance list names, used by previews.
extension WidgetSnapshot {
    static let freshInstall = WidgetSnapshot.empty
    static let streak12Done = WidgetSnapshot(schema: 1, streak: 12, dailyDone: true,
        dailyLang: "Español", shieldsEarned: 3, shieldsTotal: 5, entitledLangs: ["en", "es"])
    static let streak12NotDone = WidgetSnapshot(schema: 1, streak: 12, dailyDone: false,
        dailyLang: "Français", shieldsEarned: 3, shieldsTotal: 5, entitledLangs: ["en", "fr"])
    static let shields3of5 = WidgetSnapshot(schema: 1, streak: 4, dailyDone: false,
        dailyLang: "English", shieldsEarned: 3, shieldsTotal: 5, entitledLangs: ["en"])
}
