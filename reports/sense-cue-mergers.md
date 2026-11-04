# CC-SENSE-CUE v1.2 (Mergers) section 0 census (read-only), 2026-11-04

Method: the shipped en key (CMUdict via `tools/build_collisions.key_en`) over the en bank (3,157 rows; Expert 797). A rule's "newly removed" count is Expert words in a rule-created group that are not already in a shipped table set (10 Expert words already are). German has no generator (`status: missing`), so M3 is a spelling heuristic only.

## Step 1: v1.1 mechanism

Present on origin/main (Phase B, 59dfefb2): `config/soundalike-merger-rules.json`, `MERGER_RULES` in `tools/build_collisions.py`, `accent_sets()`. cot-caught runs through it. No HALT on this step.

## Results

| Rule | Pairs created | Expert rows newly removed | % of Expert |
|---|---|---|---|
| M1 wine-whine (en) | 0 | 0 | 0.00% |
| M2 Mary-marry-merry (en) | 0 | 0 | 0.00% |
| M3 ä-e (de) | not computable; heuristic found 6 candidate groups, 3 Expert words | about 3 | about 0.10% of 2,987 |
| pin-pen (excluded) | 3: en/in, enter/inner, sense/since | 0 | 0.00% |

No rule is near MG5's 10% threshold.

## Findings that need Eric

1. **M1 is already `kind: true`, not `accent`.** CMUdict gives which/witch, whine/wine, weather/whether, ware/wear/where the same phonemes (no HH), so they sit in true sets today. T2 (`which`/`witch` as `accent` under `wine-whine`) cannot pass as written: the rule would only re-label true sets as accent, and I9 forbids that, and it would take them out of accept-all grading and the sound-alike bonus, which contradicts "no change to Normal grading". wail/whale is not a pair either way.
2. **M2 is already registered and active** (`mary-marry-merry`) and finds no bank pairs. fairy, ferry, Harry and hairy are not in any en bank file; Mary/marry/merry are absent as well.
3. **pin-pen is already registered `active`** in `config/soundalike-merger-rules.json` and produces 3 accent sets in the shipped table. MG4 and T4 ("no pin/pen pair exists") conflict with that file. Removing it is a decision, and it costs 0 Expert rows either way.
4. **M3 has no mechanism to run on.** de is `missing` with no generator and no German phonetic key. Ähre, Ehre, Bären, gäbe, gebe, Säle and Täler are not in the German bank; Beeren (Expert), Seele (Easy) and Teller (Easy) are. The heuristic groups (Dänen/denen, Welten/wählten, beenden/bänden, and others) are unreliable: Welt/wählt differs in vowel length, so it is not a merger. A real M3 needs a German generator first, which v1.1 did not build. Expect near zero bank pairs, so the Gig B timing hope rests on a table, not on this rule.

## Recommendation

M1 and M2 add nothing to Impossible and M1 would change grading if forced; sign neither as new work. M3 is a German-generator task. Decide on finding 3.

HALT. Phase M waits for Eric.
