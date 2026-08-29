# CC-SPELL-O-PHONES v1

**Status:** F0 RUN — see "F0 RESULT" below. The measurement FALSIFIES the
expected inventory (58 against 150-250 expected). Eric banked pear/pare/rite/
wright on 2026-08-28, which makes both marquee families real and takes Sweep
families from one to three. F5 still does not stand at three. D2, D9-D13 still pending Eric. Nothing past F0 executes.
**Scope:** English only. US voice only.
**Sequencing:** Post-build-56. Does not enter the resubmission vehicle.
**Supersedes:** the four-turn design conversation of Aug 2026.

---

## Intent

Every other mode in Spell asks the player to hear a word and write it down.
This is the one mode where hearing it is provably not enough -- where the sound
/pear/ is three legal English words and the ear cannot choose between them.

That inversion is the point, and the source of the mode's one structural gift:

> **A wrong answer here is still a real word.**

Elsewhere wrong means garbage. Here wrong means a different perfectly good word
dropped into a sentence where it does not belong. `I bought a pear of shoes.`
That is funny, it teaches better than any correction message, and it makes this
the only mode in Spell where failure is the best part.

---

## F0 RESULT — 2026-08-28, measured against the audited English bank

Tool: `tools/spellophones-inventory.py` (CMUdict, already vendored; stress
stripped so homophones collide, the same rule the lexicon-ingest parser uses).

    bank words 3206      cmudict headwords 117493

    BANK-CLOSED sets                     62
      accent-conditional, excluded        4     (wh- merger)
      UNIVERSAL, usable in v1            58
    sets carrying a GRADING RISK         23
    multi-pronunciation bank words      184

    SET-SIZE DISTRIBUTION (universal)
      2-member                           55
      3-member  <- Sweep-eligible         3
      3+ MEMBER FAMILIES                  3

    BY TIER (hardest member)
      easy 41   medium 14   hard 1   expert 2

**Expected 150-250. Measured 58.** The estimate is falsified by 3-4x.

**The Sweep has essentially no families, and the real number is worse than 3.**
The three are `sea/see/si`, `do/du/due`, `to/too/two` -- and `si` and `du` are
not English words. They are bank contamination (below). Discount them and
**exactly one** genuine 3-member family exists: `to/too/two`.

The spec's stopping rule: *"If it is thin (under 15), the Sweep becomes a rare
event and F5's run structure changes."* One is not thin. It is absent. F5 as
written -- every run ends on a Sweep -- cannot be built from this bank.

**Both marquee examples were unbuildable. FIXED 2026-08-28 (Eric).**

    /pear/   pair BANKED   pear was MISSING   pare was MISSING
    /rite/   right BANKED  write BANKED       rite was MISSING   wright was MISSING

`I bought a pear of shoes` -- the sentence this whole mode is designed around
-- could not be produced, because `pear` was not a word the game knew.

`pear` (easy), `rite` (medium), `pare` and `wright` (hard) were added to
`assets/words/en/` and the bank regenerated. Tiering is human curation, not
computed: pear rides with pair; rite rides with right/write; pare and wright
are genuinely uncommon words and sit in hard. Verified that exactly those four
entries changed across all fifteen locales.

**Post-addition inventory (superseded by the heteronym pass below):**

    UNIVERSAL sets                     59   (was 58)
    3-member                            4
    4-member                            1
    3+ MEMBER FAMILIES                  5   (was 3)

    right/rite/wright/write     <- the 4-member Sweep, now real
    pair/pare/pear              <- the mode's flagship family, now real
    to/too/two
    sea/see/si                  <- si is bank contamination
    do/du/due                   <- du is bank contamination

**Contamination stripped 2026-08-28 (Eric).** Forty Wikipedia-dump artifacts
removed from the English bank: abbreviations and units (ab ac bc cf cm dc fm kg
km mm pc ph pp ss), Roman numerals (ii iii iv ix vi vii viii), foreign
particles (de del der des di du et eu le los un von si), and outright markup
debris (sfn, seealso, urbe, ibn, abc, al). `sfn` and `seealso` are citation
templates; `urbe` is Latin. Kept deliberately: la, el, en, os, don, bob, tom,
ben, lee, ion, mac, max -- every one a real English word that merely looks odd
in a list.

Both fake families collapsed as predicted. Final inventory:

    UNIVERSAL sets                     50
    3+ MEMBER FAMILIES                  3   all genuine

    right/rite/wright/write
    pair/pare/pear
    to/too/two

Removing words from a shipping bank is safe here: `deck.rs` keys its
persisted no-repeat cursor on `pool_len`, so a changed word list forces a
reshuffle and a stale queue holding a removed word is discarded rather than
served. Removal only shrinks the pool, so the length always changes.

**F5 is still not satisfied.** Discounting the two contaminated families,
THREE genuine Sweep families exist. The spec's rule -- under 15 is thin -- still
bites, and "the Sweep must be the last family" means the same three rotate
every single run. The Freshness Invariant says this mode should fail the build
rather than silently repeat, and at three families it would. Getting F5 to
stand needs roughly a dozen more Sweep families, which the grading-risk list
below sketches: air/heir+ere, way+weigh/whey, eye+aye, you+ewe/yew, be+bee,
buy/by+bye, meat/meet+mete, for/four+fore.

**Two words still have no definition.** `pare` and `wright` return
`{"error":"not found"}` from /api/meaning. F1 requires every member to exist
"with a definition", so both fail F1 as written -- but so do `rhythm`,
`absence`, and `carriage`, which are already in the bank. The definition
requirement in F1 is stricter than the bank's actual state, and that tension
belongs to Eric: either F1 relaxes, or a definition backfill precedes the mode.

### Bank contamination found on the way

`EN_EASY` contains 56 entries of one or two letters, of which roughly thirty
are not English words:

    ab ac al bc cf cm dc de di du el et eu fm ii iv ix kg km la le mm
    os pc ph pp si ss un vi

Abbreviations, units, Roman numerals, and foreign particles. This is a live
defect independent of this mode -- players are being asked to spell `cf` and
`ix` -- and it is what inflated the Sweep count from one to three. It should be
fixed on its own merits, not as part of this file.

### What F0 says to do

The measurement does not kill the mode; it reorders it. Three honest options:

1. **Bank work first.** The 2-member inventory (58) is enough for tiers 1-2 and
   the Wrong Sentence, which is the mode's actual gift. The Sweep needs
   `pear`, `pare`, `rite`, `wright` and their kin added to the audited bank
   with definitions. That is a bounded authoring job and it unlocks F5 as
   written.
2. **Ship without the Sweep.** F4 tiers 1-2, F3, F7, F9 all stand on 58
   two-member families. F5's arc changes: no climax, or the climax is the
   hardest single rather than a Sweep. Cheapest path to a playable mode.
3. **Rescope to a Trap Boss gauntlet** (this file's own fallback) and drop the
   hub-mode ambition.

The prototype this file asks for -- one screen, hear it, spell it blind, get
the Wrong Sentence -- is unaffected by any of this and is still the right next
step. It needs ten families; there are 58.

### D13, answered by measurement rather than opinion

*Which English voice speaks this mode today, and do web and app agree?*

**They do not agree, and the app does not agree with itself.** Word audio
resolves through an ordered router (`src/api.rs`): Pack -> ServerCache ->
NativeTts. ServerCache is a Google-rendered clip from `/api/speak`; NativeTts
is on-device AVSpeech, a different voice with different phonetics; Pack is a
third pre-rendered set. Which one speaks depends on network, cache state, and
installed voices -- and the August 2026 audio work proved that varies in normal
play, per device and per word.

For any other mode that is a quality difference. For this one it is
correctness: the mode asserts that two spellings sound identical, and it has no
guarantee about which engine produced the sound. A set that is homophonous in
Google's US voice and distinct in AVSpeech makes the question unanswerable, and
the player is marked wrong for hearing correctly.

**RESOLVED (Eric, 2026-08-28): the mode is PINNED.**

    Pack -> ServerCache.  NativeTts is REMOVED from the order.

Not a new mechanism -- Mandarin already does exactly this, and for a reason of
the same shape. `play_word_with` carries:

    if lang == crate::consts::ZH {
        order.retain(|s| *s != Source::NativeTts);
    }

zh drops the on-device rescue because AVSpeech cannot be handed a reading and
would guess a polyphone. Spell-O-Phones drops it because AVSpeech is a
DIFFERENT VOICE WITH DIFFERENT PHONETICS, and a mode whose entire question is
"do these two spellings sound the same" cannot let the answer depend on which
engine happened to speak. Pack and ServerCache are the same rendered audio --
a pack clip is the server clip cached on disk -- so the two agree by
construction. AVSpeech does not.

**The consequence is deliberate and worth stating in the same words zh uses:
with no pack and no server, this mode has NO AUDIO rather than wrong audio.**
A silent question a player can skip is recoverable. A question where the app
and the player disagree about what was said is not -- the player is marked
wrong for hearing correctly, which is the exact failure F1 exists to prevent,
arriving through the audio path instead of the grading path.

Implementation, when the mode is built: extend the same `retain` in
`play_word_with`, keyed on the mode rather than the language. Do not add a
second routing decision -- the file's own doctrine is ONE resolution order in
ONE place, and a mode-specific audio path elsewhere would be a second router
wearing a different hat.

**A GAP THIS OPENS, and it must be closed before any carrier is authored.**
The sets in F0 were derived from CMUdict. CMUdict is a phonemic dictionary of
General American -- it is NOT the pinned voice. It says `to`, `too` and `two`
share a pronunciation; it does not promise that the Google US voice renders
them indistinguishably, and the mode's whole premise is that the player cannot
tell them apart BY EAR.

So the pin creates a verification obligation that did not exist while the
source was undecided: every shipped set must be confirmed to actually merge in
the pinned voice. That is a listening pass over roughly 50 sets, or an
automated one -- render each member through /api/speak and compare the audio.
The zh work already built that machinery for tone (`tools/zh-tts-loopback.py`,
an F0 autocorrelation pitch probe), and the same shape applies here: two clips
that are supposed to be identical should measure as identical.

A set that CMUdict calls homophonous and the voice renders distinctly is an
unwinnable question that looks winnable -- F2's failure, arriving through the
audio path. Neither F0 nor F2 catches it.

### VOICE CHECK RESULT — 2026-08-28, `tools/spellophones-voice-check.py`

Every member synthesized through the pinned source, decoded, reduced to
per-frame log band energies, compared pairwise with DTW (alignment-invariant --
raw waveform correlation is useless here, `to` vs `too` scores -0.054 while
sounding the same).

The bar is CALIBRATED EVERY RUN, never hardcoded. Two different input strings
never produce identical audio from this engine, so "distance 0" is the wrong
bar. Spelling variants of ONE word -- gray/grey, center/centre, color/colour,
disc/disk -- are what a listener calls identical, so their worst case IS the
floor. Different-word pairs set the ceiling.

    indistinguishable floor  0.5445
    different-word ceiling   0.9169
    pass bar                 0.6189

    43 VERIFIED     14 SUSPECT     0 unusable

**The two that matter are HETERONYM failures, and F0 missed them:**

    read/red   0.8038    the voice says "reed", not "red"
    lead/led   0.7544    the voice says "leed", not "led"

CMUdict's primary entry for `read` is R EH1 D and for `lead` is L EH1 D -- the
past tense and the metal. The TTS, handed the bare word, speaks the COMMON
reading instead. F0 listed both among its 184 multi-pronunciation words but
never removed the SETS they form, because F1's homograph rule excludes a set
whose MEMBERS SHARE A SPELLING and these do not. **A set containing a heteronym
is broken for a different reason: the voice must choose a reading, and it may
not choose the one that makes the set homophonous.** That is a third exclusion
rule, and it belongs in F1.

**Not actually homophones** -- CMUdict artifacts and accent effects the
inventory could not see: hour/our (0.87), are/or (0.70), are/our (0.68),
when/win (0.68, already flagged wh-merger), cars/cause (0.67), la/law (0.67).

**Genuine homophones sitting just over a deliberately tight bar** -- these need
EARS, not a number: passed/past (0.653), wood/would (0.641), hear/here (0.641),
to/too/two (0.632), peace/piece (0.623). The tool's job is to shrink a fifty-set
listening pass to the handful that need a human, not to overrule one.

**What this tool cannot tell you.** It measures whether the synthesized audio
DIFFERS, not whether a person would notice. A set just over the bar is flagged,
not condemned.

Two bugs of my own, fixed and recorded because both produced confident wrong
answers first. The initial run reported 28 sets as having NO AUDIO; the truth
was that it fired ~110 rapid requests, Cloudflare answered `error code: 1015`,
and the tool cached those 17-byte error bodies as if they were clips. It now
checks the MP3 frame header, never caches a non-audio response, backs off and
retries, and paces itself. It also listed hour/our twice, because a word with
two cmudict entries can put one member set under two keys -- deduped by
members now, not by pronunciation string.

---

## The player's minute

1. A run starts. Five families, roughly three minutes.
2. Audio plays: /pear/. No sentence on screen. A short beat holds with
   letter-count blanks.
3. Player commits a spelling blind. They type PEAR.
4. The sentence appears with their word in it:
   **I bought a pear of shoes.** It was **pair**.
5. No red X. No buzzer. The joke does the correction.
6. The family card fills in: pair captured, pear and pare still ghosted.
7. Four more families, each miss producing its own absurd sentence.
8. Final family is a Sweep: hear once, find all four spellings.
9. Clean sweep, or not. Run ends on the family shelf.

---

## F1 — Set Closure Invariant

If the bank knows pair and pear but not pare, and a player types PARE in a
carrier where pare is legal, the app marks a correct answer wrong.

**Invariant:** the grader evaluates against the COMPLETE verified set. An
incomplete set is not a smaller set -- it is an illegal set. Sets with
unverified or unbanked members are quarantined WHOLE and never render.
Quarantine, never merge.

Construction rules, enforced at export:
- A set whose members share a spelling fails export. Homographs have no
  spelling answer and must be excluded by construction.
- A set with fewer than 2 members fails export.
- Every member must exist in the audited English bank with a definition.

**F0 note.** Reading this invariant as "no unbanked word may share the sound"
is wrong and produced a false zero on the first measurement run: `to/too/two`
is a legal three-member set even though cmudict also lists `tew`, `tu`, `tue`.
The set IS its banked members. An unbanked real word sharing the sound is a
GRADING RISK, not a missing member -- 23 sets carry one, and the grader must
accept those spellings rather than the set being suppressed.

## F2 — Carrier Uniqueness Invariant

For every carrier, exactly one member of its set is semantically legal.
Multi-legal carriers fail validation and fail export. Carriers are AUTHORED,
never generated -- LLM-drafted definitions were rejected under standing policy
and carriers are the same class of displayed string.

## F3 — The Wrong Sentence

On a `WRONG_MEMBER` verdict, render the player's own sentence with their word
in it, then the true sentence beneath. No red, no buzzer, no shake.

**Hard constraint:** fires ONLY when the player spelled a legal member of the
set. `PEARR` is a misspelling and routes to normal correction. A garbled word
in a joke sentence is not funny, it is confusing. This is the entire reason
`WRONG_MEMBER` must be a distinct verdict.

## F4 — The reveal ladder

- **Tier 1 — Sentence first.** Fair, no gamble. Tutorial and Kid Mode home.
- **Tier 2 — Sound first.** Commit blind, then the sentence reveals the
  verdict. THE CORE LOOP.
- **Tier 3 — The Sweep.** Hear once, find all spellings. Crossword energy.
- **Tier 4 — Blind.** No carriers. The whole family from memory.

## F5 — Run structure

Five families, mixed tiers, ending on a Sweep. **BLOCKED by F0** -- one Sweep
family exists. See F0 RESULT for the three options.

## F6 — The Double-Take

At tier 2+, after audio and before input opens, hold a beat under one second
showing the sound's blanks. Blanks must match the SHORTEST member -- showing
five for a family containing pare (4) and pears (5) hands over the answer.

## F7 — Sound Families

Each set is a family named by its sound. A family goes gold only when every
member has been spelled correctly at least once. The bragging metric is family
count, not word count, and it is the honest one.

## F8 — Reports integration

`WRONG_MEMBER` is a first-class verdict, distinct from `MISSPELLED`. Choosing
the wrong member is a comprehension error, not a spelling error, and counting
it as one corrupts the letter-confusion matrix in CC-REPORTS.

**The Nemesis.** Reports surfaces the worst family as a named antagonist. Beat
it three consecutive times and it retires.

## F9 — Punchline pack

A riddle where the joke only resolves once the right member is spelled. ~40
authored items in v1 per D11.

## F10 — Blitz binding

Bind as a CONTENT SOURCE for CC-BLITZ. Zero mode-specific branches in the
Blitz shell.

## F11 — Trap Boss binding

Sets derive from the CC-TRAP-BOSSES taxonomy via `homophoneSetId`. If a
standalone homophone table is being written, this feature has failed.

## F12 — Registry availability

`spellOPhonesEnabled(lang)` -- English true, all fourteen others false. Absent
from the hub, not greyed, not "coming soon".

## F13 — APP ONLY. Never on spellgame.net (Eric, 2026-08-28)

F12 gates by LANGUAGE. This gates by SURFACE, and it is a different axis: the
mode ships in the Capacitor app and must never reach the deployed site.

A runtime flag is NOT sufficient -- it would still send the markup, the styles
and the sets to spellgame.net, where anyone can read them. The site build
already solves this exactly once, for Spell Picture, and this mode uses the
same mechanism rather than inventing a second one:

1. **Sentinels.** All markup and CSS wrapped in `SPELL-O-PHONES:BEGIN` /
   `SPELL-O-PHONES:END` comments, cut by `scripts/build-web.sh` under
   SPELL_WEB=1 -- the same cut that strips Spell Picture.
2. **A scan that proves the cut happened.** Modelled on
   `web-picture-wall-scan.mjs`, which exists because a cut that is assumed is a
   cut that silently stops happening. Any sentinel content surviving into
   `dist/` fails the build.
3. **Rust gated too**, so the mode is unreachable even if markup leaked.

Cutting between explicit sentinels rather than pattern-matching selectors is
deliberate: the regions are visible in the source, and a renamed class cannot
quietly defeat the strip.

---

## Forward-compatibility seams

1. **`axis` enum** on every set. One legal value in v1: `orthographic`.
   Reserved: `tonal` (zh), `graphemic` (ja/zh), `jamo` (ko).
2. **`dialectScope` enum.** `UNIVERSAL` in v1; `US_ONLY`/`GB_ONLY` reserved.
3. **N-member sets, never pairs.** Never `word_a, word_b`.
4. **Carriers keyed `(setId, memberId)`**, not fields on a word.

Carrier authoring rule: dialect-neutral vocabulary. No sidewalk, gotten, math,
or fall-for-autumn.

---

## Decisions

| # | Resolved |
|---|---|
| D1 | Dead. English definitions are live; no CC-DEFS-DARK gate. |
| D3 | Near-homophones IN, tagged `nearHomophone`, higher tiers only. |
| D4 | Sweep scoring is binary. Completeness is the point. |
| D5 | Post-build-56. |
| D6 | `WRONG_MEMBER` costs a Climb shield segment identically to a miss. |
| D7 | Kid Mode YES, tier 1 only, curated families. Little Speller no. |
| D8 | en-GB is a full content locale, deferred entirely. |
| D12 | Tier 1 for exactly one family, then tier 2 immediately. |

| # | Pending Eric | Recommendation |
|---|---|---|
| D2 | Accent-conditional sets in v1? | Exclude. F0 found only 4, all wh- merger. |
| D9 | Wrong Sentence text only, or text + emoji? | Text + emoji. |
| D10 | Does tier 2's blind gamble cost a shield? | Yes, no exception. |
| D11 | Punchline pack in v1? | v1, ~40 items. |
| D13 | Which English voice speaks this mode? | **RESOLVED — pinned to Pack -> ServerCache, NativeTts removed, mirroring zh. Opens a verification obligation: every set must be confirmed to merge in the pinned voice, because CMUdict is a dictionary and not the voice.** |

---

## Done

1. F0 inventory table exists, includes the set-size distribution, and Eric has
   read it. **-- table exists; awaiting Eric.**
2. A homograph pair fails export by construction.
3. A set with an unbanked member fails export whole.
4. A deliberate multi-legal carrier fails validation.
5. `WRONG_MEMBER` renders the Wrong Sentence; a misspelling does not.
6. Tier 2 never reveals the carrier before input closes.
7. F6 blanks leak nothing -- tested on unequal member lengths.
8. A family reports gold only when every member has been spelled correctly.
9. Blitz consumes the content with zero mode-specific code.
10. `spellOPhonesEnabled` false for all fourteen non-English languages.
11. Schema review confirms the four seams.
12. Eric plays a tier-2 run, misses one on purpose, and laughs.

---

## Non-goals — do not touch

- Do not generate carriers or punchlines with an LLM.
- Do not build a homophone table. Sets derive from the trap taxonomy.
- Do not add a red X, buzzer, or shake to `WRONG_MEMBER`.
- Do not touch the Replay Invariant. Replay stays unlimited, penalty-free,
  0.7x slow, and is deliberately useless here.
- Do not touch the Freshness Invariant. This mode has the smallest pool of any
  mode specced and should fail the build rather than silently repeat.
- Do not touch anything in the build-56 P0 stack.
- No IPA in player-facing text.
- No merger-verification harness in v1.

---

## What to prototype before any of this

Not the mode. ONE SCREEN. Hear /pear/, spell it blind, get the Wrong Sentence.
Ten families, one tier, no scoring, no shelf, no Blitz, no Reports.

Twenty minutes of play answers the only question that matters: **does the Wrong
Sentence stay funny at repetition twenty, or go flat at three?**

F0 does not block this. It needs ten families and 58 exist -- though `pair`
cannot be one of them until `pear` is banked, which is itself a small piece of
evidence about how much bank work the full mode needs.
