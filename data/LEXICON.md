# Lexicon layer

Structured, per-language dictionaries that supersede the flat word lists as the
build-time input to difficulty scoring, pool assignment, and curation. One file
per language: `data/<lang>/lexicon.jsonl` (one JSON object per line).

The lexicon is a **build-time artifact** — it lives in `data/`, is NOT shipped
in the WASM bundle, and is NOT read at runtime. Only the derived pool/assignment
data ships. Ingestion runs offline (no network in CI); committed lexicons are
the build inputs.

## Entry schema

```json
{
  "word": "びょういん",        // ANSWER string, normalized (kana ja, pinyin zh, hangul ko, plain Latin)
  "display": null,             // shown after answering when != word (zh: hanzi); else null
  "lang": "ja",
  "freq_rank": 1243,           // 1 = most common; null if unknown
  "pos": ["noun"],             // shared tagset (below)
  "domains": ["health"],       // semantic/theme tags (Kid Mode themes, Daily); may be empty
  "pron": "byooin",            // pronunciation/reading key for homophone collision (P1)
  "grade": "JLPT-N5",          // pedagogical grade (HSK-n / JLPT-Nn / TOPIK-n); null if none
  "gloss": "hospital",         // short English gloss (internal; not shown by default)
  "kid_ok": null,              // tri-state: true reviewed-in / false reviewed-out / null unreviewed
  "sources": ["jmdict"],       // provenance
  "flags": []                  // homophone | sandhi | loanword | function_word
}
```

`word` obeys the language's answer charset + normalization, validated at ingest
(not runtime). Every entry passes `content_filter` at ingest.

## Shared POS tagset

`noun verb adj adv num pron det prep conj part interj`

- **Content POS** (`noun verb adj adv`) = good spelling words, preferred for pools.
- **Function POS** (`pron det prep conj part interj`) — an entry with ONLY
  function POS gets the `function_word` flag and is excluded from pools by
  default (kept in the lexicon).

## Pipeline

`tools/lexicon-ingest/` (offline Python):

1. Parse a source → raw entries (per-source parser).
2. Normalize `word` via the language engine (`normalize.py`, mirrors src/norm,
   viet, pinyin, hangul).
3. Validate charset (drop + log).
4. content_filter pass (drop + log; review gates as established).
5. Merge across sources by normalized `word` (union domains/sources/pos; keep
   best freq_rank; prefer first non-null scalar).
6. POS-normalize; flag `function_word`.
7. Emit sorted, deterministic `lexicon.jsonl` + `ingest-report/<lang>.md`.

## Downstream wiring (once lexicons carry metadata)

- **Difficulty scorer:** `freq_rank` replaces ad-hoc frequency; `grade` becomes
  a calibration check (rank-correlation CI test).
- **Assignment:** excludes `function_word` and `kid_ok:false`; Kid Mode requires
  `kid_ok:true`; P1 homophone hold-out from `pron` collision groups.
- **Themes:** `words(lang, domain, grade_band, pool)` query — themes are configs
  listing domain tags, not hand-lists.

## Migration status

`plainlist` migrates today's shipped `assets/words/<lang>/*` into the schema
(source `curated`, tier recorded as a `tier:*` domain). This gives every current
word a lexicon row with **0% metadata coverage** — the "before" state. The
dictionary parsers (jmdict, cedict, kaikki, wordfreq, cmudict) add
freq/pos/pron/grade/gloss once their dumps are supplied under
`tools/lexicon-ingest/sources/` and their licenses recorded in each
`data/<lang>/SOURCES.md` (STOP GATE).

`zh` has no `assets/words/zh/` files (its starter bank lives in `src/words.rs`);
it populates from CC-CEDICT directly once that dump is supplied.
