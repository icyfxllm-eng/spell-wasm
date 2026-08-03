// CC-OFFLINE-PACKS (BD-2) — the native half of the pack pipeline.
//
// Per-file downloads (the manifest is the unit of truth): each file is
// fetched, SHA-256-verified against the SIGNED manifest, and stored in
// staging; activation is one atomic rename. A pack is verified-active or
// absent — nothing in between (I2). Resume-after-kill is structural:
// relaunch re-scans staging for missing files and fetches only those.
import Capacitor
import CryptoKit
import Foundation

extension NativeLanguageKitPlugin {

    private static var packsRoot: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return base.appendingPathComponent("packs", isDirectory: true)
    }

    /// Verify the manifest's Ed25519 signature against the embedded public
    /// key (I4: a bad signature quarantines the whole pack, one honest
    /// error, before a single audio byte is trusted).
    @objc func packVerifyManifest(_ call: CAPPluginCall) {
        guard let manifest = call.getString("manifest"),
              let sigB64 = call.getString("sig"),
              let pubB64 = call.getString("pubKey"),
              let sig = Data(base64Encoded: sigB64),
              let pub = Data(base64Encoded: pubB64) else {
            call.reject("bad args")
            return
        }
        do {
            let key = try Curve25519.Signing.PublicKey(rawRepresentation: pub)
            let ok = key.isValidSignature(sig, for: Data(manifest.utf8))
            call.resolve(["valid": ok])
        } catch {
            call.resolve(["valid": false])
        }
    }

    /// Fetch ONE pack file into staging, verifying its sha256. Wi-Fi-only
    /// discipline is enforced by the CALLER (the Rust manager holds the
    /// cellular toggle); this method just moves verified bytes.
    @objc func packFetch(_ call: CAPPluginCall) {
        guard let urlStr = call.getString("url"), let url = URL(string: urlStr),
              let lang = call.getString("lang"),
              let name = call.getString("name"),
              let sha = call.getString("sha256") else {
            call.reject("bad args")
            return
        }
        let staging = Self.packsRoot.appendingPathComponent("staging-\(lang)", isDirectory: true)
        let dest = staging.appendingPathComponent(name)
        if FileManager.default.fileExists(atPath: dest.path) {
            call.resolve(["state": "present"])
            return
        }
        URLSession.shared.dataTask(with: url) { data, _, err in
            guard let data = data, err == nil else {
                call.reject("fetch failed: \(err?.localizedDescription ?? "no data")")
                return
            }
            let digest = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
            guard digest == sha else {
                call.reject("hash mismatch for \(name)")
                return
            }
            do {
                try FileManager.default.createDirectory(at: staging, withIntermediateDirectories: true)
                try data.write(to: dest, options: .atomic)
                call.resolve(["state": "fetched"])
            } catch {
                call.reject("write failed: \(error.localizedDescription)")
            }
        }.resume()
    }

    /// Which manifest files are still missing from staging — the resume
    /// scan. Returns names only; the Rust side fetches the gaps.
    @objc func packMissing(_ call: CAPPluginCall) {
        guard let lang = call.getString("lang"),
              let names = call.getArray("names") as? [String] else {
            call.reject("bad args")
            return
        }
        let staging = Self.packsRoot.appendingPathComponent("staging-\(lang)", isDirectory: true)
        let missing = names.filter { !FileManager.default.fileExists(atPath: staging.appendingPathComponent($0).path) }
        call.resolve(["missing": missing])
    }

    /// Atomic activation: staging -> active in one rename (I2). The old
    /// active pack (if any) is removed first; a crash between the two
    /// leaves us verified-absent, never partially-active.
    @objc func packActivate(_ call: CAPPluginCall) {
        guard let lang = call.getString("lang") else { call.reject("bad args"); return }
        let fm = FileManager.default
        let staging = Self.packsRoot.appendingPathComponent("staging-\(lang)", isDirectory: true)
        let active = Self.packsRoot.appendingPathComponent(lang, isDirectory: true)
        do {
            if fm.fileExists(atPath: active.path) { try fm.removeItem(at: active) }
            try fm.moveItem(at: staging, to: active)
            call.resolve()
        } catch {
            call.reject("activate failed: \(error.localizedDescription)")
        }
    }

    /// Delete an active pack; the language reverts to streaming cleanly.
    @objc func packDelete(_ call: CAPPluginCall) {
        guard let lang = call.getString("lang") else { call.reject("bad args"); return }
        let active = Self.packsRoot.appendingPathComponent(lang, isDirectory: true)
        try? FileManager.default.removeItem(at: active)
        let staging = Self.packsRoot.appendingPathComponent("staging-\(lang)", isDirectory: true)
        try? FileManager.default.removeItem(at: staging)
        call.resolve()
    }

    /// Active-pack states + measured sizes for the storage UI.
    @objc func packStates(_ call: CAPPluginCall) {
        let fm = FileManager.default
        var out: [[String: Any]] = []
        if let langs = try? fm.contentsOfDirectory(atPath: Self.packsRoot.path) {
            for lang in langs where !lang.hasPrefix("staging-") {
                let dir = Self.packsRoot.appendingPathComponent(lang)
                var bytes: Int64 = 0
                if let files = try? fm.contentsOfDirectory(atPath: dir.path) {
                    for f in files {
                        let attrs = try? fm.attributesOfItem(atPath: dir.appendingPathComponent(f).path)
                        bytes += (attrs?[.size] as? Int64) ?? 0
                    }
                }
                out.append(["lang": lang, "bytes": bytes])
            }
        }
        call.resolve(["packs": out])
    }

    /// A webview-loadable src for one word's clip — the head of the audio
    /// resolution order (I3). The pack's own manifest (stored beside the
    /// audio at activation) maps word+variant -> file name, so the web
    /// side never re-derives cache hashes. Empty when absent: the router
    /// falls through to cache -> network.
    private static var manifestCache: [String: [String: String]] = [:]

    @objc func packSrcFor(_ call: CAPPluginCall) {
        guard let lang = call.getString("lang"),
              let word = call.getString("word"),
              let variant = call.getString("variant") else {
            call.reject("bad args")
            return
        }
        let key = "\(word)|\(variant)"
        if Self.manifestCache[lang] == nil {
            let mf = Self.packsRoot.appendingPathComponent(lang).appendingPathComponent("manifest.json")
            guard let data = try? Data(contentsOf: mf),
                  let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let files = obj["files"] as? [[String: Any]] else {
                call.resolve(["src": ""])
                return
            }
            var map: [String: String] = [:]
            for f in files {
                if let w = f["word"] as? String, let v = f["variant"] as? String, let n = f["name"] as? String {
                    map["\(w)|\(v)"] = n
                }
            }
            Self.manifestCache[lang] = map
        }
        guard let name = Self.manifestCache[lang]?[key] else {
            call.resolve(["src": ""])
            return
        }
        let file = Self.packsRoot.appendingPathComponent(lang).appendingPathComponent(name)
        guard FileManager.default.fileExists(atPath: file.path) else {
            call.resolve(["src": ""])
            return
        }
        call.resolve(["src": bridge?.portablePath(fromLocalURL: file)?.absoluteString ?? ""])
    }

    /// Store the verified manifest beside the staged audio so activation
    /// carries the word map with it.
    @objc func packStoreManifest(_ call: CAPPluginCall) {
        guard let lang = call.getString("lang"), let manifest = call.getString("manifest") else {
            call.reject("bad args")
            return
        }
        let staging = Self.packsRoot.appendingPathComponent("staging-\(lang)", isDirectory: true)
        do {
            try FileManager.default.createDirectory(at: staging, withIntermediateDirectories: true)
            try manifest.data(using: .utf8)?.write(to: staging.appendingPathComponent("manifest.json"), options: .atomic)
            Self.manifestCache[lang] = nil
            call.resolve()
        } catch {
            call.reject("manifest store failed")
        }
    }
}
