CC-BUY-DRIVERS — Batch Orchestrator: Why the App, Not the Website
Status: REVIEW-GATED. Five subordinate files. Subordinate files stay authoritative on their own scope; this file governs order, shared law, and cross-file conflicts. Conflict implying a strategy reversal → stop and ask.
Intent
The web/app split is a capability gap, not a policy: web = try the game, app = your family lives here — your photos, your voices, your kid's learner model, your keepsakes. Every feature in this batch must be something spellgame.net physically cannot replicate, or it doesn't belong in the batch.
Scheduling law
Photo Picture unblocking (P1: v6/v8.x layout engine proven + fish golden holding; P2: D10 tool shipping a pack's expert piece) outranks everything here. No BD file may consume effort that delays P1/P2. If a BD task and a layout-engine task compete, the layout engine wins — silently deprioritizing Photo Picture is the batch's forbidden failure mode.
Build order
1. BD-1 CC-IOS-SURFACES — cheapest, ships nearest build 56 era
2. BD-2 CC-OFFLINE-PACKS — infrastructure other files reuse
3. BD-3 CC-GUARDIAN-DASH — blocked on CC-LEARNING-ENGINE L1 surfaces existing
4. BD-4 CC-FAMILY-VOICES — V1 executable; V2 design-ahead
5. BD-5 CC-YEARBOOK — blocked on CC-FINALE export path shipping
Stop and ask before starting each file (per-file greenlight, arcade-batch precedent).
Shared law (inherited by all five, build-failing)
* S1: All privacy posture from CC-LEARNING-ENGINE D5 is global: everything on-device, zero telemetry, no engagement optimization, no server-side accounts.
* S2: Little Speller zero-purchase-surface invariant applies everywhere, including widgets, notifications, and exports.
* S3: App-only features declare `platforms: ["app"]` in the registry (CC-PICTURE-PLATFORM law); web symbol scan denylist auto-extends.
* S4: No new displayed strings outside hard-capped audited pools; anything user-facing in 15 languages rides the registry/audit machinery.
* S5: One codebase, edition profiles only. Education edition inherits each feature only where its file says so.
Cross-file decisions awaiting Eric
* BD-D1: Climb Live Activity — include or cut from BD-1 v1?
* BD-D2: Offline pack granularity — per-language or per-language-per-tier?
* BD-D3: Guardian Dash free teaser — English dashboard free, Complete unlocks all languages?
* BD-D4: Family Voices V2 training path — where Piper training runs is undecided; V2 executes nothing until this is signed.
* BD-D5: Yearbook default period — school year (Sep–Jun) vs calendar year vs Climb season.

---
DECISION LEDGER (Eric, 2026-08-02 — recorded at greenlight)
* All five files greenlit 2026-08-02, in order BD-1..BD-5.
* BD-D1: Climb Live Activity CUT from BD-1 v1 — ActivityAttributes stubbed behind a disabled flag, nothing else built.
* BD-D2: per-language pack granularity. PACKS-D2: 150 MB budget signed. PACKS-D3: auto-suggest signed.
* BD-D3: signed — full English dashboard free, Complete unlocks all languages.
* BD-D4: OPEN — V2 Family Voices executes nothing until signed.
* BD-D5: CALENDAR YEAR default (overrides the school-year proposal; all four periods selectable).
* IOS-D4: last-active profile, no per-widget pinning v1. VOICES-D4: celebrations allowed from gate-failed voices — signed. VOICES-D5: multi-voice + grandma copy — signed. YEARBOOK-D4: photo pieces excluded by default — signed. YEARBOOK-D5: opt-in ready notification — signed.
* CC-REPORTS decisions (Eric, 2026-08-02): D3 SIGNED — new on-device per-keystroke timing capture approved (the batch's only new data collection; D3 recon evidence: no timestamps existed — input_provenance counts only, misses/wordstats word-level). Hesitation Map ships in v1. D4: cross-language floor = >=2 languages with >=100 scored attempts each (stricter than the 50 proposal). D6 SIGNED: kid tab may ship before Guardian Dash. CC-REPORTS itself awaits its per-file greenlight.
* CC-CALENDAR v2 decisions (Eric, 2026-08-02): D1 final per v2 (kid-origination, I7 CI-enforced). D2 SIGNED (recap+goals free; multi-week planner Complete). D3 SIGNED — the daily journal is the batch's second sanctioned new-data store (date-granularity, outcomes-only, 730-day cap, ReportsQuery sole reader). D4: 5 new words per WEEK (adjustable 3-15). D5 SIGNED: goal ring joins the widget snapshot. File awaits per-file greenlight.
* FOR THE RECORD (Eric, 2026-08-02): "all these read mes are app exclusives" — every file in this batch and its amendments (BD-1..BD-5, CC-REPORTS, CC-CALENDAR) is app-only: platforms:["app"], web symbol-scan denylist auto-extends (S3), zero web mention/symbol/asset.
* CC-CALENDAR greenlit (Eric, 2026-08-02) — all eight files now live. Slots after the 121-123 ladder (depends on ReportsQuery from 121, snapshot fields from 120); journal + planner + dealt hand + cheer flow as one wave on Eric's next ship number.
* CC-TRANSLATE-TOOLS decisions (Eric, 2026-08-02): D5 — WHOLE SUITE APP-ONLY (blanket app-exclusive ruling stands; tools 1-4 join the wall, denylist auto-extends). D1 SIGNED (core+daily+Passport free; depth tools Complete). D2 SIGNED (Kid Mode: full core+Passport+daily, no camera; Little Speller: no translator). D9 decided (word-level only, no phrase path even as stub). Wave-3 gates D3/D4/D6/D7/D8 UNSIGNED — design docs only, zero executable paths, CI-checked. Slots after CC-CALENDAR. NOTE: CC-BANK-TRANSLATE (authoritative parent on gloss pivot) not yet in hand; Phase A gloss column is deadline-critical and untouchable. File awaits per-file greenlight.
* CC-TRANSLATE-TOOLS GREENLIT (Eric, 2026-08-02) — ninth file live. Slots after CC-CALENDAR in the build ladder. Wave-3 gates remain unsigned. Parent file CC-BANK-TRANSLATE still not in hand; gloss-pivot data availability governs when the core can light up (schema-first build, dark until glossAudited languages exist).

* CC-BANK-COMPLETE decisions (Eric, 2026-08-03): "D-FLOOR as proposed, D1-D5 as proposed."
  D-FLOOR per-language rank floors signed (en frozen reference; sw 3-tier structural cap).
  D1 form-level ranking. D2 hash-pinned composite (re-pin = Eric-acknowledged CI event).
  D3 proper-noun allowlist EMPTY at launch. D4 German compounds rank-decided, >=4-morpheme
  HUMAN_ONLY. D5 en zero-writes / hi design-ahead only. Solutions plan approved
  ("kets make 1-5 work! like it"). Still gated: CC-BANK-EXPANSION + CC-WORDLIST-SOURCES
  pastes (Features 1-2), sampled-audit amendment (UNMUNCH/GENERATE). Findings for the
  record: wordfreq LACKS sw and silently substitutes English — sw composite rides
  FrequencyWords+Leipzig only; TTS gate pre-proven by the pack builder run.

* SHIP 121 (build 128 era): BD-2 app half + CC-REPORTS foundation. Wall scanner
  earned its keep — caught shared code importing the Spell Picture subtree (I3);
  fixed with a local base64, not an exception.

* 122 WAVE (per standing trigger "green light bd-3 and reports when 121 ships"):
  CC-REPORTS surfaces (D3 keystroke timing live at both keyboard append paths,
  gaps never span words; kid Quest Log tile via the hub, rows are doors ->
  interim smart-review per DEEP_LINKS) + BD-3 guardian dash core (parent-gated
  by the worded-math challenge — its own instance, never touches the agegate
  verdict; mastery map / trouble spots / hesitation list / what's-next;
  BD-D3 English-free honored, gdash pool x16). Weekly digest + PDF ride a
  later wave under the signed D6 decoupling.

* SHIP 123 (build 130): BD-4 V1 Personal Voice. BD-D4 still the batch's one
  open decision; the V2 symbol scan is a standing gate step. Eric's on-device
  spot-check (acceptance #1) pending his Personal Voice recording.

* 124 WAVE — BD-5 CC-YEARBOOK, honesty consequences recorded:
  (a) completed pictures carry NO completion date (Run never stored one) and
  I1 forbids starting to keep them -> the gallery spread is ALL-TIME, labeled
  as such; dated spreads (journey/milestones/firsts) honor the period.
  (b) Climb has no dated season store -> period menu ships calendar year
  (BD-D5 SIGNED default) / school year / all-time; Climb season joins when
  seasons have dates. (c) Export entitlement rides progress_reports as the
  Complete proxy until CC-ENTITLEMENTS lands its own flag. (d) Acceptance #3
  holds BY CONSTRUCTION: the page embed calls the same plan+export_svg the
  finale export uses, via the new I3 picture bridge in surface_hooks
  (install_picture_bridge — picture registers at boot, compiler never
  imports the subtree; the wall scanner stays the enforcer). D4 photo
  exclusion is moot until Photo Picture unblocks (recorded, default-exclude
  wired into the bridge filter when it does). D5 ready-ping: ONE-SHOT `at`
  schedule (an `on` clause would repeat yearly — a nag, not a ping), id 2,
  opt-in, cancelled on opt-out.

* SHIP 124 (build 131): BD-5 Yearbook delivered. 125 WAVE — CC-CALENDAR:
  journal (D3 signed: date-keyed, outcomes-only, 730-cap, today-only-writable
  API), dealt-hand goals (D4=5/week signed; deterministic, small-win
  guaranteed, no impossible cards; select_goal = the ONE writing door, I7
  gate-grepped), planner with planned_pick standing aside whenever anything
  in the band window is overdue (I2 order as code), read-and-cheer parent
  section (5-line hard-capped pool, no free text), widget goal ring (D5
  signed IN — the reserved snapshot fields now fill), 12-mode registry pins.
  Recorded reading: I4's no-free-text applies to the planner too — sources
  are pick-only from the kid's own pools.

* SHIP 125 (build 132): CC-CALENDAR delivered. 126 WAVE — CC-TRANSLATE-TOOLS
  core, the honest pivot-less start: CC-BANK-TRANSLATE (authoritative on the
  gloss pivot, NOT in hand; Phase A gloss column still riding the Fiverr
  audits) means no gloss data exists — so 126 ships every pivot-INDEPENDENT
  tool: the tab (13th tile, kid-safe per signed D2, app-only per signed D5),
  pick-only lookup (strict I4: no free-text field exists), single-word cards
  with audio + the zh transliteration toggle (a re-render of the bank's own
  pinyin|hanzi halves, never generated), and both doors — "Now spell it" via
  the new surface-agnostic request_word door (standard flow, normal scoring),
  "Add to plan" via the planner's own legality + honest decline. gloss_rows()
  is THE resolver seam (entitlement AND glossAudited, one function): empty
  today, so fan-out rows are ABSENT, not locked. CI added: closed-space
  MT-symbol scan + Wave-3 zero-code check (D3/D4/D6/D7/D8 unsigned). Wave 2
  (daily, Passport, Home Pair, Match) queues behind the pivot landing.
  BATCH-COMPLETE BUILD: all nine files now have their executable-today scope
  shipped; remaining items are human-gated or paste-gated.

* SHIP 126 (build 133): CC-TRANSLATE-TOOLS core delivered — BATCH COMPLETE.
  The Wave-3 zero-code CI immediately earned a footnote: its first pattern
  matched loanword_spelling (a learner SKILL id, prior art) — tightened to
  loanword_explorer|loanword_pack. BD-3 REMAINDER (signed D6 decoupling)
  built and committed behind this entry: weekly digest (opt-in, off by
  default, id 3, Sunday 17:00, body re-templated from live counts at every
  boot so the fired copy is never staler than the last open) + the printable
  PDF report (one page per language with learner data, mastery/trouble/
  week-over-week, via the yearbook's deterministic metadata-free assembler;
  D5 decided: wordmark carried — teacher-bound page). week_over_week lives
  in ReportsQuery, preserving the journal's sole-reader law. Awaiting a
  ship number.

* CC-PICKER-SEARCH (all D1-D6 signed; final gate = Eric on-device):
  registry migrated one-shot (categories/canonicalCategory/aliases +
  categoryList; old format = CI failure). Search: pinned bar, fold_lenient
  (the existing normalizer), corpus = id+subject+aliases+category names+
  masterpiece captions, uiLang only (D5), kid filter BEFORE matching (D1),
  zero telemetry. Cross-listing per D2 (rhino/mona/starrynight/redfuji).
  Feature-5 sort + unified labels + See All(count) + 2-line captions.
  Seven-test CI battery incl. synthetic pack + 3-char reachability x364 +
  the +40% caption budget standing in for the pseudo-locale sweep (no
  CC-LOCALE-LAYOUT cascade exists in-repo — recorded).
  FLAGGED FOR THE ON-DEVICE PASS: (1) spiderweb left Masterpieces (in-house
  original, no artist; Feature 3 makes artist-less masters impossible) ->
  now Animals. (2) learn shelf keeps Eric's script-first ordering as the
  Not-Started tie-break — Feature 5's strict registry-order reading would
  regress it. (3) "completed by date" rides the monotonic touched counter
  (dates were never stored). (4) Localized subject NAMES don't exist as
  strings yet — ids+aliases carry search until the audit era adds names.
  D3 VERDICT (Eric): "Proposal A, Zodiac and sky stay together, lets
  expand sky and masterpieces when all the read mes are done." Proposal A
  implemented same-session (registry-only, zero picker code — Feature 7
  held): world 89 -> world 34 + landmarks 16 + music 10 + symbols 13; the
  split also surfaced and repaired 18 late-batch generics misfiled into
  world by their cultureN packs (tulip/penguin/castle/... -> things,
  penguin -> animals, eiffel -> landmarks, violin -> music). Proposals B/C
  unsigned - untouched. Sky+masterpiece EXPANSION queued post-readmes.

* CC-MASTERPIECE-TONAL received + D1-D4 signed (2026-08-04). CONSEQUENCES:
  (1) greatwave/sunflowers/scream wiring is BLOCKED until mona's tonal gate
  passes (the masterpiece lane opens on her D4 verdict). Their photoreal
  previews stand as review material only. (2) mona's interim machine-trace
  hosting map (six silhouette arcs + four carriers) rides ship 129 ONLY as
  CI stopgap — the expanded ko/vi pools made her hand-traced short strokes
  unhostable and extension was whack-a-mole; the tonal re-author replaces
  it. (3) Built same-session: extractionClass on all 364 subjects + lint;
  tonal.py (bands/boundaries/tensor-flow, monotonic by construction);
  vision-landmarks.swift (Vision CLI, T0-only); first mona extraction:
  boundaries 141, flow 124/82/51/19/17 monotonic, 7/12 features auto-
  seeded (hands=lasso, hairMass/frame/highlightBand=geometry+review).
  Sky five + sunflower (LINE-class, PASSED) still wire freely.

* CC-PHOTO-SHADOWS received + D1-D4 signed (2026-08-04). Design-ahead,
  execution-blocked, unblocks nothing. Done #1 design-acceptance run
  same-day: zero contradictions vs texts in hand; one watch-item (LR6
  edge emphasis = density-domain inside faces, structurally enforced by
  F1 anyway); full-text re-review owed at Phase B. The FACEPASS->SHADOWS
  inheritance chain is now: Eric-in-the-loop law (tool) -> confidence-
  floor law (runtime), same vocabulary, same band machinery — shared by
  extraction into a common module WHEN the parent's P1/P2 gates open,
  never by the app importing tool code.

* SHIP 129: THE GENERATION WAVE + the bank era's foundations, one commit.
  ~33,000 new words generated to the signed pool floors across 13 languages
  (wordfreq-primary interim recipe; sw via Leipzig Wikipedia — FrequencyWords
  has NO Swahili; zh via pypinyin numbered-tone rows; ja via reading
  conversion to typeable hiragana; en FROZEN and hi design-ahead per D5,
  hi candidates parked in tools/bank/candidates-hi). All candidates passed
  the repo's own gates (keyboard charsets, exclusions, NFC, dedup); the 840
  export cap gave way to floor*1.25 (legacy-840 floor kept so pre-expansion
  banks can't become retroactive violations). CC-BANK-COMPLETE slab: floors
  as data + schema CI, the F2 pin law (verify_pins in the gate; unpin =
  Eric-ACK event), the U-engine (gates 1-3 live, 4-5 pending their
  authority files, nothing reaches full U provisionally), F5 red-flip +
  reverse fixtures (removing one is itself a build failure), F6 pool
  report. Mona's hosting map: six machine-traced silhouette arcs, INTERIM
  by decree — the tonal re-author (CC-MASTERPIECE-TONAL) replaces it at
  her gate. Also riding: extractionClass on all 364 subjects + lint,
  tonal.py + F0 de-light (signed), vision-landmarks CLI, facepass_steps
  protocol runner, batch-27 verdicts recorded.

* CC-TONAL-POLISH received (2026-08-04): approved in principle, BLOCKED on
  D2/D3/D4 signatures by its own text — nothing executed. Consequence noted:
  F0 makes the PICTURE-COLOR goldens (dog+eiffel) + Mona's dark-painting
  color side-by-side the PREREQUISITE for all tonal polish, pulling color's
  Mona case forward of the general post-readme queue (consistent with
  PICTURE-COLOR D5 rollout + TONAL D4's single combined session).

* INTERIM CLASS NOTE (2026-08-04): mona's registry rides extractionClass
  LINE while her decreed-interim arc map ships — the v7.4 lint rightly
  demands requiredFeatures from any TONAL trace, and those twelve features
  arrive WITH the tonal export at her gate. The registry describes what
  ships today; the flip to TONAL + requiredFeatures is part of the tonal
  export, atomically. (The tonal candidates already carry all metadata.)

* SHIP RECORD — ship 130 = build 137 (2026-08-04): batch-27 wired (the
  sky shelf: sun/rainbow/comet/saturn/raincloud/sunflower + the three
  photoreal masters as guide art + mona's photoreal carriers) and the
  tonal-polish tool era rollup. The L9 saga en route produced a new
  CONTENT DESIGN LAW: no ink tapering to a point, no white channel
  narrower than ~50px draw-space — a traced outline's converging sides
  host colliding words (the capped sweep report had hidden 177
  failures; the panic now stays capped but WP_SWEEP_LOG dumps all, and
  WP_SWEEP_ONLY=ids scopes a content iteration to ~23s). Geometry fixes
  by the law, never hand-placed: trapezoid sun rays, one fat comet
  tail, six blunt sunflower petals, rainbow re-spaced + one-edge-per-
  band hosting (38px min separation between host lines). LOOK-FLAGS for
  Eric's on-device pass: sun's ray tips are now flat; sunflower is 6
  fat petals (was 16); comet is one fat tail (was 3 lobes); all three
  differ from the previously passed drawings — L9 forced the change.

* SHIP 131 ARMED (Eric verbatim: "ship 131 for the streak-feed when
  gates pass"): POLISH D2 streak-feed engine (see CC-TONAL-POLISH.md
  D2 DELIVERED) + CC-PICTURE-COLOR groundwork riding along — palette
  schema (inert data fields), Done #1 byte-identical layout CI
  (color_never_touches_layout, all subjects), Done #2 resolution lint,
  Done #3 contrast lint (band-driven 3.0/4.5 floors vs the shipped
  #0e1420 canvas), mona tonal exporter staged (gates checklist, D4
  export-block live). Renderer seam intentionally NOT in this ship —
  waits on Eric's mona D2 faithful-vs-floored verdict (Done #6 order).

* SHIP 132 ASSIGNED (Eric verbatim: "ship 132 for the mona flip when
  gates pass"): the atomic LINE->TONAL flip — extractionClass TONAL,
  the amended TEN requiredFeatures, 21 hosts (12 tonal flows + Eric's
  9 operator-drawn strokes, lipParting focal at max order), 200 guide
  strokes. Session gates all verbatim in CC-TONAL-FACEPASS.md. First
  TONAL subject shipped; masterpiece lane OPEN; the streak-feed (138)
  goes live on her smile in this build.

* SHIP 133 ASSIGNED (Eric verbatim: "greenlight both ship 133 for the
  picker fixes when gates pass" + the two field repairs found en route):
  1. PICKER NAMES — every tile identified (Eric: "call the pictures by
     their name like for eye of horus"): Picture.name registry field,
     370 curated en names (letter tiles = their letter), wp.name.<id>
     locale override hook, name joins the search corpus, name lint
     (non-empty, <=24 chars) in wordpic-check.
  2. SEARCH SPEED (Eric: "massive delay... takes forever"): the input
     handler no longer rebuilds the picker per keystroke — 150ms
     generation-counted debounce + per-open corpus cache (corpus_of
     walked i18n 370x per keystroke).
  3. v8.3 SCAN PRECEDENCE (the 139 field bug): extractionClass TONAL
     outranks the scan bundle at all five play-path gates — the shipped
     TONAL mona becomes reachable.
  4. MASTERS REPAIR: greatwave/sunflowers/scream were NEVER in the
     registry (ship 130's wire silently skipped; its ledger entry was
     wrong). Wired now as LINE photoreal per the original approved
     design, carriers re-selected under the full L9 law (presplit,
     min-dist separation, Munch's new SELF-CLEARANCE law — a swirl can
     collide with itself; budget floor [2,10] on expert). mp captions
     added in all 15 locales. Sweep green all languages x seeds.

* SHIP 134 ASSIGNED (Eric verbatim: "greenlight 1 and 2 ship 134 when
  gates pass", then "3 and 4", then "NO more sliding menu options
  please", then "finish the hub mockup and ship 134 when gates pass"):
  PICKER v3 — THE HUB. Every horizontal shelf deleted; one scroll
  direction. Hub = Jump back in (recent finished pieces) + Favorites
  (long-press to star, storage-backed) + nine category CARDS; a card
  opens the category as one full wrapped grid. Learn opens to EIGHT
  script folder cards (221 letter tiles collapsed; registry `folder`
  data + closed-set cargo lint; the player's own script leads — the
  ru-sees-Cyrillic e2e law survives translated to folders). A-Z mode
  toggle beside search (letters excluded — they live in folders).
  12 new i18n keys x 15 locales. Hub mockup approved at artifact
  558a7d0f. RIDING ALONG: TRANSLATE Wave 2 (daily word, Passport,
  Home Pair) + the D5-strong platform wall (whole suite app-only) —
  decisions ledgered in CC-TRANSLATE-TOOLS.md.

* SHIP 134 SCOPE (all of the above, one build): PICKER v3 HUB (no
  sliding anywhere; Jump back in + Favorites + nine category cards;
  Learn -> 8 script folders, own script first; A-Z; long-press stars) +
  TRANSLATE Waves 2 AND 3 (all 13 tools; suite app-only per D5-strong) +
  FAMILY-VOICES V2 on the local-Mac path (BD-D4) + PICTURE-COLOR
  renderer seam & goldens + LEARNING-ENGINE L1 flags ON / L2 insight
  dark + BANK gates 4-5 + the sampled-audit machinery.
  E2E 77/80 — the three remaining reds are the pre-existing hub-tile
  set the gate knows. Two e2e fixes landed en route: a shared
  `pinBaseline` so every self-built context gets the same flag baseline
  (the learner flags going ON had paused a first serve behind the
  placement card), and the suite is now MUTED (--mute-audio + a
  speechSynthesis stub) after test runs narrated themselves aloud
  through the Mac's speakers.

* PICKER v3 POLISH (in 134): the no-slide law made literal. Three
  surfaces still scrolled sideways after the hub landed — the In
  Progress strip, the base shelf class, and the trophy gallery (which
  grows forever and needed it most). All three are wrapped grids now,
  and `wordpic-check` gained a CI guard so an `overflow-x:auto` cannot
  creep back into a picker surface one CSS line at a time. Favorites
  also got the missing half: a starred tile now SHOWS its star — a
  favorite you cannot see is not a favorite.
  RACE NOTE (my error): I edited the tree while chain-134's third gate
  was running, which broke its web-wall step on a half-written check
  script. No tree edits between gate-start and commit — the law exists
  for exactly this.

* SHIP 135 (Eric: "yes fix the three hub reds for 135"): the board is
  CLEAN — e2e 80/80 for the first time. The finding: none of the three
  was an app bug. All three tests encoded a pre-CC-HUB-CLEANUP-D5
  world while the Rust unit tests in modes.rs had been asserting the
  current law and passing all along:
    - the teaser test demanded online_spelloff tile, but D5 RETIRED the
      hub teaser and `hidden` beats a flag;
    - the es test pinned "tiles are localized" to syllable_replay,
      which the registry has since marked hidden — pinning a law to one
      mode is how it rotted;
    - the Kid Mode test carried a two-name allowlist and so had been
      calling a CORRECT app a kid-safety leak ever since `practice`
      became kidSafe.
  All three now assert the current law and, where possible, the
  underlying safety property rather than a list that can rot (Kid Mode
  checks "no adult-only mode is ever kid-visible, and nothing shown is
  locked or a teaser"; modes.rs keeps owning the exact menu).
  CONSEQUENCE: the gate's tolerance clause is RETIRED. It permitted up
  to three hub reds, which meant a new hub failure could hide behind
  "just the known three". Zero means zero now.

* SHIP 136 (Eric: "ship 136 for all of this when gates pass") — the
  scans-and-verdicts build.
  SCANS: 16 of the 21 registry-only pictures entered the SCAN-STACK
  pipeline (bundle 366 -> 382), so they finally get capacity planning
  and the 95% coverage floor the rest of the bank has always had. Five
  constructed symbols were REFUSED by that gate and stay centerline-
  hosted, which is right: for authored geometry we know the exact lines,
  so words ride the real triangle edges instead of the outside of a
  stroke. The scan pipeline earns its keep where geometry is DISCOVERED.
  VERDICTS: insight copy ON; mona option D (two of five regions ship as
  Leonardo painted them); the audit rule split 300/3 unmunch, 300/0
  generator; the photo matrix confirmed on iOS 26.5 — with a correction,
  hi is Unsupported (Devanagari), which my first report missed because
  the probe's own list omits it.
  MONA: her mouth is unsmeared — the two lip strokes sat 3.2px apart and
  rendered as one blob; Eric's focal parting keeps hosting and the
  lower-lip shadow becomes guide ink. Density 19 -> 21 hosts.
  TRANSLATE: the gloss pivot exists (config/gloss/, loader, one
  resolver, scripts/gloss-check.mjs). 39 validated rows across es/fr/de,
  ALL DARK — `audited` is a human claim and no language is signed.
  SYMBOLS: 12 wired from centerlines; tools/wire_symbols.py is now a
  repo tool (its scratchpad copy was lost twice, and a `git checkout`
  during the mona work silently reverted the lane once).
  PHOTO-EXPERT: giza in (31 paths, PD Library of Congress plate). TIGER
  HELD BACK — stripes trace as closed thin shapes that self-face; the
  fix is medial-axis extraction, a real build. Two CC-BY candidates were
  rejected on the way: invariant 1 is PD/CC0 only.
  CALENDAR D3 signed, closing the last ledger/tree contradiction.

* SHIP 137 (Eric: "ship 137 for the gloss when gates pass") — the gloss
  wave. The translator's pivot goes from 39 rows in three languages to
  2,662 across all fourteen banked ones, authored from a single
  300-concept core list and filtered by the three laws, so the yield is
  a measurement rather than a target. zh is included and was never a
  candidate for cutting: 6,500 words, second-deepest bank in the game.
  My "zh is empty" reading came from grepping word_data.rs, which holds
  the other fourteen; Chinese lives in words.rs because it stores
  pinyin|hanzi pairs. gloss-check.mjs learned to read that file.
  EVERYTHING STAYS DARK. `audited` is false in all fourteen and the
  resolver still gates on it — now covered by a test that loads all
  2,662 rows through the real include_str! pipe and asserts the
  resolver yields None anyway. Rows existing must never be what lights
  a language up; only a native speaker's signature is.
  THIS SHIP CHANGES NOTHING A USER CAN SEE. That is the point: the
  tables, loader, CI and resolver are now complete, so the ONLY thing
  between the translator and being usable is one signature. Spanish at
  258 rows is the obvious first ask.
  FOUND ON THE WAY, all filed and none worked around: BD-G1 the German
  bank lacks mann/frau/mutter/vater/kopf/bein; BD-G2 the German bank is
  97% lowercase so its nouns are stored against German orthography
  (grossmutter, not Großmutter) and a German speller is being taught
  wrong spelling; BD-G3 the widest one — core-vocabulary coverage is
  poor across most banks (hi 9/15 on a fifteen-word floor, missing even
  "book" and "milk"), because the generation wave measured volume and
  never measured coverage. Gloss yield now serves as that missing
  coverage metric and is recomputed on every gate run.

* SHIP 138 (Eric: "ship 138 when the e2e comes back clean") — the Aug 6
  audit P0s. The orb was dead because a modal was nested in a hidden
  screen and the serve handed it the turn anyway; Practice's exit was
  under the notch because a CSS shorthand ate the safe-area inset; and
  four scrollers plus two whole surfaces could not scroll because no one
  had ever written down that a flex-child scroller needs min-height:0.
  All three are now laws in the gate rather than fixes, because each was
  a class and not an instance — the modal law immediately caught #wpHk
  too. Two test faults fell out: placement.mjs had never once run (the
  runner dropped every module after the first in a tuple), and the app
  e2e report was being overwritten by the site run. Both fixed; the app
  suite is 82/82. F5 is NOT in this ship — the code contradicts D5, so
  it goes back to Eric instead of getting a fix aimed at the wrong
  cause. Full diagnoses in docs/CC-AUG6-AUDITPASS.md.

* SHIP 140 (Eric: "ship 140 if it's clean") — the settings-truth gate.
  Seventeen controls, seventeen declared effects, thirteen live web
  effect tests and four owed a device pass. The invariant is permanent
  and self-checking: a new toggle without an effect test fails the
  build, and the gate proves it still bites on every run. Big Text now
  reaches the play surface, which it never has. One real dead switch
  fell out of writing the tests, and it folds into the F7 cut. Full
  account in docs/CC-AUG6-AUDITPASS.md.

* SHIP 141 (Eric: "ship 141 for f16 when gates pass") — the app stops
  telling iPhone owners to install Chrome. Two strings across fifteen
  locales, one platform-aware resolver that every [data-i18n] key now
  flows through, and a lint that fails the build if either key regresses
  to plain t(). The fourteen non-English drafts are unaudited by
  construction and marked as such.

* SHIP 141 (Eric: "a build where Spell Pic is usable would be nice so I
  can actually start taking notes") — the exception to the ship freeze.
  Spell Picture was DEAD on build 147 and the cause was mine: ship 138's
  min-height:0 let .wp-grid collapse to zero height. Verified playable
  end to end before arming — 9 categories, 35 tiles under "world", tap
  one and the play screen opens with real artwork and a word waiting.
  Rides along: F16's platform copy, F6's fourth tile, F7's cuts, and the
  corrected scroll law that can no longer enforce a collapse. The freeze
  continues after this; Eric wants the rest of the audit finished before
  the next one.

* SHIP 142 — F14: OCR smash-ups come apart. The segmenter proposes and
  never applies, and it shows the PIECES so a parent can catch the ones
  it gets wrong ("Sundeep" -> sun · deep is a pinned test, not a
  hypothetical). Verified through a new seam that renders the review
  sheet in a browser, because the photo flow is native-gated and this
  UI would otherwise have shipped never having rendered. 93/93.

* SHIP 143 — Calendar, Translate and Reports become reachable. Three
  live modes had no flag arm, so the hub filtered them out and their
  tiles never rendered on device. modes-check had been reporting it to
  nobody; it is in the gate now.

* SHIP 144 — Spell Picture search goes from 3.7 SECONDS to 65ms. The
  cause was never DOM: counting the words under each tile ran the
  scan-stack planner once per tile. Measured first, which is the only
  reason the fix is right — windowing would have hidden it. Plus the
  bottom safe-area inset the picker never had, on-device timing and
  geometry readouts, and the four Maestro flows for the native-gated
  controls. The device scroll failure is still open and now instrumented.
