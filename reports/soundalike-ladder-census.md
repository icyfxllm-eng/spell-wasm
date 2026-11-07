# CC-SOUNDALIKE-LADDER v1: §0 census

Run 2026-11-07, read-only, on branch `census/ortho-all` (416e2d24, unpushed). Nothing was built or changed except this report.

## Stop-and-ask items found

1. **D1 names es-419, which was withdrawn.** The spec says "es-419 per CC-ORTHO-FIX-FIL-ROMANCE D1". Eric withdrew that on 2026-11-06 ("keep the Spain voice"); the server voice is `es-ES-Neural2-G` and the es table is verified for it. I probed that voice. Unless told otherwise the ladder uses es-ES-Neural2-G.
2. **F3 cannot be met for seven of the eight languages with what is on this machine** (C3 below). kaikki IPA is cached locally for fil only (plus en, ru, fa, outside this job). Without it nothing in fr, pt, pl, sw, ko or vi can pass F3, so they would stop at machine_checked until the dumps are fetched. Fetching is a download of untrusted files and needs your yes.
3. **The loopback needs same-sound and different-sound controls per language**, and only en, es and ru have them (`tools/soundalike_loopback.py` CONTROLS). Its bar is calibrated on them. For fr, pt, pl, fil, sw, ko and vi I would have to author the control pairs, and a bad "same" control moves the bar. Proposal: pick controls from the closed seed lists (fr ou/où-type, pt, pl ó/u and rz/ż, ko batchim) and list them in the report for your review.

## C1: table status, seeds, staged rules

| Lang | Table status | Sets | Seed lines in `homophones.seed.txt` | Staged rules (`data/<lang>/`) |
|---|---|---|---|---|
| en | verified | 252 true, 27 accent | 32 | none |
| es | verified | 9 true | 5 | `sound-alike-rules.json` (5 rules, 19 seeds), `near-sound-rules.json` (2) |
| fr | missing | 0 | 0 | rules `fr_g2p` true, `fr_in_un` accent; 21 seeds; near-sound 3 |
| pt | missing | 0 | 0 | rules `pt_sibilant`, `pt_chx`, `pt_h`, `pt_gj` true; `pt_unstressed`, `pt_lu_final` accent; 21 seeds; near-sound 2 |
| pl, fil, sw, ko, vi | missing | 0 | 0 | none |
| de | missing (but 22 true sets from seeds are read by grading) | 22 | 22 | none |
| ru | verified | 8 true, 4 accent | 0 | none |
| zh, hi (design note only) | missing | 0 | 0 | none |

The table schema has `kind` (true/accent) and a `loopbackFlag` per set, and one status per language. **It has no per-set status**, so F1 (missing, machine_checked, provisional, verified per set; language = lowest set; reason codes) needs a schema addition. The existing en/es/ru "verified" would map to the new `verified`; de's seed sets have no loopback and would sit at machine_checked-or-lower, which must not change de grading (constraint).

## C2: loopback reachability (local server on :8080, `/api/speak`)

Four easy and four expert bank words per language, fetched through the Mac's server. Every one rendered (valid mp3, HTTP 200). The expert words came back freshly written (Last-Modified equal to Date), so live synthesis works right now, not only cached clips. I cannot tell from here whether Google billing is back on; this contradicts my 2026-09-30 note and should be checked before anyone relies on it. The probe sent 64 requests to the live server and may have added cache entries.

| Lang | Server voice | Renders |
|---|---|---|
| es | es-ES-Neural2-G (spec says es-419: see item 1) | 8/8 |
| fr | fr-FR-Neural2-F | 8/8 |
| pt | pt-BR-Neural2-B | 8/8 |
| pl | pl-PL-Wavenet-B | 8/8 |
| fil | fil-PH-Wavenet-A | 8/8 |
| sw | sw-TZ-RehemaNeural | 8/8 |
| ko | ko-KR-Wavenet-D | 8/8 |
| vi | vi-VN-Wavenet-A | 8/8 |

"Northern (Hanoi)" for vi cannot be confirmed from the repo or the voice name; it needs a listening check or a voice-catalogue lookup. The sw voice is Tanzanian (sw-TZ). No language failed to render, so the voice stop condition has not fired.

## C3: kaikki IPA coverage of the banks

Local kaikki caches: fil (Tagalog), en, ru, fa. None for fr, pt, pl, ko, vi, sw. The repo's `lexicon.jsonl` files are 160-200-line curated samples with `pron: null`, not usable. Hunspell dictionaries exist for pt, pl, sw (spelling only, no IPA).

| Lang | Rows with IPA |
|---|---|
| fil | 63.0% overall (Easy 98.8%, Medium 81.4%, Hard 62.4%, Expert 53.4%) |
| fr, pt, pl, sw, ko, vi | no dump on this machine: 0% available. Coverage unknown until fetched |

## Not touched

Production grading, en, de, the build-56 path, and the ortho flags. No kaikki or Wiktionary IPA is in any shipped file; the grep test the done-when asks for does not exist yet.
