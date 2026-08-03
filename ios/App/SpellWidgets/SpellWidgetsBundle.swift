// CC-IOS-SURFACES (BD-1). Two widgets ship; the Climb Live Activity is
// CUT per BD-D1 (Eric, 2026-08-02) — its attributes are stubbed behind a
// disabled compile flag (season-2-config-stub precedent) and nothing else
// of it exists.
import SwiftUI
import WidgetKit

@main
struct SpellWidgetsBundle: WidgetBundle {
    var body: some Widget {
        DailyChallengeWidget()
        StreakShieldsWidget()
    }
}

#if SPELL_LIVE_ACTIVITY // disabled: BD-D1 cut for v1
import ActivityKit
struct ClimbActivityAttributes: ActivityAttributes {
    struct ContentState: Codable, Hashable {
        var segment: Int
        var shieldForging: Bool
        var progress: Double
    }
    var runId: String
}
#endif

struct SnapshotEntry: TimelineEntry {
    let date: Date
    let snap: WidgetSnapshot
}

struct SnapshotProvider: TimelineProvider {
    func placeholder(in context: Context) -> SnapshotEntry {
        SnapshotEntry(date: .now, snap: .streak12NotDone)
    }
    func getSnapshot(in context: Context, completion: @escaping (SnapshotEntry) -> Void) {
        completion(SnapshotEntry(date: .now, snap: context.isPreview ? .streak12NotDone : .load()))
    }
    func getTimeline(in context: Context, completion: @escaping (Timeline<SnapshotEntry>) -> Void) {
        // One entry; the app reloads timelines when it writes the snapshot
        // (I3: extensions never write, never compute schedules of their own).
        completion(Timeline(entries: [SnapshotEntry(date: .now, snap: .load())], policy: .never))
    }
}
