# Decisions pending — Eric

**Written 2026-07-16.** Everything below is blocked on you. Nothing here is blocked
on engineering.

Nine branches are pushed. All gates are green except one, which is red **on
purpose** (§1). Every claim in this document cites the file or commit it comes
from — if something looks wrong, the citation is where to start.

**Read §1 first. It costs you two lines and it is the sentence you are selling to
school districts.**

---

## 1. The license gate is red, and two lines fix it

`scripts/license-gate.mjs` currently **fails on four things**, not two — see §10;
the gate learned to see text it was previously blind to. `en` and `th` are Active
with `permitted_use: UNKNOWN`, and **`definitions` and `enrich` are text the app
puts on screen right now with no recorded licence at all.** 14 other languages
warn.

That red is the deliverable, not a bug. The license addendum's pitch is, verbatim:

> "every word list traces to a documented source whose license permits this use,
> and the build fails otherwise"

Before this week that was true for **1 of 16 languages**, and the build did **not**
fail — the old gate only checked `wordlists/*.txt`, and `es` is the only one that
exists. `assets/words/<lang>/`, where 15 languages' shipped banks actually live,
was never checked at all.

Those 15 aren't undocumented. `assets/words/LICENSES.md` records them as *"Original
curation for this app — Owned — ships freely — Hand-authored common vocabulary."*
The story exists; it just wasn't anywhere a machine could read, which is exactly
the gap between policy and enforcement the addendum exists to close. They now have
registry entries with `kind: "original"`.

**What I need:** you own that curation, so the verdict is yours to state. Set on
each original-curation entry in `sources/registry.json`:

```json
"permitted_use": "PERMITTED",
"verified_by": "eric",
"verified_date": "2026-07-16"
```

I cannot do this. The addendum is explicit that `verified_by` must name a person
and that only a human edit promotes a verdict. I probed it: with those fields set
for `en` + `th` **and the two `displayed_content` entries in §10**, the gate exits
0.

`node scripts/emit-license-table.mjs` renders the manifest for proposal
appendices. It currently prints, accurately: *"16 of 16 sources are UNKNOWN — this
table is not yet a claim that can be made in a proposal."*

**Branch:** `feature/wordlist-sources` (`82397b8`).

---

## 2. Turkish — ✅ ANSWERED 2026-07-17: cut permanently

> **RESOLVED — no sign-off needed; this section is history.**
>
> CC-HINDI-PHASE0 **D1** cuts Turkish permanently (Hindi replaces it), restoring
> your original call in `5fc69ff` and overriding CC-LINEUP-SWAP D7. Actioned in
> `433baeb` on `feature/ru-parity`: `tr` is out of the registry (**15** languages),
> out of the map (Turkey unmapped, like India), and its content is archived under
> `archive/` beside the cut four.
>
> It also vindicates the original diagnosis: CC-LINEUP-SWAP predates the cut and
> was never rebased onto it.
>
> **Two consequences.** `feature/minimal-pair-candidates` deleted Turkish
> candidates — I flagged that as a risk if Turkish returned; it was **right** all
> along, and needs no repair. And CC-LINEUP-SWAP's "exactly 16 languages"
> done-criterion is now **unmeetable**: 16 only ever reconciled *with* Turkish. The
> registry is 15 and the snapshot test says 15, deliberately.
>
> The original analysis follows, for the record.

### (superseded) This reverses your own call from a day earlier

`ed6964f` puts `tr` back into the registry, the country map, and the audit round,
because **CC-LINEUP-SWAP D7** lists it among the 11 content-ready languages.

That reverses **`5fc69ff` (2026-07-15)**:

> *"Thai and Turkish don't fit the game, and cutting them trims the audit load."*

One day apart. The tell is arithmetic: CC-LINEUP-SWAP requires **exactly 16**
languages, and 16 only reconciles **with** Turkish present (`en` + 11 ready incl.
`tr` + 4 new = 16). The spec appears to predate your cut and was never rebased onto
it — the same way several other files below predate it.

**What I need:** confirm or reverse. It's flagged under a `*** REVERSAL ***` banner
in the commit body so review can't miss it.

**Knock-on if Turkish stays cut:** two branches assumed it was gone —
`feature/minimal-pair-candidates` has a commit *"remove Thai + Turkish candidates
— languages cut"*, and `feature/script-paths-curricula` says *"14 languages"* when
the registry now has 16. If Turkish returns, the first one deleted work you need.

**Branch:** `feature/ru-parity` (`ed6964f`).

---

## 3. Two specs contradict the code — they need rebasing, not deciding

I did **not** act on either.

### CC-NEW-LANG-CONTENT D4 — the Spanish premise is backwards

D4 says writing `س` for `ص` should be wrong *"exactly like a Spanish b/v error
would be"*, then instructs: verify against how Spanish actually behaves, and **stop
and ask if Spanish accepts them**.

Spanish accepts them. `src/homophones.rs`:

> *"in Spanish, `b/v`, a silent `h`, seseo and yeísmo all sound identical, so a
> learner who spells a real homophone of the prompt word should not be marked
> wrong for a difference their ear could never catch (decision addendum, 2026-07)."*

Live in grading — `game.rs`, in the answer-matching branch (`homophones::accepts`). The analogy D4 rests on is inverted. The stance
it wants for ru/ar/fa/ur may still be right — "the traps ARE the game" is a
defensible product call — but it can't be justified by a Spanish comparison that
says the opposite.

### CC-NEW-LANG-CONTENT F1 — Russian's activation is circular

F1 says Russian *"activates into every game mode now."* The same file says
Russian's Gig A/B audit pair is commissioned *"when F1 is green."* So Russian ships
to players **before** the audit that gates it — and English is currently the only
Active language, held that way for the English-only App Store launch.

Russian stays `ComingSoon`.

### CC-WORDLIST-RAFU can't start

It forbids introducing new sources while requiring every word to come from a
per-language source of record that exists only for `es`. Its F3 (the trap registry)
is done and shipped; the rest is blocked behind §4.

---

## 4. Russian's source of record

Full analysis: **`docs/russian-source-options.md`** on `feature/ru-parity`.

Four questions, in the order that collapses the decision fastest:

1. **Copyleft posture.** Your own registry already records this as yours:
   > *"shipping copyleft-derived data inside a closed binary is a posture decision
   > for Eric, not this pipeline."*

   `es` is Tier B (GPL/LGPL/MPL) and ships in a closed App Store binary **today**.
   Russian doesn't create this question, but the license addendum forces it: the
   claim in §1 and an unresolved copyleft posture cannot both stand. **Answering
   this may collapse the source choice to one candidate.**

2. **The source.** Hunspell `ru_RU` (same pipeline shape as `es` — pinned tarball,
   unmunch; reported BSD, which would be Tier A and sidestep §4.1), OpenCorpora
   (CC BY-SA, marks ё well, needs new extraction), or Wiktionary/kaikki (CC BY-SA).
   Every licence is **reported, not verified** — I fetched nothing.

3. **A verifier** — `verified_by` must name a person. Same constraint as §1.

4. **Length rules** — ingest gates at `3..15`, `build-wordlists.py` at `2..16`.
   Harmless for `es`; Russian's inflection makes the boundary livelier.

**The finding that should actually drive the choice is ё**, not licence. D4 requires
canonical forms to store `ё`, but everyday Russian writes `е` and many corpora
follow. A ё-less surface index makes every canonical ё-form a `genuine_miss` — and
that must be zero. Spanish needed 6 exceptions out of 202; Russian could need
dozens, at which point the source isn't backing the content.

---

## 5. CC-RTL P0.3 — the prototype is done and waiting

**`spike/rtl-feedback/FINDINGS.md`** + a screenshot. Branch `spike/rtl-feedback`
(`c56fea3`), never merged, per the spec's non-goals.

**The blocker, quantified.** Today's per-letter `<span>`s (`render_letters` in `game.rs`, line 250 on `build-54`) render
Arabic **wider in 40/40 cases** — median **+26.6%**, worst **+55.7%**. That width
overhang *is* joining breaking: كتاب renders as **ك ت ا ب**, four disconnected
letterforms. Unreadable, not merely ugly. This is what CC-LINEUP-SWAP D2's
unconditional gate has been protecting you from.

**Recommendation: approach A** — one text node, markers beneath positioned from
`Range.getBoundingClientRect()`. Joining intact by construction, markers exact by
construction, ~30 lines. **B fails**: canvas has no per-glyph geometry, so cluster
boxes come from prefix measurement, which re-shapes each prefix's final letter —
median max error **11.15px**, worst **29.10px**, >2px misplacement in 40/40 cases.
At 32px that points the marker at the *wrong letter*. B also renders text to
canvas, invisible to screen readers — for a children's product sold to
institutions, close to disqualifying, and the spec doesn't mention it.

**Your call — the one I deliberately left open:** A is script-agnostic, so should
it replace the per-letter DOM for **every** script, deleting the special-casing
rather than growing a parallel path? The evidence supports the spec's instinct. But
it's a visible change to English, your only Active language and 100% of current
players: the per-letter `pop` animation dies, colouring moves beneath the word, and
Korean's per-jamo coaching needs re-siting. No capability is lost; the *feel*
changes.

Nothing in RTL Phase 1+ starts until you answer.

### 5.1 Urdu is the exception — approach A cannot position markers under Nastaliq

Spike run since the above (`spike/urdu-nastaliq/` on `feature/rtl-feedback`,
`FINDINGS.md` + `boxes-chromium.png` / `boxes-webkit.png`). **First, the thing
people assume and get wrong: Urdu RENDERS FINE.** The screenshots show it in
correct Nastaliq — joins, cascade and all. Nothing here says Urdu can't be
displayed. What it says is narrower and only about the **§5 feedback marker**.

Approach A places each marker from `Range.getBoundingClientRect()` — i.e. from
where the browser says each letter sits. That works for Arabic and Persian
(**Naskh**: letters keep one horizontal baseline). It does **not** work for Urdu
(**Nastaliq**: letters cascade diagonally down-and-left and overlap). I tested the
one API that might have rescued it — SVG `getExtentOfChar` — in **both Chromium and
WebKit** (WebKit because production is iOS WKWebView), against the real Noto
Nastaliq Urdu font:

- The **vertical cascade is invisible** to both engines: every character's box
  comes back at the same `y` and the same (line-box) height. The browser measures
  against the text baseline, not the glyph, so the very thing that makes Nastaliq
  Nastaliq is not in the data. `Range` behaves identically.
- The **horizontal boxes are unusable in WebKit** specifically: they overlap and
  fall out of order (پاکستان: five overlaps), so a letter cannot be mapped to a
  distinct marker slot. The overlay screenshots show boxes that are tall vertical
  strips ignoring the diagonal ink.
- This is a limitation of the browser text model (a horizontal run on one
  baseline), not a bug — no text-geometry API (canvas, `Range`, `getExtentOfChar`)
  exposes a glyph's true 2D position.

**So Urdu forces a separate decision from ar/fa, whichever way you answer §5:**

- **A — word-level feedback for Urdu.** Per-letter markers for ar/fa; Urdu shows
  correctness on the whole word. Ships now, costs per-letter precision for Urdu
  only. *My recommendation, unless per-letter Urdu feedback is a hard requirement.*
- **B — render Urdu in Naskh, not Nastaliq.** Makes per-letter work (Naskh is
  horizontally separable) and the Naskh font is already bundled — but Urdu readers
  strongly prefer Nastaliq and Naskh Urdu reads as wrong to many. A cultural call,
  not a technical one.
- **C — HarfBuzz in wasm.** Shape the text ourselves and read real glyph positions.
  The only path to true per-letter Nastaliq markers. Feasible (the app is already
  wasm) but a real project, not a tweak.

Two facts that bound this: the app does **not yet bundle a Nastaliq font** (Naskh
was bundled for ar/fa only, `d7fcbcf`), so shipping Urdu at all first needs a
Nastaliq font on the device or in the bundle; and this whole item is moot until
`RTL_SUPPORTED` flips — Urdu can't be selected today.

**Related and also pending: the keyboard split.** `feature/rtl-feedback` also
carries `docs/rtl-keyboard-split.md` — a separate one-decision write-up on whether
ar/fa/ur word-bank content may start against the new charset declarations before
the RTL input half of F5 lands. Belongs next to this when the branches converge.

---

## 6. Smaller sign-offs

| # | Decision | Where | Default I took |
|---|---|---|---|
| 6.1 | **CC-MODE-HUB D1** — does `tools_hub` ("Pillar 3", shipped build 55, 4 E2E tests) retire in favour of `modes.json`? | `feature/ru-parity` | **Kept it.** Deleting a days-old feature on a reconstructed spec's say-so needs you. If it does retire, it should *read* `modes.json`, not duplicate it |
| 6.2 | **CC-MODE-HUB D6/D7** — tile order = file order; no notify-me hook on `coming_soon` | `config/modes.json` | Implemented as written, still marked RECONSTRUCTED |
| 6.3 | **License addendum structure** — I used the existing manifest/provenance + `credits.json` instead of a separate `licenses.json` | `feature/wordlist-sources` | Adopted per operator's call; deviation recorded in the registry's `$licenseAddendum` |
| 6.4 | **RAFU trap quotas (D6)** — per-tier trap-class minimums | `config/trap-registry.json` | `UNDECIDED`. The tagger reports coverage but cannot fail a build on a quota nobody approved |
| 6.5 | **Education bundle ID** + lifting the fastlane freeze | — | Not started. The *name* is permanent once registered; needs an App Store Connect record only you can create |
| 6.6 | **Word Stories** — CC BY-SA attribution + a kid-appropriateness spot-check of the en/es strings | `docs/word-stories-review.md` (`37e5a0f`) | Stays `hidden`. Its own doc says the attribution blocker is now solvable via the credits infra built in §1 |

---

## 7. Two things worth knowing (no decision needed)

**Education edition is byte-identical to consumer today**, for languages. English
is the only audit-passed language and FREE_TIER already gives it Full; everything
else is audit-gated, and CC-EDITIONS D3(a) says those resolve exactly as in
consumer. That isn't a bug — it's what "editions never bypass audit gates" *means*
while one language has passed audit. What education actually buys today is the
parent-premium set and the absence of purchase surfaces.

**A bug that nearly shipped.** Adding Russian's Cyrillic keyboard broke the build:

```
locale ru: char 'b' in "bed" not reachable on keyboard
```

`tier_for` ends in `_ => en_tier(tier)`, so all four new languages were silently
inheriting the **English** word bank. Invisible while Russian had no keyboard,
because English words are typeable on English keys. It would have shipped as
"Russian" that spells *bed*. Fixed at the source in `50f0909`; two tests pin it.

---

## 8. The audit-model amendment — one decision wearing two hats

**CC-SWAHILI-WORDBANK D3** and **CC-BANK-EXPANSION D2** are the same proposal at
two scales, and both demand explicit sign-off *separate from approving the file*.
Neither may be nodded through.

**What they change.** Today's doctrine: *audit verdicts attach to CONTENT in the
source-of-record* (CC-EDITIONS D5). The amendment: for generated/expanded forms,
verdicts attach to

  (a) the lemma base,
  (b) **the mechanism itself** — the generator rule set or affix ruleset version,
      reviewed as a document with worked examples per rule, and
  (c) a random sample (500 forms, or 5% — whichever is smaller), uniform across
      rules and tiers, decoys planted per the standing protocol.

Reject rate >2% fails **that language's entire expanded set** and returns the
mechanism to revision.

**My read: they are right, and the honesty is the point.** Nobody reviews 30,000
rows. A doctrine that says they do is theatre, and theatre is worse than a smaller
claim honestly enforced — it is exactly the gap between prose and machine-checked
fact that §1 exists to close. Making the *rules* audited content is a real answer:
a rule is reviewable in a way 30k rows are not, and it is where the errors
actually live.

**But it is your call, and it is load-bearing**, because it changes what "audited"
means in a sentence you say to school districts. Two things worth weighing:

- The sample is the only thing standing between a bad rule and 30k bad words. A
  2% threshold on 500 forms means ~10 bad rows pass unnoticed per language.
- A generated form can be *well-formed nonsense from a real lemma* — the exact
  failure no license or license-gate catches, because the provenance is perfect.
  `swahili_gen`'s "unattested noun class generates nothing" test exists for
  precisely this.

**Consequence if you reject it:** CC-BANK-EXPANSION says so itself — every
UNMUNCH/GENERATE language shows BLOCKED and only COLLECT work proceeds. That is a
coherent outcome, not a disaster.

**Status:** the Swahili generator is drafted flag-off (`3d121cf`,
`feature/swahili-gen`) as D1 permits. It executes on nothing. Its rules all carry
`NeedsNativeAudit` and a test forbids any of them claiming otherwise.

---

## 9. CC-BANK-EXPANSION contradicts the file it declares itself subordinate to

It says it is *"Subordinate to CC-WORDLIST-SOURCES (license gate,
**one-source-of-record**, no-scraping)"*. Then F1 proposes **"source-of-record
additions (e.g. OpenCorpora for Russian, SCOWL for English)"**.

`sources/registry.json` states the rule verbatim:

> **"D2: exactly ONE source per language, no merging."**

Those cannot both hold. An "addition" to a language's source of record *is* a
second source, which is *is* merging. This needs resolving before F1's profile
table is written, because the table is declared the single source of truth for
what runs — and it would be encoding the conflict.

**It also pre-empts §4.** Classing `ru` as "OpenCorpora-augmented" quietly answers
the Russian source question (§4.2) — a decision still open, whose right answer I
argued should be driven by ё fidelity rather than convenience. OpenCorpora is one
of the three candidates in `docs/russian-source-options.md`; it may well be right.
But it should be *chosen*, not inherited from an expansion file's example
parenthetical.

**Two smaller notes for when the table is written:**

- **Wave 1 is `es` + `pl`, and `es` is Tier B (GPL/LGPL/MPL).** Unmunching rla-es
  produces *more* copyleft-derived data — so Wave 1 runs straight into the
  unresolved copyleft posture in §4.1. Worth answering that first, or Wave 1
  produces a bank nobody has decided they can ship.
- **`zh` is classed COLLECT, and it has no `assets/words/zh/` at all.** Mandarin's
  banks live in `words.rs` as `pinyin|hanzi` pairs, deliberately outside the
  pipeline. Any tooling that globs `assets/words/*/` scores it zero — mine did,
  until I checked. The profile table should say so explicitly or `zh` will look
  like a gap that needs filling.

---

## 10. The app is showing text whose licence is recorded nowhere

**Found 2026-07-17 while answering "how do I get definitions in the other
languages".** It turned out to be a bigger question than it looked.

### What is on screen now

| | Source | Reachable | Licence |
|---|---|---|---|
| **definitions** | `dictionaryapi.dev` | **shipping, every round** | **unrecorded** |
| **enrich** (meaning-card insights) | unrecorded — believed original curation | **shipping** | **unrecorded** |

Definitions are fetched at runtime and rendered on the meaning card after every
answered word. They appear in **no licence record anywhere** — not
`sources/registry.json`, not `assets/words/LICENSES.md`, not any doc. The gate
could not see them because every check scanned *files*, and a runtime API call is
not a file. That is now fixed (`1c310a5`): `displayed_content` is in the registry
and both entries fail the build.

### VERIFIED 2026-07-17 — it is Wiktionary, and it says so itself

I asked the API. This is its own response for `yacht`:

```json
"license":    {"name": "CC BY-SA 3.0", "url": "https://creativecommons.org/licenses/by-sa/3.0"}
"sourceUrls": ["https://en.wiktionary.org/wiki/yacht"]
```

**So the app ships CC BY-SA Wiktionary text to children in every round** — the
exact thing Word Stories is held dark for:

> *"Wiktionary text is CC BY-SA, so it stays dark until the attribution approach is
> approved."* — `docs/word-stories-review.md`

The two are provably inconsistent. This is **not a new risk; it is an existing gap**,
and Word Stories has been gated for a licence exposure the app already takes
everywhere else, all day.

**And the attribution is being thrown away.** `license` and `sourceUrls` sit at the
top level of every response. `backend/app.py` reaches *past* them into
`meanings[0].definitions[0]` and keeps only `pos`, `definition`, `example`. The API
hands us precisely what CC BY-SA requires and the code discards it — which also
means the fix is small: what you need is already in the payload, and
`gen-credits.mjs` already knows how to surface a Tier-B credit.

`enrich` is likely easier: each row has a `verified` flag and release builds render
only `verified: true`. But that is an **accuracy** verdict, not a licence one — "an
auditor approved this sentence" does not say who owns it. If it is original
curation, promoting it is one edit, like the word banks.

### `def_lang()` — I got this wrong twice; here is what it actually is

**CORRECTION.** I wrote here that `def_lang` was never called ("dead code", then
"fiction"). **Both were wrong.** It is live: `game.rs:526` calls it, and the split
is deliberate —

- **English** → `api::fetch_meaning` → our backend `/api/meaning` (cached, maskable)
- **Everything else** → `def_lang` builds the URL and **the browser calls
  dictionaryapi.dev directly**, bypassing the backend entirely.

The code says so: *"still go straight to dictionaryapi.dev, since our backend only
knows English."* The backend does not call `def_lang` because the **client** does.

**What survives the correction:** those endpoints 404. Every language it claims,
tested with unambiguous native words:

| | | | |
|---|---|---|---|
| `en/hello` → **200** | `es/hola` → 404 | `fr/bonjour` → 404 | `de/haus` → 404 |
| `ru/дом` → 404 | `ja/日本` → 404 | `ko/한국` → 404 | `ar/كتاب` → 404 |
| `hi/नमस्ते` → 404 | `pt-BR/casa` → 404 | | |

dictionaryapi.dev serves English only. So the non-English path is **live and
broken**: every non-English word fires a request that 404s, and the card shows
nothing. Deleting `def_lang` expands nothing — it removes a path that already
fails.

(The cheap option I proposed before testing — "wire `def_lang` through the
backend, ~5 lines, covers 9 of 15" — was worthless twice over: the endpoints do
not exist, and the client was already calling them.)

### ⚠ A child's browser called a third party directly — FIXED 3dd633b, but read this

**Fixed on `build-54`, and the fix cost nothing** — but you should know it happened,
because the decision it implies is yours and it outlives the patch.

For **every non-English word**, `fetch_definition` called
`https://api.dictionaryapi.dev/...` **straight from the browser** — the word, plus
the child's IP, to a third party, with no backend in between.

That routes around the reason your proxy exists. From `api.rs`:

> *"Routing through our backend — rather than calling dictionaryapi.dev straight
> from the browser — means the masked hint's network response itself never
> contains the unmasked word."*

English respects that. Nothing else does.

**Why it matters beyond tidiness:**

- It is a children's product. Kid Mode's whole posture is that a child's data does
  not leave the device unnecessarily — Say-It is on-device only "no audio
  off-device, ever", ghost racing is local, and the Education Edition is
  local/unranked for COPPA/FERPA reasons (CC-EDITIONS D7). A direct
  browser→third-party call for every word sits oddly beside all of that.
- It is invisible in the Education Edition's story. A district asking "what leaves
  the device?" would get the wrong answer today.
- It is unrecorded: no privacy policy line, no CSP entry, no note anywhere.

It fired and 404'd, so nothing came back — but the request was still made and the
word still left the device.

**What was done (`3dd633b`):** `def_lang`, the non-English branch, and the local
`urlencode` that served it are deleted. English still routes through the proxy;
everything else returns `None`. **User-visible behaviour is identical** — the path
rendered nothing before (404) and renders nothing now, at zero cost. Verified in
the artifact: `api.dictionaryapi.dev` no longer appears in the compiled wasm (0
hits, against a control string returning 1). The call cannot be made because the
URL is not in the binary.

**Why it is still in this document.** The patch removes the call; it does not
answer the question the call raised:

1. **Nobody decided this.** A direct browser→third-party request for every
   non-English word was not a choice anyone recorded — it accreted, and it survived
   because it lived one branch away from the proxy that exists to prevent exactly
   it. Worth asking what else did.
2. **The Education Edition has to answer "what leaves the device?"** and that
   answer should be written down before a district asks. Today there is no privacy
   line, no CSP entry, and nothing in `LICENSES.md` about definitions at all.
3. **It constrains how §10 gets fixed.** Any route to non-English definitions that
   keeps client-side third-party calls re-creates this, and next time it would
   *work* — the word would leave the device AND come back with text. That is a
   second reason to prefer **`lexicon-ingest`**, independent of coverage: glosses
   ship in the bundle, so there is no runtime call to leak, and `fetch_definition`
   stays exactly as it is now.

**How it was found, because the method matters more than the bug.** I recorded
`def_lang` as uncalled twice in this document — "dead code", then "fiction" — and
both times I had not read its caller. Checking *why* I was wrong is what surfaced
the direct fetch. That is the third claim in two days that survived until it was
tested: the RTL spike's first drift metric returned a perfect 0.00 because it was a
tautology; `--check`'s docstring promised drift protection it structurally cannot
give; and this. **Eleven instruction files have been written against this codebase
this week.** The three that were checkable were all wrong in the same direction —
confidently, and about something nobody had run.

So any plan involving a cross-language definitions audit (CC-DEF-PRECHECK assumes
one) is planning against content that does not exist. CC-DEF-PRECHECK is blocked on
more than that — it patches a Gig A export script that does not exist, over staged
rows that do not exist, produced by an authoring workflow that does not exist.

### Why this is really §4.1 again

Every path to non-English definitions runs through the same gate you have not
opened:

- ~~Wire `def_lang` through~~ — **dead on arrival**, see above. 404s all the way.
- **Extend `lexicon-ingest` to keep per-language glosses** — the pipeline already
  exists and already parses glosses "across every language"; `schema.py` has the
  field. Covers all 15, offline, no runtime API. **But it is CC BY-SA** — the same
  licence the app is already shipping unattributed.
- **Author them** — ~2,800 rows, native speakers. Owned outright.

So it is two options, not three. The first needs the **copyleft posture** (§4.1).
So does Word Stories. So does Russian's source (§4.2). So does `es`, which ships
GPL-derived data *today* — and so, it turns out, do the definitions already on
screen.

**One sentence from you unblocks all of it**, or closes the door and forces
authoring. It is the keystone, and it is why these all feel stuck at once.

---

## Branch map

| Branch | Tip | What |
|---|---|---|
| `build-54` | `868fbd9` | mainline + purity-gate fix |
| `feature/ru-parity` | `3033374` | lineup swap, editions, mode hub, trap registry, Russian keyboard |
| `feature/wordlist-sources` | `82397b8` | license gate + verdict fields (**red by design**) |
| `spike/rtl-feedback` | `c56fea3` | Phase 0 prototype + findings (**never merge**) |
| `feature/climb-shields` | `3f100f9` | salvaged shield forge (**incomplete** — HUD markup + i18n missing; would panic) |
| `feature/app-intents` | `8622986` | Siri intents + widgets |
| `feature/script-paths-curricula` | `9e4c8d4` | draft curricula (**says 14 languages**) |
| `feature/minimal-pair-candidates` | `8d784e4` | candidate lists (**deleted Turkish**) |

Nothing merges without your approval — every spec in flight is review-gated, and
the pipeline freeze is in force.
