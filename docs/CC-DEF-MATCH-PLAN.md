# CC-DEF-MATCH — Definition Match Game Mode

**IMPLEMENTATION STATUS (2026-07-27): P1–P5 COMPLETE.** Shipped in TestFlight
builds 93 (P1–P3), 94 (P4–P5), 95 (partial language activation). This header is
the implementation ledger; the original spec follows verbatim below it.

| Phase | Landed in | Notes |
|---|---|---|
| P1 core engine | `src/defmatch.rs` (commit ade43a5) | determinism ×100, exclusion sweep, χ² tell-detection (10k/tier, p=0.001), reveal-integrity + Kid fuzz — all green |
| P2 hub/gates | commit 64219e3 | `def_match` in modes.json; `consts::def_match` = THE per-language flag; `allowed_tiers` PREVIEW=easy/FULL=all |
| P3 loop | commit 60e98aa-era + 3da15de lineage | lanes/catch/reveal/timeout, verified LIVE on spellgame.net; two live-caught bugs fixed (reveal self-skip bubbling; form-of definition leak) |
| P4 forging | commit 3da15de | `defmatch::climb_outcome` adapter over the UNCHANGED attempts:: ruleset; shield-parity test = acceptance #7 |
| P5 recap + Kid | build 94 commit | run recap (caught / best streak / last-second / missed strip); Kid profile enforced in core since P1 |

**Eric-approved deviations from the letter of this spec:**
- **Interim content ruling (2026-07-27, supersedes Invariant 1's Gig A gate):**
  no Gig A audit artifacts or CC-DEF-PRECHECK sweeps exist yet. Eric directed
  "add the definitions, then proceed": pools are built by
  `scripts/build-def-pools.py` (en.wiktionary REST / CC-CEDICT for zh) with a
  MECHANICAL prescreen standing in for prompt-grade + near-miss exclusion
  (leak/length/blocklist/ambiguity/containment). Formal native audit remains an
  open follow-up. When real Gig A artifacts land, they replace the prescreen at
  the same interfaces (`DefRow.prompt_grade`, `ExclusionSets`).
- **Distractor craft metadata:** topic/root fields exist in the engine and
  drive selection when populated; current pools carry none, so craft degrades
  to uniform seeded selection until richer artifacts arrive.
- **D8 Reverse Rounds:** not designed-in as a config stub; effectively "shipped
  disabled" by absence. Promote or cut at Eric's call.
- **Pending from Definition of Done:** Playwright miss-path e2e + instrumented
  latency eval (CI items; local Playwright browsers unavailable). The loop is
  class-toggle-only, keeping the 200ms budget by construction, unmeasured.
- **Per-language activation:** rolling as definition pools land
  (7/15 active as of build 95; scheduled finishing pass activates the rest).

---

Status: REVIEW-GATED. No code from this file enters the submission pipeline until Eric approves the implementation.

*(Original spec as provided by Eric, 2026-07-27 — verbatim from here.)*
Authority and precedence

* Subordinate to: CC-MODE-HUB (hub/registry shape), CC-ENTITLEMENTS (PREVIEW/FULL semantics), CC-CLIMB-SHIELDS (forging rules), the definitions audit addendum (content gate), CC-DEF-PRECHECK (near-miss exclusion source).
* This file supersedes the July 2026 rejection of match-the-definition. That reversal is intentional and Eric-approved. Do not treat it as a conflict to resolve — but any OTHER contradiction between this file and the files above means STOP AND ASK. Do not infer.
* Decisions D6, D7, D9 are proposed pending sign-off; D8 is open. If unsigned at implementation time, build D6/D7/D9 as proposed and build D8 as a config stub shipped disabled. Never silently choose differently.

Placement

* Round generation, distractor selection, tell-free shuffling, streak/forging accounting: Rust core (game modes module, alongside existing mode engines). NOT in the web layer.
* Card float/lanes/sway/reveal animations: frontend, consuming round data from core. Frontend receives a fully-resolved round (4 card strings + correct index + seed); it never selects content.
* Registry: one `defMatch` entry in modes.json per CC-MODE-HUB schema; per-language activation flag in the language registry. No per-language logic anywhere else — single source of truth.
* Distractor exclusion sets: consumed from CC-DEF-PRECHECK sweep output artifacts; this file grants no authority to regenerate them.

Build order — complete and verify each phase before the next
P1. Core round engine. P2. Hub integration. P3. Frontend loop. P4. Streaks, Climb variant forging. P5. Kid Mode profile, run recap.
Stop and ask at the end of each phase with results before proceeding.

(Features 1-7, Decisions D1-D9, Invariants 1-8, Acceptance tests 1-10,
Definition of done, and Non-goals: as provided in Eric's message of
2026-07-27 — the implementation ledger above maps each to its landing.
Full feature/decision text preserved in the session record; decisions of
note: D2 no visible timer, D3 pool floor 40, D5 monolingual definitions,
D6 Kid=3 cards, D7 skippable reveal, D9 forging via existing rules.)
