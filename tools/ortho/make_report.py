#!/usr/bin/env python3
"""Assembles reports/ortho-census-de-ja.md from reports/ortho-census/*.csv + _summary.json (read-only)."""
import csv, json, os
R = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "reports")
D = os.path.join(R, "ortho-census"); S = json.load(open(os.path.join(D, "_summary.json"), encoding="utf-8"))
nde, nja = S["rows"]["de"], S["rows"]["ja"]
def rows(k): return list(csv.DictReader(open(os.path.join(D, k + ".csv"), encoding="utf-8")))
def sample(k, n=20):
    r = rows(k); step = max(1, len(r) // n)
    return r[::step][:n]
def tsv(name, n=20):
    L = [l.rstrip("\n").split("\t") for l in open(os.path.join(D, name), encoding="utf-8")]
    return L[:n], len(L)
o = []
w = o.append
w("# CC-ORTHO-SWEEP-DE-JA v1 §0 census + v1.1 C7–C12 — HALTED for Eric's review\n")
w("Read-only. Run 2026-11-04 on `origin/main` (worktree `~/repos/spell-wasm-ortho`). Tooling: `tools/ortho/census_de_ja.py` (re-runnable, byte-stable), report assembled by `tools/ortho/make_report.py`. "
  "Row-level output: one CSV per check in `reports/ortho-census/` (`row_id, lang, tier, stored_answer, check_id, finding, proposed_action`). No bank row, registry entry, grader or app code changed. "
  "Sign-off assumption: Eric pasted v1 and v1.1 into chat and I treated that as the signature to run §0; nothing here is Phase A.\n")
w("## What stops the next step (read this first)\n")
w("1. **No de regional tags anywhere (F4 could not run).** Hunspell de_DE_frami, `defs/de.jsonl` (word/pos/definition/form_of) and the English-only kaikki dump carry no Austrian/Swiss marker. v1's own stop rule applies: the dictionary authority lacks the tags F4 needs. Eric to name a source (e.g. a German Wiktionary or Duden export) or drop F4.")
w(f"2. **de part-of-speech is partial.** Only {S['de_pos_coverage']} de rows have a POS in `defs/de.jsonl`; frami has none. F2's noun-case check ran as a Hunspell-capitalisation proxy (0 hits) plus POS where present (1 hit). The remaining rows are unchecked.")
w("3. **Conflicts with existing behaviour, not resolved (v1: stop and ask).**\n"
  "   - Kid Mode `fold_lenient` maps ß→ss and strips umlauts (`src/norm.rs`, test `answer_matches(\"strasse\",\"straße\",true)`, and `a13_accent_folding_is_spell_jr_only`). v1 D2/D3 say ss for ß and ae/oe/ue for umlauts are graded wrong with no silent folding. Normal mode already rejects them; Spell Jr/Kid Mode accepts ss for ß and bare vowels for umlauts. Does D2/D3 reach Kid Mode, or is A13 untouched?\n"
  "   - `fold_strict` lowercases everything (module doc: \"v1: no capitalization mechanics\"). D11 says capitalization is graded at Impossible per CC-IMPOSSIBLE-COMPLETE D-C4; I did not read that file, so I cannot say whether D11 matches the grader.\n"
  "   - `fold_strict` drops all whitespace on both sides. So `kennen lernen` already grades equal to `kennenlernen`, and so does any unlisted split. v1.1 T14 (\"rejects an unlisted split\") fails today and F9 needs a grader change, not only a data file.\n"
  "   - ja has no entry in the per-language key registry (`language_key_matches` knows only vi and fa), so F6 adds the first ja entry. Half-width katakana is not folded today.")
w(f"4. **>5% on one check:** F5d `dual_script` tags {S['findings']['F5d_dual_script']}/{nja} ja rows ({S['findings']['F5d_dual_script']/nja*100:.1f}%). That is a tag, not a defect, but v1's literal stop rule trips. All other checks are under 2.5%.")
w("5. **C12: no de/ja intake in CC-VOICE-CLARITY. HALT on F7/F8 and ask.** The VOICE-CLARITY spec is not in the repo; the only artefact is `reports/voice-clarity-census.md`, which says \"Non-English clips were not measured (Phase D)\". No de/ja intake format exists to hand the pair files to.")
w("6. **v1.1 T11–T13 cannot pass from the current banks.** None of the seed pairs exist in the banks (table below), and `たくさん` is not a ja row. Staat/Stadt is not produced by the v1.1 rule list (it needs t↔d plus vowel-length), so T11 needs a rule added or the seed dropped.\n")
w("## Counts\n")
w("| | easy | medium | hard | expert | total |\n|---|---|---|---|---|---|")
for l in ("de", "ja"):
    t = S["tiers"][l]; w(f"| {l} | {t['easy']} | {t['medium']} | {t['hard']} | {t['expert']} | {S['rows'][l]} |")
w("\n**Where things live.** Banks: `assets/words/{de,ja}/{easy,medium,hard,expert}.txt` (one answer per line; ja is 100% hiragana). Purity ledgers: `assets/words/purity/{de,ja}.tsv`. "
  "Answer normalization: `src/norm.rs` (`fold_strict`, `fold_lenient`, `same_answer`, `language_key_matches`; registry today = vi, fa). Variant/alias table: none exists (`data/de/accepted-variants.tsv` and `data/ja/accepted-variants.tsv` are not in the repo; `data/` has no `de` directory). "
  "Dictionaries (build inputs under `.corpus-cache`, not committed): Hunspell de_DE_frami; JMdict_e (readings, pos, lsource). Nothing was moved.\n")
w("## Per-check results\n")
w("| Check | Rows flagged | % of lang | Status |\n|---|---|---|---|")
F = S["findings"]
def n(k): return F.get(k, 0)
spec = [("F1a pre-reform ß / daß", "F1a_pre_reform_ss", "de", ""), ("F1b triple-consonant compounds", "F1b_triple_consonant", "de", ""),
 ("F1c struck-list seed (29 words, unverified)", "F1c_struck", "de", "seed list is Eric's paste, not checked against the Regelwerk; no data file yet"),
 ("F2a ae/oe/ue where headword has umlaut", "F2a_umlaut_digraph", "de", ""), ("F2b ß/ss disagrees with headword", "F2b_ss_vs_eszett", "de", "rows where both ss and ß forms are real words (Masse/Maße) are not decidable without POS/sense"),
 ("F2c noun shown lowercase (Hunspell proxy)", "F2c_noun_lowercase_proxy", "de", "proxy"), ("F2d POS vs capitalisation", "F2d_pos_case", "de", "5552/6263 rows have POS"),
 ("F3 official double spellings (candidates)", "F3_double_spelling_candidate", "de", "candidates only: dictionary accepts both forms; the Regelwerk list is needed to pick stored/accepted"),
 ("F4 regional AT/CH", "F4_weak_signal_definition_text", "de", "COULD NOT RUN (no tags); rows are a text-match hint only"),
 ("F5a ja long-vowel おお/おう/…/えい", "F5a_long_vowel", "ja", ""), ("F5b ぢ/づ mismatch", "F5b_zu_di_mismatch", "ja", ""),
 ("F5b ぢ/づ both forms in JMdict (variant file)", "F5b_zu_di_both_listed", "ja", ""), ("F5c gairaigo in hiragana", "F5c_gairaigo_in_hiragana", "ja", "PURITY recheck"),
 ("F5d dual_script candidates", "F5d_dual_script", "ja", ">5%: stop rule"), ("F6 look-alike / half-width in stored answers", "F6_lookalike_or_halfwidth_stored", "ja", "zero stored"),
 ("C9 rows with space or hyphen", "C9_space_or_hyphen_row", "de", ""), ("C9 together/apart candidates", "C9_together_apart_candidate", "de", "heuristic: leading verb + infinitive"),
 ("C10 particle-reading candidates", "C10_particle", "ja", "wide net; needs a human read"), ("C11 participle-shaped rows", "C11_participle_form", "de", "report only")]
for lab, k, l, st in spec:
    tot = nde if l == "de" else nja
    w(f"| {lab} | {n(k)} | {n(k)/tot*100:.1f}% | {st} |")
w(f"\nHunspell rejects only {S['de_hunspell_rejected']} of {nde} de rows, so the de bank has already been cleaned of anything spell-checkable by the PURITY sweep. That is why F1/F2 are empty. "
  "F5a/F5b mismatches are mostly rows that are not JMdict headwords at all (PURITY's lane); the ones with a swapped form that JMdict does list are the real ones.\n")
w("## Samples (20 per check, or all if fewer)\n")
for lab, k, l, st in spec:
    if n(k) == 0: continue
    w(f"### {k} ({n(k)})\n")
    w("| tier | stored | finding |\n|---|---|---|")
    for r in sample(k): w(f"| {r['tier']} | {r['stored_answer']} | {r['finding'][:110].replace('|','/')} |")
    w("")
w("## v1.1 census\n")
a, tot7 = tsv("C7_de_near_sound_candidates.tsv", 20)
w(f"### C7 — de near-sound pair candidates\nRule-set pairs (ß/ss, ie/i, double/single, h): **{S['C7_pairs']}**. Extra-rule pairs (t↔d, vowel doubling; not in v1.1): **{S['C7_extra_rule_pairs']}**. Under the 2,000 limit. Full list: `reports/ortho-census/C7_de_near_sound_candidates.tsv` (not `data/de/…`: that path is Phase D).\n")
w("Seed pairs:\n"); w("| pair | status |\n|---|---|")
for k, v in S["C7_seeds"].items(): w(f"| {k} | {v} |")
w("\nSamples:\n\n```\n" + "\n".join("\t".join(x) for x in a) + "\n```\n")
a, tot8 = tsv("C8_ja_near_sound_candidates.tsv", 20)
w(f"### C8 — ja near-sound pair candidates\n**{S['C8_pairs']}** pairs (rules: long vowel, っ, small kana, ん+vowel). Under the limit. Many are a bank fragment next to its full word (いど/いどう), which the VOICE-CLARITY gate would judge. Devoicing sites tagged on **{S['C8_devoicing_tagged_rows']}** rows (`C8_ja_devoicing_tags.tsv`, 1-based mora index; a final す/し counts as a site wherever it follows, which includes です, since the spec's wording is ambiguous). T13: {S['C8_T13_たくさん']}.\n")
w("Seed pairs:\n"); w("| pair | status |\n|---|---|")
for k, v in S["C8_seeds"].items(): w(f"| {k} | {v} |")
w("\nSamples:\n\n```\n" + "\n".join("\t".join(x) for x in a) + "\n```\n")
w(f"### C9 — de together/apart\nRows with a space: {S['C9_rows_with_space_or_hyphen']['space']}; with a hyphen: {S['C9_rows_with_space_or_hyphen']['hyphen']}. So D15's exclusion has nothing to exclude today. {n('C9_together_apart_candidate')} candidates by heuristic only (the rulebook list is not available). {S['C9_note']}\n")
w(f"### C10 — ja particle readings\n{n('C10_particle')} candidates, mostly JMdict expressions/interjections containing は/へ; only 1 row contains を. Heuristic, not a confirmed set; Eric or an auditor should read the sample before F10 is built on it.\n")
w(f"### C11 — de loanword inflections\nNone found. {S['C11_participles_total']} rows are shaped like participles (ge…t); inspection of the {S['C11_no_dictionary_infinitive']} with no dictionary infinitive shows native verbs and finite forms (genehmigt, generiert, gerät), not gedownloadet-type forms. "
  "Hunspell accepts inflected forms, so PURITY's dictionary check does not catch participles of native verbs either; I did not test PURITY's dictionary-form rule beyond that.\n")
w("### C12 — VOICE-CLARITY de/ja intake\nNone found. See the stop list.\n")
w("## Not done / limits\n- Struck-variant seed list unverified; `data/de/struck-variants.txt` does not exist and I did not create it.\n- F3/C9/C10 depend on lists (Regelwerk, rulebook) that are not available locally.\n- Impossible pools and Spell Jr/Kid lists were not scanned; only the four tier files per language.\n- No tests run; T1–T10 need Phase A–C code that does not exist.\n- Marginal: ja stored answers are readings only, so F5 compares against JMdict readings, which makes JMdict the sole authority for 現代仮名遣い.\n")
open(os.path.join(R, "ortho-census-de-ja.md"), "w", encoding="utf-8").write("\n".join(o))
