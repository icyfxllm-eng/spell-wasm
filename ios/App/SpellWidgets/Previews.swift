// Acceptance #1 — WidgetKit previews for the four fixture snapshots.
import SwiftUI
import WidgetKit

struct DailyPreviews: PreviewProvider {
    static var previews: some View {
        Group {
            DailyView(snap: .freshInstall).previewContext(WidgetPreviewContext(family: .systemSmall))
                .previewDisplayName("fresh install")
            DailyView(snap: .streak12Done).previewContext(WidgetPreviewContext(family: .systemMedium))
                .previewDisplayName("streak 12 done")
            DailyView(snap: .streak12NotDone).previewContext(WidgetPreviewContext(family: .systemSmall))
                .previewDisplayName("streak 12 not done")
            StreakView(snap: .shields3of5).previewContext(WidgetPreviewContext(family: .accessoryRectangular))
                .previewDisplayName("shields 3/5")
        }
    }
}
