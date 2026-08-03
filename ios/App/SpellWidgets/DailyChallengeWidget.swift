// Daily Challenge widget (small + medium). Kid-Mode-safe BY CONSTRUCTION:
// language name, streak number, orb art — no words, no definitions (I2:
// a lock screen is a public surface). Zero purchase surfaces (I1).
import SwiftUI
import WidgetKit

struct DailyChallengeWidget: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "SpellDaily", provider: SnapshotProvider()) { entry in
            DailyView(snap: entry.snap)
                .widgetURL(URL(string: "spellgame://daily"))
        }
        .configurationDisplayName(String(localized: "widget.daily.name"))
        .description(String(localized: "widget.daily.desc"))
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}

struct DailyView: View {
    let snap: WidgetSnapshot
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Circle()
                    .fill(snap.dailyDone ? Color.green.opacity(0.85) : Color.orange.opacity(0.85))
                    .frame(width: 14, height: 14)
                Text(snap.dailyDone
                     ? String(localized: "widget.daily.done")
                     : String(localized: "widget.daily.notyet"))
                    .font(.caption).bold()
            }
            if !snap.dailyLang.isEmpty {
                Text(snap.dailyLang).font(.headline).lineLimit(1)
            }
            Spacer(minLength: 0)
            HStack(spacing: 4) {
                Image(systemName: "flame.fill").imageScale(.small)
                Text("\(snap.streak)").font(.title3).bold().monospacedDigit()
                Text(String(localized: "widget.streak.unit")).font(.caption2)
            }
        }
        .padding(12)
        .containerBackgroundCompat()
    }
}

extension View {
    @ViewBuilder
    func containerBackgroundCompat() -> some View {
        if #available(iOSApplicationExtension 17.0, *) {
            self.containerBackground(for: .widget) { Color(red: 0.05, green: 0.07, blue: 0.13) }
        } else {
            self.background(Color(red: 0.05, green: 0.07, blue: 0.13))
        }
    }
}
