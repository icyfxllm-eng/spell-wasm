# Sequenced plan — word counts + correct rendering for ru / ar / fa / ur

**Written 2026-07-16.** Companion to `docs/DECISIONS-PENDING.md`. This is the
"how", that one is the "what's blocked".

---

## The one-paragraph version

**Rendering is nearly solved and cheap. Content is blocked on a decision nobody
has made.** The browser already renders Arabic correctly — the app breaks it by
wrapping every letter in its own `<span>`, which shatters cursive joining. Stop
splitting, and it renders; the Phase 0 spike proves it in ~30 lines. What actually
blocks everything is that **no language here has a source of record**, and the word
pipeline cannot build a language without one.

Two tracks. They are **independent** and should run in parallel — word lists,
tiers, TTS and definitions are all direction-agnostic data (CC-NEW-LANG-CONTENT
D2 says so explicitly), so ar/fa/ur content can be built while RTL is still
unsolved.

---

## Track 1 — Word counts

### How the pipeline actually works

Curation is human. The source's job is to **prove every curated word is real**.

```
sources/<lang>/fetch.sh          pinned tarball + sha256 (never scrapes)
        ↓ unmunch
sources/<lang>/surface-index.txt every valid surface form   (es: 951,893)
        ↓
assets/words/<lang>/{easy,medium,hard,expert}.txt   ← humans write these
        ↓ provenance-validate: every curated word must EXIST in the index
wordlists/<lang>.provenance.json  (es: 202 curated · 196 backed · 6 reviewed
                                   exceptions · 0 genuine misses)
        ↓ scripts/build-wordlists.py
src/word_data.rs                  @generated — never hand-edit
```

**Target size:** the ready-language median is **41 / 42.5 / 41 / 44** per tier,
totals 160–202. So ~40–50 words per tier, ~200 total. That is deliberately
hand-sized — "increasing the word count" is not a scraping problem, it is a
curation problem with a machine-checked provenance backstop.

**Four gates fail the build:** charset (every character typeable on that locale's
keyboard), exclusions, balance (±20% of English's 50), determinism. Plus NFC,
alphabetic-only, length 2–16 per word.

### The dependency nobody has noticed

**`scripts/build-wordlists.py` cannot build a language without
`assets/keyboards/<code>.json`.** The charset gate reads it to learn which
characters are typeable — `reachable_chars()` opens that file unconditionally.

So **ar/fa/ur content is blocked on keyboards**, which CC-NEW-LANG-CONTENT lists
as a non-goal ("NO RTL rendering, input, **keyboard**, or UI mirroring work") and
CC-RTL D8/F5 owns — behind the unfinished P0.3 gate. Content is supposedly
direction-agnostic, but the *build* is not.

> **Proposal (needs Eric).** Split the keyboard into two things that are currently
> conflated:
> 1. **A charset declaration** — `assets/keyboards/ar.json` listing which letters
>    exist. Pure data. Unblocks the content pipeline immediately. No rendering, no
>    input handling, no RTL.
> 2. **RTL input handling** — CC-RTL F5, stays where it is.
>
> This trespasses CC-NEW-LANG-CONTENT's non-goal as written, which is why it is a
> proposal and not a fait accompli. But without it, **no ar/fa/ur word can be built
> until CC-RTL F5 lands** — and that inverts the spec's own claim that their
> content is direction-agnostic.

### Sequence per language

| Step | Russian | ar / fa / ur |
|---|---|---|
| 1. Pick a source + license verdict | **BLOCKED** — `DECISIONS-PENDING` §4 | **BLOCKED** — same, ×3 |
| 2. `sources/<lang>/` + registry entry | after 1 | after 1 |
| 3. Surface index; **report ё / ZWNJ coverage** | after 2 | after 2 |
| 4. Keyboard charset declaration | ✅ **done** (`50f0909`) | **BLOCKED** — see proposal above |
| 5. Curate ~200 words, tiered per D3 | needs a native speaker | needs a native speaker ×3 |
| 6. Provenance-validate to 0 misses | after 5 | after 5 |
| 7. Trap quotas | **BLOCKED** — RAFU D6 undecided | same |
| 8. TTS voices cached | ru-RU | ar / fa-IR / ur-PK; **stop and report if no acceptable voice** (D5) |
| 9. `LANGS` + `tier_for` arm + build | after 8 | after 8 |
| 10. Audit (Gig A/B) → activation | Eric commissions | after CC-RTL ships |

**Russian is one decision away from being buildable.** Its keyboard is done, it is
LTR, nothing about RTL touches it. Pick the source and steps 2–9 are mechanical.

---

## Track 2 — Rendering (ar / fa / ur)

### Why it's closer than it looks

The OS text stack does shaping and bidi **for free**, if you feed it canonical
codepoints and never manipulate strings visually. The app currently does the one
thing that breaks it:

```rust
// game.rs — render_letters
for (i, ch) in value.chars().enumerate() {
    html.push_str(&format!("<span class=\"ltr {}\">{}</span>", pop, ...));
}
```

Every character in its own element. Shaping engines join within a text run;
separate elements break the run. Measured: Arabic renders **wider in 40/40 cases**,
median **+26.6%**, worst **+55.7%** — كتاب becomes **ك ت ا ب**.

**The fix is ~30 lines.** One text node, markers positioned beneath from
`Range.getBoundingClientRect()` over grapheme clusters. Joining intact by
construction. The spike renders it correctly today (`spike/rtl-feedback`).

### Sequence

| Phase | What | Gate |
|---|---|---|
| **P0.3** | Approach **A** + the unify-or-parallel call | **← ERIC. Everything below waits.** |
| **F1** | `direction` in the registry; play surface reads it; `rtlSupported` exposed (still false) | — |
| **F2** | CSS logical-properties sweep + a lint so physical props can't return | Visual regression: LTR **pixel-identical** |
| **F3** | `<bdi>` / isolation at every embed point | Punctuation lands correctly in both embedding directions |
| **F4** | Implement A for reveal, per-letter correctness, Kid Mode | Joining-integrity suite green at all sizes |
| **F5** | Keyboards (D8) — **also unblocks Track 1 step 4** | Full alphabet enterable; ZWNJ + hamza through the core |
| **F6** | Fonts (D5), full-mode harness, then **flip `rtlSupported`** | Every mode green on real content |

Then, **separately**, each language's audit. The flag enables *renderability*;
audits enable *activation*. Both are required.

### The three risks worth naming now

1. **Unify-or-parallel is a UX change to English.** A is script-agnostic, so it
   *can* delete the per-letter DOM for every script — which is less code than
   today. But the per-letter `pop` animation dies, colouring moves beneath the
   word, and Korean's per-jamo coaching needs re-siting. English is your only
   Active language and 100% of current players. That is the real content of P0.3.
2. **Nastaliq (Urdu).** D5 already says stop and ask with evidence rather than
   ship Urdu in Naskh. Nastaliq is taller and heavier; whether its metrics break
   marker placement is untested — the spike ran on macOS system Arabic.
3. **ZWNJ (Persian).** CC-RTL D7's policy is still *proposed*. It mirrors ё
   exactly — store the canonical form, accept what people actually type — so
   deciding ё well settles ZWNJ cheaply. Until then, ZWNJ-requiring words are held
   under `zwnj-pending` in the trap registry.

---

## What to do first, in order

1. **`DECISIONS-PENDING` §1** — two lines make the license gate green. Minutes.
2. **Copyleft posture** (§4.1). May collapse the source choice to one candidate.
3. **Russian's source.** Unblocks Track 1 entirely for ru. **Let ё decide it, not
   the licence** — a ё-less index makes every canonical ё-form a `genuine_miss`,
   and that must be zero.
4. **P0.3.** Unblocks all of Track 2, and via F5, Track 1 for ar/fa/ur.
5. **The keyboard-split proposal** above — or accept that ar/fa/ur content cannot
   start until F5.

Steps 1–3 are Track 1. Step 4 is Track 2. **They do not block each other** — the
only crossing wire is F5 → keyboards → the charset gate, which the proposal in
Track 1 exists to cut.

---

## What I'd estimate, and what I wouldn't

Honest about the difference:

- **Confident:** the rendering fix itself is small — the spike is written and
  measured. F2's CSS sweep is mechanical but wide. Russian steps 2–9 are a known
  quantity because `es` is the worked example.
- **Not confident, and won't pretend:** curation time for four languages by native
  speakers; whether an acceptable fa/ur TTS voice exists at the quality bar (D5
  says report rather than silently pick a bad one); whether Nastaliq co-operates.
  Those are discovery, not estimation.
