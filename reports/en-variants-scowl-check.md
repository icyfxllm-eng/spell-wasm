# CC-EN-VARIANTS: SCOWL confirmation check on the 135 new candidates

Read-only measurement. V2 (Eric): SCOWL confirms, it never admits; an unconfirmed candidate is held for review, not dropped.

**Source:** SCOWL rel-2026.02.25 (github.com/en-wl/wordlist), Hunspell builds en_GB-ise, en_AU, en_CA and en_US-large (standard size, not the GB/AU/CA large builds). Downloaded to a scratch directory; not committed.

**Licence (read from README_en_GB-ise.txt):** permissive. Copyright 2000-2026 Kevin Atkinson. Use, copy, modify, distribute and sell are granted without fee, provided the copyright notice and permission notice appear in all copies and in supporting documentation. The affix file is Geoff Kuenning's BSD-style licence (modified versions must be marked). The Australian data (Benjamin Titze, 2016) has the same style of notice. The README also says part of the data comes from COCA 3-gram data, used within an NDA the author signed. We would ship none of SCOWL's text, so the duty is a credit line.

**Test:** a candidate is *confirmed* when the variant is a valid word in at least one of GB-ise / AU / CA and the stored US form is valid in en_US-large.

## Result

- Candidates: 135 (the clean candidates from the census that are not already in the shipped file).
- **Confirmed: 110.** Variant valid in GB+AU for 86, GB+AU+CA for 21, GB only for 2, AU only for 1.
- **Not confirmed: 25.**
- Of the 110 confirmed, 101 are not valid US spellings and 9 are valid in en_US too (see "Confirmed, but check").
- By bank group, confirmed / unconfirmed: staged 82 / 16, retier 23 / 4, live 5 / 5.

## Unconfirmed list (held for review)

| stored (US) | variant | bank group |
|---|---|---|
| adapter | adaptor | staged |
| amoxicillin | amoxycillin | retier-expert |
| annex | annexe | staged |
| bagel | beigel | staged |
| bylaw | byelaw | staged |
| carburetor | carburetter | staged |
| country | counterie | live-hard |
| cipher | cypher | staged |
| defensible | defencible | staged |
| discoloration | discolouration | retier-hard |
| doughnut | donut | staged |
| encyclopedic | encyclopaedic | retier-hard |
| fjord | fiord | staged |
| focused | focussed | live-hard |
| fete | fête | staged |
| gram | gramme | staged |
| humorous | humourous | live-hard |
| kilogram | kilogramme | staged |
| neuron | neurone | staged |
| pedagogical | paedagogical | staged |
| sole | soal | live-easy |
| strength | strenth | live-hard |
| synthesizer | synthesiser | retier-hard |
| warp | wharp | staged |
| yogurt | yogourt | staged |

Two kinds are mixed here.

- **Likely real variants that the standard-size SCOWL build omits:** `encyclopaedic`, `discolouration`, `synthesiser`, `focussed`, `gramme`, `kilogramme`, `neurone`, `fiord`, `cypher`, `donut`, `annexe`, `adaptor`, `yogourt`, `fête`.
- **Junk that SCOWL correctly rejects:** `counterie`, `soal`, `strenth`, `humourous`, `wharp`, `paedagogical`, `beigel`, `byelaw`, `defencible`, `carburetter`, `amoxycillin`.

Re-running against the GB/AU/CA large builds would likely sort most of the first group. I did not fetch those.

## Confirmed, but check before admitting

These nine are also valid in en_US, so the "variant" may be a different word rather than a spelling of the same one: `bogie` for `bogey`, `castor` for `caster`, `drier` for `dryer`, `likeable` for `likable`, `practised` for `practiced`, `sizeable` for `sizable`, `verandah` for `veranda`, `vice` for `vise`, `whizz` for `whiz`.

`vice`/`vise`, `castor`/`caster` and `drier`/`dryer` are separate words with their own meanings, so a mechanical pass would accept a different word. They need a human decision. I would hold all three.

## Recheck against the large builds (GB, AU, CA large; SCOWL rel-2026.02.25)

Fetched `hunspell-en_GB-large`, `hunspell-en_AU-large`, `hunspell-en_CA-large` (about 300 KB each) to the scratch directory. Run on the 14 held words and the 11 junk words.

| Result | Words |
|---|---|
| **Now confirmed, non-US variety only** | `encyclopaedic`, `discolouration`, `synthesiser`, `focussed`, `cypher`, `adaptor` (GB, AU, CA); `gramme`, `kilogramme` (GB, AU); `yogourt` (CA only); `carburetter` (moved up from the junk list: GB and CA large accept it) |
| **Valid in US too, so not a regional variant** | `neurone`, `annexe`, `fête` |
| **US-only** | `donut` (a US variant of `doughnut`, not a British one) |
| **No SCOWL variety has it** | `fiord` |
| **Still rejected (junk)** | `counterie`, `soal`, `strenth`, `humourous`, `wharp`, `paedagogical`, `beigel`, `byelaw`, `defencible`, `amoxycillin` |

Net effect on the 135: confirmed 110 + 10 = **120**. Unconfirmed **15**: the 10 junk words stay out, and `neurone`, `annexe`, `fête`, `donut` and `fiord` go to review. `fête` is the accent form (F4), not a spelling variant. The three different-word pairs (`vice`/`vise`, `castor`/`caster`, `drier`/`dryer`) are still held out of the 120 and are not in either count above.
