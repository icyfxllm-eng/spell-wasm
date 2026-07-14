import SwiftUI
import WidgetKit

/// Deep-link targets (custom scheme `spellgame://`, registered in the App's
/// Info.plist CFBundleURLTypes). Tapping a widget opens the app via this scheme;
/// full in-app routing to the exact surface is the F4 follow-up (see the PR).
enum WidgetLink {
    static let streak = URL(string: "spellgame://streak")!
    static let daily = URL(string: "spellgame://daily")!
}

extension View {
    /// iOS 17 requires an explicit widget container background; older systems
    /// take a plain background. One modifier so every widget view stays uniform.
    @ViewBuilder
    func widgetContainerBackground(_ style: some ShapeStyle) -> some View {
        if #available(iOS 17.0, *) {
            containerBackground(style, for: .widget)
        } else {
            background(style)
        }
    }
}

/// Shared warm gradient (flame-adjacent) used behind both widgets.
enum WidgetPalette {
    static let flame = LinearGradient(
        colors: [Color(red: 1.0, green: 0.42, blue: 0.21),
                 Color(red: 0.98, green: 0.24, blue: 0.34)],
        startPoint: .topLeading, endPoint: .bottomTrailing
    )
    static let calm = LinearGradient(
        colors: [Color(red: 0.16, green: 0.20, blue: 0.34),
                 Color(red: 0.10, green: 0.12, blue: 0.22)],
        startPoint: .topLeading, endPoint: .bottomTrailing
    )
}
