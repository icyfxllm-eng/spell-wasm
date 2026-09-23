# CC-AR-HI-PREAUDIT — Arabic + Hindi hardening

Status: **SIGNED and RESCOPED** (Eric, 2026-09-23). Drafted 2026-09-22 as
"pre-audit hardening"; the premise did not survive the census, so it is now
live-language hardening. Languages: ar (MSA, unvocalized), hi (Devanagari).

---

## Why the rescope

The draft assumed ar and hi were behind release gates and that the work was
protecting a future auditor. Both halves are false:

- **`src/consts.rs:447` asserts `"the app gates no language"`** — every one of
  the 15 built-ins is Active in the app (Eric, 2026-07-31).
- **The Arabic RTL render gate has been open since the 2026-07-25 ungate**, and
  `rtl_gate_matches_the_build_config` exists specifically as the tripwire
  against ever re-gating it.

So the draft's closing line — "Arabic remains behind the CC-RTL release gate
regardless of this file's status" — was wrong when it was written. These are
shipping languages with players in them. The work is ordered by what protects
those players now, not by what impresses an auditor later.

The other two HALT conditions:

- **C6 vocalization coverage: 0%**, not the 90% floor. No `vocalized` field
  exists anywhere in the repo. F1 is a data-sourcing project (up to 6,238
  Arabic words needing a dictionary-cited vocalization), and I3 as written
  would silence Arabic entirely until that data exists. **F1 is deferred** until
  an auditor has sourced it.
- **C10 passes.** Exactly one function decrements shields
  (`attempts::spend_shield`, src/attempts.rs:131), called from exactly one
  non-test site (src/game.rs:2680). `shield_note_miss` only resets the earn
  streak. No `shield_cost` yet, which is what F7 would add.

## Collisions with signed decisions, and how they were settled

1. **Two closed trap registries claimed Arabic.** `config/trap-registry.json`
   (CC-WORDLIST-RAFU F3, consumed by `scripts/trap-tag.mjs`) already held 6
   Arabic classes in kebab-case. The draft's F5 proposed 7 `AR-*` ids owned by
   the CC-PERSIAN-FOUNDATION tagger, and RAFU was not in its ownership table.
   **Eric's call (2026-09-23): the existing registry wins.** New classes
   register into `config/trap-registry.json` in kebab-case; the `AR-*` ids are
   aliases. RAFU's `auto` / `assisted` / `manual` tagging contract stands, which
   means RAFU decision 9 also stands: the tagger never asserts that a hamza seat
   is *correct*, because unvocalized orthography deletes the vowel context that
   would decide it.

   Mapping, recorded now so Phase C does not relitigate it:

   | draft id | registry id | status |
   |---|---|---|
   | AR-HZ | `hamza-seat` | same class |
   | AR-TM | `taa-marbuta-vs-haa` | same class |
   | AR-AM | `alif-maqsura-vs-yaa` | same class |
   | AR-SUN | `sun-letter-assimilation` | same class |
   | — | `madda` | registry only; no draft equivalent |
   | — | `dagger-alif` | registry only; nearest draft id is AR-LONG |
   | AR-WA, AR-EMPH, AR-LONG | — | new; not yet registered |

   There are two further Arabic trap taxonomies in `config/practice/ar.json`
   (`hamza`, `taa-marbuta`, `alif-maqsura`) and a fourth for Hindi in
   `config/practice/hi.json` (`matra`, `retroflex`, `aspirate`). Hindi is not in
   the trap registry at all.

2. **`vocalized` vs the unvocalized standard.** The registry records
   `orthography: "unvocalized (CC-LINEUP-SWAP D5)"` and RAFU F4 rejects tashkeel
   from stored forms. Moot while F1 is deferred; it needs Eric's explicit word
   before any stored form carries marks.

---

## Done (2026-09-23)

### The live bug: typed Arabic vowel marks were graded wrong

Arabic falls to the shared `else` branch in `game.rs` and grades through
`norm::answer_matches` → `fold_strict`, which was NFC + lowercase + whitespace
only. Harakat survived it. A player typing مَدْرَسَة — correct Arabic, one
long-press away on the iOS Arabic keyboard — was told they had misspelled
مدرسة. This was acceptance test 5 in the draft, failing in a shipping language.

`fold_strict` now drops **silent marks** on both sides: tashkeel
(U+064B–U+065F), dagger alef (U+0670), tatweel (U+0640) and ZWNJ (U+200C).

- Safe by construction, and checked rather than assumed:
  `no_shipping_answer_depends_on_a_silent_mark` walks every word of every tier
  of all 15 languages and proves no canonical answer loses a character to the
  fold. With that premise, the change cannot flip an existing verdict — it can
  only accept a correct spelling that used to be refused.
- ZWNJ follows Persian's signed precedent (`fa_canon` F2, Eric 2026-08-09): the
  stored form keeps its ZWNJ, the comparison ignores it, because a player cannot
  be held to an invisible character.
- **ZWJ (U+200D) is deliberately not stripped.** My Words takes arbitrary text
  and emoji sequences are joined with it.
- In `fold_lenient` the strip runs *before* the NFD pass, not after. NFD
  decomposes أ into ا + U+0654, which sits inside the tashkeel range — stripping
  it there would have made Kid Mode hamza-blind, quietly removing Arabic's main
  trap class for the players least able to notice.
- Letters are untouched: ة/ه, ى/ي and the hamza seats still grade.

### The tripwire: `scripts/ar-hi-ortho-check.mjs` (F2/F3), wired into the gate

L-AR1–L-AR5 and L-HI1–L-HI5 fail the build; L-HI6 and L-HI7 report only.

**Both banks already pass every fail-mode rule** — ar 6,238 words with zero
harakat, tatweel, tanwin, presentation forms, Persian lookalikes or non-NFC; hi
2,674 words clean on L-HI1–L-HI5. This check cleans nothing up. It exists
because these are the two banks nobody on the project can read: a stray fatha or
a Persian ی pasted from a web page is invisible at arm's length, and the first
person to notice would be a paying auditor or a player.

11 deliberate-failure fixtures prove each rule bites, and four correct words
prove none of them false-positives (acceptance tests 2 and 3).

L-AR6 is **not** implemented: it lints a field that does not exist yet.

### Already satisfied, no code written

**F8 / L-IN1 / I11 keyboard reachability.** `keyboard::every_word_char_is_typeable`
already iterates all 15 built-ins across all four tiers and asserts every
character of `fold_strict(answer)` is reachable on that locale's keyboard —
and `fold_strict` preserves every matra, nukta, halant and anusvara, so the
Hindi marks really are covered. The draft said to register into an existing
shared lint rather than add a second one; ar and hi were already registered.

---

## Open

- **L-HI7 flagged 8 words ending in a halant.** Five are correct Sanskrit-derived
  forms (अर्थात्, पश्चात्, तत्पश्चात्, पर्यावरणविद्, पुरातत्त्वविद्) and should
  stay. Three want an auditor's eye: आदित्यवर्धन् and इन्द्रवर्मन् are proper
  names, and इन्टरैक्शन्स् is an English loanword with a non-standard final
  halant. Flagged, not touched — nobody here can judge Hindi spelling, and I4
  says sources decide.
- **F9 audio.** The CC-AUDIO-CLARITY F2 gate has not been run over ar or hi. The
  Arabic vocalization probe is blocked with F1.
- **F4 spelling variants** (हिंदी / हिन्दी, ज़रूर / जरूर). The draft is right
  that marking a correct spelling wrong is the worst headache a spelling game
  can cause, and this is the biggest remaining player-protection item. The
  mechanism is buildable now; the rows are not, since D8 requires a dictionary
  citation per variant. Same shape as the audio lexicon: ship the machinery
  empty.
- **F5–F7** (trap tagging, tier placement, TRAP_MISS, rule cards) remain Phase C.
  Note the name collision to resolve first: `Channel::Trap` in `src/learner.rs`
  already means TRAP_MISS in the persisted learner log, but for a *different*
  thing — a Spell Search player picking a decoy (CC-WORDGRID F-S2). The draft's
  TRAP_MISS is a verdict about *where* the wrong characters landed. Two things
  under one name in Reports would make the data unreadable.
