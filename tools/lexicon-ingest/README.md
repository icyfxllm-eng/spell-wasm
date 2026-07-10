# lexicon-ingest

Offline build-time tool that turns dictionary sources into the structured
`data/<lang>/lexicon.jsonl` files (schema: `data/LEXICON.md`). Pure Python 3
(stdlib only), deterministic, no network — it reads local dumps you place under
`sources/`.

## Run

```bash
cd tools/lexicon-ingest

# Migrate the shipped flat lists into the lexicon schema (works today):
python3 ingest.py --all --plainlist

# Add real metadata once you've downloaded the dumps into sources/:
python3 ingest.py ja --plainlist --jmdict sources/jmdict-eng.json --wordfreq sources/ja-freq.tsv
python3 ingest.py zh --cedict sources/cedict.txt --wordfreq sources/zh-freq.tsv
python3 ingest.py en --plainlist --cmudict sources/cmudict.dict --kaikki sources/kaikki-en.jsonl

python3 test_ingest.py     # scaffold tests (no pytest needed)
```

Each run writes `data/<lang>/lexicon.jsonl` + `ingest-report/<lang>.md`.

## Sources (download once into `sources/`, gitignored)

A parser raises `SourceMissing` with the exact URL if its dump is absent — the
tool never fabricates data. Per-language provenance + license go in
`data/<lang>/SOURCES.md` **before** ingesting (STOP GATE).

| Parser | Source | License |
|--------|--------|---------|
| `jmdict` | jmdict-simplified (JSON) | EDRDG / CC BY-SA |
| `cedict` | CC-CEDICT | CC BY-SA 4.0 |
| `cmudict` | CMUdict | BSD-style |
| `kaikki` | kaikki.org Wiktionary extract | CC BY-SA |
| `wordfreq` | wordfreq data / Leipzig | verify per source |
| `plainlist` | this repo's `assets/words` | owned |

## Design notes

- **Deterministic:** entries sorted by `word`, compact JSON with sorted keys →
  byte-identical output for identical inputs (tested).
- **Normalization** (`normalize.py`) mirrors the Rust runtime engines so the
  stored `word` is exactly what the app compares against; charset checks read
  the same keyboard-layout SSOT (`assets/keyboards/*.json`) as the word-list
  pipeline and its Rust drift test.
- **Not in CI / not in the bundle:** lexicons are committed build inputs; the
  WASM ships only derived pools. Bundle-size delta from this tool ≈ 0.
- Adding a source = one file in `parsers/` exposing `parse(path, lang)`.
