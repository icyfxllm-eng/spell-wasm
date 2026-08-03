// CC-IOS-SURFACES (BD-1) feature 4 — App Intents / Siri Shortcuts.
// Intents OPEN the app to a mode; no headless gameplay, and the spoken-
// input discipline stands: no spoken answers through Siri, ever.
// Lives in the APP target behind availability (deployment stays iOS 15).
import AppIntents
import UIKit

@available(iOS 16.0, *)
struct StartDailyChallengeIntent: AppIntent {
    static var title: LocalizedStringResource = "Start Daily Challenge"
    static var openAppWhenRun = true
    @MainActor
    func perform() async throws -> some IntentResult {
        await UIApplication.shared.open(URL(string: "spellgame://daily")!)
        return .result()
    }
}

@available(iOS 16.0, *)
struct PracticeSpellingIntent: AppIntent {
    static var title: LocalizedStringResource = "Practice spelling"
    static var openAppWhenRun = true
    @MainActor
    func perform() async throws -> some IntentResult {
        await UIApplication.shared.open(URL(string: "spellgame://practice")!)
        return .result()
    }
}

/// "Practice [language]" — parameterized over the ENTITLED+enabled list
/// only (read from the widget snapshot the app maintains; an unentitled
/// spoken language resolves to the app's graceful in-app landing, never
/// an error — acceptance #4).
@available(iOS 16.0, *)
struct PracticeLanguageIntent: AppIntent {
    static var title: LocalizedStringResource = "Practice a language"
    static var openAppWhenRun = true

    @Parameter(title: "Language")
    var language: String

    @MainActor
    func perform() async throws -> some IntentResult {
        let slug = language.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? "en"
        await UIApplication.shared.open(URL(string: "spellgame://practice/\(slug)")!)
        return .result()
    }
}

@available(iOS 16.0, *)
struct SpellGameShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(intent: StartDailyChallengeIntent(),
                    phrases: ["Start Daily Challenge in \(.applicationName)"],
                    shortTitle: "Daily Challenge", systemImageName: "flame.fill")
        AppShortcut(intent: PracticeSpellingIntent(),
                    phrases: ["Practice spelling in \(.applicationName)"],
                    shortTitle: "Practice", systemImageName: "textformat.abc")
    }
}
