# Provenance — Spanish (es)

| Field | Value |
|-------|-------|
| **Source** | Recursos Lingüísticos Abiertos del Español (rla-es) |
| **Source URL** | https://github.com/sbosio/rla-es |
| **License** | GNU GPL v3-or-later **/** GNU LGPL v3-or-later **/** MPL 1.1 (disjunctive tri-license) |
| **License tier** | **B** — attribution + share-alike (see `sources/registry.json`) |
| **Version / tag** | v2.9 |
| **Commit** | `c67eae826908d05a8dfabf3f7a012ce280678208` |
| **Retrieved** | 2026-07-15 |
| **Pinned artifact** | `codeload.github.com/sbosio/rla-es/tar.gz/refs/tags/v2.9` |
| **Tarball sha256** | `3930b1e5d9fdf8ddc19247798a77ae2b9efcfe6848555df80bd13f8c9597211e` |
| **Consumed files** | `source-code/hispalabras-0.1/hispalabras/es_ANY.dic` + `es_ANY.aff` |

> The retrieval date is recorded here (and only here) because the pipeline
> cannot read the current date at build time and must stay byte-deterministic.
> No emitted file contains a timestamp.

## What we ingest

We consume the **packaged Hunspell dictionary** shipped inside the pinned rla-es
release: `es_ANY.{dic,aff}` (the pan-Hispanic "any variant" dictionary). It is
expanded with `unmunch` (Hunspell 1.7.x) and then run through the es
game-eligibility filter (see `FILTER-RULES.md`).

`es_ANY` is chosen over the region-specific `es_ES` because it is the most
inclusive Spanish variant.

## Determinism / re-pinning

The fetch (`fetch.sh`) refuses to proceed unless the downloaded tarball's sha256
matches the pin above. GitHub's tag tarballs have been byte-stable in practice
(verified: two independent downloads produced the identical sha256). If upstream
ever regenerates the archive differently, the checksum fails **loudly** and a
human must review + re-pin here and in `registry.json` — there is no silent
fallback to a stale list.

## License obligations (for Eric — not a legal conclusion)

rla-es is copyleft (Tier B). Attribution is emitted automatically into
`credits.json` (rendered on the app's About screen). **Shipping copyleft-derived
word data inside a closed App Store / MAS binary is a posture decision that only
Eric makes** — this pipeline enforces that the attribution exists, not that the
posture is acceptable. See `assets/words/LICENSES.md`, which already flags that a
language whose Hunspell license is incompatible with a closed binary may need an
alternative open lexicon substituted.

## Full upstream license texts

- `sources/es/LICENSE` — upstream `LICENSE.md` (tri-license statement, verbatim)
- `sources/es/license-texts/GPLv3.txt`, `LGPLv3.txt`, `MPL-1.1.txt` — verbatim
