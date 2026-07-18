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

---

# Fonts (bundled in the app)

These font files ship inside the app (`fonts/`). All are self-hosted; the app
loads no font from any external host. Copyright notices below are taken from each
font's own metadata. The SIL OFL 1.1 fonts share one license text in
`fonts/OFL.txt`; OpenDyslexic has its own, noted below.

## Interface faces (Latin) — SIL OFL 1.1
- **Bricolage Grotesque** — © 2022 The Bricolage Grotesque Project Authors.
  `bricolage-latin.woff2`, `bricolage-latinext.woff2`, `bricolage-viet.woff2`.
- **Instrument Sans** — © 2022 The Instrument Sans Project Authors.
  `instrument-latin.woff2`, `instrument-latinext.woff2`.
- **Space Mono** — © 2016 The Space Mono Project Authors.
  `spacemono-400/700-{latin,latinext,viet}.woff2`.

## Study-language faces — SIL OFL 1.1
- **Noto Naskh Arabic** — © 2022 The Noto Project Authors. Arabic/Persian (ar/fa).
  `naskh-arabic.woff2`.
- **Noto Nastaliq Urdu** — © 2014 Google Inc. Urdu (ur). `nastaliq-urdu.woff2`.
- **Noto Sans Devanagari** — © 2022 The Noto Project Authors. Hindi (hi).
  `devanagari.woff2`.

The full SIL Open Font License, Version 1.1, for all of the above is in
`fonts/OFL.txt`.

## Readable mode — separate licence
- **OpenDyslexic** — `opendyslexic-regular.woff`. Original fonts © Bitstream;
  OpenDyslexic changes and additional glyphs by Abelardo Gonzalez, licensed under
  a **Creative Commons Attribution 3.0 Unported License**. Based on a work at
  dyslexicfonts.com. Bitstream licence: http://opendyslexic.org/legal/.
