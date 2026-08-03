// CC-IOS-SURFACES (BD-1) — the snapshot writer + App Intents surface.
//
// D2 (decided): state flows ONE way. The app calls writeWidgetSnapshot on
// session end; the widget extension only reads the App Group file. I3:
// nothing here reads back, nothing in the extension writes.
import Capacitor
import CoreSpotlight
import Foundation
import WidgetKit

extension NativeLanguageKitPlugin {

    @objc func writeWidgetSnapshot(_ call: CAPPluginCall) {
        guard let json = call.getString("json"), !json.isEmpty else {
            call.reject("empty snapshot")
            return
        }
        guard let dir = FileManager.default.containerURL(
            forSecurityApplicationGroupIdentifier: "group.net.spellgame.app") else {
            call.reject("no app group container")
            return
        }
        do {
            try json.data(using: .utf8)?.write(
                to: dir.appendingPathComponent("widget-snapshot.json"), options: .atomic)
            if #available(iOS 14.0, *) {
                WidgetCenter.shared.reloadAllTimelines()
            }
            call.resolve()
        } catch {
            call.reject("snapshot write failed: \(error.localizedDescription)")
        }
    }

    /// Spotlight (feature 5): the app passes the platform- and edition-
    /// filtered registry at boot; we donate searchable entities. Read-only
    /// over the registry — filtering happened where the registry lives.
    @objc func donateSpotlight(_ call: CAPPluginCall) {
        guard let modes = call.getArray("modes") as? [[String: String]] else {
            call.reject("modes missing")
            return
        }
        let items: [CSSearchableItem] = modes.compactMap { m in
            guard let id = m["id"], let name = m["name"] else { return nil }
            let attrs = CSSearchableItemAttributeSet(contentType: .content)
            attrs.title = name
            attrs.contentDescription = m["desc"] ?? ""
            let item = CSSearchableItem(
                uniqueIdentifier: "mode-\(id)",
                domainIdentifier: "net.spellgame.modes",
                attributeSet: attrs)
            return item
        }
        CSSearchableIndex.default().indexSearchableItems(items) { _ in }
        call.resolve()
    }
}
