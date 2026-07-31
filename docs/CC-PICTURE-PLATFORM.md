# CC-PICTURE-PLATFORM — Spell Picture Is App-Only. It Never Ships to spellgame.net.

**Status: REVIEW-GATED.** Execute nothing beyond drafting until Eric approves
this file. Reads with: CC-WORD-PICTURE v6/v8.x (layout engine),
CC-PHOTO-PICTURE (inherits this wall), CC-ENTITLEMENTS (web grants),
CC-MODE-HUB (registry).

## Intent

Spell Picture (formerly Word Picture) is the app's flagship exclusive — the
reason to download SpellGame instead of playing free in a browser.
spellgame.net remains the free web experience. The exclusivity only works if it
is airtight: a "hidden" web mode that still ships in the bundle is one DevTools
session away from being un-hidden, leaks the asset library, and makes the App
Store exclusive a lie.

**Therefore: exclusivity is a build-time fact, not a runtime `if`.** If any
Spell Picture code, asset, route, string, or registry entry can be found in the
web bundle by any means, this file has failed — regardless of whether a user
can "see" the mode.

When this file conflicts with any other instruction file in a way that implies
a strategy reversal, stop and ask (standing SpellGame rule).

## Features

**1. Registry `platforms` field.** Every entry in `modes.json` gains a required
`platforms` array. Legal values: `"app"`, `"web"`. Spell Picture (and Photo
Picture when it unblocks) declares `["app"]`. Every existing mode is explicitly
declared — no mode may rely on a default. The registry is the single source of
truth for platform availability (single-source doctrine — no scattered
`if (isWeb)` checks anywhere else). Registry schema validation fails the build
on a missing or empty `platforms` field.

**2. Build-time platform wall (compile-out, not hide).** Web builds contain
zero Spell Picture machinery.

- **Rust/WASM core:** all Spell Picture layout/solver/capacity code lives behind
  a cargo feature `picture` that is off for the wasm web target. It is not
  compiled, not merely unreachable.
- **Frontend:** Spell Picture UI lives in an isolated module subtree
  (`src/modes/spell-picture/**`). Web build config excludes the subtree at the
  bundler level (build-time define + entry exclusion), so tree-shaking is a
  backstop, not the mechanism.
- **No shared module may import from the Spell Picture subtree.** Direction of
  dependency: Spell Picture may import shared code; shared code may never import
  Spell Picture. Enforced by lint rule.

**3. Asset and route exclusion.** The web deploy artifact contains no Spell
Picture payload. Scan stroke maps, path manifests, scene packs, golden fixtures,
sticker/gallery assets, finale/share-card templates: none present in the web
`dist/`. The asset pipeline keys asset groups to `platforms` from the registry.
All Spell Picture routes (`/picture`, gallery, finale, deep links) are absent
from the web router table — **404 by absence, not by guard**. The hub renders no
Spell Picture tile on web because the registry filter removes it before the tile
list exists.

**4. Web-bundle symbol-scan CI.** A job on every web build greps the emitted
bundle (JS + wasm + asset manifest) for a maintained denylist: Spell Picture
symbol names, path-manifest keys, scene/subject IDs (dog, eiffel, mona-lisa, …),
and localized mode-name strings for all 15 languages. Any hit = build failure.
The denylist is **generated** from the app-side registry + asset index, not
hand-maintained, so a new picture pack cannot silently dodge the scan.

**5. Playwright web-crawl CI.** Against the built web app: crawl all routes and
hub surfaces and assert (a) no Spell Picture tile, route, or string appears in
any of the 15 languages; (b) direct navigation to every known Spell Picture
route yields 404/redirect with no partial render; (c) the network panel shows
zero requests for Spell Picture asset paths. Runs on every web deploy, blocking.

**6. Wall inherited by CC-PHOTO-PICTURE.** Photo Picture (design-ahead,
execution-blocked) is declared `platforms: ["app"]` from day one. Its privacy
posture (photos never networked) makes web exposure doubly forbidden. The
tracer's runtime-portable core stays an offline/app concern; none of it enters
any web target. **This clause is binding on the Photo Picture file — record it
there too.**

**7. Web entitlements untouched.** This wall removes exactly one mode from web.
It must not disturb the web's language stance, CF-IPCountry grants,
audit-subdomain builds (Paul's Filipino web audit), or any other web behaviour.
If implementing the wall would require touching entitlement resolution, **stop
and ask**.

## Decisions

| # | Decision | Status |
| --- | --- | --- |
| D1 | Compile-out over runtime-hide | **DECIDED**. Non-negotiable. |
| D2 | Registry `platforms` is the single availability source | **DECIDED**. Per-call-site platform checks are banned. |
| D3 | Denylist for the symbol scan is generated, not hand-written | **DECIDED**. |
| D4 | Web behaviour for a shared link/QR referencing a Spell Picture keepsake | **PROPOSED — Eric signs off**: a plain landing page, "Spell Picture lives in the SpellGame app", plus an App Store link. No preview render (would require shipping the renderer). |
| D5 | Audit builds (AUDIT_MODE, Paul's web audit subdomain) also exclude Spell Picture | **PROPOSED — Eric signs off**. Web auditors cannot audit an app-only mode, and a web audit build containing Spell Picture would violate I1. |

## Invariants (permanent, build-failing)

- **I1**: No byte of Spell Picture code, asset, or string exists in any web
  deploy artifact — including audit-subdomain builds.
- **I2**: `platforms` in the registry is the only place platform availability is
  declared.
- **I3**: Shared code never imports the Spell Picture subtree.
- **I4**: The wall never alters web language unlocking, entitlements, or grants.
- **I5**: Photo Picture inherits I1–I4 automatically; no separate opt-in.

## Non-goals / constraints

- Do not touch scoring, entitlements, the v6/v8.x layout engine, the tracer, or
  app-side Spell Picture behaviour in any way.
- Do not add a "coming soon to web" teaser, web waitlist, or any web marketing
  surface for Spell Picture without a separate greenlight.
- Do not fork builds — one codebase, edition/target profiles only.

## Done when

1. `cargo build --target wasm32-unknown-unknown` (web profile) succeeds with
   feature `picture` off; `cargo tree`/symbol dump shows no `picture`-feature
   crate items.
2. Web `dist/` symbol scan: zero denylist hits (job `web-picture-wall-scan`).
3. Playwright crawl suite `picture-wall.spec` green: no tile/route/string/asset
   request in any of 15 languages; direct routes 404.
4. App build (iOS): Spell Picture fully present and unchanged — existing golden
   fixtures (dog, Eiffel, fish) render byte-identical to pre-wall baselines.
5. Registry schema validation rejects a fixture mode with missing `platforms`.
6. Lint fails on a test import of the Spell Picture subtree from shared code.
7. Web language behaviour regression: language list and unlock state on
   spellgame.net identical before/after (snapshot test).
8. Paul's audit-subdomain build passes the same symbol scan (per D5).

If any acceptance test cannot be made to pass without violating a constraint
above, **stop and ask — do not weaken the test**.

---

## Status against this file (2026-07-31)

Nothing executed; D4 and D5 unsigned. Five things that bear on the plan as
written, the first two of which need a decision before code.

**The intent paragraph is now out of date, and I have not silently reconciled
it.** This file says spellgame.net "remains the free web experience with all
languages unlocked," and I4 makes untouched web language unlocking a permanent
invariant. Minutes after sending this file Eric asked for English-only on
spellgame.net, and the registry now has English `Active` with the other fourteen
`ComingSoon` (commit f08193b). The wall did not cause that and I4 is not
violated — a separate ruling did — but the sentence describing the web
experience is no longer true. **Rewrite it when this file is approved rather
than letting a future reader treat it as the current strategy.**

**D4 describes a scenario CC-FINALE does not produce.** D4 covers "a shared
link/QR referencing a Spell Picture keepsake" opened in a browser. CC-FINALE
feature 4 is explicit that the share payload is an image and nothing else — "no
URLs with identifiers… the image is the whole payload." So there is no link to
open and no landing page to build. Either D4 is anticipating a share form that
FINALE forbids, or it is dead scope. Worth resolving alongside the sign-off.

**The frontend layout the file assumes does not exist.** Feature 2 specifies a
`src/modes/spell-picture/**` TypeScript subtree excluded at the bundler level.
There is no such subtree and no bundler: Spell Picture is Rust compiled to WASM
(`src/wordpic*.rs`, `src/spellpic*.rs`) plus markup inlined directly in
`index.html`. So the frontend half of the wall is really three different jobs —
a cargo feature over the Rust modules, extracting the picture markup out of the
shared `index.html`, and keying `tools/bundle_scans.py` output to the target.
The cargo-feature half matches the file cleanly; the other two do not.

**I1 is comprehensively violated today, and the shape of the violation is
better news than it looks.** The web bundle carries every wordpic string in all
15 locales, the whole picture UI inlined in `index.html`, and 163 scanlock
symbols plus every subject ID in the wasm. But the 241KB of stroke data is
**not a separate asset**: `spellpic.rs` pulls it in with
`include_str!("../config/wordpic/scans.json")`, so it is compiled INTO the
wasm binary.

That matters twice over. It means feature 3's "asset pipeline keys asset groups
to `platforms`" has nothing to key on for the scans — there is no file in
`dist/` to exclude. And it means the cargo-feature approach in feature 2 does
the whole job for the stroke data by itself: gate the module and the
`include_str!` goes with it, which is compile-out in the strictest sense the
file asks for. The remaining work is the markup and the strings, neither of
which a cargo feature reaches.

**Feature 4's denylist would catch our own subject IDs immediately** — `dog`,
`star`, `house`, `fish` and `cat` are also ordinary English word-bank entries
that legitimately appear in the web bundle. A generated denylist keyed on
subject IDs alone will produce false positives on day one; it needs to match
manifest keys and asset paths rather than bare IDs.
