# German struck list and double spellings vs the Regelwerk 2024 overview

For your approval. `data/de/struck-variants.txt` is **not changed** by this; it already holds your seed list on main, marked unverified.

**Source read:** "Aktualisiertes Amtliches Regelwerk der deutschen Rechtschreibung 2024 – Zusammenfassung der wesentlichen Änderungen", Rat für deutsche Rechtschreibung, 10 pages, 203,609 bytes, from `rechtschreibrat.com/DOX/RfdR_Amtliches-Regelwerk_2024_UeberblickAenderungen.pdf`. Saved in the scratch directory only.

**What it is, and is not:** a summary of the 2024 changes, not the Regelwerk or the Wörterverzeichnis. It lists the struck variants (page 1, section "Laut-Buchstaben-Zuordnung / Variantenschreibungen"). It does not list the 2017 removals and it does not list the official double spellings, so those are marked "not in this document".

## A. The struck list

The document (page 1) names 23 struck forms: Buklee, Busuki, Dekolletee, Dränage, Dublee, Exposee, Frappee, Frigidär, Glacee, Hämorriden, Jogurt, Katode, Kurtage, Kusine, Lymphdränage, Panter, photogen, Polonäse, Pralinee, Sorbett, Soufflee, Spagetti, Tunfisch.

| Form | Verdict | Page |
|---|---|---|
| Buklee, Busuki, Dekolletee, Dränage, Dublee, Exposee, Frappee, Frigidär, Glacee, Hämorriden, Jogurt, Katode, Kurtage, Kusine, Lymphdränage, Panter, photogen, Polonäse, Sorbett, Spagetti, Tunfisch (21) | **keep**: named in the document | 1 |
| Pralinee | **add**: in the document, missing from your seed | 1 |
| Soufflee | **add**: in the document, missing from your seed | 1 |
| Katarr | **cannot verify**: not in this document | n/a |
| Myrre | **cannot verify**: not in this document | n/a |
| Majonäse, Ketschup, Anschovis, Komplice, Wandalismus (2017 removals) | **cannot verify**: this document covers the 2024 update only; the 2017 changes are in an earlier edition | n/a |

Proposal: keep the 21, add `Pralinee` and `Soufflee`, and leave the 7 unverifiable forms in the file as "unverified" rather than dropping them, until the Wörterverzeichnis or the 2017 edition confirms or refutes them. The census found no active German row matching any of them, so none of this changes the bank.

The document also says three foreign-language spellings were **added**: Couvert, Façon, Portrait. If any of those is in the bank spelled the integrated way, it would be affected. I have not checked that.

## B. The four double spellings (data/de/accepted-variants.tsv)

| Stored | Accepted | In this document? |
|---|---|---|
| Ghetto | Getto | not in this document |
| Xylophon | Xylofon | not in this document |
| geographische | geografische | not in this document |
| fantastischen | phantastischen | not in this document |

All four are unverified against the Regelwerk. They stay sourced to Hunspell de_DE_frami. Confirming them, or citing the Regelwerk instead, needs the Wörterverzeichnis (online at grammis, IDS Mannheim) or the full Regelwerk PDF.

The document does confirm one thing I had already assumed: **Panter and Spagetti are struck**, so `Panther` and `Spaghetti` are the only correct forms. That is why the pairs `Panther/Panter` and `Spaghetti/Spagetti` were left out of the accepted-variants file.

## What I need from you

- Approve the struck-list change in section A (add two, keep the 21, keep the 7 flagged unverified).
- Say how to confirm section B: I fetch the Wörterverzeichnis entries from grammis, or you supply the Wörterverzeichnis.

## Resolution, 2026-11-04 (full Regelwerk 2024, 348 PDF pages; printed page = PDF page - 2)

- **Struck list:** approved and written. 23 entries `verified` (changes document p. 1; none appears as a valid entry in the Wörterverzeichnis), 7 `unverified` with an open row to check the 2011/2017 editions. The file blocks admission only and no grader reads it. The Wörterverzeichnis lists only the kept form for all 7 (Katarrh, Myrrhe, Mayonnaise, Ketchup, Anchovis, Komplize, Vandalismus), which supports them but is not the 2017 text.
- **Conflict found:** the rules text (PDF p. 41) still gives Pralinee among the ee-ending examples. The Wörterverzeichnis (p. 285) lists Praline, Praliné only and the 2024 changes document strikes Pralinee. I followed the dictionary and the changes document, and noted the conflict in the file.
- **Added foreign spellings:** Kuvert 0 and Fasson 0 in the bank, nothing to add. Porträt: the bank holds only the plural `Porträts`, so `Porträts -> Portraits` is added (Wörterverzeichnis p. 284). Façon has no row, so the ç input question does not arise yet.
- **Double spellings:** all four confirmed in the Wörterverzeichnis and § 32(2), so they are live with page citations: Ghetto/Getto p. 225, Xylophon/Xylofon p. 339, Geografie/Geographie p. 224 (the adjective has no own entry; § 32(2) -graph- covers it), fantastisch/phantastisch p. 213 and p. 282. No other inflected rows of these lemmas exist in the bank.
