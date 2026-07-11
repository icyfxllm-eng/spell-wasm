# Third-party data notices

SpellGame's word data is generated (build-time only) from these open datasets.
Only the emitted, filtered word lists ship in the app; the raw datasets are
build inputs and are not redistributed.

## Mandarin (zh)
- **CC-CEDICT** — Chinese-English dictionary (definitions, pinyin).
  © MDBG, licensed **CC BY-SA 4.0**. https://www.mdbg.net/chinese/dictionary?page=cc-cedict
- **HSK 3.0 word lists** — via `drkameleon/complete-hsk-vocabulary` (GitHub).
  Used as the educator-graded tier anchor. See the repository for its license.
- **Unihan Database** — character stroke counts / script metadata.
  © Unicode, Inc., **Unicode License**. https://www.unicode.org/reports/tr38/

Derived works (definitions shown in-app) are shared under CC BY-SA 4.0 where
CC-CEDICT-derived, per that license's share-alike terms.

## Japanese (ja)
- **JLPT word lists** — via `elzup/jlpt-word-list` (GitHub): word + reading +
  meaning + JLPT level. Meanings are JMdict-derived (EDRDG, CC BY-SA).
- **KANJIDIC2** — © EDRDG, **Creative Commons BY-SA**. Kyōiku school grade per kanji.

## Korean (ko) / Thai (th)
- **OpenSubtitles frequency lists** — `hermitdave/FrequencyWords`, **CC BY-SA 4.0**.
  Used as a documented SUBSTITUTE for TOPIK / Thai school-grade lists (not freely
  redistributable). Tiering signal only.
- **PyThaiNLP** word list (th) — Apache-2.0. Dictionary validation.
- NOTE: no open ko-en / th-en dictionary was reachable; these pools ship without
  English definitions pending a licensed source.
