import WidgetKit
import SwiftUI

/// (a) Streak widget — flame + current consecutive-day streak + a "practiced
/// today" check. Reads the App Group snapshot written by the app; never computes
/// streak rules itself.
struct StreakEntry: TimelineEntry {
    let date: Date
    let state: SpellWidgetState
}

struct StreakProvider: TimelineProvider {
    private func currentEntry() -> StreakEntry {
        StreakEntry(date: Date(), state: WidgetShared.read() ?? .empty)
    }

    func placeholder(in context: Context) -> StreakEntry {
        StreakEntry(date: Date(), state: .empty)
    }

    func getSnapshot(in context: Context, completion: @escaping (StreakEntry) -> Void) {
        completion(currentEntry())
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<StreakEntry>) -> Void) {
        // One entry; refresh just after local midnight so "practiced today"
        // clears on a new day even without an app launch. The app also nudges
        // WidgetCenter on every sync.
        let next = Calendar.current.nextDate(
            after: Date(),
            matching: DateComponents(hour: 0, minute: 1),
            matchingPolicy: .nextTime
        ) ?? Date().addingTimeInterval(6 * 3600)
        completion(Timeline(entries: [currentEntry()], policy: .after(next)))
    }
}

struct StreakWidgetView: View {
    @Environment(\.widgetFamily) private var family
    let entry: StreakEntry

    private var s: SpellWidgetState { entry.state }
    private var lang: String { s.language }

    var body: some View {
        Group {
            switch family {
            case .systemMedium: medium
            default: small
            }
        }
        .widgetURL(WidgetLink.streak)
        .widgetContainerBackground(WidgetPalette.flame)
    }

    private var small: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 4) {
                Text("🔥").font(.title3)
                Text(WidgetStrings.t("streak.title", lang))
                    .font(.caption).fontWeight(.semibold)
                    .foregroundColor(.white.opacity(0.9))
            }
            Spacer(minLength: 0)
            Text("\(s.streak)")
                .font(.system(size: 44, weight: .heavy, design: .rounded))
                .foregroundColor(.white)
                .minimumScaleFactor(0.6).lineLimit(1)
            Text(streakSubtitle)
                .font(.caption2).foregroundColor(.white.opacity(0.85))
                .lineLimit(1).minimumScaleFactor(0.7)
            practicedRow
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }

    private var medium: some View {
        HStack(spacing: 16) {
            VStack(spacing: 2) {
                Text("🔥").font(.system(size: 40))
                Text("\(s.streak)")
                    .font(.system(size: 40, weight: .heavy, design: .rounded))
                    .foregroundColor(.white)
                    .minimumScaleFactor(0.6).lineLimit(1)
            }
            VStack(alignment: .leading, spacing: 8) {
                Text(WidgetStrings.t("streak.title", lang))
                    .font(.headline).foregroundColor(.white)
                Text(streakSubtitle)
                    .font(.subheadline).foregroundColor(.white.opacity(0.9))
                    .lineLimit(2).minimumScaleFactor(0.7)
                practicedRow
                if s.bestStreak > 0 {
                    Text("\(WidgetStrings.t("streak.best", lang)): \(s.bestStreak)")
                        .font(.caption).foregroundColor(.white.opacity(0.8))
                }
            }
            Spacer(minLength: 0)
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }

    private var streakSubtitle: String {
        s.streak == 0
            ? WidgetStrings.t("streak.start", lang)
            : WidgetStrings.t("streak.days", lang)
    }

    private var practicedRow: some View {
        HStack(spacing: 4) {
            Image(systemName: s.practicedToday ? "checkmark.circle.fill" : "circle")
                .foregroundColor(s.practicedToday ? .white : .white.opacity(0.7))
            Text(s.practicedToday
                    ? WidgetStrings.t("streak.practiced", lang)
                    : WidgetStrings.t("streak.notYet", lang))
                .font(.caption2).foregroundColor(.white.opacity(0.9))
                .lineLimit(1).minimumScaleFactor(0.7)
        }
    }
}

struct StreakWidget: Widget {
    let kind = "SpellStreakWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: StreakProvider()) { entry in
            StreakWidgetView(entry: entry)
        }
        .configurationDisplayName(WidgetStrings.t("widget.streakName", "en"))
        .description(WidgetStrings.t("widget.streakDesc", "en"))
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}
