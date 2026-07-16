# Korean Script Path - sequence summary (DRAFT for native review)

**Scope: the writing system only.** Teaches a true beginner to READ and SPELL Hangul.
No grammar, no word meanings, no comprehension. Words are drawn unchanged from the
already-audited lists in `assets/words/ko/`.

Strictly linear. Full JSON: `config/courses/ko.json`.

## Element order and rationale

Hangul is a featural alphabet whose letters (jamo) pack into square **syllable
blocks**. The path teaches the jamo first, then how they compose, then the harder jamo.

1. **Basic consonants (jamo)**: ㄱ ㄴ ㄷ ㄹ ㅁ ㅂ ㅅ ㅇ ㅈ ㅎ  then aspirated ㅋ ㅌ ㅍ ㅊ
   - `ㅇ` is flagged as silent at the start of a block, "ng" at the bottom.
2. **Basic vowels (jamo)**: ㅏ ㅓ ㅗ ㅜ ㅡ ㅣ  then y-vowels ㅑ ㅕ ㅛ ㅠ
   - Vertical vowels (ㅏㅓㅣ...) sit to the RIGHT of the first consonant; horizontal
     vowels (ㅗㅜㅡ) sit BELOW. This right/below rule is taught on the first of each.
3. **Syllable-block composition + batchim**: the 받침 (final-consonant slot)
   - One element card teaches the block reading order (initial -> vowel -> batchim)
     and that a consonant at the bottom is the final sound.
4. **Tense + complex jamo**: tense ㄲ ㄸ ㅃ ㅆ ㅉ, then complex vowels
   ㅐ ㅔ ㅚ ㅟ ㅘ ㅝ ㅢ ㅒ ㅖ
   - Held to the end because they are visually and phonetically the trickiest, and
     the pilot's first words don't need them.

## First 20 words (in teaching order)

Single CV blocks first, then two-syllable CV+CV, then CVC (batchim) words later.

1. 소  2. 코  3. 비  4. 나무  5. 다리  6. 도시  7. 모자  8. 시계  9. 나라  10. 거리
11. 머리  12. 도마  13. 우유  14. 아기  15. 지도  16. 사과  17. 이마  18. 허리  19. 수저  20. 의자

Then the remaining easy words, most of them CVC batchim blocks (감 강 국 눈 달 말 문
물 밤 밥 방 별 봄 불 ...), then medium -> hard -> expert by tier.

## Notes for the auditor

- The batchim is introduced as a single structural element (the 받침 slot), not as a
  second copy of each consonant. Confirm this is a sane way to teach final consonants
  for a beginner, or suggest the pieces be split out.
- ㅐ vs ㅔ are treated as near-merged for most modern speakers; confirm.

## Card strings

English source text, to be localized later. Budget: <=2 strings per element,
<=120 chars each, ~100 per language. **Korean uses 40 strings** (well within budget).
All element strings are in `ko.json` and shown to the auditor as a named checklist
section (see `README-REVIEW.md`).
