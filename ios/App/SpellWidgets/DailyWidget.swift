import WidgetKit
import SwiftUI

/// (b) Daily Challenge widget — today's status (not started / in progress /
/// done) + the language. Reads the App Group snapshot; the effective status is
/// derived so a midnight rollover reads as "not started" without a fresh write.
struct DailyEntry: TimelineEntry {
    let date: Date
    let state: SpellWidgetState
}

struct DailyProvider: TimelineProvider {
    private func currentEntry() -> DailyEntry {
        DailyEntry(date: Date(), state: WidgetShared.read() ?? .empty)
    }

    func placeholder(in context: Context) -> DailyEntry {
        DailyEntry(date: Date(), state: .empty)
    }

    func getSnapshot(in context: Context, completion: @escaping (DailyEntry) -> Void) {
        completion(currentEntry())
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<DailyEntry>) -> Void) {
        let next = Calendar.current.nextDate(
            after: Date(),
            matching: DateComponents(hour: 0, minute: 1),
            matchingPolicy: .nextTime
        ) ?? Date().addingTimeInterval(6 * 3600)
        completion(Timeline(entries: [currentEntry()], policy: .after(next)))
    }
}

private struct DailyStatusStyle {
    let label: String
    let symbol: String
    let tint: Color
}

struct DailyWidgetView: View {
    @Environment(\.widgetFamily) private var family
    let entry: DailyEntry

    private var s: SpellWidgetState { entry.state }
    private var lang: String { s.language }

    private var style: DailyStatusStyle {
        switch s.effectiveDailyStatus {
        case "done":
            return DailyStatusStyle(label: WidgetStrings.t("daily.done", lang),
                                    symbol: "checkmark.seal.fill",
                                    tint: Color(red: 0.30, green: 0.80, blue: 0.55))
        case "in_progress":
            return DailyStatusStyle(label: WidgetStrings.t("daily.inProgress", lang),
                                    symbol: "hourglass",
                                    tint: Color(red: 0.98, green: 0.75, blue: 0.30))
        default:
            return DailyStatusStyle(label: WidgetStrings.t("daily.notStarted", lang),
                                    symbol: "circle.dashed",
                                    tint: .white.opacity(0.85))
        }
    }

    var body: some View {
        Group {
            switch family {
            case .systemMedium: medium
            default: small
            }
        }
        .widgetURL(WidgetLink.daily)
        .widgetContainerBackground(WidgetPalette.calm)
    }

    private var small: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 4) {
                Text("🗓").font(.title3)
                Text(WidgetStrings.t("daily.title", lang))
                    .font(.caption).fontWeight(.semibold)
                    .foregroundColor(.white.opacity(0.9))
                    .lineLimit(1).minimumScaleFactor(0.7)
            }
            Spacer(minLength: 0)
            statusBadge
            Text(lang.uppercased())
                .font(.caption2).foregroundColor(.white.opacity(0.7))
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }

    private var medium: some View {
        HStack(spacing: 16) {
            Image(systemName: style.symbol)
                .font(.system(size: 40))
                .foregroundColor(style.tint)
            VStack(alignment: .leading, spacing: 8) {
                Text(WidgetStrings.t("daily.title", lang))
                    .font(.headline).foregroundColor(.white)
                statusBadge
                Text("\(WidgetStrings.t("daily.tapToPlay", lang)) · \(lang.uppercased())")
                    .font(.caption).foregroundColor(.white.opacity(0.75))
                    .lineLimit(1).minimumScaleFactor(0.7)
            }
            Spacer(minLength: 0)
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }

    private var statusBadge: some View {
        HStack(spacing: 5) {
            Image(systemName: style.symbol).foregroundColor(style.tint)
            Text(style.label)
                .font(.subheadline).fontWeight(.semibold)
                .foregroundColor(.white)
                .lineLimit(1).minimumScaleFactor(0.7)
        }
        .padding(.horizontal, 10).padding(.vertical, 6)
        .background(Capsule().fill(Color.white.opacity(0.14)))
    }
}

struct DailyWidget: Widget {
    let kind = "SpellDailyWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: DailyProvider()) { entry in
            DailyWidgetView(entry: entry)
        }
        .configurationDisplayName(WidgetStrings.t("widget.dailyName", "en"))
        .description(WidgetStrings.t("widget.dailyDesc", "en"))
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}
