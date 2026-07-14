import Foundation
import UIKit
import WebKit
import Capacitor

/// F4 — the ONE native entry that hands a `spellgame://` URL to the webview's JS
/// deep-link shim (`window.SpellHandleUrl`, index.html), which forwards it to the
/// Rust router (`window.SpellRouter.open`) or buffers it for a cold launch. Both
/// the URL opener (`AppDelegate.application(_:open:)`, for F3's widgets) and the
/// F4 App Intents call this, so intents and widgets share exactly one path — no
/// divergent entry plumbing.
enum SpellURLRouter {
    /// Deliver `url` to the webview, retrying until the JS shim is present (the
    /// page/WASM has finished loading). Safe to call from a cold launch: the shim
    /// buffers the URL until the Rust router installs. Capped so a launch that
    /// never loads can't spin forever. No-op for non-`spellgame` URLs.
    static func route(_ url: URL, attempt: Int = 0) {
        guard url.scheme?.lowercased() == "spellgame" else { return }
        guard attempt < 30 else { return } // ~6s ceiling at 0.2s spacing.
        DispatchQueue.main.async {
            guard let webView = resolveWebView() else {
                retry(url, attempt)
                return
            }
            let literal = jsStringLiteral(url.absoluteString)
            let js = "(function(){ if (window.SpellHandleUrl) { window.SpellHandleUrl(\(literal)); return true; } return false; })()"
            webView.evaluateJavaScript(js) { result, _ in
                let delivered = (result as? Bool) ?? false
                if !delivered { retry(url, attempt) }
            }
        }
    }

    /// Convenience for the App Intents: route by host name (`daily` | `missed` |
    /// `streak`). Returns false only if the URL couldn't be formed.
    @discardableResult
    static func route(host: String) -> Bool {
        guard let url = URL(string: "spellgame://\(host)") else { return false }
        route(url)
        return true
    }

    private static func retry(_ url: URL, _ attempt: Int) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            route(url, attempt: attempt + 1)
        }
    }

    /// The Capacitor bridge's WKWebView from the active key window, if up yet.
    private static func resolveWebView() -> WKWebView? {
        let root = UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .flatMap { $0.windows }
            .first(where: { $0.isKeyWindow })?.rootViewController
        if let bridgeVC = root as? CAPBridgeViewController {
            return bridgeVC.webView
        }
        return root?.children.compactMap { ($0 as? CAPBridgeViewController)?.webView }.first
    }

    /// A safe, quoted JS string literal for `s` (escapes quotes, backslashes, …).
    private static func jsStringLiteral(_ s: String) -> String {
        if let data = try? JSONSerialization.data(withJSONObject: [s]),
           let json = String(data: data, encoding: .utf8) {
            // json is `["…escaped…"]`; strip the array brackets to get the literal.
            return String(json.dropFirst().dropLast())
        }
        return "\"\""
    }
}
