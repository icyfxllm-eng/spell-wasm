# CC-ZH-PINYIN-DISPLAY v1

**Status:** F0 recorded, **F1 MERGED**. F3 (font coverage) and F4 (span/grapheme)
outstanding. F6 (verdict fidelity) outstanding — see below. Awaiting Eric's
device pass, which is what closes this file.

**Trigger:** Aug 2026 device screenshot — the zh reveal for 游戏 rendered `you` +
a detached mark and `xi` + a detached mark instead of `yóu xì`.

---

## F0 RESULT — runtime dump, 2026-08-29

Emitted by `game::zh_display_f0`, from the runtime value handed to the text
view, not the source literal.

    rendered: "you´xi`"
      'y'  U+0079  segment
      'o'  U+006F  segment
      'u'  U+0075  segment
      '´'  U+00B4  MARK (separate <sup>)
      'x'  U+0078  segment
      'i'  U+0069  segment
      '`'  U+0060  MARK (separate <sup>)

**TWO decision-table rows matched, not one.**

  * *spacing accents emitted* → F1. `TONE_MARKS_DISPLAY` was a table of
    spacing accents: U+00AF, U+00B4, U+02C7, an ASCII backtick U+0060, U+02D9.
    The backtick is not even in the set the table names.
  * *mark codepoint in a different span from its base letter* → F4. The mark
    was rendered in its own `<sup class="zh-tone">` element.

**The mark was never on the wrong vowel. It was never on a vowel at all.**

Both stop-and-ask conditions were met and were reported before any edit:

  * **(a)** fixing F1 masks F4 — precomposed output leaves no separate mark to
    split, so F4's symptom vanishes while its rule stays untested.
  * **(b)** THE DISPLAY STRING WAS BUILT BY A PATH THAT DID NOT GO THROUGH ANY
    CANONICALIZER. There was no `PinyinKey → display` function anywhere in the
    codebase. F1 asks for "exactly one function"; that function did not exist
    to be fixed. The fix was a merge.

---

## F1 — Precomposed Display Law: MERGED

`pinyin::display_syllable(segment, tone)` and `pinyin::display_key(key)` are
now the ONE conversion from a syllable to display text. `PRECOMPOSED` carries
all 48 marked vowels plus bare ü/Ü; uppercase is included per D5 even though no
surface emits it today.

`TONE_MARKS_DISPLAY` is retired to `#[cfg(test)]` — kept only so the F0
evidence can still name what was wrong.

The reveal calls the one function. A syllable F1 cannot render falls back to
the BARE segment, never to a combining mark: toneless beats illegally encoded.

**Invariant 7 is preserved, and this is why the change is a fix rather than a
deletion.** The `<sup>` existed so colour was never the sole carrier of tone. A
precomposed `ó` carries the tone in the glyph itself — visible in greyscale and
to a colour-blind player — so the redundancy survives, carried by the letter
instead of beside it.

**L1** (`scripts/pinyin-display-check.mjs`, gated + selftested) enforces one
builder, no spacing accents, no combining marks, and no detached mark element.

**L1 IS SCOPED TO OUTPUT, DELIBERATELY.** Its first draft scanned all of `src/`
and failed on `viet.rs`, `keyboard.rs`, `editor.rs` and pinyin's own parser
tables — every one of them correct. Combining marks are REQUIRED on the input
side: the canonicalizer must accept a decomposed `yóu`, and the keyboard's
long-press builds marks to send. A lint that fails the build on correct code
gets disabled, and then it guards nothing. The exhaustive alphabet guarantee
lives where it can be exhaustive — `l1_only_precomposed_reaches_the_player`
runs the whole inventory × 5 tones through `display_syllable`.

## F2 — Tone placement: DONE

a → o → e → last of `i u ü`; tone 5 bare. The spec's seven-row worked table is
a test, verbatim.

## F5 — One form, one key: DONE, and it found something

Round-trip identity `PinyinKey → display → parse → PinyinKey` runs the FULL
pinned inventory × 5 tones. It passes for everything except three syllables:

    biang   nia   rua

**These are NOT display defects.** The segmenter splits all three identically
in EVERY form, digit or mark — `biang2` parses as `bi`+`ang2` exactly as
`biāng` does. They are a pre-existing inventory/segmenter disagreement, and
this file's non-goals forbid touching the canonicalizer's matching semantics,
so they are NAMED in an allowlist that cannot grow silently rather than skipped.

All three are **unreachable from the zh bank** — an exact-syllable scan over
every `ZH_*` entry finds zero uses. (A substring scan disagrees and is wrong:
`nian2` contains "nia" but its syllable is `nian`.) No player can meet one.

---

## Outstanding

* **F3 font coverage** — all 50 codepoints present in the resolved pinyin face,
  no fallback. Not started. If the display font genuinely lacks coverage that
  is a stop-and-ask (D4), not a substitution.
* **F4 span/grapheme** — belt-and-braces under F1 now that no combining mark
  survives to be split, but the spec says keep it and the spec is right: it is
  the failure that returns the moment a mark-bearing script joins this
  component.
* **F6 verdict fidelity** — not yet investigated. The screenshot showed
  `拼错了` on a syllable whose only visible problem was the mark. Now that F1
  has landed, re-run the case: if a correctly-typed `yóu` still grades wrong,
  it is branch A; if it grades `MISSPELLED` instead of `TONE_MISS`, it is
  branch B and a routing bug.
* **Device pass (Done 10)** — 游戏 reads `yóu xì`, marks on their vowels, the
  `i` showing no tittle under its mark, one unbroken typeface. **The suite does
  not close this file; the device call does.**

---

## The zh voice — DECIDED, keep Wavenet (Eric, 2026-08-29)

cmn-CN offers 4 Standard, 4 Wavenet, and 30 Chirp3-HD voices. No Neural2, so
Chirp3-HD is the only newer tier. Both Wavenet and Chirp3-HD honour
`<phoneme alphabet="pinyin">`, verified against the API rather than assumed, so
the choice was free of the forced-reading constraint.

Eric listened to both on identical content -- 妈麻马骂 (four tones, one
syllable), 你好, 孩子, 重, 游戏 -- and chose **A, `cmn-CN-Wavenet-A`**, which is
what already ships. No change.

Recorded because "newer tier" is a trap here: Chirp3 returned 19KB against
Wavenet's 47KB for the same content, and a more natural voice can be a WORSE
teacher of tone if its contours are less exaggerated. Anyone proposing the
upgrade later should re-run the listen, not read the version numbers.
