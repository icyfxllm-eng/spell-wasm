// CC-GAME-SERVER G-D11 — the online session token's home: the iOS Keychain.
//
// The room server knows a player only by an anonymous random token. This is
// the one place it is kept on the device: never localStorage, never a URL,
// never iCloud. Three methods, one generic-password item per key, in the
// app's own default access group (no extra entitlement). The bridge only
// stores and returns what it is given; it decides nothing.
import Capacitor
import Foundation
import Security

extension NativeLanguageKitPlugin {

    private static let keychainService = "net.spellgame.app.online"

    private func keychainQuery(_ key: String) -> [String: Any] {
        return [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: NativeLanguageKitPlugin.keychainService,
            kSecAttrAccount as String: key,
        ]
    }

    @objc func keychainGet(_ call: CAPPluginCall) {
        guard let key = call.getString("key"), !key.isEmpty else {
            call.reject("key missing")
            return
        }
        var query = keychainQuery(key)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var out: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &out)
        if status == errSecItemNotFound {
            call.resolve([:])
            return
        }
        guard status == errSecSuccess, let data = out as? Data, let value = String(data: data, encoding: .utf8) else {
            call.reject("keychain read failed: \(status)")
            return
        }
        call.resolve(["value": value])
    }

    @objc func keychainSet(_ call: CAPPluginCall) {
        guard let key = call.getString("key"), !key.isEmpty,
              let value = call.getString("value"), let data = value.data(using: .utf8) else {
            call.reject("key or value missing")
            return
        }
        let query = keychainQuery(key)
        // This device only, readable once it has been unlocked after boot; no iCloud sync.
        let attrs: [String: Any] = [
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
        ]
        var status = SecItemUpdate(query as CFDictionary, attrs as CFDictionary)
        if status == errSecItemNotFound {
            var add = query
            add.merge(attrs) { _, new in new }
            status = SecItemAdd(add as CFDictionary, nil)
        }
        guard status == errSecSuccess else {
            call.reject("keychain write failed: \(status)")
            return
        }
        call.resolve()
    }

    @objc func keychainDelete(_ call: CAPPluginCall) {
        guard let key = call.getString("key"), !key.isEmpty else {
            call.reject("key missing")
            return
        }
        let status = SecItemDelete(keychainQuery(key) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            call.reject("keychain delete failed: \(status)")
            return
        }
        call.resolve()
    }
}
