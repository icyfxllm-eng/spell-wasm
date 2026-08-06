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
