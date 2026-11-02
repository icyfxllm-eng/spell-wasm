// CC-GAME-SERVER G-D11 — the online session token's home: the iOS Keychain.
//
// The room server knows a player only by an anonymous random token. This is
// the one place it is kept on the device: never localStorage, never a URL,
// never iCloud. Three methods, one generic-password item per key, in the
// app's own default access group (no extra entitlement). The bridge only
// stores and returns what it is given; it decides nothing.
import AuthenticationServices
import Capacitor
import CryptoKit
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


// CC-ACCOUNTS Phase 2: Sign in with Apple, first-party (no SDK). The bridge only runs Apple's sheet and
// hands back what Apple returned; the identity service decides whether to believe it.
//
// It stays OFF until a person turns it on: the capability needs the "Sign in with Apple" entitlement and a
// provisioning profile that carries it, and a build without both would fail at signing. Set the Info.plist
// key SpellSIWAEnabled to YES in the same change that adds the entitlement. Until then `siwaAvailable`
// answers false and the sheet offers email only.
private final class SIWACoordinator: NSObject, ASAuthorizationControllerDelegate, ASAuthorizationControllerPresentationContextProviding {
    static var live: SIWACoordinator?
    private let call: CAPPluginCall
    private let rawNonce: String
    private let window: UIWindow?

    init(call: CAPPluginCall, rawNonce: String, window: UIWindow?) {
        self.call = call
        self.rawNonce = rawNonce
        self.window = window
    }

    func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
        return window ?? ASPresentationAnchor()
    }

    func authorizationController(controller: ASAuthorizationController, didCompleteWithAuthorization authorization: ASAuthorization) {
        defer { SIWACoordinator.live = nil }
        guard let cred = authorization.credential as? ASAuthorizationAppleIDCredential,
              let tokenData = cred.identityToken,
              let token = String(data: tokenData, encoding: .utf8) else {
            call.reject("no identity token")
            return
        }
        var out: [String: Any] = ["identity_token": token, "nonce": rawNonce]
        if let codeData = cred.authorizationCode, let code = String(data: codeData, encoding: .utf8) {
            out["authorization_code"] = code
        }
        call.resolve(out)
    }

    func authorizationController(controller: ASAuthorizationController, didCompleteWithError error: Error) {
        defer { SIWACoordinator.live = nil }
        call.reject("sign in with apple failed: \((error as NSError).code)")
    }
}

extension NativeLanguageKitPlugin {

    @objc func siwaAvailable(_ call: CAPPluginCall) {
        let on = (Bundle.main.object(forInfoDictionaryKey: "SpellSIWAEnabled") as? Bool) ?? false
        call.resolve(["available": on])
    }

    @objc func signInWithApple(_ call: CAPPluginCall) {
        guard (Bundle.main.object(forInfoDictionaryKey: "SpellSIWAEnabled") as? Bool) == true else {
            call.reject("not configured")
            return
        }
        var bytes = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else {
            call.reject("no randomness")
            return
        }
        let raw = bytes.map { String(format: "%02x", $0) }.joined()
        let hashed = SHA256.hash(data: Data(raw.utf8)).map { String(format: "%02x", $0) }.joined()
        DispatchQueue.main.async {
            let request = ASAuthorizationAppleIDProvider().createRequest()
            request.requestedScopes = [.email]
            request.nonce = hashed
            let coordinator = SIWACoordinator(call: call, rawNonce: raw, window: self.bridge?.viewController?.view.window)
            SIWACoordinator.live = coordinator
            let controller = ASAuthorizationController(authorizationRequests: [request])
            controller.delegate = coordinator
            controller.presentationContextProvider = coordinator
            controller.performRequests()
        }
    }
}
