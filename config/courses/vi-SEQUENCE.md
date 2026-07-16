# Vietnamese Script Path - sequence summary (DRAFT for native review)

**Scope: the writing system only.** Teaches a true beginner to READ and SPELL the
Vietnamese Latin alphabet (chu Quoc ngu). No grammar, no word meanings, no
comprehension. Words are drawn unchanged from the audited lists in `assets/words/vi/`.

Strictly linear. Full JSON: `config/courses/vi.json`.

## Element order and rationale

The learner may already know the Latin shapes, so the path moves quickly through the
plain letters and spends its weight on what is genuinely Vietnamese: the extra
letters, the multi-letter units, and the tone diacritics.

1. **Base Latin letters**: vowels a e i o u y, then consonants b c d g h k l m n p q r s t v x
   - `d` is explicitly taught as the plain letter (sounds "z"/"y"), distinct from `đ`.
   - Flag: f, j, w, z are NOT letters of the Vietnamese alphabet.
2. **Special letters**: đ  ă â ê ô ơ ư
   - Taught as *separate letters*, not accented versions - the breve/circumflex/horn
     is part of the letter's identity, NOT a tone mark. This distinction is called out
     on the first card, because it is the classic beginner confusion.
3. **Digraphs & trigraphs**: ch gh gi kh ng ngh nh ph th tr qu
   - Read as single units; needed before most real words can be spelled.
4. **Tone diacritics**: a (level, no mark), a-grave, a-acute, a-hook, a-tilde, a-dot
   (huyen, sac, hoi, nga, nang)
   - Taught last, as a layer that sits on the vowel and changes only pitch. This keeps
     the letter-vs-tone distinction from step 2 clean.

## First 20 words (in teaching order)

Level-tone, plain-letter words first; then the first special letter (đ), then the
first tone marks.

1. to  2. xe  3. cao  4. cam  5. cau  6. dao  7. rau  8. sao  9. tai  10. tay
11. mua  12. hoa  13. bay  14. vui  15. voi  16. đi  17. ăn  18. áo  19. cá  20. bò

Then the remaining easy words (more đ and diacritic words), then medium -> hard ->
expert by tier, which naturally layers on the horned vowels + stacked tone marks
(cuoi, nuoc, duong, truong ...).

## Notes for the auditor

- Composed vowels like `ấ` `ắ` `ế` are single Unicode characters in the word lists but
  are taught as base-special-letter + tone-mark. Confirm learners should compose them
  this way (matches how they are typed).
- North/south pronunciations differ for d, gi, r, s, tr, v; the cards give a neutral
  gloss. Auditor: note any that read as wrong for a general learner.

## Card strings

English source text, to be localized later. Budget: <=2 strings per element,
<=120 chars each, ~100 per language. **Vietnamese uses 48 strings** (well within
budget). All element strings are in `vi.json` and shown to the auditor as a named
checklist section (see `README-REVIEW.md`).
