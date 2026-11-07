# CC-SOUNDALIKE-LADDER: Korean (ko) status

Voice: ko-KR-Wavenet-D. Key: `key_ko@1`. ladderStatus: **provisional** (both sets).

## Key
Hangul syllables decomposed to jamo; batchim neutralized to the 7 representative finals (낮=낫=낯=낱, 빗=빛, 입=잎; ㄺ→ㄱ, ㄻ→ㅁ, ㄼ/ㄽ/ㄾ/ㅀ→ㄹ,
ㄿ→ㅂ). A final followed by a vowel-initial syllable (liaison: 꽃이, 낮이) keeps its consonant, so liaison is never a merger. Assimilation
(국물/궁물) and vowel mergers (ㅐ/ㅔ) are not applied. Tests: 8 must-merge, 12 must-not-merge (낮이/낫이, 강/간, 개/게 ...); selftest passes.

## Result
The ko bank (3,163 words) holds only **2 sets**, both single syllables: 빗/빛 and 입/잎. Both are **provisional**: key, kaikki IPA check and
loopback agree. 읽다/잃다 and 발기/밝기 are correctly NOT merged (익따/일타, 발기/발끼).
Loopback controls: same 낮/낫, 밭/밧, 부엌/부억, 낱/낯; different 사과/학교, 나무/바다. Floor 0.575, ceiling 0.942, bar 0.648; 0 flagged.
IPA check note: ko entries carry only narrow [..] transcriptions, so the check reads those (tools/soundalike_ipa_check.py, ko only); never stored.

## Effects
Production grading unchanged (provisional reads only with `soundalike_provisional`, off by default). The ko Impossible pool matches a fresh build.
Gig B sheet: reports/soundalike-gigb-ko.csv (2 rows, not sent).
