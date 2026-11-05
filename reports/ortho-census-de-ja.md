# CC-ORTHO-SWEEP-DE-JA v1 §0 census + v1.1 C7–C12 — HALTED for Eric's review

Read-only. Run 2026-11-04 on `origin/main` (worktree `~/repos/spell-wasm-ortho`). Tooling: `tools/ortho/census_de_ja.py` (re-runnable, byte-stable), report assembled by `tools/ortho/make_report.py`. Row-level output: one CSV per check in `reports/ortho-census/` (`row_id, lang, tier, stored_answer, check_id, finding, proposed_action`). No bank row, registry entry, grader or app code changed. Sign-off assumption: Eric pasted v1 and v1.1 into chat and I treated that as the signature to run §0; nothing here is Phase A.

## What stops the next step (read this first)

1. **No de regional tags anywhere (F4 could not run).** Hunspell de_DE_frami, `defs/de.jsonl` (word/pos/definition/form_of) and the English-only kaikki dump carry no Austrian/Swiss marker. v1's own stop rule applies: the dictionary authority lacks the tags F4 needs. Eric to name a source (e.g. a German Wiktionary or Duden export) or drop F4.
2. **de part-of-speech is partial.** Only 5552/6263 de rows have a POS in `defs/de.jsonl`; frami has none. F2's noun-case check ran as a Hunspell-capitalisation proxy (0 hits) plus POS where present (1 hit). The remaining rows are unchecked.
3. **Conflicts with existing behaviour, not resolved (v1: stop and ask).**
   - Spell Jr `fold_lenient` maps ß→ss and strips umlauts (`src/norm.rs`, test `answer_matches("strasse","straße",true)`, and `a13_accent_folding_is_spell_jr_only`). v1 D2/D3 say ss for ß and ae/oe/ue for umlauts are graded wrong with no silent folding. Normal mode already rejects them; Spell Jr/Spell Jr accepts ss for ß and bare vowels for umlauts. Does D2/D3 reach Spell Jr, or is A13 untouched?
   - `fold_strict` lowercases everything (module doc: "v1: no capitalization mechanics"). D11 says capitalization is graded at Impossible per CC-IMPOSSIBLE-COMPLETE D-C4; I did not read that file, so I cannot say whether D11 matches the grader.
   - `fold_strict` drops all whitespace on both sides. So `kennen lernen` already grades equal to `kennenlernen`, and so does any unlisted split. v1.1 T14 ("rejects an unlisted split") fails today and F9 needs a grader change, not only a data file.
   - ja has no entry in the per-language key registry (`language_key_matches` knows only vi and fa), so F6 adds the first ja entry. Half-width katakana is not folded today.
4. **>5% on one check:** F5d `dual_script` tags 420/4974 ja rows (8.4%). That is a tag, not a defect, but v1's literal stop rule trips. All other checks are under 2.5%.
5. **C12: no de/ja intake in CC-VOICE-CLARITY. HALT on F7/F8 and ask.** The VOICE-CLARITY spec is not in the repo; the only artefact is `reports/voice-clarity-census.md`, which says "Non-English clips were not measured (Phase D)". No de/ja intake format exists to hand the pair files to.
6. **v1.1 T11–T13 cannot pass from the current banks.** None of the seed pairs exist in the banks (table below), and `たくさん` is not a ja row. Staat/Stadt is not produced by the v1.1 rule list (it needs t↔d plus vowel-length), so T11 needs a rule added or the seed dropped.

## Counts

| | easy | medium | hard | expert | total |
|---|---|---|---|---|---|
| de | 272 | 1000 | 2000 | 2991 | 6263 |
| ja | 209 | 947 | 1479 | 2339 | 4974 |

**Where things live.** Banks: `assets/words/{de,ja}/{easy,medium,hard,expert}.txt` (one answer per line; ja is 100% hiragana). Purity ledgers: `assets/words/purity/{de,ja}.tsv`. Answer normalization: `src/norm.rs` (`fold_strict`, `fold_lenient`, `same_answer`, `language_key_matches`; registry today = vi, fa). Variant/alias table: none exists (`data/de/accepted-variants.tsv` and `data/ja/accepted-variants.tsv` are not in the repo; `data/` has no `de` directory). Dictionaries (build inputs under `.corpus-cache`, not committed): Hunspell de_DE_frami; JMdict_e (readings, pos, lsource). Nothing was moved.

## Per-check results

| Check | Rows flagged | % of lang | Status |
|---|---|---|---|
| F1a pre-reform ß / daß | 0 | 0.0% |  |
| F1b triple-consonant compounds | 0 | 0.0% |  |
| F1c struck-list seed (29 words, unverified) | 0 | 0.0% | seed list is Eric's paste, not checked against the Regelwerk; no data file yet |
| F2a ae/oe/ue where headword has umlaut | 0 | 0.0% |  |
| F2b ß/ss disagrees with headword | 0 | 0.0% | rows where both ss and ß forms are real words (Masse/Maße) are not decidable without POS/sense |
| F2c noun shown lowercase (Hunspell proxy) | 0 | 0.0% | proxy |
| F2d POS vs capitalisation | 1 | 0.0% | 5552/6263 rows have POS |
| F3 official double spellings (candidates) | 23 | 0.4% | candidates only: dictionary accepts both forms; the Regelwerk list is needed to pick stored/accepted |
| F4 regional AT/CH | 11 | 0.2% | COULD NOT RUN (no tags); rows are a text-match hint only |
| F5a ja long-vowel おお/おう/…/えい | 8 | 0.2% |  |
| F5b ぢ/づ mismatch | 10 | 0.2% |  |
| F5b ぢ/づ both forms in JMdict (variant file) | 1 | 0.0% |  |
| F5c gairaigo in hiragana | 5 | 0.1% | PURITY recheck |
| F5d dual_script candidates | 420 | 8.4% | >5%: stop rule |
| F6 look-alike / half-width in stored answers | 0 | 0.0% | zero stored |
| C9 rows with space or hyphen | 0 | 0.0% |  |
| C9 together/apart candidates | 0 | 0.0% | heuristic: leading verb + infinitive |
| C10 particle-reading candidates | 81 | 1.6% | wide net; needs a human read |
| C11 participle-shaped rows | 138 | 2.2% | report only |

Hunspell rejects only 32 of 6263 de rows, so the de bank has already been cleaned of anything spell-checkable by the PURITY sweep. That is why F1/F2 are empty. F5a/F5b mismatches are mostly rows that are not JMdict headwords at all (PURITY's lane); the ones with a swapped form that JMdict does list are the real ones.

## Samples (20 per check, or all if fewer)

### F2d_pos_case (1)

| tier | stored | finding |
|---|---|---|
| medium | Hamburger | POS adjective, stored capitalised |

### F3_double_spelling_candidate (23)

| tier | stored | finding |
|---|---|---|
| expert | Dänen | dictionary accepts both Dänen and Denen (rule ä~e); Denen in bank: False |
| expert | Ghetto | dictionary accepts both Ghetto and Getto (rule gh~g); Getto in bank: False |
| expert | Panther | dictionary accepts both Panther and Panter (rule th~t); Panter in bank: False |
| expert | Photoshop | dictionary accepts both Photoshop and Fotoshop (rule ph~f); Fotoshop in bank: False |
| expert | Spaghetti | dictionary accepts both Spaghetti and Spagetti (rule gh~g); Spagetti in bank: False |
| expert | Theresa | dictionary accepts both Theresa and Teresa (rule th~t); Teresa in bank: False |
| expert | Umsätze | dictionary accepts both Umsätze and Umsetze (rule ä~e); Umsetze in bank: False |
| expert | Xylophon | dictionary accepts both Xylophon and Xylofon (rule ph~f); Xylofon in bank: False |
| expert | bewährte | dictionary accepts both bewährte and bewehrte (rule ä~e); bewehrte in bank: False |
| expert | fantastischen | dictionary accepts both fantastischen and phantastischen (rule f~ph); phantastischen in bank: False |
| expert | geographische | dictionary accepts both geographische and geografische (rule ph~f); geografische in bank: False |
| hard | Ansätze | dictionary accepts both Ansätze and Ansetze (rule ä~e); Ansetze in bank: False |
| hard | Europäer | dictionary accepts both Europäer and Europeer (rule ä~e); Europeer in bank: False |
| hard | Hertha | dictionary accepts both Hertha and Herta (rule th~t); Herta in bank: False |
| hard | Märkte | dictionary accepts both Märkte and Merkte (rule ä~e); Merkte in bank: False |
| hard | Sätzen | dictionary accepts both Sätzen and Setzen (rule ä~e); Setzen in bank: False |
| hard | gewährt | dictionary accepts both gewährt and gewehrt (rule ä~e); gewehrt in bank: False |
| hard | während | dictionary accepts both während and wehrend (rule ä~e); wehrend in bank: False |
| medium | Josef | dictionary accepts both Josef and Joseph (rule f~ph); Joseph in bank: False |
| medium | Rhein | dictionary accepts both Rhein and Rein (rule rh~r); Rein in bank: False |

### F4_weak_signal_definition_text (11)

| tier | stored | finding |
|---|---|---|
| expert | Mitteleuropa | definition mentions region: Central Europe (a geographic region in the center of Europe, usually including A |
| expert | Sitten | definition mentions region: Sion (a municipality and town, the capital of Vaud canton, Switzerland) |
| expert | österreichisch | definition mentions region: Austrian |
| hard | Bundesrat | definition mentions region: a federal council, especially: the federal council of the Federal Republic of Ge |
| hard | Gemeinderat | definition mentions region: a local authority in Germany, Austria, Switzerland or East Belgium |
| hard | Genf | definition mentions region: Geneva (a canton of Switzerland) |
| hard | Jura | definition mentions region: Jura (a mountain range on the border of France and Switzerland, extending into G |
| hard | Kanton | definition mentions region: canton (state of Switzerland) |
| hard | Kärnten | definition mentions region: Carinthia (a former duchy and historical region of Central Europe, now split pol |
| hard | Steiermark | definition mentions region: Styria (a state of Austria) |
| medium | Rhein | definition mentions region: Rhine (a major river in Switzerland, Liechtenstein, Austria, Germany, France and |

### F5a_long_vowel (8)

| tier | stored | finding |
|---|---|---|
| expert | うっとうし | not a JMdict reading; no swapped form either |
| expert | おもいとうり | not a JMdict reading; no swapped form either |
| hard | ありほう | not a JMdict reading; no swapped form either |
| hard | おおくぼ | not a JMdict reading; no swapped form either |
| hard | おおくり | not a JMdict reading; no swapped form either |
| hard | おおにし | not a JMdict reading; no swapped form either |
| hard | こうりやま | not a JMdict reading; no swapped form either |
| hard | ざいじょうほう | not a JMdict reading; no swapped form either |

### F5b_zu_di_mismatch (10)

| tier | stored | finding |
|---|---|---|
| expert | つづい | not a JMdict reading |
| expert | つづか | not a JMdict reading |
| expert | づがわしりつ | not a JMdict reading |
| expert | づみみずはら | not a JMdict reading |
| expert | づらく | not a JMdict reading |
| expert | てづか | not a JMdict reading |
| expert | ぬまづ | not a JMdict reading |
| hard | たからづか | not a JMdict reading |
| hard | ちかづけ | not a JMdict reading |
| hard | づのおかといい | not a JMdict reading |

### F5b_zu_di_both_listed (1)

| tier | stored | finding |
|---|---|---|
| expert | きづく | JMdict lists both きづく and きずく |

### F5c_gairaigo_in_hiragana (5)

| tier | stored | finding |
|---|---|---|
| expert | あつし | every JMdict entry for this reading has a foreign-language source |
| expert | ざんまい | every JMdict entry for this reading has a foreign-language source |
| expert | てんぷら | every JMdict entry for this reading has a foreign-language source |
| expert | ぱっち | every JMdict entry for this reading has a foreign-language source |
| hard | ぎょうざ | every JMdict entry for this reading has a foreign-language source |

### F5d_dual_script (420)

| tier | stored | finding |
|---|---|---|
| easy | おっぱい | same JMdict entry also writes オッパイ |
| expert | あつし | same JMdict entry also writes アツシ |
| expert | おす | same JMdict entry also writes オス |
| expert | きす | same JMdict entry also writes キス |
| expert | けち | same JMdict entry also writes ケチ |
| expert | しこしこ | same JMdict entry also writes シコシコ |
| expert | たね | same JMdict entry also writes タネ |
| expert | とかげ | same JMdict entry also writes トカゲ |
| expert | のぼり | same JMdict entry also writes ノボリ |
| expert | ひえ | same JMdict entry also writes ヒエ |
| expert | ぶれ | same JMdict entry also writes ブレ |
| expert | むらむら | same JMdict entry also writes ムラムラ |
| expert | わる | same JMdict entry also writes ワル |
| hard | かっこ | same JMdict entry also writes カッコ |
| hard | こら | same JMdict entry also writes コラ |
| hard | つら | same JMdict entry also writes ツラ |
| hard | ぼきん | same JMdict entry also writes ボキン |
| medium | うんこ | same JMdict entry also writes ウンコ |
| medium | さつ | same JMdict entry also writes サツ |
| medium | ねこ | same JMdict entry also writes ネコ |

### C10_particle (81)

| tier | stored | finding |
|---|---|---|
| easy | こんにちは | ends in / contains a particle-looking は・へ |
| easy | ばくはつ | JMdict expression/interjection containing は/へ |
| expert | おそらくは | ends in / contains a particle-looking は・へ |
| expert | はいび | JMdict expression/interjection containing は/へ |
| expert | はこん | JMdict expression/interjection containing は/へ |
| expert | はっこう | JMdict expression/interjection containing は/へ |
| expert | はっぽう | JMdict expression/interjection containing は/へ |
| expert | はれ | JMdict expression/interjection containing は/へ |
| expert | へえ | JMdict expression/interjection containing は/へ |
| expert | へんそう | JMdict expression/interjection containing は/へ |
| expert | をめぐって | contains を |
| hard | すいはん | JMdict expression/interjection containing は/へ |
| hard | はじめまして | JMdict expression/interjection containing は/へ |
| hard | はてな | JMdict expression/interjection containing は/へ |
| hard | はんきょう | JMdict expression/interjection containing は/へ |
| hard | はんする | JMdict expression/interjection containing は/へ |
| hard | へいこう | JMdict expression/interjection containing は/へ |
| medium | こうはい | JMdict expression/interjection containing は/へ |
| medium | はいぼく | JMdict expression/interjection containing は/へ |
| medium | はる | JMdict expression/interjection containing は/へ |

### C11_participle_form (138)

| tier | stored | finding |
|---|---|---|
| expert | gearbeitet | past-participle pattern; infinitive in dictionary: arbeiten; hunspell_accepts_form=True |
| expert | gefordert | past-participle pattern; infinitive in dictionary: fordern; hunspell_accepts_form=True |
| expert | gehandelt | past-participle pattern; infinitive in dictionary: handeln; hunspell_accepts_form=True |
| expert | gekrönt | past-participle pattern; infinitive in dictionary: krönen; hunspell_accepts_form=True |
| expert | geopfert | past-participle pattern; infinitive in dictionary: opfern; hunspell_accepts_form=True |
| expert | geschickt | past-participle pattern; infinitive in dictionary: schicken; hunspell_accepts_form=True |
| expert | geschützt | past-participle pattern; infinitive in dictionary: schützen; hunspell_accepts_form=True |
| expert | gestartet | past-participle pattern; infinitive in dictionary: starten; hunspell_accepts_form=True |
| expert | gewandt | past-participle pattern; infinitive in dictionary: wanden; hunspell_accepts_form=True |
| hard | gedreht | past-participle pattern; infinitive in dictionary: drehen; hunspell_accepts_form=True |
| hard | gefeiert | past-participle pattern; infinitive in dictionary: feiern; hunspell_accepts_form=True |
| hard | gefärbt | past-participle pattern; infinitive in dictionary: färben; hunspell_accepts_form=True |
| hard | gekämpft | past-participle pattern; infinitive in dictionary: kämpfen; hunspell_accepts_form=True |
| hard | gelöscht | past-participle pattern; infinitive in dictionary: löschen; hunspell_accepts_form=True |
| hard | gepflegt | past-participle pattern; infinitive in dictionary: pflegen; hunspell_accepts_form=True |
| hard | gereicht | past-participle pattern; infinitive in dictionary: reichen; hunspell_accepts_form=True |
| hard | gespeichert | past-participle pattern; infinitive in dictionary: speichern; hunspell_accepts_form=True |
| hard | gestört | past-participle pattern; infinitive in dictionary: stören; hunspell_accepts_form=True |
| hard | gewechselt | past-participle pattern; infinitive in dictionary: wechseln; hunspell_accepts_form=True |
| hard | gewählt | past-participle pattern; infinitive in dictionary: wählen; hunspell_accepts_form=True |

## v1.1 census

### C7 — de near-sound pair candidates
Rule-set pairs (ß/ss, ie/i, double/single, h): **6**. Extra-rule pairs (t↔d, vowel doubling; not in v1.1): **12**. Under the 2,000 limit. Full list: `reports/ortho-census/C7_de_near_sound_candidates.tsv` (not `data/de/…`: that path is Phase D).

Seed pairs:

| pair | status |
|---|---|
| masse/maße | absent from bank |
| busse/buße | absent from bank |
| miete/mitte | absent from bank |
| staat/stadt | absent from bank |
| ofen/offen | absent from bank |
| hüte/hütte | absent from bank |

Samples:

```
beten	betten	single->double
busen	bussen	single->double
liegen	ligen	i->ie
nase	nasse	single->double
wahren	waren	h-removed
wider	wieder	i->ie
armeen	armen	EXTRA:vowel-doubling
boden	booten	EXTRA:t<->d + vowel-doubling
boden	boten	EXTRA:t<->d
booten	boten	EXTRA:vowel-doubling
enden	enten	EXTRA:t<->d
endstand	entstand	EXTRA:t<->d
leiden	leiten	EXTRA:t<->d
leidet	leitet	EXTRA:t<->d
seide	seite	EXTRA:t<->d
stadt	statt	EXTRA:t<->d
weide	weite	EXTRA:t<->d
worden	worten	EXTRA:t<->d
```

### C8 — ja near-sound pair candidates
**119** pairs (rules: long vowel, っ, small kana, ん+vowel). Under the limit. Many are a bank fragment next to its full word (いど/いどう), which the VOICE-CLARITY gate would judge. Devoicing sites tagged on **870** rows (`C8_ja_devoicing_tags.tsv`, 1-based mora index; a final す/し counts as a site wherever it follows, which includes です, since the spec's wording is ambiguous). T13: たくさん not in the ja bank.

Seed pairs:

| pair | status |
|---|---|
| おばさん/おばあさん | absent from bank |
| きて/きって | absent from bank |
| びょういん/びよういん | absent from bank |
| きんえん/きねん | absent from bank |

Samples:

```
あいこ	あいこう	long-vowel removed
あに	あんい	n-row->ん+vowel
あんど	あんどう	long-vowel removed
いいえ	いえ	long-vowel removed
いかん	いっかん	っ added
いせい	いっせい	っ added
いたい	いったい	っ added
いちょう	いっちょう	っ added
いっつも	いつも	っ removed
いとし	いとしい	long-vowel removed
いど	いどう	long-vowel removed
いやぁ	いやあ	small->full
いよ	いよう	long-vowel removed
うっとうし	うっとうしい	long-vowel removed
うらやまし	うらやましい	long-vowel removed
えいり	えり	long-vowel removed
おいし	おいしい	long-vowel removed
おう	おおう	long-vowel removed
おし	おしい	long-vowel removed
おれ	おれい	long-vowel removed
```

### C9 — de together/apart
Rows with a space: 0; with a hyphen: 0. So D15's exclusion has nothing to exclude today. 0 candidates by heuristic only (the rulebook list is not available). Today fold_strict DROPS whitespace on both sides, so 'kennen lernen' already grades equal to 'kennenlernen', and so does any split (v1.1 T14 'rejects an unlisted split' fails today).

### C10 — ja particle readings
81 candidates, mostly JMdict expressions/interjections containing は/へ; only 1 row contains を. Heuristic, not a confirmed set; Eric or an auditor should read the sample before F10 is built on it.

### C11 — de loanword inflections
None found. 138 rows are shaped like participles (ge…t); inspection of the 15 with no dictionary infinitive shows native verbs and finite forms (genehmigt, generiert, gerät), not gedownloadet-type forms. Hunspell accepts inflected forms, so PURITY's dictionary check does not catch participles of native verbs either; I did not test PURITY's dictionary-form rule beyond that.

### C12 — VOICE-CLARITY de/ja intake
None found. See the stop list.

## Not done / limits
- Struck-variant seed list unverified; `data/de/struck-variants.txt` does not exist and I did not create it.
- F3/C9/C10 depend on lists (Regelwerk, rulebook) that are not available locally.
- Impossible pools and Spell Jr/Kid lists were not scanned; only the four tier files per language.
- No tests run; T1–T10 need Phase A–C code that does not exist.
- Marginal: ja stored answers are readings only, so F5 compares against JMdict readings, which makes JMdict the sole authority for 現代仮名遣い.
