# CC-ORTHO-SWEEP-DE-JA v1 / v1.1: decision record

The spec text was pasted in chat on 2026-11-04 and is not in the repo; this file records what Eric decided against it and where it landed. Census: `reports/ortho-census-de-ja.md`.

## Signed (Eric, 2026-11-04)

| Item | Decision | Landed in |
|---|---|---|
| F4 regional tags | **Dropped from v1.** The census could not run it (no dictionary tag source); D5 and T5 are deferred with it. | nothing built |
| Tag coverage | The 5% tag-coverage bar is **waived for tags**. | n/a |
| D2 / D3 | **Normal mode only**; Spell Jr keeps the lenient fold. A real grader change, not a data file. | `src/norm.rs` `same_answer` |
| F7 / F8 | **Held** until CC-VOICE-CLARITY v1.1 (intake format) is committed. Candidate lists are staged as data only; nothing reads them. | `data/de/near-sound-candidates.tsv`, `data/ja/near-sound-candidates.tsv`, `data/ja/devoicing-pairs.tsv`, `tools/ortho/stage_near_sound.py` |
| T11 to T13 | **Rewritten to fixture words**, with a t/d plus vowel-length rule. Final devoicing (d/t before a consonant or at word end) makes a homophone, which goes to the SENSE-CUE table, not the near-sound list. | `data/de/homophone-candidates.tsv`, `assets/words/de/homophones.seed.txt` |
| ja づ/ず | 現代仮名遣い 本文 第2: ずつ and なかんずく stored; づつ and なかんづく accepted; きづく kept; づき quarantined as a fragment. | `data/ja/accepted-variants.tsv`, `tools/bank/purity_overrides.csv` |
| de lists | Source is the Amtliches Regelwerk 2024 (full PDF), every row cites a printed page. The Wörterverzeichnis is not exhaustive: F3/F10 cover only variants it lists; F9 covers listed together/apart entries only, no rule-inferred splits. | `data/de/struck-variants.txt`, `data/de/accepted-variants.tsv`, `data/de/together-apart.tsv` |
| D11 | CC-IMPOSSIBLE-COMPLETE D-C4 is **unsigned**: lowercase grading everywhere. If signed: Impossible-only case grading. | no change |

| D-NOBYSA | Wiktionary and JMdict are **check-only**, never a source for shipped data. The ja variant rows now cite 現代仮名遣い and Eric's rulings as source, with JMdict as the check. | `data/ja/accepted-variants.tsv` |

## F9 in practice (merged to main with Eric's go, 2026-11-04)

In German, a typed answer with a space inside it grades only if `data/de/together-apart.tsv` lists it for the stored word (`accepted::split_listed`). Before this change `fold_strict` dropped all whitespace, so `kennen lernen` graded equal to `kennenlernen` and so did any split. Four bank words have a listed split: dahingehend, hierzulande, sodass, weitgehend. Tests: `accepted::tests::de_listed_splits_are_accepted_and_unlisted_splits_are_not` (T14, T15) and `de_split_rule_is_normal_mode_and_german_only` (T16).

## Open

- ja dual_script (420 rows): derived from JMdict, so provisional under D-NOBYSA. Eric chose an auditor-signed replacement (2026-11-04): the ja auditor fills `reports/ja-dual-script-auditor-sheet.csv`, then `python3 tools/ortho/ja_dual_ingest.py <sheet> --auditor NAME` rewrites `data/ja/dual-script.tsv`.
- Frigidär: neither it nor a replacement is in the Regelwerk, so it has no page; parked in `data/de/struck-variants.unverified.txt`.
- ざつぎょ: quarantined as unsure until the ja auditor rules.
- F7/F8 intake converter: waits for VOICE-CLARITY v1.1.
