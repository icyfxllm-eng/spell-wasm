# CC-SNAP-CLEAN v1 §0 census — read-only, nothing built

Run 2026-09-30 against `main` at `b0e900c7`.

**Two HALTs (C4 and a second half of C1), and one census item I cannot
produce.** C3 came back clean in a way that dissolves the feature it was
meant to inform.

---

## C1 — OCR hand-off: lines are separate. Half a HALT.

**The named HALT does not fire.** `NativeLanguageKitPlugin+PhotoList.swift`
collects one `VNRecognizedTextObservation` per line, keeps
`cand.confidence` per line, and resolves
`["lines": [{"text":…, "confidence":…}, …]]`. The Rust entry point is
`photo_import::extract_classified(lang: &str, lines: &[(String, f32)])`. So
lines reach the core separately, each with its score. Good.

**But there is no per-line `lang`, and F0's `OcrLine` requires one.** The
bridge takes a single language for the whole call —
`let lang = call.getString("lang") ?? "en-US"` — sets it as
`request.recognitionLanguages`, and `extract_classified` likewise takes one
`lang: &str` for the page. Nothing tags an individual line.

That is not cosmetic. It makes two parts of this file unbuildable as written:

- **F1's** "scheme detection runs separately for each contiguous run of
  lines that share a `lang`" has no runs to separate — every line on a page
  shares the one language the picker was opened with.
- **Golden row 19** (`кот`, `замок` as `ru`, then `dog` as `en`, schemed
  separately) cannot be constructed from a real photo, only from a
  hand-built `Vec<OcrLine>`.

Per-line language tagging is CC-SNAP-LIST's to own — this file says so
itself. So this is the same shape as the named HALT: a dependency that has
not landed. **Stopping to ask** rather than inventing a `lang` per line.

## C2 — existing cleanup, every site

| Where | What it does |
|---|---|
| `src/native_lang.rs:394` `parse_candidates` | splits each line on whitespace, NFC, `strip_edges`, drops digit-bearing tokens, drops lone letters outside `ONE_LETTER_WORDS`, dedupes case-folded, caps at 2000 |
| `src/native_lang.rs:381` `strip_edges` | `trim_matches(|c| !c.is_alphabetic())` — the whole edge trim, in one line |
| `ios/.../NativeLanguageKitPlugin+PhotoList.swift:146` `healSplitWords` | rejoins words Vision split across a line break, using UITextChecker as a tiebreaker |
| `ios/.../NativeLanguageKitPlugin+PhotoList.swift:123` | `ChineseScript.toSimplified` on the whole line for zh |
| `src/photo_list.rs:132` | F14's one-tap split of a chip the user chose to split — a UI action, not cleanup |

**Two of these conflict with the spec rather than merely duplicating it, and
deleting them per I8 changes behaviour that ships today:**

1. **`parse_candidates` tokenizes.** It turns each LINE into whitespace
   tokens and returns words. CC-SNAP-CLEAN is line-shaped: one `Candidate`
   per line (F4 splits only on `, ; / |`), with `MultiWord` as a *flag* for
   3+ tokens. Today "Week 3 Spelling List" becomes three candidates and a
   dropped `3`; golden row 14 wants one candidate, flagged and unchecked.
2. **It drops any token containing an ASCII digit.** F6 says flag with
   `HasDigits` and never delete, and golden row 18 expects `3D` to survive.
   Today `3D` is discarded before anything can flag it.

Neither is a bug in the old code — it predates this file — but I8's "every
location in C2 is deleted" is a behaviour change on a shipped path, not a
refactor. Worth your eye before Phase A.

`healSplitWords` is interesting: I8's grep gate would forbid it, but it is
not marker or punctuation stripping — it repairs Vision's line-wrap
splitting using a platform dictionary the core cannot reach. I read it as
out of scope for the gate and would leave it. Say if you disagree.

## C3 — apostrophe census: there are none. No HALT, and D1 has no target.

Every bank, every tier:

| lang | entries | entries containing an apostrophe |
|---|---|---|
| ar, de, en, es, fa, fil, fr, hi, ja, ko, pl, pt, ru, sw, vi, zh | 3,165 – 8,340 | **0** |

Not one entry in any of the sixteen banks contains U+0027, U+2019 or U+02BC.
So no bank mixes forms, and the HALT does not fire — but D1 says "fold every
apostrophe variant to the single form each bank stores", and **no bank
stores one**. There is nothing to derive the target from.

This also undercuts two golden rows: 6 expects `don't` and 7 expects
`don’t` → `don't`, both of which read as bank-canonicalization. `don't` is
not in the English bank at all, so under F6 both land as `NotInBank`
(checked, highlighted) whatever the fold does. The rows still make sense as
*trim* tests — the point of row 6 is that the apostrophe and hyphen survive
F3 — but row 7's expected output is a decision, not a lookup.

**You choose the fold target.** My recommendation: U+0027 `'`, because it is
what a phone keyboard produces by default and what My Words entries will
therefore be typed with, and because the banks are silent.

## C4 — **HALT.** The lookup does not exist, and the bank form is discarded.

`src/word_index.rs` is the only exact-match index:

```rust
pub fn is_valid(lang: &str, word: &str) -> bool          // bool, not the form
fn comparable(entry: &str) -> String { fold_strict(entry.split('|').next()…) }
```

The index is built as `comparable(entry)` for every bank word, so it stores
the **folded** form only — the bank's own casing and any `pinyin|hanzi`
right-hand side are dropped at index time. `word_at` and `starting_with`
return those folded strings too.

So `(lang, word) -> Option<bank_form>` cannot be answered by what exists.
The data is there (`words::tier_for(lang, tier)` holds the raw entries), but
recovering the form needs either a value alongside each key or a scan.

The file says: if it does not exist, **HALT and ask; do not build a second
index.** So I am asking. Three ways, and the middle one is what I would do:

- **(a)** Extend `word_index` to store `(folded, bank_form)` pairs instead of
  folded strings. One index still, slightly larger, `is_valid` unchanged.
  *My recommendation.*
- **(b)** Add `word_index::canonical(lang, word) -> Option<String>` that
  binary-searches the existing index and then scans that tier for the
  matching raw entry. No memory cost, slower, and the scan is the thing you
  told me not to build.
- **(c)** Drop F5's casing rule and keep scanned casing always. Cheapest, and
  it loses D2, which exists for good reason — worksheet title-casing really
  is layout.

## C5 — one platform, and the distribution I cannot produce

**iOS Vision only.** `android/` is a Capacitor shell with no ML Kit text
recognition in it, so there is no second confidence scale and D8's "per
platform" is one number, not two.

Scale is 0..1. `photo_import.rs:57` already ships a provisional threshold:

```rust
pub const LOW_CONFIDENCE: f32 = 0.4;
```

Its own comment says Vision's accurate path reports ~1.0 for clean print and
0.3–0.5 for shaky handwriting, and that the number "is brought to him, not
buried" — so D8's constant already exists and already awaits your sign-off.

**What I cannot do is the distribution over 10 sample photos.** I have no
worksheet photographs, and I cannot take any. If you give me ten images I
will run them through the real pipeline and produce the histogram D8 wants;
otherwise D8 has to stay at 0.4 by your decision rather than by measurement.

## C6 — `regex` is already there. No crate needed.

`Cargo.toml:57-59`: `regex = "1"`, `unicode-normalization = "0.1"`,
`unicode-segmentation = "1"`. The spec's `\p{Nd}`, `\p{P}`, `\p{L}\p{M}*`
classes are available without adding anything.

## C7 — test command

The gate runs the whole suite:

```
cargo test
```

judged by `^test result: ok` with no `FAILED` (`scripts/gate.sh:189-196`).
The full ship gate is `./scripts/gate.sh && cargo test --lib --features audit_preview`.
Golden rows would live in `tests/snap_clean_golden.rs` as the file asks, and
`cargo test` picks that up without configuration.

---

## What I need before Phase A

1. **C4 (HALT).** Which of (a)/(b)/(c) for the bank-form lookup.
2. **C1's second half (HALT).** There is no per-line language. Either
   CC-SNAP-LIST lands per-line tagging first, or F1 drops the per-lang-run
   rule and golden row 19 goes with it.
3. **D1's fold target**, since no bank has an apostrophe to copy.
4. **I8 vs the shipped path.** Deleting `parse_candidates` changes what a
   parent sees today: multi-word lines stop becoming several words, and
   digit-bearing words stop vanishing. Both are what this file wants — I
   just want you to have said so.
5. **Ten sample photos**, if you want D8 measured rather than decided.

Nothing else in the file is blocked. C2, C6 and C7 are clean, and the
pipeline F1–F6 is buildable as specified once 1–3 are answered.
