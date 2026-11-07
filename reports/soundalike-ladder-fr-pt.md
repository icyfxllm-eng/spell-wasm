# CC-SOUNDALIKE-LADDER: fr and pt status report (2026-11-07)

Branch `census/ortho-all`, nothing pushed. Both languages report **machine_checked** (their lowest set); `status` stays `missing`, so cues and the bonus are off and Normal grading reads none of these sets in production.

## Set counts

| Lang | Ladder status | True sets | provisional | machine_checked | Why machine_checked did not advance |
|---|---|---|---|---|---|
| fr | machine_checked | 1,165 | 44 | 1,121 | same_lemma_inflection 1,066; loopback_flag 28; ipa_missing 25; key_disagrees 1; ipa_differ 1 |
| pt | machine_checked | 25 | 20 | 5 | loopback_flag 2; ipa_missing 1; key_disagrees 1; ipa_differ 1 |

(plus fr and pt each carry their accent seed sets: fr brin/brun; pt mal/mau, kept `machine_checked`, reason `accent_rule`.)

## STOP CONDITION FIRED: more than 20% of rule-derived sets disagree under F3

| Lang | Key-derived lexical sets | Disagree (loopback flag, IPA differ, key disagrees) | Share |
|---|---|---|---|
| fr | 77 | 25 | 32% |
| pt | 5 | 2 | 40% (n is tiny) |

The three signals are working as designed: the key proposes, the loopback and IPA block. A set the voice may pronounce differently never reaches provisional. Read it as "the key over-proposes", not "the table is wrong". For fr the disagreements are mostly the loopback (28): borrowed words (bugs/bus, cash/casse, gangs/gens), interjections (euh/heu) and sets where the voice reads inflected endings differently. I fixed three key bugs along the way (annexe/année, guet/jet, fig/fis); the remaining cases are below the key's reach without a real G2P. **Decision for Eric:** keep the key as it is (it only proposes, and the ladder blocks the bad ones), or stop here and treat the fr key as not good enough.

## What was built

- `tools/soundalike_keys.py`: `key_fr`, `key_pt`, a must-merge list (reported when the key misses a closed seed) and a **must-not-merge list that fails the selftest** (fr 24 pairs incl. brin/brun, pêche/péché, poisson/poison, dessus/dessous, guet/jet, annexe/année; pt 19 pairs incl. mal/mau, pode/pôde, avó/avô, caça/casa, assar/azar). Gate hook added.
- `tools/soundalike_ladder.py`: builds the table (key groups, staged seeds, the four fr closed-list pairs ou/où a/à la/là sur/sûr), with a `ladder {status, reason, origin}` object on every set. Wired into `tools/build_collisions.py` (`--check` passes).
- `src/soundalikes.rs`: a set with no `ladder` object (en, es, ru, de) reads exactly as before. A ladder set is readable for grading only when `verified`, or `provisional` while the new flag `soundalike_provisional` is on. Flag default **off**; TestFlight and Theme Lab release branches flip it. `sets_for_exclusion` reads machine_checked and above. Python and node APIs take `use=grading|testbuild|exclusion`. Test `ladder_status_gates_every_reader` pins all of it.
- `tools/soundalike_ipa_check.py`: reads the kaikki dumps (check only) and writes one word per set (agree, differ, missing) to `data/<lang>/soundalike-ipa-check.json`. `scripts/ipa-leak-check.py` fails the gate if any IPA character or IPA key appears in a ladder file (passes; I reworded two phoneme names in `data/pt/sound-alike-rules.json` that it caught).
- Loopback controls for fr and pt come from the closed seed lists (fr same: sang/sans, cou/coup, saint/sein, chant/champ; different: chat/maison, pain/bureau. pt same: hora/ora, cela/sela, cinto/sinto, cheque/xeque; different: mesa/janela, carro/livro). Bars: fr 0.638 (floor 0.563, ceiling 0.936), pt 0.702 (floor 0.645, ceiling 0.931). Voices: fr-FR-Neural2-F, pt-BR-Neural2-B; both rendered.
- `reports/soundalike-gigb-fr.csv` (99 rows) and `-pt.csv` (25 rows): disagreement sets first, then provisional, then the rest. Not sent. fr leaves off 1,066 same_lemma_inflection sets.

## Decisions I made that you should check

1. **Inflection sets are capped at machine_checked** (technique/techniques, aimer/aimé/aimez). They sound identical in French, but typing the plural for the singular is a spelling error, not another sense, so accept-any would mark a wrong spelling correct. They still feed Impossible exclusion, which is correct there (a player cannot tell them apart by ear). Curated and closed-list seeds are never treated as inflections (vers/vert/verre).
2. **Impossible fr pool shrank 1,683 to 1,609** (74 words) from the new exclusion sets. That is the intended F2 effect, but it is a production change to a shipped artifact. pt did not change (369 either way).
3. The pt key keeps written accents (vera is not verá, avó is not avô); an earlier draft dropped them and over-merged.
4. A sound-alike set can also be an accent or inflection case the key cannot see; the loopback store keeps a few stale set ids from earlier runs (carry-forward).

## Done-when check

- fr and pt report machine_checked or higher, with counts: yes.
- Key tests pass and every must-not-merge pair stays separate: yes.
- No shipped file carries kaikki IPA, with a CI grep: `scripts/ipa-leak-check.py` in the gate.
- Production grades every non-verified set as today: yes, test above; `cargo test --lib` 1,677 pass.
- TestFlight accepts both members of a provisional set: **fr ou/où and pt hora/ora yes (tested). es te/té is not covered by this table**: es is not a ladder language here; te/té is accepted by the es ortho profile path (accent-collisions.tsv) when `ortho.profile.es` is on, not by `soundalike_provisional`. Say if you want es moved onto the ladder.
- Gig B sheet exists and is sorted: yes, not sent.

## Not done / not checked

- The full gate was not rerun after these changes (lib tests, collision check, api check, keys selftest, leak check pass individually). e2e not rerun.
- pl, fil, sw, ko, vi: not started (order: stop for your review after fr and pt).
- vi voice "northern" still unconfirmed; es-419 conflict still unresolved (es unchanged).
