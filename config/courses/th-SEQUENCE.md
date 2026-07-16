# Thai Script Path - sequence summary (DRAFT for native review)

**Scope: the writing system only.** This teaches a true beginner to READ and SPELL
the Thai script. No grammar, no word meanings, no comprehension. Words are drawn
unchanged from the already-audited lists in `assets/words/th/`.

Strictly linear. Full JSON: `config/courses/th.json`.

## Element order and rationale

The spine is Thai's own logic: **consonant class decides tone**, so consonants are
taught grouped by class before anything that depends on class (vowels, tone marks).

1. **Mid-class consonants** (อักษรกลาง): ก จ ด ต บ ป อ
   - The "default" tone class and the smallest group; the natural anchor. `อ` is
     introduced here as the silent consonant / vowel-carrier.
2. **High-class consonants** (อักษรสูง): ส ห ข ถ ผ ฝ ศ ษ
   - `ห` is flagged as the silent "leading h" that raises a following low consonant.
3. **Low-class consonants** (อักษรต่ำ): ง ม น ร ว ย ล พ ท ค ธ ฟ ภ ณ ช
   - Largest group; split into two lessons. Ordered by frequency in the th lists.
4. **Vowels**: ะ า ิ ี ึ ื ุ ู เ แ โ ไ ใ ำ  plus  ั (han-akat) and ็ (maitaikhu)
   - Long/short pairs taught together; the "written before but sounded after" forms
     (เ แ โ ไ ใ) are explicitly flagged.
5. **Tone marks + silencer**: ่ ้ ๊ ๋  and  ์ (thanthakhat)
   - Introduced last, because reading a tone mark requires already knowing the
     consonant's class and the vowel length.

**Coverage note:** this DRAFT teaches the 30 consonants that actually occur in the
audited th word lists. Five very rare letters (ฆ ฬ ญ ฑ ฒ, each appearing once) and
the unused letters (ซ ฉ ฐ ฎ ฏ ฮ) are deferred; no spell word in this path uses them.
Auditor: confirm this subset is acceptable for a v1 pilot, or name letters to add.

## First 20 words (in teaching order)

Chosen for transparent orthography first: short, live syllables, no tone marks,
mid/low consonants, then closed syllables, then the silent-`ห` tone words later.

1. ตา  2. นา  3. งา  4. ดี  5. ปู  6. งู  7. นม  8. นก  9. กบ  10. มด
11. ลม  12. รถ  13. ดิน  14. ผัก  15. วัด  16. วัว  17. ยาย  18. ยาว  19. ลิง  20. ลูก

Then the remaining easy words (silent-`ห` group หมา หมู หมี หนู; the เ- vowel group
เสือ เรือ เล็ก; tone-marked ช้า น้ำ ปู่), then medium -> hard -> expert by tier.

## FLAG - Thai consonant names & audio (decision 12)

Thai consonants are conventionally named with a mnemonic word ("ก = ko kai",
"ข = kho khai"). The current TTS voice may pronounce these letter-names poorly or
unnaturally. **Recorded human audio may be needed for the consonant intro cards.**
This is flagged only - nothing is being commissioned here. Auditor: please note which
letter-names sound wrong through the app's current voice.

## Card strings

English source text, to be localized later. Budget: <=2 strings per element,
<=120 chars each, ~100 per language. **Thai uses 57 strings** (well within budget).
All instructional strings for each element are in `th.json` and shown to the auditor
as a named checklist section (see `README-REVIEW.md`).
