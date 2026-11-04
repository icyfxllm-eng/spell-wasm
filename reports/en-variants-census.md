# CC-EN-VARIANTS v1 — §0 census

Read-only. Nothing in `src/`, `assets/` or `data/` was touched. Run 2026-11-04 on a worktree off origin/main (6d14a78c).
Candidate rows are in `reports/en-variants-census-candidates.tsv` (291 rows, none admitted).

**HALT. Five findings need Eric before Phase A, and the first conflicts with a signed file.**

## HALT items (read these first)

1. **An English variants mechanism already ships, signed by Eric on 2026-10-19.** `src/variants.rs` plus `assets/words/en/spelling-variants.tsv`
   (commit 1e602477, "both spellings should be accepted"). The spec does not mention it. The two designs differ:

   | | Shipped | CC-EN-VARIANTS v1 |
   | --- | --- | --- |
   | File | `spelling-variants.tsv`, 2 columns, British first | `data/en/accepted-variants.tsv`, 4 columns |
   | Direction | Symmetric: either spelling grades for the other | One way: stored US, accepted variant |
   | Source of pairs | Hand-written, 209 pairs | Dictionary-tagged only, no hand pairs |
   | Player-facing | Settings switch "British" shows the British form in prompts (`variants::display`, `prefers_british`) | Never labelled, voice and bank stay US |
   | Kinds | none | our / re / ise / ll / ae/oe / ogue / compound / diacritic / other |

   Of the 209 shipped pairs, 128 touch a bank word. 93 of those 128 match a dictionary "British/Commonwealth spelling of" gloss. **35 do not**,
   and they include things that are not respellings: `learnt/learned`, `smelt/smelled`, `spoilt/spoiled`, `towards/toward`, `backwards/backward`,
   `forwards/forward`, `afterwards/afterward`, `practise/practice`, `defence/defense`, `enquire/inquire`, `speciality/specialty`, `aeroplane/airplane`.
   Under F1's "only dictionary-tagged pairs" rule these would have to be removed, which takes accepted answers away from players. Under "single
   source of truth", F1 means replacing the shipped file, not adding a second one. Needs a call: migrate the shipped list into the new file (and
   which of the 35 survive), or keep the shipped design and narrow this spec to F2–F4. Also: does the Settings British switch survive E2?
2. **The bank is not all US-stored.** 11 rows store the British form: `glamour`, `analogue`, `monologue`, `prologue`, `epilogue`, `archaeology`,
   `archaeological`, `amoeba`, `smelt` (all staged), `towards` (live hard), `afterwards` (live expert). F1 says the stored form is always US.
   Either those rows flip (a bank edit; the brief says not to touch tiers) or F1 admits "stored = whatever the bank holds".
3. **Dictionary tags alone are not clean enough to admit pairs.** See item 2 below. The raw harvest includes dialect respellings that carry the same
   regional tags (`dat→that`, `are→our`, `counterie→country`, `soal→sole`, `strenth→strength`). Every pair needs an eyeball; a mechanical tag filter will
   not do.
4. **Licence.** Variants come from English Wiktionary (via kaikki), CC BY-SA 4.0. `tools/en_growth/README.md` records the attribution approach as
   still unapproved. The "CC-VI-BANK-GROWTH D5" check named in the spec is not in `docs/` on origin/main, so I could not run it. T8 cannot be met until it exists.
5. **F2 is a different bug than the spec describes.** See item 5 below. Typed ’ cannot reach the grader on the game screen today. It is dropped
   at the key filter. The place it really bites is My Words import.

## 1. Where things live

| Thing | Path |
| --- | --- |
| en bank, live | `assets/words/en/{easy,medium,hard,expert}.txt` (762 / 798 / 800 / 797 = 3,157 distinct) |
| en bank, behind `en.retier` | `assets/words/en/retier/*.txt` (825 / 2,466 / 596 / 278) |
| en bank, staged (`en.expert_growth`) | `assets/words/en/staged.tsv` (14,124 words, no tier) |
| Union of all of the above | 17,402 distinct words |
| Cue text | `backend/def_pools/en.json` (2,967 entries; served, not bundled) |
| Normalization | `src/norm.rs`: `fold_strict` (NFC, lowercase, whitespace and silent marks dropped), `fold_lenient` (Kid), `same_answer`, `answer_matches_in`. No apostrophe handling. |
| Shipped variants | `src/variants.rs`, `assets/words/en/spelling-variants.tsv` |
| German/Japanese accepted-variants files (CC-ORTHO-SWEEP-DE-JA) | **Not landed.** No `accepted-variants*` file and no ORTHO-SWEEP doc anywhere on origin/main. The nearest existing mechanisms are `data/impossible/accepted/{de,en,es,fr}.json` (Impossible only) and `impossible::engine::accepted_alternates`. |
| Dictionary authority | kaikki English dump, `.corpus-cache/en-wikt/`, sha256 9978ce34…2c021195 (local, not committed) |

Grading call sites. **Using** the variants list: `game.rs:2268` (the typed-answer path for Normal, Daily and Climb), `boardgame_grade.rs:43`,
`impossible/engine.rs:106`, `display_diff.rs:445` (a diagnostic). **Not using it** (call `norm` directly): `bee_screen.rs:376`, `wordcross_ui.rs:464,533`,
`wordsearch_ui.rs:508`, `practice.rs:400`. Randospell and Spelluzzle have no call into `norm` or `variants` at all; I did not trace how they grade. F1's "everywhere" list needs that traced in Phase B.
Note `game.rs:2268` resolves the variant locally so the answer skips `backend_verify`; the server compares one spelling only.

## 2. UK/Commonwealth variants, by tier (census item 2)

Method: stream the dump, take every lowercase headword whose gloss reads "… spelling of <X>" where X is a bank word and the gloss or tags name
British / UK / Commonwealth / Australia / Canada / Ireland / NZ / South Africa / India / Oxford. This reads a dictionary field; it generates no pair from a pattern.
`kind` is a descriptive label only.

| Bank group | Raw pairs | After dropping glosses flagged dialect / pronunciation / obsolete / archaic / dated / misspelling / nonstandard / plural-past forms |
| --- | --- | --- |
| live easy | 23 | 2 |
| live medium | 13 | 3 |
| live hard | 14 | 9 |
| live expert | 10 | 6 |
| retier (all tiers) | 36 | 35 |
| staged | 195 | 172 |
| **Total** | **291** (280 distinct stored words) | **227** |

Kinds, after the flag filter: ise 97, our 26, ae/oe 31, ll 5, re 3, ogue 1, other 64.

Read the 227 as an upper bound. Dialect and misspelling junk survives the filter (`counterie→country`, `soal→sole`, `strenth→strength`).
Of the 227, 92 are already in the shipped file and 135 are new. The 135 include real ones (`organisation`, `recognised`, `specialised`, `focussed`,
`neighbourhood`, `programme`, `dependant`, `licorice/liquorice`) and the junk above. The flagged 64 are almost all dialect respellings of live-easy words (`dat`, `dis`, `yer`, `zun`).
The live tiers are thin because the bank is mostly short, plain words. The yield is in retier and staged (organise-type -ise/-ize and -our/-or).

The dictionary lacks a structured "variant" field. Wiktionary expresses these as gloss text ("Commonwealth and Ireland standard spelling of color"),
sometimes with a structured `alt_of`, sometimes not (`travelled` is filed only as a form of `travel`). The spec's stop condition "dictionary lacks variant tags"
is borderline: the information exists but must be parsed from text.

**The bank holds the stored forms for only some of the spec's own examples:** `color` (live medium), `gray` (live easy) are in. `organize` is staged only.
`theater`, `traveled`, `cafe`, `naive` (staged only), `resume`, `email` and `don't` are **not in any en tier or staged file** (the en bank has no
apostrophe, hyphen, space or accent anywhere). T2, T4, T5 and T6 therefore cannot run against real bank rows. They need fixtures, or those words need adding (a bank decision, outside this spec's no-tier-touching rule).

## 3. Stored answers with `'`, `-`, a space or a diacritic (census item 3)

**Zero, in every tier of every en file** (live, retier, staged). Every en row is plain a-z. So E5 (space/hyphen rows out of Easy, Jr, Kid, Little Speller) and T7 have nothing to exclude today.
They bind only if F3 or F4 add such rows. What exists in the dictionary (raw upper bounds, not usable counts):

- Hyphen/space/closed splits of a bank word: about 1,200 hits, but most are noise (affix entries like `-less`, `-ship`, contractions `it's→its`, `we'll→well`). A usable F3 list is a small hand-reviewed subset, e.g. `cease-fire/ceasefire`, `non-fiction/nonfiction`, `take off/takeoff`.
- Accented forms of a bank word: about 120 hits, e.g. `naïve`, `façade`, `crêpe`, `rosé`, `coöperate`. Also noise (`markèd`, `learnèd`, `nō`, `lóng`).

## 4. Proper-noun overlap (census item 4)

Capitalised headword with a `name` entry whose lowercase form is a bank word: **874 live/retier words, 3,021 including staged**. That is a loose upper bound
because almost any short word is also a surname or given name. The ones that matter are where the proper noun is the dominant sense: `march`, `china`, `reading`, `may`, `will`, `bill`, `mark`, `jack`, `mac`, `java`.
`polish` and `turkey` (the spec's examples) are **not in the bank**, so there is nothing to confirm for them.

Only the lowercase form is stored. Checking the cue against it (`backend/def_pools/en.json`, first gloss per word) found these bad matches:

| Word | Cue text | Problem |
| --- | --- | --- |
| `mac` | `“a raincoat”).` | A broken fragment, not a definition |
| `may` | "To be strong; to have power (over)." | Archaic verb; the player hears the modal or the month |
| `mark` | "Boundary, land within a boundary." | Obsolete sense |
| `jack` | "A coarse medieval coat of defence…" | Obscure sense |
| `bill` | "A written list or inventory. (Now obsolete…)" | Obsolete sense |
| `reading` | "present participle and gerund: To look at…" | Inflection, not a definition |
| `java` | "A blend of coffee imported from the island of Java." | Fine, but the cue names the proper noun |
| `march` | "A formal, rhythmic way of walking…" | OK, common-noun sense |

`china` has no def-pool entry at all. I fixed nothing; E6 says check only. These cues are the real E6 work for Phase C. A systematic pass is needed, since the first-gloss rule is what produces them.

## 5. Where U+2019 can enter (census item 5)

- **Device (on-screen) keyboard:** none of the 16 layout files in `assets/keyboards/` contains `'`, ’, ʼ or ‘. The punctuation keys come from `keyboard.rs:311`, which adds a straight `'` and `-`. The system keyboard is never used: the answer is not a real `<input>`. A player on the on-screen keyboard cannot produce ’.
- **Hardware keyboard (native and web, same code):** `keyboard.rs:399` accepts a keydown only if the character is alphabetic, an ASCII digit, `'` or `-`.
  U+2019 and U+2018 are punctuation, so they are **silently dropped before the grader sees them**. U+02BC and U+02BB are modifier letters, so they pass as letters and are then compared literally. Result: a typed ’ never causes a wrong answer, it causes a dead key.
- **Paste and dictation:** blocked on the game screen (`wire_paste_guard`, `input_provenance`).
- **My Words import (the real bug):** `importer.rs:11` admits `’` inside a word and stores it as given (only `snap_clean::fold_apostrophes` folds, and only on the photo path). A pasted "don’t" is saved with U+2019. The player cannot type it, so the item is unwinnable.
- **Say-It / recognizers:** ASR transcripts commonly carry ’, and `spoken_matches` compares via `fold_strict`, so a fold there helps.
- **Web input on spellgame.net:** the same game keyboard and keydown handler. No separate path.

**F2 risk check:** no word bank in any language holds `'`, ’, ʼ, ʻ or ‘ in a stored answer (scanned every `assets/words/*/*.txt` and `retier/`). No keyboard layout holds one either.
No lineup language is affected, so the F2 HALT condition is not triggered. Hindi and Arabic have no such marks in their banks. The one thing to guard in code: ʻokina (U+02BB) is not in the fold set (the spec lists ’ ʼ ‘ only), and `snap_clean` folds ʼ but not ‘.
Since the fold does not touch any bank today, D2-56 (ride build 56) is contained if done as: fold in `fold_strict` and `fold_lenient`, the `keyboard.rs:399` filter, and `importer::extract_words`. The first alone makes T4 pass; the other two are what fix real players.
Existing My Words lists with ’ stored need a one-time fold on load.

## 6. What I did not check

- The server-side bank (`~/spellgame-server/bank-src`) was not diffed against the repo's.
- How Randospell, Spelluzzle, Climb's server-side verify and Daily's server verify treat a variant answer.
- Whether any existing test asserts that a US-spelled Climb or Daily score is unchanged (T9). No tests were run; this census changes no code.
- The dump is a local cache dated Oct 16; no network refresh.
