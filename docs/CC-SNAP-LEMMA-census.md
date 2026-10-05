# CC-SNAP-LEMMA §0 census — read-only, nothing built

Run 2026-10-04 against `main` at `8f4f8500`.

CC-SNAP-ROADMAP 2.3 says this file "does not re-run that census; it waits for
it", and the roadmap's own open list records that the census had still not
run — blocking Level 2 at its start, since D1's signed order puts Lemma early.
It has now run. The same question also blocks CC-RU-ORTHO §0 D3 and
CC-TRANSLATE-SCREEN D11.

**The question:** does the bank hold inflected forms, or lemmas only?

**The answer: inflected forms, in every language that inflects by suffix.**
Not marginally — English alone carries 110 lemmas with two or more of their
own inflections as separate bank entries.

And one consequence the roadmap did not anticipate, in §3.

---

## 1 — Method, and what it can and cannot support

For each language, count bank pairs `(X, X+suffix)` where **both** are bank
words, over a list of that language's orthographic inflectional suffixes.

This is deliberately not morphological analysis. It is a mechanical fact
about the bank, and it needs no linguist to verify: either both strings are
in the file or they are not. Nothing here was generated from a pronunciation
model and nothing here is a claim about how a language works.

Read it with two bounds in mind:

- It **under**-counts inflected forms, because it only finds a form whose
  lemma is ALSO in the bank. A form whose lemma is absent is invisible to it.
  That makes every number below a floor for "the bank holds inflected forms".
- Any individual suffix **over**-counts, because orthography coincides:
  `car` + `d` is `card`, `care` + `er` is `career`, `lay` + `er` is `layer`.
  So the per-suffix totals are an upper bound on that suffix.

The method's validity check is at the bottom of the table: **zh, ja and vi
return exactly zero.** Those are the three languages that do not inflect by
concatenating a suffix, and the method finds nothing where there is nothing
to find.

---

## 2 — Results

| lang | bank | suffix-pairs | % | examples |
|---|---|---|---|---|
| en | 3165 | 764 | 24.1% | note→notes, return→returning, university→universities |
| fa | 8340 | 1648 | 19.8% | کش→کشی, امام→امامی |
| de | 6148 | 1075 | 17.5% | unser→unsere, beobachte→beobachten |
| fr | 6109 | 912 | 14.9% | perte→pertes, décennie→décennies |
| es | 6099 | 678 | 11.1% | estrella→estrellas, profesor→profesora |
| pt | 6110 | 628 | 10.3% | criança→crianças, nome→nomes |
| ru | 6208 | 625 | 10.1% | работ→работу, сезон→сезона |
| ar | 6238 | 585 | 9.4% | طائر→طائرة |
| pl | 6176 | 472 | 7.6% | podróż→podróży, fabryk→fabryki |
| fil | 4083 | 254 | 6.2% | kawal→kawalan |
| hi | 2674 | 89 | 3.3% | अंग्रेज→अंग्रेजों |
| ko | 6233 | 80 | 1.3% | 프로그래머→프로그래머가 |
| sw | 2845 | 44 | 1.5% | shule→shuleni |
| **ja** | 6160 | **0** | 0.0% | — |
| **vi** | 4094 | **0** | 0.0% | — |
| **zh** | 6182 | **0** | 0.0% | — |

**English, by suffix**, so the coincidences can be seen rather than argued
about:

| suffix | pairs | reliability |
|---|---|---|
| -s | 388 | high |
| -ed | 91 | high |
| -d | 73 | mixed — includes car→card |
| -ly | 72 | derivational, not inflectional |
| -ing | 71 | high |
| y→ies | 35 | high |
| -er | 19 | mixed — includes care→career, lay→layer |
| -es | 11 | high |
| -est | 4 | high |

Restricting to the three suffixes that almost never coincide (`-ing`, `-ed`,
`y→ies`) still leaves **197 English entries, 6.2% of the bank**, that are an
inflected form whose own lemma is a separate bank entry. That is the floor,
and it already settles the question.

**110 English lemmas carry two or more of their own inflections**, for
example `play` → played, player, playing, plays — five bank entries for one
lemma. Also `form`, `remain`, `offer`, `allow`, `record`, `concern`, `call`.

---

## 3 — The finding that changes how 2.3 reads

2.3 is written around one shape: "`running` in a book maps to bank `run`" —
a scanned form that cannot anchor, rescued by swapping it for the lemma that
can.

The bank says that is often not the situation. `running` is itself a bank
word, and so are `returning`, `playing` and 68 others. For those, **both the
scanned form and its lemma anchor perfectly well**, and a lemma swap does not
rescue an unanchorable word — it replaces one valid bank word with a
different valid bank word, changing which word the child practises.

That is a different feature from the one 2.3 describes, and it wants a
different default. D4 (lemma swap is suggest-only everywhere, including
English) already prevents the harm, so nothing is broken — but D4's stated
reason is caution about inflection tables, and this is a second and stronger
reason: **when both forms are in the bank, the swap is a choice about
practice content, not a correction.**

Worth 2.3 distinguishing the two cases explicitly when it is written:

- scanned form **not** in bank, lemma **is** → the swap anchors a word that
  otherwise could not be practised. This is the feature.
- scanned form **is** in bank → nothing to fix. Offering a swap here
  silently edits the teacher's list. Suggest only, and arguably not at all.

The census cannot tell you how often each case occurs in the wild (§5).

---

## 4 — 2.4's stop-and-ask, answered: no ask needed

2.4 says mixed-script pages should produce "separate My Words lists (one per
language)", and instructs: confirm with CC-MYWORDS-LISTS, and if that file's
list model forbids auto-creating two lists from one import, stop and ask.

The model forbids nothing, and the question dissolves. In
`src/word_lists.rs`, **`ListEntry` carries its own `lang`, and `WordList` has
no language field at all**:

```rust
pub struct ListEntry { pub text: String, pub lang: String, ... }
pub struct WordList  { pub id: String, pub name: String, pub entries: Vec<ListEntry>, ... }
```

A single list already holds words in different languages, each speaking in
its own. The migration test asserts exactly this: three words round-trip as
`en-US, es-ES, en-US` in one list.

So one list per import is not merely permitted, it is the model's natural
shape, and splitting into two lists is a product choice rather than a
constraint. Recommending 2.4 be rewritten as the choice it is. Not acted on:
this census builds nothing.

---

## 5 — The census item I cannot produce

The number 2.3 would most like is: **of the word forms a reader actually
photographs, what fraction are absent from the bank while their lemma is
present?** That is the size of the feature.

It cannot be measured from the bank. It needs a corpus of real scanned pages
with their forms, and no such fixture exists — the nearest thing, the
20-photo highlight fixture in Level 1, has not been assembled yet either.

I am not going to estimate it. Any number I produced would come from guessing
what a page looks like, and the roadmap's whole doctrine is against that.

---

## 6 — What this unblocks, and what it does not

**Unblocked:** 2.3 may now be written. The bank is not lemma-normalized, so
the inflection tables 2.3 calls for are still needed — and per 2.3's own
sourcing rule they come from the dictionary authority list, never generated.
Nothing here supplies one.

**Also answered:** CC-RU-ORTHO §0 D3 and CC-TRANSLATE-SCREEN D11 both asked
this same question. Russian holds case forms — 625 pairs, 10.1%.

**Still blocked:** the per-language suffix lists used here are mine, not an
auditor's. They are adequate to prove presence, which is all this census
claims. They are NOT an inflection table and must not be used as one for
ko, ar, hi, fa, fil or sw, where the counts above are indicative only.
