import WidgetKit
import SwiftUI

/// F3 entry point — the two home-screen widgets (Streak + Daily Challenge). Both
/// read the App Group snapshot the app writes via NativeLanguageKit
/// `syncWidgetState`; the extension holds no game logic.
@main
struct SpellWidgetBundle: WidgetBundle {
    var body: some Widget {
        StreakWidget()
        DailyWidget()
    }
}
