import Foundation

/// The App Group contract shared by the app (writer, via NativeLanguageKitPlugin
/// `syncWidgetState`) and the readers (this WidgetKit extension today; the F4 App
/// Intents next). This file is compiled into BOTH the App target and the
/// SpellWidgets extension target.
///
/// Schema mirrors `src/widgets.rs::WidgetState` (camelCase JSON):
///   App Group: `group.net.spellgame.app`
///   Key:       `widget_state_v1`  → a JSON string of `SpellWidgetState`.
enum WidgetShared {
    static let appGroup = "group.net.spellgame.app"
    static let stateKey = "widget_state_v1"

    /// Persist the raw JSON string produced by the Rust core, verbatim. Storing
    /// the JSON (not decoded fields) keeps the schema a single contract shared by
    /// Rust, this extension, and F4.
    static func write(rawJSON: String) {
        UserDefaults(suiteName: appGroup)?.set(rawJSON, forKey: stateKey)
    }

    /// Decode the current snapshot, or `nil` when nothing has been synced yet.
    static func read() -> SpellWidgetState? {
        guard let json = UserDefaults(suiteName: appGroup)?.string(forKey: stateKey),
              let data = json.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(SpellWidgetState.self, from: data)
    }
}

/// Decoded widget snapshot. Field names MUST stay identical to the camelCase JSON
/// keys emitted by `src/widgets.rs`.
struct SpellWidgetState: Codable {
    var schemaVersion: Int
    var streak: Int
    var bestStreak: Int
    var lastCompletedDate: String
    var dailyStatus: String
    var dailyDate: String
    var language: String
    var updatedAtMs: Double

    /// Safe default for first run / decode failure — a "no streak yet" state.
    static let empty = SpellWidgetState(
        schemaVersion: 1, streak: 0, bestStreak: 0,
        lastCompletedDate: "", dailyStatus: "not_started",
        dailyDate: "", language: "en", updatedAtMs: 0
    )

    /// Local `yyyy-MM-dd` for today — matches `src/daily.rs::today()` (local
    /// calendar), so the day-rollover derivations below agree with the core.
    static func todayString() -> String {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "yyyy-MM-dd"
        return f.string(from: Date())
    }

    /// "Practiced today" = the last finished daily is today's local date.
    var practicedToday: Bool { lastCompletedDate == Self.todayString() }

    /// The stored `dailyStatus` is authoritative only while `dailyDate` is today;
    /// after midnight, today's daily is "not started" again (no extra write).
    var effectiveDailyStatus: String {
        dailyDate == Self.todayString() ? dailyStatus : "not_started"
    }
}
