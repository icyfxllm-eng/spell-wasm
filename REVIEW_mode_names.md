# REVIEW-GATED: mode-name translations (machine-draft — native audit needed)

All 4 renamed modes × 17 locales are shipped as machine-drafts (better than the
English fallback for non-English users) but every name needs a native yes/no.
Brief (§6): does it feel like a native app's menu AND do the same emotional job?

## Rejection criteria (D5 — state to auditors)
- **Tricky Words**: MUST keep "the WORDS are tricky" — any name meaning "your
  mistakes / failures / wrong words" is REJECTED (blame the words, not the kid).
- **Little Spellers**: a warm "small/beginner spellers" concept — never
  transliterated English.
- **Spell-Off**: a friendly contest/duel framing per culture. **No sword, ever.**
- **Quick Bee**: keep the bee only where spelling-bee culture exists; elsewhere a
  "quick round" equivalent is correct and preferred.

## Table (locale × mode → proposed name · back-translation)
| locale | Spell-Off | Little Spellers | Tricky Words | Quick Bee |
|---|---|---|---|---|
| en | Spell-Off | Little Spellers | Tricky Words | Quick Bee |
| es | Duelo de Letras (Letter Duel) | Pequeños Deletreadores (Little Spellers) | Palabras Difíciles (Hard Words) | Ronda Rápida (Quick Round) |
| fr | Duel d'orthographe (Spelling Duel) | Petits Épeleurs | Mots malins (Clever Words) | Manche rapide |
| de | Buchstaben-Duell | Kleine Buchstabierer | Knifflige Wörter (Tricky Words) | Schnelle Runde |
| pt | Duelo de Soletração | Pequenos Soletradores | Palavras Traiçoeiras (Sneaky Words) | Rodada Rápida |
| it | Sfida di Spelling | Piccoli Spellatori | Parole insidiose (Sneaky Words) | Round veloce |
| nl | Spelduel | Kleine Spellers | Lastige Woorden (Tricky Words) | Snelle Ronde |
| pl | Pojedynek ortograficzny | Mali Literowacze | Podchwytliwe Słowa (Tricky Words) | Szybka Runda |
| sv | Stavduell | Små Stavare | Kluriga Ord (Clever Words) | Snabb Runda |
| nb | Stavedyst | Små Stavere | Lure Ord (Sneaky Words) | Rask Runde |
| tr | Yazım Düellosu | Küçük Hececiler | Yanıltıcı Kelimeler (Deceptive Words) | Hızlı Tur |
| vi | Đấu chính tả | Bé Tập Đánh Vần | Từ Khó Nhằn (Tough Words) | Vòng Nhanh |
| ko | 맞춤법 대결 (Spelling Match) | 꼬마 스펠러 | 까다로운 단어 (Tricky Words) | 빠른 라운드 |
| ja | スペル対決 (Spell Showdown) | ちびっこスペラー | てごわい言葉 (Tough Words) | クイックラウンド |
| zh | 拼写对决 (Spelling Duel) | 小小拼写家 | 刁钻词 (Tricky Words) | 快速一局 |
| th | ประลองสะกดคำ | นักสะกดตัวน้อย | คำหลอกตา (Deceptive Words) | รอบเร็ว |
| fil | Labanan sa Ispeling | Munting Speller | Mga Mailap na Salita (Elusive Words) | Mabilis na Round |

## Known follow-ups for the audit pass (not tiles — mid-sentence prose)
- `vs.leaveBlurb` and `error.offline` in the 16 non-English locales still embed
  the PRIOR localized mode name mid-sentence (EN updated to "Spell-Off").
- `ach.timed10.desc` in the 16 non-English locales still says the prior "Timed
  mode" name (EN updated to "Quick Bee"). Pluralization: no "Spell-Off(s)"
  count strings exist yet (no head-to-head achievement).
