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

`scripts/license-gate.mjs` currently **fails**. `en` and `th` are Active with
`permitted_use: UNKNOWN`, so the build stops. 14 other languages warn.

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
for `en` + `th`, the gate exits 0.

`node scripts/emit-license-table.mjs` renders the manifest for proposal
appendices. It currently prints, accurately: *"16 of 16 sources are UNKNOWN — this
table is not yet a claim that can be made in a proposal."*

**Branch:** `feature/wordlist-sources` (`82397b8`).

---

## 2. Turkish — this reverses your own call from a day earlier

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
