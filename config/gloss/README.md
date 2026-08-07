# The gloss pivot (CC-BANK-TRANSLATE bones)

One file per language: `<lang>.json`. Each maps a BANK WORD to its
sense-locked English concept — the pivot every translator tool turns on.

```json
{
  "lang": "es",
  "audited": false,
  "auditor": "",
  "rows": { "agua": "water", "casa": "house" }
}
```

Three laws the loader and CI enforce, so the closed space cannot leak:

1. **Every key is a live bank word** in that language. A gloss for a
   word the app cannot serve is dead weight and a lie about coverage.
2. **Every concept is a live ENGLISH bank word.** The pivot is English
   by construction, so a concept the English bank cannot spell would
   make an unspellable card.
3. **`audited` is a human claim, never a tool's.** A language renders
   in the translator ONLY when a named native speaker has signed its
   file. Until then the rows sit here, validated but dark — which is
   why `gloss_rows` returns nothing and the fan-out shows one row.

Adding a language: write its rows, run `node scripts/gloss-check.mjs`,
get a native speaker to review, then set `audited: true` and record who.

## What is here today (2026-08-06)

2,662 rows across all fourteen banked languages, **none audited**, so
the translator is still dark by design. The rows were authored from a
300-concept core list (body, family, animals, numbers, colour, weather,
house, food, motion, a thin abstract tail) and then filtered by the
three laws above — which is why no language reaches 300:

```
es 258  fr 258  fil 248  pt 240  vi 218  sw 210  ja 180
zh 170  de 169  pl 167   ar 163  ru 157  hi 119  ko 105
```

Two notes for whoever audits next, because the shortfalls are not all
the same kind of problem:

* **`de` is limited by its BANK, not by this table.** The German bank
  has no `mann`, `frau`, `mutter`, `vater`, `kopf` or `bein` in any
  casing, while it does have `kind`, `haus` and `hund`. It is also 97%
  lowercase, so German nouns are stored against German orthography
  (`grossmutter`, not `Großmutter`). Fixing German glosses means fixing
  the German bank first — see the ledger.
* **`ko` at 38% is the outlier to check.** The likely cause is verb
  form: rows were authored in dictionary `-다` form and the bank may
  store stems. That is a re-authoring pass, not a bank hole.

`ja` keys are hiragana because the Japanese bank holds no kanji. `zh`
keys are the pinyin half of the bank's `pinyin|hanzi` pairs
(`fang2zi5` → house), which is the form `pool()` yields in
`scripts/gloss-check.mjs`.
