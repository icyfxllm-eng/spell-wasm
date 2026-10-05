# DOC-1 bank anchor — census, read-only, nothing built

Run 2026-10-05 against `main` at `5c32e831`.

CC-SNAP-ROADMAP's open list says "DOC-1's CI gate has nothing to scan for.
There is no `bank_id` field anywhere in the tree … The gate needs a schema
addition that no file owns." That is correct, and it understates the problem.

**DOC-1 is not waiting to be implemented. It is already being broken, on a
path that ships today, with both its flags defaulting on.**

DOC-1 says: *a word enters a game mode only when it matches an audited bank
row.* A My Words list already offers Spell Search and Spell Cross, and the
code behind those buttons never asks the bank anything.

This census reports what exists, why the obvious anchor will not work, and
what the missing schema has to be able to say. It builds nothing and decides
nothing.

---

## 1 — The live violation

`src/lists_screen.rs` renders two mode buttons on every My Words list, each
behind a flag that `src/flags.rs` defaults to **true**:

```rust
cross  = if crate::flags::spell_cross()  { "<button … data-l-cross=…>" }
puzzle = if crate::flags::spell_search() { "<button … data-l-puzzle=…>" }
```

`wordsearch_ui::open_list` is what the second one calls:

```rust
let words: Vec<String> =
    list.entries.iter().filter(|e| e.lang == lang).map(|e| e.text.clone()).collect();
… serve::from_list(&lang, tier, list_id, &words, ledger().counter)
```

`e.text` is whatever the player typed or photographed. Nothing between the
list and the puzzle consults the bank. A word the bank has never heard of
becomes a Spell Search answer today.

So the CI gate DOC-1 asks for would fail on `main` the day it was written,
and it would fail on a path that predates CC-SNAP entirely. **Whether that
is a bug or an accepted exception is Eric's call, and it should be made
before 3.1 fan-out is specified**, because 3.1's whole eligibility rule
("only bank-anchored words count toward eligibility") is written as if the
rule already holds.

## 2 — There is no `bank_id`, and the obvious substitute is unsound

`bank_id` appears **zero times in code**. It exists only in roadmap prose.

The only bank-membership primitive is `src/word_index.rs`, whose
`canonical(lang, word) -> Option<String>` returns the bank's own spelling.
The tempting move is to call that string the anchor. It does not hold.

`word_index` is a VALIDITY index — "is this a real word", and "give me the
i-th word deterministically" for Letter Forge and Impostor. It was never an
identity map, and it says so in its own code: the builder **deliberately
discards rows**.

```rust
words.dedup_by(|a, b| a.0 == b.0);
// "two bank entries that fold together are one word to every caller here"
```

Correct for its purpose. Fatal for identity. Measured over all 86,864
distinct bank entries, using the real fold (`fold_strict` = NFC + lowercase
− whitespace − silent marks) and the real key (`comparable` splits zh on `|`
and keeps the left half):

| lang | colliding keys | rows unreachable via `canonical()` |
|---|---|---|
| zh | 254 | **372 of 6,182 — 6.0%** |
| fa | 69 | 70 of 8,340 — 0.8% |
| every other language | 0 | 0 |

**zh is the structural case, not an edge case.** The index key is the pinyin
reading, and `bank_form` returns the reading too — the hanzi half is thrown
away before the pair is ever stored:

```rust
fn bank_form(entry: &str) -> String { entry.split('|').next().unwrap_or(entry).to_string() }
```

So `canonical("zh", "ba1")` answers `"ba1"`, and `ba1|八` and `ba1|扒` are
indistinguishable to every caller. Six per cent of the Chinese bank cannot
be named by this function at all.

**fa is a narrower case with the same shape.** ZWNJ (U+200C) is a silent
mark, so `می‌شود` and `میشود` fold together: two bank rows, one key.

## 3 — My Words rows carry no anchor, and nowhere to put one

```rust
pub struct ListEntry { pub text: String, pub lang: String, pub added_at: f64, pub starred: bool }
```

No anchor field, and no second structure holding one. An imported word keeps
only what the camera or the keyboard produced. Any eligibility rule phrased
over "rows that have a bank_id" is today a rule over a field that does not
exist, on a struct with no room reserved for it.

## 4 — The web half, which D12 frames as an OCR question and is not

`src/lib.rs`:

```rust
#[cfg(not(feature = "web"))]
mod word_index;
```

The index is **app-only**, which is why `snap_clean::bank_lookup` is
`#[cfg(feature = "web")] -> None`. On spellgame.net there is no bank-anchor
primitive at all — not a worse one, none.

D12 asks whether Levels 1–3 ship on the web "only if the census shows
acceptable OCR quality". That is the wrong gate. Even with perfect OCR, no
imported word can anchor on the web, so Levels 1–3 there are My Words only
and every mode that DOC-1 gates is unreachable. **That is decidable now and
does not need an OCR census.**

## 5 — What the missing schema has to be able to say

Not a prescription — the file that owns this does not exist yet — but the
census can bound it.

A bank anchor must distinguish `ba1|八` from `ba1|扒`. The folded reading
cannot, and the displayed form cannot, because for zh they are the same
string. What does distinguish them is the **bank entry as written**, and
that is unique: across all 20 bank directories, no two distinct rows share
a full entry string.

So `(lang, full bank entry)` is a sound identity with no new numbering
scheme, no migration of the bank files, and no auditor involvement. Its
costs are real and should be weighed by whoever owns the file: it is long,
it is the spelling itself (so a bank edit changes the id of the row it
edits), and it would need storing on `ListEntry` where nothing is stored
today.

What a CI gate could then scan for, in DOC-1's own terms: every call that
takes list entries into a mode — today `wordsearch_ui::open_list`,
`wordcross_ui::open_list`, and whatever 3.1 adds — reads an anchored field
rather than `ListEntry::text`. That is a grep-able shape, which is what
DOC-1 wanted and could not have while the field was absent.

## 6 — What this census does not answer

- **Whether the live violation is a bug.** Spell Search on an unanchored My
  Words list may be deliberate: the mode needs words, not audited words, and
  no audio or definition claim is made. DOC-1 as written forbids it. One of
  the two should change and I am not going to guess which.
- **Whether zh should be anchored on the hanzi instead of the reading.**
  That is a language decision with an auditor's name on it, not a schema one.
- **What it costs to make `word_index` available to the web build.** Not
  measured; `word_index` pulls in the full tier data, and whether that is
  acceptable in a WASM bundle served over the wire is a size question nobody
  has asked yet.

Reproduce the measurements with `python3 tools/doc1_census.py`.
