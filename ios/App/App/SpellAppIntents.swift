import AppIntents
import Foundation

// F4 — Siri / App Intents (iOS 16+). Exactly THREE intents, no parameters, no
// in-Siri gameplay:
//
//   1. StartDailyChallengeIntent  — foregrounds into today's Daily Challenge.
//   2. PracticeMissedWordsIntent  — foregrounds into missed-words review.
//   3. CheckMyStreakIntent        — answers your streak with a spoken/shown
//                                   dialog, read from the SAME App Group blob F3
//                                   writes (`widget_state_v1`); no app launch.
//
// Intents 1 & 2 enter their surface through the SAME deep-link path F3
// established — `SpellURLRouter.route(host:)` posts `spellgame://daily` /
// `spellgame://missed` to the webview, exactly like a widget tap — so intents and
// widgets share ONE entry plumbing.
//
// These live in the APP target (no separate extension is needed to foreground the
// app). AppShortcuts are compiled into the binary and iOS registers them at
// install time regardless of the runtime `flags::app_intents()` gate — see that
// flag's docs and the PR for the "always registered / feature is dark" nuance.

// MARK: - Runtime en/es strings

/// Minimal en/es table (like the widget's `WidgetStrings`) for the one piece of
/// copy resolved at RUNTIME in the app process — the Check-My-Streak dialog —
/// where the device language is known. The Siri phrase/title localization is
/// handled separately (bilingual phrases below); this table is purely for the
/// spoken/shown streak result. Any non-Spanish device language falls back to en.
@available(iOS 16.0, *)
enum IntentStrings {
    private static let table: [String: [String: String]] = [
        "streak.none": [
            "en": "You haven't started a streak yet. Practice today to begin one!",
            "es": "Aún no tienes una racha. ¡Practica hoy para empezar una!",
        ],
        // %d = current streak length.
        "streak.practiced": [
            "en": "You're on a %d-day streak and you've already practiced today. Keep it going!",
            "es": "Llevas una racha de %d días y ya practicaste hoy. ¡Sigue así!",
        ],
        "streak.pending": [
            "en": "You're on a %d-day streak. Practice today so you don't lose it!",
            "es": "Llevas una racha de %d días. ¡Practica hoy para no perderla!",
        ],
    ]

    /// The device UI language collapsed to "es" (any Spanish variant) or "en".
    static func deviceLang() -> String {
        let code = Locale.preferredLanguages.first ?? "en"
        return code.lowercased().hasPrefix("es") ? "es" : "en"
    }

    private static func t(_ key: String) -> String {
        let l = deviceLang()
        return table[key]?[l] ?? table[key]?["en"] ?? key
    }

    /// The spoken/shown streak result for the App Group snapshot.
    static func streakDialog(streak: Int, practicedToday: Bool) -> String {
        if streak <= 0 { return t("streak.none") }
        return String(format: t(practicedToday ? "streak.practiced" : "streak.pending"), streak)
    }
}

// MARK: - Intent 1: Start Daily Challenge

@available(iOS 16.0, *)
struct StartDailyChallengeIntent: AppIntent {
    static var title: LocalizedStringResource = "Start Daily Challenge"
    static var description = IntentDescription("Open today's Daily Challenge in Spell.")
    // Foreground the app; perform() then routes to the Daily surface.
    static var openAppWhenRun: Bool = true

    @MainActor
    func perform() async throws -> some IntentResult {
        SpellURLRouter.route(host: "daily")
        return .result()
    }
}

// MARK: - Intent 2: Practice Missed Words

@available(iOS 16.0, *)
struct PracticeMissedWordsIntent: AppIntent {
    static var title: LocalizedStringResource = "Practice Missed Words"
    static var description = IntentDescription("Open your missed-words review in Spell.")
    static var openAppWhenRun: Bool = true

    @MainActor
    func perform() async throws -> some IntentResult {
        SpellURLRouter.route(host: "missed")
        return .result()
    }
}

// MARK: - Intent 3: Check My Streak

@available(iOS 16.0, *)
struct CheckMyStreakIntent: AppIntent {
    static var title: LocalizedStringResource = "Check My Streak"
    static var description = IntentDescription("Hear your current Spell practice streak.")
    // Answer straight from the App Group — no app launch needed.
    static var openAppWhenRun: Bool = false

    func perform() async throws -> some IntentResult & ProvidesDialog {
        // SAME App Group blob F3 writes: `group.net.spellgame.app` / `widget_state_v1`.
        let state = WidgetShared.read() ?? .empty
        let message = IntentStrings.streakDialog(
            streak: state.streak,
            practicedToday: state.practicedToday
        )
        return .result(dialog: IntentDialog(stringLiteral: message))
    }
}

// MARK: - Shortcuts (phrases localized en + es)

/// Registers the three intents as App Shortcuts. Phrases are bilingual — both the
/// English and Spanish spoken forms are listed for each shortcut, so a user in
/// either language can invoke it (App Shortcut phrases are indexed by the system
/// at install time, so both must be compiled in). `\(.applicationName)` is
/// required in every phrase.
@available(iOS 16.0, *)
struct SpellAppShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: PracticeMissedWordsIntent(),
            phrases: [
                "Practice my missed words in \(.applicationName)",
                "Practice missed words in \(.applicationName)",
                "Review my missed words in \(.applicationName)",
                "Practica mis palabras falladas en \(.applicationName)",
                "Repasa mis palabras falladas en \(.applicationName)",
            ],
            shortTitle: "Practice Missed Words",
            systemImageName: "arrow.uturn.backward.circle"
        )
        AppShortcut(
            intent: StartDailyChallengeIntent(),
            phrases: [
                "Start my Daily Challenge in \(.applicationName)",
                "Start the Daily Challenge in \(.applicationName)",
                "Comienza el Reto diario en \(.applicationName)",
                "Empieza el Reto diario en \(.applicationName)",
            ],
            shortTitle: "Start Daily Challenge",
            systemImageName: "calendar"
        )
        AppShortcut(
            intent: CheckMyStreakIntent(),
            phrases: [
                "Check my streak in \(.applicationName)",
                "What's my streak in \(.applicationName)",
                "Consulta mi racha en \(.applicationName)",
                "Cuál es mi racha en \(.applicationName)",
            ],
            shortTitle: "Check My Streak",
            systemImageName: "flame"
        )
    }
}
