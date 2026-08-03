// Streak & shields lock-screen widget (accessoryCircular + rectangular).
// Read-only shield state; shield art is SF-symbol composition — no new
// drawn strings (spec feature 2), no purchase surfaces (I1).
import SwiftUI
import WidgetKit

struct StreakShieldsWidget: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "SpellStreak", provider: SnapshotProvider()) { entry in
            StreakView(snap: entry.snap)
                .widgetURL(URL(string: "spellgame://daily"))
        }
        .configurationDisplayName(String(localized: "widget.streak.name"))
        .description(String(localized: "widget.streak.desc"))
        .supportedFamilies([.accessoryCircular, .accessoryRectangular])
    }
}

struct StreakView: View {
    @Environment(\.widgetFamily) var family
    let snap: WidgetSnapshot
    var body: some View {
        switch family {
        case .accessoryRectangular:
            HStack(spacing: 8) {
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 3) {
                        Image(systemName: "flame.fill").imageScale(.small)
                        Text("\(snap.streak)").bold().monospacedDigit()
                        Text(String(localized: "widget.streak.unit")).font(.caption2)
                    }
                    HStack(spacing: 2) {
                        ForEach(0..<snap.shieldsTotal, id: \.self) { i in
                            Image(systemName: i < snap.shieldsEarned ? "shield.fill" : "shield")
                                .imageScale(.small)
                        }
                    }
                }
                Spacer(minLength: 0)
            }
            .containerBackgroundCompat()
        default:
            ZStack {
                AccessoryWidgetBackgroundCompat()
                VStack(spacing: 0) {
                    Image(systemName: "flame.fill").imageScale(.small)
                    Text("\(snap.streak)").font(.headline).bold().monospacedDigit()
                }
            }
            .containerBackgroundCompat()
        }
    }
}

struct AccessoryWidgetBackgroundCompat: View {
    var body: some View {
        if #available(iOSApplicationExtension 16.0, *) {
            AccessoryWidgetBackground()
        } else {
            Color.clear
        }
    }
}
