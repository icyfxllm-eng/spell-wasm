# Swahili generator — rule review

> GENERATED from `src/swahili_gen.rs` `RULES` (generator `0.1.0-draft`). Do not hand-edit — change the rules and regenerate with
> `cargo test --features swahili_gen -- --ignored regenerate_review_document`. `review_document_matches_the_rules` fails if this drifts.

**This is a PROPOSAL, not verified Swahili.** An engineer who does not speak Swahili wrote these rules; CC-SWAHILI-WORDBANK D3 makes them *audited content*, which means a named native speaker must red-pen each one. A rule becomes `Verified` only by that reviewed act — never by default — and until every rule a form depends on is verified, that form cannot ship (D3).

For each rule below: is the description right, and is every "generator emits" output a real, correct Swahili form? Mark ✗ and give the correction where not.

## Status

7 rules — **0 verified, 7 awaiting native audit.**

---

## SW-V-01 — ⚠ needs native audit

Monosyllabic-root ku- retention. A one-syllable verb root keeps the infinitive ku- in the na-/li-/ta-/me- tenses, because Swahili stress falls on the penult and the bare root leaves nothing to carry it. Without ku-, *anala is not a word.

| input (slots + gloss) | generator emits | correct? | correction |
|---|---|---|---|
| a + na + -la 'eat' | `anakula` | ☐ | |
| tu + li + -ja 'come' | `tulikuja` | ☐ | |
| wa + ta + -nywa 'drink' | `watakunywa` | ☐ | |
| ni + me + -pa 'give' | `nimekupa` | ☐ | |

## SW-V-02 — ⚠ needs native audit

Object marker m- -> mw- before a vowel-initial root. The class-1 object marker is a bare nasal and cannot stand before a vowel.

| input (slots + gloss) | generator emits | correct? | correction |
|---|---|---|---|
| a + na + m + -ona 'see' | `anamwona` | ☐ | |
| tu + li + m + -uliza 'ask' | `tulimwuliza` | ☐ | |
| ni + ta + m + -ambia 'tell' | `nitamwambia` | ☐ | |

## SW-V-03 — ⚠ needs native audit

Vowel-initial roots take the TAM marker unchanged; no elision at the TAM/root juncture. Recorded as an explicit NON-rule so an auditor can reject it if Swahili actually does contract here — an absent rule is invisible to review, which is how wrong output gets shipped.

| input (slots + gloss) | generator emits | correct? | correction |
|---|---|---|---|
| a + na + -ona 'see' | `anaona` | ☐ | |
| wa + li + -andika 'write' | `waliandika` | ☐ | |
| tu + ta + -elewa 'understand' | `tutaelewa` | ☐ | |

## SW-N-01 — ⚠ needs native audit

Noun class 1/2 (m-/wa-), people. Both members generate ONLY when the lemma's class is source-attested (D4) — guessing a noun's class invents words.

| input (slots + gloss) | generator emits | correct? | correction |
|---|---|---|---|
| mtoto 'child' | `watoto` | ☐ | |
| mwalimu 'teacher' | `walimu` | ☐ | |
| mgeni 'guest' | `wageni` | ☐ | |

## SW-N-02 — ⚠ needs native audit

Noun class 7/8 (ki-/vi-), things. Same attestation condition as SW-N-01.

| input (slots + gloss) | generator emits | correct? | correction |
|---|---|---|---|
| kitabu 'book' | `vitabu` | ☐ | |
| kiti 'chair' | `viti` | ☐ | |
| kisu 'knife' | `visu` | ☐ | |

## SW-N-03 — ⚠ needs native audit

Noun class 5/6 (ji-/ma-). The ji- is often zero on the singular.

| input (slots + gloss) | generator emits | correct? | correction |
|---|---|---|---|
| jicho 'eye' | `macho` | ☐ | |
| jina 'name' | `majina` | ☐ | |
| yai 'egg' | `mayai` | ☐ | |

## SW-N-04 — ⚠ needs native audit

Noun class 11/10 (u-/n-). The plural often surfaces with no prefix at all.

| input (slots + gloss) | generator emits | correct? | correction |
|---|---|---|---|
| ukuta 'wall' | `kuta` | ☐ | |
| ufunguo 'key' | `funguo` | ☐ | |
| uso 'face' | `nyuso` | ☐ | |

