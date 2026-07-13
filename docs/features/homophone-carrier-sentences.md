# PARKED FEATURE: require-sentence homophone disambiguation

**Status:** parked — post-relaunch. No code this iteration (per decision
addendum, 2026-07). This file exists only to record the long-term fix.

## Problem
Some Spanish word pairs are true homophones — the audio cannot carry the
spelling difference (`b/v`, silent `h`, seseo `s/z/c`, yeísmo `y/ll`). The
current handling (decision addendum, bucket 2) is **accept-any**: for such a
pair, typing either member scores correct (see `assets/words/es/homophones.txt`,
consumed by `src/homophones.rs`). That is safe but imprecise — it never tests
which spelling the player actually meant.

## The eventual fix (require-sentence)
For an accept-any pair, disambiguate with a **carrier sentence** that forces one
member, plus a per-pair TTS clip of that sentence:

- Prompt audio becomes the word inside a disambiguating sentence
  (e.g. *"La **casa** es grande"* vs *"La **caza** del zorro"*), so the intended
  member is unambiguous and only that spelling is accepted.
- Requires: (1) a carrier-sentence per member, (2) TTS generation for those
  sentences, (3) grading switches that pair from accept-any to exact-member.

## Why not now
- Needs new TTS content + backend generation per pair (out of scope; the
  addendum forbids TTS backend changes this iteration).
- Needs native-authored carrier sentences (review-gated content).
- accept-any is a correct, shippable interim behavior.

## When picked up
Promote confirmed accept-any pairs from `homophones.txt` into a
sentence-carrier table; keep `homophones.txt` as the fallback for pairs without
a carrier sentence yet.
