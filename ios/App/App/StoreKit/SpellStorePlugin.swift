import Foundation
import StoreKit
import Capacitor

/// CC-LAUNCH-EN F4: the iOS purchase adapter for the pure Rust entitlement core
/// (src/entitlements.rs).
///
/// It gathers signals and makes no decisions. It reports which product ids are
/// owned, loads products, and runs a purchase or restore when explicitly asked.
/// What any of that unlocks is answered in Rust, by `resolve_entitlements`.
/// Product ids come IN from Rust (`PRODUCT_IDS`); none are written here.
///
/// It uses StoreKit 2 only, with no third-party SDK (a child's device; privacy
/// labels). Every transaction is verified on device by StoreKit's own signature
/// check, and an `.unverified` result reads as not owned. The plugin makes no
/// network call of its own and sends no receipt anywhere.
@objc(SpellStorePlugin)
public class SpellStorePlugin: CAPPlugin, CAPBridgedPlugin {
    public let identifier = "SpellStorePlugin"
    public let jsName = "SpellStore"
    public let pluginMethods: [CAPPluginMethod] = [
        CAPPluginMethod(name: "products", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "purchase", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "restore", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "entitlements", returnType: CAPPluginReturnPromise),
    ]

    private var updatesTask: Task<Void, Never>?

    /// Starts the `Transaction.updates` listener as soon as the plugin loads.
    /// Ask to Buy approvals, Family Sharing changes, refunds and purchases made
    /// on another device all arrive here, so they reach the web layer without a
    /// relaunch (A4.5).
    override public func load() {
        updatesTask = Task.detached { [weak self] in
            for await result in Transaction.updates {
                if case .verified(let t) = result {
                    await t.finish()
                }
                await self?.pushEntitlements()
            }
        }
    }

    deinit { updatesTask?.cancel() }

    /// The verified, unrevoked product ids the player owns right now. Only
    /// `Transaction.currentEntitlements` is read, and it is never cached here, so
    /// a refund is gone on the next read (I4.6, D15).
    static func ownedIds() async -> [String] {
        var ids: [String] = []
        for await result in Transaction.currentEntitlements {
            guard case .verified(let t) = result else { continue }
            if t.revocationDate == nil { ids.append(t.productID) }
        }
        return ids
    }

    private func pushEntitlements() async {
        let ids = await Self.ownedIds()
        notifyListeners("entitlements", data: ["ids": ids])
    }

    @objc func entitlements(_ call: CAPPluginCall) {
        Task { call.resolve(["ids": await Self.ownedIds()]) }
    }

    /// Loads products by id. Prices are StoreKit's localized `displayPrice` (I4.7).
    @objc func products(_ call: CAPPluginCall) {
        let ids = call.getArray("ids", String.self) ?? []
        Task {
            do {
                let products = try await Product.products(for: ids)
                let out: [[String: Any]] = products.map { p in
                    ["id": p.id, "displayName": p.displayName, "displayPrice": p.displayPrice]
                }
                call.resolve(["products": out])
            } catch {
                call.reject("Could not load products: \(error.localizedDescription)", "PRODUCTS_FAILED")
            }
        }
    }

    /// Buys one product. The ordinary non-success outcomes (cancelled, Ask to Buy
    /// pending) resolve with a status instead of rejecting, so the caller never
    /// shows a cancel as an error. Every outcome returns the fresh owned set, so
    /// an interrupted purchase leaves no partial state (A4.7).
    @objc func purchase(_ call: CAPPluginCall) {
        guard let id = call.getString("id") else {
            call.reject("id is required", "BAD_ARGS")
            return
        }
        Task {
            do {
                guard let product = try await Product.products(for: [id]).first else {
                    call.reject("Product \(id) not found", "PRODUCT_NOT_FOUND")
                    return
                }
                var status = "unknown"
                switch try await product.purchase() {
                case .success(let verification):
                    if case .verified(let t) = verification {
                        await t.finish()
                        status = "success"
                    } else {
                        status = "unverified"
                    }
                case .userCancelled:
                    status = "cancelled"
                case .pending:
                    status = "pending"
                @unknown default:
                    status = "unknown"
                }
                call.resolve(["status": status, "ids": await Self.ownedIds()])
            } catch {
                call.reject("Purchase failed: \(error.localizedDescription)", "PURCHASE_FAILED")
            }
        }
    }

    /// The Restore Purchases button (C5.9). It calls `AppStore.sync()`, which may
    /// ask for an App Store password, so it runs only from an explicit tap.
    @objc func restore(_ call: CAPPluginCall) {
        Task {
            do {
                try await AppStore.sync()
                call.resolve(["ids": await Self.ownedIds()])
            } catch {
                call.reject("Restore failed: \(error.localizedDescription)", "RESTORE_FAILED")
            }
        }
    }
}
