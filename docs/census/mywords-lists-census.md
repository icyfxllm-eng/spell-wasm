# CC-MYWORDS-LISTS §0 — blocking census

Run 2026-09-15, before any code. Every line below is read out of this repo at the
paths given; nothing here is inferred from the spec.

## C1 Storage

My Words lives in **device storage only**: `localStorage["byear_custom_v1"]`
(`src/model.rs:136`), written through `storage::set_json` and read through
`storage::get_json` (`src/importer.rs:35`, `src/importer.rs:50`). Current schema,
verbatim from `CustomSet` (`src/model.rs`):

| Field | JSON | Type | Meaning |
|---|---|---|---|
| `words` | `words` | `Vec<String>` | the whole of My Words, one flat ordered list |
| `speak_lang` | `speakLang` | `String` | set-wide "Speak in" language |
| `word_lang` | `wordLang` | `Map<String,String>` | per-word speak language, side map |
| `word_batch` | `wordBatch` | `Map<String,u64>` | the import batch that INTRODUCED each word |
| `next_batch` | `nextBatch` | `u64` | next batch id, monotonic |
| `custom_marks` | `customMarks` | `Set<String>` | words imported as out-of-dictionary |

There is no Rust-core store, no Pi/backend copy (`backend/*.py` has no custom-word
route), and no iCloud path. **My Words does not sync across devices.**

## C2 Free cap

`FREE_CUSTOM_LISTS_CAP = 2` and `custom_lists_cap: Option<u32>`
(`src/entitlements.rs:53`, `:98`). It is **enforced nowhere**: the only consumer
outside the entitlement module is `play_hub.rs:123`, which turns
`custom_lists_unlimited()` into a premium string for the hub. No save path checks
it. So today the cap counts **nothing**, and the data model holds exactly **one**
list. See the stop condition below.

## C3 Per-word progress

Keyed by word and language, never by position in My Words:

- `wordstats::norm_key(lang, word)` — `src/wordstats.rs:65`, stored under `spell_word_stats_v1`.
- `misses::miss_key(word, lang)` → `word_id0(lang, word)` — `src/misses.rs:9`.
- `selection::note_outcome` matches the most recent trace by word text — `src/selection.rs:150`.

Migration therefore cannot reset progress by moving words into a list.

## C4 Profiles

**There is no profile system.** `multiple_profiles` exists only as an entitlement
field (`src/entitlements.rs:102`) with no implementation, and no storage key is
profile-scoped. My Words is **per device**.

## C5 Consumers

| Surface | Path | Uses |
|---|---|---|
| Save path (both doors) | `src/importer.rs:64` `save_words` | merges into `words`, assigns a batch id |
| Snap a word list review + save sheet | `src/photo_list.rs` (replace flag at `:491`–`:499`) | reads the sheet, saves |
| Save sheet markup, replace checkbox | `index.html:2257`–`2259` (`#photoReplace`, checked) | destructive option, default ON |
| Manual add sheet | `index.html:1808` `#importScrim`, `#importBtn` at `:1577` | paste words |
| Speak-language resolution | `src/game.rs:19`–`31` | per-word then set-wide language |
| Play source `__mine` | `src/consts.rs:21`, `src/lib.rs:162`, `:1111` | My Words as a study language |
| Spell Translate "Save to My Words" | `src/translate_ui.rs` `save` | same `save_words` path (spec D5) |

**There is no My Words screen.** Nothing lists, opens, renames or deletes saved
words today; `＋ My words` opens the paste sheet. F3 therefore builds a new
screen rather than regrouping one.

## C6 Existing data

- **Timestamps: none.** No entry carries a created/added time. The only ordering
  evidence is `wordBatch`/`nextBatch` (`src/model.rs`), which records which import
  introduced a word, not when.
- **Fixture DB: none for My Words.** No test seeds `byear_custom_v1`; only
  `tests/e2e/specs/translate-screen.mjs` reads it back after a save.
- **Eric's TestFlight device: not readable from here.** The set lives in the app's
  own storage on his iPhone; this census cannot count its rows.

## Stop conditions

1. **C2 fires.** The cap's declared unit is *lists*, the very thing this spec
   multiplies: one list today becomes one per save, roughly one a week. The cap is
   unenforced, so nothing breaks the day this ships — but the moment it is enforced
   as written, a free family is over the limit after two weeks of saving. D6 has to
   settle the unit before F1 lands.
2. **C4 has no referent.** `profileId`, "per profile" and I6 profile isolation
   describe a system that does not exist. Lists would be per device.
3. **CC-SNAP-LIST v1 is not in this repo.** §8 says to use the command names from
   its C6, and §0/Done reference its D8. The nearest files are
   `docs/CC-SCAN-STACK-V1.1.md`, `docs/CC-SCAN-STACK-V1.2.md` and
   `docs/CC-PHOTO-IMPORT-PLAN.md`, none of which carry that C6.
4. **The §8 commands do not match this repo**, the same way CC-TRANSLATE-SCREEN's
   did not: there is no `-p <core-crate>` (the crate is `spell_wasm`, tested with
   `cargo test --lib`), Playwright specs live in `tests/e2e/specs/*.mjs` and run
   through `tests/e2e/run.mjs`, and Maestro is not installed on this Mac.

C3 does **not** fire (progress is keyed by word + language) and there is no
cross-device sync, so those two risks are clear.
