import Foundation

/// Minimal en/es string table for the widget extension. SwiftUI widgets can't
/// reach the app's JS i18n (`src/i18n`), so widget copy lives here. Any
/// non-Spanish device language falls back to English — the app currently ships
/// en + es as the active languages; more can be added as they activate.
enum WidgetStrings {
    private static let table: [String: [String: String]] = [
        "streak.title":     ["en": "Streak",              "es": "Racha"],
        "streak.days":      ["en": "day streak",          "es": "de racha"],
        "streak.practiced": ["en": "Practiced today",     "es": "Practicado hoy"],
        "streak.notYet":    ["en": "Not practiced yet",   "es": "Aún sin practicar"],
        "streak.start":     ["en": "Start your streak",   "es": "Empieza tu racha"],
        "streak.best":      ["en": "Best",                "es": "Mejor"],
        "daily.title":      ["en": "Daily Challenge",     "es": "Reto diario"],
        "daily.notStarted": ["en": "Not started",         "es": "Sin empezar"],
        "daily.inProgress": ["en": "In progress",         "es": "En curso"],
        "daily.done":       ["en": "Done today",          "es": "Completado hoy"],
        "daily.tapToPlay":  ["en": "Tap to play",         "es": "Toca para jugar"],
        "widget.streakName":  ["en": "Spell Streak",        "es": "Racha de Spell"],
        "widget.streakDesc":  ["en": "Your practice streak at a glance.",
                               "es": "Tu racha de práctica de un vistazo."],
        "widget.dailyName":   ["en": "Daily Challenge",     "es": "Reto diario"],
        "widget.dailyDesc":   ["en": "Today's Daily Challenge status.",
                               "es": "El estado del Reto diario de hoy."],
    ]

    /// Look up `key` for the app's language (as synced), collapsing any
    /// Spanish variant to "es" and everything else to English.
    static func t(_ key: String, _ lang: String) -> String {
        let l = lang.lowercased().hasPrefix("es") ? "es" : "en"
        return table[key]?[l] ?? table[key]?["en"] ?? key
    }
}
