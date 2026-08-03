CC-OFFLINE-PACKS — Offline Language Packs (BD-2) CC-OFFLINE-PACKS — Works on the Plane (BD-2)
Status: REVIEW-GATED. Inherits CC-BUY-DRIVERS shared law S1–S5.
Intent
"Works on the plane / in the car / at grandma's with no wifi" is a real parent purchase trigger, and it's structurally app-only: spellgame.net streams TTS from the Pi backend by design. The promise being sold is identical gameplay offline — not a degraded mode. If any mode plays differently offline (missing audio, skipped words, silent fallbacks), the feature is a lie and violates the no-silent-fallback doctrine.
Features
1. Per-language audio pack
A pack = every audio asset for a language's full playable content (all tiers the player can reach), built server-side from the same TTS cache that serves the app, with a signed manifest: version, per-file hashes, total size, TTS voice ID. Granularity is BD-D2 (pending): per-language (default proposal) vs per-language-per-tier.
2. Download manager
Wi-Fi by default (cellular behind an explicit toggle), resumable, background-download capable, integrity-checked file-by-file against the manifest. A pack is atomic: it is either fully verified and ACTIVE, or it is absent. No partially-active packs, ever.
3. Storage manager UI
Per-language rows: size, downloaded/active state, delete. Deleting reverts that language to streaming cleanly. Lives in settings; parent-gate not required (deleting a pack destroys no progress).
4. Offline session behavior
Audio resolution order is fixed and singular: active pack → runtime cache → network. With an active pack, a full session in that language performs zero audio network requests even when online (packs also cut Pi load — that's a feature, not a side effect). Fully offline with no pack: the language selector marks unpacked languages as needing connection before a session starts — never a mid-game failure.
5. Entitlement-following
A player can download packs for exactly the languages they're entitled to play (free English, home-country grant, PREVIEW-tier scope for previews, everything under Complete). Pack availability is computed from the existing entitlement resolver — no second entitlement logic.
6. Future voice packs (design-ahead only)
When Voice Studio custom Piper voices ship, a voice = an alternate audio pack under the same manifest schema (voice ID field already present). Build the schema affordance now; build zero voice functionality.
Decisions
* D1 = BD-D2 (granularity).
* D2 (proposed): Size budget: a language pack must land ≤150 MB or the build fails and we compress/trim, stop-and-ask if impossible. Sign off on the number.
* D3 (proposed): Home-country-grant language auto-suggests (not auto-downloads) its pack on first launch. No silent downloads ever.
* D4 (decided): Pack building is a CI artifact of the existing TTS cache — no new recording or synthesis path.
Invariants
* I1: Identical offline gameplay: word selection, scoring, modes, and pacing are byte-identical with and without network (audio source is the only variable).
* I2: Atomic packs — no state between "absent" and "verified active".
* I3: Single audio resolution order, one implementation, no per-mode overrides.
* I4: Manifest hash verification is mandatory; a failed hash quarantines the whole pack and surfaces one honest error.
* I5: Web is untouched — packs are app-only (`platforms: ["app"]`), web keeps streaming.
Non-goals
No offline sync/accounts, no pack sharing between devices, no delta updates v1 (full re-download on pack version bump), no video/definition assets in packs (audio only; definitions are text and already local).
Acceptance tests / Done
1. Airplane-mode Maestro run: full en and es sessions (standard mode + Daily Challenge) with active packs — zero failures, zero network attempts (proxy log empty).
2. Tamper test: flip one byte in a pack file → verification quarantines pack, language reverts to streaming, one error surfaced, no crash, no partial audio.
3. Online-with-pack session: network inspector shows zero audio requests.
4. Delete pack → storage reclaimed (measured) → language streams normally.
5. Kill app mid-download → relaunch resumes → completes → verifies ACTIVE.
6. Entitlement check: PREVIEW-only player's pack for language X contains exactly tier-1 audio (BD-D2-dependent), nothing more.
7. Offline with no pack: pre-session marking visible; starting an unpacked language offline is impossible, not broken.
