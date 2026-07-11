# CJK Expert → Drawing routing

## Why (the input-mode report)
The app collects answers via each language's *phonetic* input. For two languages
that silently deletes the entire Expert difficulty:

| Lang | Typed | Character difficulty tested? |
|---|---|---|
| 中文 | **pinyin** (`game.rs` splits `pinyin\|hanzi`, checks pinyin via `pinyin::matches`) | ❌ none — pinyin tests romanization, not the character |
| 日本語 | **hiragana** (word banks are all kana) | ❌ kanji recall (難読漢字 / 四字熟語) untestable; only kana ambiguity survives |
| 한국어 | Hangul via jamo composition | ✅ real spelling — sound-change traps survive |
| ไทย | Thai script | ✅ real spelling — การันต์ etc. survive |

So for 中文/日本語, Expert must collect a **drawn character**, scored by the stroke
recognizer, or it tests nothing. Korean/Thai need no change.

## Decision
**Route 中文/日本語 Expert through drawing mode.** Lower tiers (easy/medium/hard)
stay phonetic typing — they're common words where character recall matters less,
and forcing drawing everywhere is too heavy for young learners.

## How it's wired (today)
`consts.rs`:
- `is_char_expert(lang, tier)` — the *intent*: true for zh/ja Expert.
- `expert_requires_drawing(lang, tier)` — the *runtime decision*:
  `is_char_expert(...) && draw_available(lang)`.

`draw_available` is gated behind `DRAW_MLKIT_READY` (currently `false`), so
`expert_requires_drawing` is **always false today** and Expert falls back to the
current typed input. This is the honest interim: pinyin/kana can't test the
character, but a half-working draw pad (e.g. Tesseract on handwritten hanzi)
would be *worse*. No visible change ships until the recognizer is real.

## Activation checklist (flip `DRAW_MLKIT_READY` only when ALL true)
1. **Native `DigitalInkPlugin`** (ML Kit Digital Ink) built for iOS/Android,
   returning a ranked `Candidate` list.
2. **`draw_judge`** (already built + tested) wired to score the drawn strokes
   against the target character using the constrained-verification path.
3. **`game.rs` input flow** consults `expert_requires_drawing` per word: when
   true, present the draw pad instead of the keyboard and route the result
   through `draw_judge` rather than `pinyin::matches` / kana compare.
4. **Character Expert pools** (成语 / 四字熟語 / 難読漢字) sourced from graded/
   frequency lists — NOT model-generated — and **native-reviewed** before ship.
   Kid Mode never draws from these classes.
5. Cultural practice grids (stroke-order guides) for zh/ja per the drawing spec.

Until 1–5 land, zh/ja Expert remains a phonetic test and is documented as such
(honest-pool rule). This file is the reference for that integration.
