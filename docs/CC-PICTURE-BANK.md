# CC-PICTURE-BANK — Spell Picture Bank & Difficulty Climb

**Status: GREENLIT by Eric.** EXECUTABLE for curated (traced) content on the
shipping Spell Picture mode. Masterpiece tier and multi-session expert pieces
additionally require pan/zoom (D3). Parent files: CC-WORD-PICTURE v6/v8.1/v8.2
(layout law), tracer v7.x (authoring), CC-SCAN-STACK (capability layer, Phase
T0). Authority order, inherited: **trust > layout law > lifelikeness > speed.**

## Intent

Spell Picture works — the turtle proves it. What it lacks is a *bank*: a
tiered library where the picture itself is the difficulty curve, in every
language. Two ideas govern everything below.

- **Word difficulty maps to path prominence.** Easy, frequent words earn the
  big outline strokes; rare, hazard-heavy words earn the fine detail. Anyone
  finishes the silhouette; only strong spellers earn the eye glint.
- **Same picture, native-calibrated climb.** The turtle is the same turtle in
  Tagalog and French; the words per layer differ by each language's real
  hazards. Banding lives in word-bank config, never in layout code.

## Features

**1. Tiered picture bank — wave 1 content.** Enough breadth that every session
offers a fresh, tier-appropriate piece; every entry hand-traceable with tracer
v7.x under existing fixtures.

| Tier | Pictures |
| --- | --- |
| Starter (1 session, bold outline) | balloon, mug, kite, snail, mushroom, sailboat, cactus, ladybug, crescent moon + star |
| Intermediate (features layer) | rooster, seahorse, bicycle, hot-air balloon, lighthouse, T-Rex skeleton, violin, windmill, hummingbird at a flower |
| Advanced (texture/flow layer) | wolf howling, koi pair, peacock tail, galloping horse, octopus, oak tree in wind, dragon (containment tree: body > wings > claws) |
| Expert (multi-session, pan/zoom) | Eiffel Tower, Golden Gate Bridge, Taj Mahal, Colosseum, full constellation map, coral reef scene |
| Masterpiece (PD-Art, Mona Lisa's provenance gate) | The Great Wave off Kanagawa (flow-field showcase), Starry Night, Girl with a Pearl Earring, a Monet water-lily pond |

Every source image passes the existing provenance/license CI gate — PD or
Eric-original reference, PD-Art tag recorded in the manifest.

**2. Per-language culture packs.** Same engine, different resonance — and the
STARTALK slide. Wave 1: jeepney + carabao (Tagalog — Paul audits), torii gate
+ Mt. Fuji (Japanese), hanbok + turtle ship (Korean), windmill + tulips
(Dutch), Day of the Dead calavera (Spanish). Culture packs are **additive**
per language; every language still gets the full core bank.

**3. Layer ladder.** The climb is visible in the art itself. Four layers per
picture: outline → features → texture → shading. Starter pictures may declare
fewer; masterpieces declare all four. A layer unlocks only when the previous
one is complete. Each layer pulls exclusively from its declared word band.
Layer completion persists across sessions — expert and masterpiece pieces are
multi-session by design.

**4. Per-language hazard banding.** Difficulty means *this* language's
difficulty. Bands are defined per language in word-bank config: frequency
baseline plus hazard weighting — French silent endings and accents, German
compound length, Korean syllable-block density, Spanish b/v and ll/y traps,
Tagalog reduplication, English irregulars. Banding rules never appear in
layout or rendering code. Hazard taxonomy per language is D2.

**5. Audio modifiers as tier gates.** Listening skill climbs with spelling
skill. Starter and intermediate: Replay + Slow. Advanced: Slow removed.
Expert and masterpiece: the word plays once, no Replay. Buttons **hide**, not
disable, at gated tiers. Strings route through the audit gate.

**6. Error economics.** Pressure without punishment at the bottom, stakes at
the top. Starter/intermediate: unlimited retries, no stroke cost. Advanced:
retry allowed, a miss dims the earned stroke until corrected. Expert and
masterpiece: a miss charges the stroke, and erase-to-recover (the existing
repair loop) is the comeback path. **No mechanic ever deletes a different
correct stroke.**

**7. Picture manifest schema — the contract.** Each picture ships a manifest
declaring: id, tier, source + provenance tag, part/containment tree, `layers[]`
with path-ids and words-per-layer × band per language, audio-gate tier, and
completion criteria. The manifest is the single source of truth; CI validates
every manifest against the schema *and* against the traced paths — no orphan
paths, no unassigned words.

## Decisions

| # | Decision | Status |
| --- | --- | --- |
| D1 | Wave-1 scope | **PROPOSED, needs Eric's sign-off**: starter + intermediate complete (18 pictures), 3 advanced (wolf, koi, dragon), 1 expert (constellation map — cheapest to trace), 1 masterpiece (Great Wave), Tagalog culture pack first for Paul's audit. Rest lands in waves 2+. |
| D2 | Hazard taxonomy ownership | **DECIDED**: hazard weights live in each language's word-bank config next to the existing banks. English taxonomy drafted first as the template; non-English taxonomies flagged for native-speaker audit (Tagalog → Paul) before ship. |
| D3 | Pan/zoom dependency | **DECIDED**: expert + masterpiece require the pan/zoom canvas. If it is not built, wave 1 ships those manifests and traces but gates them off with the existing feature-flag pattern. If pan/zoom is found **partially** built, stop and ask rather than finishing it under this file. |
| D4 | Multi-session persistence | **DECIDED**: layer/stroke completion saves per picture per language profile, survives restart, and is included in the existing progress-reset flow. |
| D5 | Difficulty monotonicity is law | **DECIDED**: within any picture × language, band difficulty strictly increases per layer. CI-enforced (Done #3); a violating manifest fails the build, no override flag. |

## Constraints and non-goals

- Do not touch scoring logic, the v6/v8.1 solver, junction margins, or overlap
  gates. This file adds content, config and gating — **zero layout-law
  changes**.
- No photo-upload work, no CC-PHOTO-PICTURE execution, no Vision/scan pipeline
  in the app target. CC-SCAN-STACK gating stands.
- No new fonts, no new rendering features. If a picture seems to need a
  rendering capability that doesn't exist, **cut the picture from wave 1 and
  log it** — do not build the capability.
- All player-facing strings (tier names, gate explanations, rejection text)
  route through the existing audit gate.
- Kid Mode: masterpiece portraits and the calavera are Eric-review before
  appearing in Kid Mode rotation; default them out pending review.

## Done when

1. `manifest validate` passes for every wave-1 picture: schema-valid,
   provenance tag present, every traced path assigned to exactly one layer,
   every layer's word count met in every shipped language.
2. Tracer fixtures: every wave-1 picture renders through the v6/v8.1 solver
   with zero layout-law violations at all four device classes.
3. Monotonicity CI (D5): for every picture × language, mean **and** max band
   difficulty strictly increase per layer — build fails otherwise.
4. Audio-gate test: automated UI test confirms Slow absent at advanced, Replay
   absent at expert, both present at starter.
5. Error-economics test: a scripted expert session shows miss → stroke charged
   → erase-to-recover restores it; a scripted starter session shows unlimited
   retries with no stroke loss; no test ever loses an unrelated correct stroke.
6. Persistence test (D4): complete layer 1 of the constellation map, kill the
   app, relaunch — layer 1 intact, layer 2 unlocked.
7. Tagalog culture pack marked "pending Paul audit" until his sign-off is
   recorded; CI blocks its release flag until then.
8. Great Wave passes the same provenance evidence check as the Mona Lisa row —
   PD-Art source URL + tag in the manifest.

---

## Where this file meets what already exists (2026-07-31)

- The bank today is **20 pictures**, all scan-locked, all in the shipping mode.
  Six of the wave-1 names are already traced and passing: snail (starter),
  peacock tail and dragon (advanced), Eiffel Tower (expert), Mona Lisa
  (masterpiece), plus the horse, now re-authored standing. Wave 1 is therefore
  not a from-zero build.
- There is **no tier ladder in the data yet**. Scans carry a single `tier`
  string used only to pick a word pool; there are no `layers[]`, no bands, no
  per-language word counts, and no completion state. Feature 7's manifest is
  the real first task — everything else in this file hangs off it.
- Provenance CI exists (`scripts/wordpic-check.mjs` validates schema +
  provenance against `content-pipeline/wordpic/ref/provenance-f8.json`), which
  is what Done #8 extends rather than invents.
- **Pan/zoom is not built at all** — not partially, so D3's stop-and-ask does
  not trigger. Expert and masterpiece ship flag-gated.
