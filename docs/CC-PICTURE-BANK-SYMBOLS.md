# CC-PICTURE-BANK-SYMBOLS
[Received from Eric 2026-08-05, verbatim in chat. D1-D4 SIGNED same day.]

Intent. The symbol catalog is nearly perfect Spell Picture material:
language-neutral (all 15 languages automatic), high recognition, and
mostly ancient/geometric — meaning most subjects need no photo sourcing
at all. The risk isn't finding assets; it's provenance discipline and
kid-safety curation.

Feature 1 — Three source lanes, assigned per subject.
* GEOMETRIC (construct, no photo): triquetra, triskelion, pentagram,
  yin-yang, Merkaba, Sri Yantra, Metatron's Cube, vegvisir, Helm of Awe,
  sun, moon, compass, Laguz, ankh, cross, diamond, arrow, all 12 zodiac
  glyphs. Authored as mathematical constructions. Zero copyright surface.
* PHOTO-TRACE (PD/CC0 via SCAN-STACK T0): pyramid, mountain, pagoda,
  lotus, rose, lily, carnation, anchor, feather, eagle, raven, stag,
  tiger, owl, scarab, butterfly, fish. Reference never ships.
* COMMISSIONED-INK (WAVE2 lane, contract-hash provenance): dragon,
  phoenix, dreamcatcher, cornucopia, Holy Grail, Tree of Life, hamsa.

Feature 2 — Cross-listing per the rhino pattern (D2 precedent).
Feature 3 — Zodiac as its own pack.
Non-goals: no book text imported, no dagger, no runtime art generation,
no horoscope content — symbols only, this is a spelling app.

## DECISIONS SIGNED (Eric, 2026-08-05): "D1 agree, D2 add the norse
## staves, D3 revise, D4 yes"
* D1 — Dagger excluded (final). ARROW KEEPS, THOR'S HAMMER EXCLUDED.
  The deciding reason is not the weapon shape: Mjolnir is one of the
  Norse symbols appropriated by hate groups alongside its legitimate
  Heathen religious use, and that ambiguity is the cost.
* D2 — Include all with per-subject `kidFilterReview`, held out of Kid
  Mode pending Eric's explicit pass. The review list is FOUR, not two:
  pentagram, Eye of Providence, VEGVISIR and HELM OF AWE — the two
  Icelandic staves carry the same appropriation history as Mjolnir.
* D3 — REVISED at wiring time: the Zodiac pack this file proposed
  shipping first ALREADY SHIPS (12 glyphs, own pack). The order is
  therefore geometric-new -> photo-trace -> commissioned.
* D4 — YES. Sri Yantra and Metatron's Cube ship expert-tier. They also
  fix a real gap: the bank had only 8 playable expert pictures against
  42 easy and 84 medium.

## INVENTORY AT SIGNING (measured, not estimated)
28 of the file's 53 subjects were ALREADY in the bank. Genuinely new:
12 geometric, 8 photo-trace, 5 commissioned.

## GEOMETRIC LANE DELIVERED 2026-08-05 (12 subjects, bank -> 385)
tools/draw_symbols.py constructs all twelve mathematically — no photo,
no scan, no third-party art — so invariant 1 (PD-only) is satisfied by
construction and invariant 2 (book-art firewall) is trivially clean:
the generator is in the repo and reproduces them exactly.
  easy    laguz, cross, diamond, arrow
  medium  triquetra, pentagram, yinyang, merkaba
  hard    vegvisir, helmofawe
  expert  sriyantra, metatron
Sri Yantra was re-spaced after review (v1 stacked the nine triangles
into one dark mass; each now steps in width and height so the
interlocking reads).
TWO LAWS the L9 sweep taught on this lane:
  1. Budget starvation again — a [2,5] ceiling starves the long-word
     pools; every symbol path floors at [2,10].
  2. CLOSED RINGS SELF-FACE. A ring is one path the engine slices into
     many segments, and each slice faces its neighbour across the ring's
     own width (triquetra's lobes, yinyang's circle). A closed host is
     now cut to ONE ~55% arc; the full ring stays as guide ink.
Sweep GREEN across all 15 languages x 5 seeds.
STILL ERIC'S: the kidFilterReview pass on pentagram / vegvisir /
helmofawe (they ship kid:false until then), and the on-device pass.
