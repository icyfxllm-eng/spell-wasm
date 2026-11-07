# CC-SOUNDALIKE-LADDER: Vietnamese (vi) status

Voice: vi-VN-Wavenet-A. Key: `key_vi@1`. ladderStatus: **machine_checked** (lowest set). Counts: 42 true sets = **34 provisional, 8 machine_checked**.

## Key (northern / Hanoi reading)
Onsets d = gi = r, s = x, ch = tr; a lone y after a consonant is i (hí/hý, lì/lỳ). Tones are part of the vowel letter and are **never merged**
(ma/má/mà/mả/mã/mạ stay six keys). đ/d, ng/ngh spelling pairs, finals and every other letter are untouched. The bank excludes multi-syllable
entries (spaces), so sets are single syllables. Tests: 10 must-merge, 14 must-not-merge; selftest passes.

## Voice
The northern reading is **supported by the audio, not confirmed by a listener**: the voice renders da/gia (0.448) and da/ra (0.484) as one sound and
sa/xa (0.607), chanh/tranh (0.518) close, against different-sound controls at 0.82-0.91. The kaikki IPA check uses only Hà-Nội-tagged readings.
Eric should still listen once; the voice name does not say which dialect it is.

## Loopback
Controls: same da/gia, sa/xa, chanh/tranh, dì/gì; different nhà/cửa, bàn/ghế. Floor 0.607, ceiling 0.824, bar 0.651. Two control choices I
changed after seeing them: ma/má cannot be a "different" control (the loopback hears spectra, not pitch, so a tone pair looks the same), and
chơi/trơi was a poor "same" control (0.794: a non-word the voice reads oddly), replaced by chanh/tranh. 4 sets flagged `voice-may-distinguish`:
chích/trích, chóc/tróc, chệch/trệch, sắt/xắt.

## Not promoted (8 of 42 = 19%, under the 20% stop line)
loopback_flag: chích/trích, chóc/tróc, chệch/trệch, sắt/xắt. ipa_missing: chình/trình, chảo/trảo, gì/rì, lì/lỳ.
The IPA check agreed on all 37 sets that had IPA for every member. Tone is part of the compared transcription, so a tone difference would show as differ.

## Effects
Production grading unchanged (provisional reads only with `soundalike_provisional`, off by default). vi Impossible stays locked (no exclusion gate);
pools match a fresh build. Gig B sheet: reports/soundalike-gigb-vi.csv (42 rows, not sent).
