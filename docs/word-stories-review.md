# Word Stories — current-behavior writeup (CC-MODE-HUB Feature 7 review artifact)

**Status: review artifact. No bones added pending Eric's decision.**

## What it does (build 55)
Feature F5 "Word Stories" (`src/word_stories.rs`). After a word is answered, it surfaces a one-line origin story ("From Latin *iudicare*, 'to judge'") to turn drill into curiosity. Renders in Kid Mode too (marked educational).

## Content source — machine-derived, NOT hand-authored per language
- An **offline pipeline** (`tools/lexicon-ingest`, a kaikki.org / Wiktionary parse) extracts a first-hop etymology per list word into shipped stores `src/i18n/etymology/{lang}.json` — a plain `word → story` map.
- At runtime `story(lang, word)` looks the word up, **re-compresses to a single first-hop sentence ≤120 chars** (no etymology chains), and screens it through the **shared profanity filter** like any other displayed content.
- **Coverage today: en and es only.** Every other language resolves to `None`.

## Current gating
Behind `flags::word_stories`, **default OFF** (Decision D3): Wiktionary text is **CC BY-SA**, so it stays dark until the attribution approach is approved. Flag off ⇒ `story()` returns `None`, nothing renders, zero behavioral difference.

## Audit-surface verdict (against the CC-MODE-HUB Feature-7 filter)
- **PASSES** the "no authored narrative text per language" test — content is machine-extracted from an external open-licensed source, not written by us, hard-capped at 120 chars, profanity-screened.
- **BUT** the displayed strings are **un-audited external (Wiktionary) text** — never native-reviewed for accuracy or kid-appropriateness. That is the real, narrower concern.

## Two blockers before it can leave the hidden state
1. **CC BY-SA attribution** — now solvable with the infra already built in CC-WORDLIST-SOURCES: register kaikki/Wiktionary as a Tier-B source and surface it in the generated credits screen. This retires the D3 gate cleanly.
2. **Kid-appropriateness spot-check** — a sample review of the en/es etymology strings (a batch of a few hundred; Eric or a native reviewer) to confirm nothing off surfaces to children.

## Recommendation
**Keep it — do NOT pull it.** It is machine-derived, not authored, so it does not fail the audit-surface filter. Ship it **hub-hidden / flag-OFF** until (1) attribution is wired via the credits system and (2) the en/es strings pass a spot-check. No bones (UI structure) needed beyond that; it's an after-answer flourish, not a session mode.
