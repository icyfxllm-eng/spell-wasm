# CC-BUILD219-FIXES — Phase 0 forensics

Produced 2026-09-02. Read §0 first: it changes what the rest of this file is about.

## §0 The build is 218, not 219 — and one finding overturns the plan

**There is no build 219.** The newest build on TestFlight is **218**, uploaded
2026-08-31T17:39:45-07:00, live to internal and external testers. Ship 219 was
queued and never ran: the chain died with its session, so `HEAD` is still ship
218's commit (`ad5d0020`) and the Russian Phase 4 work is sitting uncommitted in
the tree. The screens in §1 of the spec are **build 218**. The findings stand —
they were real screens — but every "build-219 asset set" below means 218's, and
the next build off this repo is numbered **219**.

**The important correction is not the number.** §0 of the spec says class 3(a)
is "Eric approved specific renders; different renders shipped." That is not what
happened. The approved artwork is present, intact, and correct in the shipping
bundle. What is broken is the **gallery thumbnail**, which never draws it.

Everything the spec proposes to pull (F6: 17 assets) would be pulling good
artwork because of a defect in a different file.

## §1 Root cause: thumbnails render word carriers, never the artwork

Each picture holds two path sets:

| field | what it is | where |
|---|---|---|
| `guide` | the traced artwork — the recognition layer | `config/wordpic/pictures.json` |
| `paths` | word carriers: the rails letters flow along | same, mirrored into `scans.json` |

`thumb_svg` (`src/wordpic_screen.rs:369`) renders **only** `scan_paths(pic)` —
the carriers. It never touches `p.guide`. There is no `wp-guide` element in a
thumbnail at all, and the behaviour is deliberate and tested:
`thumbs_are_strokes_only_and_track_progress` (`:3024`).

For an ordinary picture the carriers roughly follow the subject, so a
strokes-only thumb still reads as a dog. For a **masterpiece** the carriers are
a handful of near-horizontal rails, so the card is a few dashes — on the beige
gallery ground, exactly what §1 items 10–12 describe.

Guide geometry actually present in the shipping bundle:

| subject | guide paths | guide points | carriers |
|---|---|---|---|
| flamingo | 67 | 1041 | 6 |
| redfuji | 40 | 1126 | 6 |
| carp | 182 | 3200 | 6 |
| taos | 154 | 3200 | 10 |
| greatwave | 225 | 2571 | 8 |
| mona | 199 | 2200 | 21 |
| **rhino** | **0** | **0** | 39 |

Every master except **rhino** carries real artwork. Reproduced side by side in
`scratchpad/thumb-proof.html`: carriers-only gives Eric's dashes; carriers plus
guide gives a recognisable Mount Fuji and a recognisable flamingo.

`pictures.json` is `include_str!`-ed into the wasm (`src/wordpic.rs:304`), so the
guide ships on device. `guide_polys` (`wordpic_layout.rs:309`) filters nothing.
The CSS is correct on both grounds (`index.html:599`, `:683`). The data, the
transform and the styling are all fine; only the thumbnail omits the layer.

**rhino is the one genuine content gap:** 0 guide paths, 2 scan paths, and it is
the one master the band scan did not flag. It has no artwork to show.

## §2 Why the paths look like scan lines — and why that is not the defect

12 of 391 subjects carry 3+ full-width horizontal bands. All 12 are masterpieces;
no other pack is touched. Nine are 100% bands (paths == bands): hare, sunflowers,
banana, beetle, carp, flamingo, kosonowl, rose, wing. Flamingo's six sit at a
75.0px pitch, spanning 3.0px each, all starting at x=0.

That is what a word carrier *is*. `tools/build_f9_scans.py:84` says so:

> These rails are authored word carriers, not traced ink

So the regular bands are correct by construction. Eric's "tonal-band or scan-line
pass" hypothesis in §2 of the spec is **half right**: the geometry is machine-
regular, but it is not a failed trace — it is the carrier layer doing its job,
shown alone because the thumbnail drops its partner.

**A real provenance defect exists alongside it.** `build_f9_scans.py:98` writes

```python
"authoring": "hand-traced",
```

as a hardcoded literal on every file it emits. Eight masters (commit `ce2dd56e`,
2026-08-22, "F9 registered: eight masters, 7 traced to 15") claim hand-tracing
from a constant. There is no tool name, no version, no reference hash, no date.
`authoring` is an unverified string and must not be read as provenance.

## §3 The beige card

Not a second renderer — a per-picture ground flag.

- `wordpic::ground_for(p)` (`src/wordpic.rs:134`) returns `LIGHT` when
  `p.categories` contains `"masters"`, else `DARK`.
- `LIGHT` is `bg #f4efe4`, class `.wp-light`; introduced ship 171.
- Routing through it: every `masters` picture — packs `worldart2`, `worldart3`,
  `wave3`, `starter`, `animals` (15 subjects).

D3 is signed (dark canvas for all), so this is a flag flip, not a deletion. Note
the adjacent history in `src/wordpic.rs:96` — the guide was once painted
near-white on this same LIGHT ground and was invisible for three ships. The
current bug is the same *symptom* (masters lose their artwork) from a different
*cause* (thumbnail omission, not colour).

## §4 Approval records: none exist, for any asset

The shipping subject schema across all 391 entries in `scans.json` is exactly:

```
{attr, layers, paths, req, tier}
```

No `approval`, no `status`, no `traceHash`, no `layoutHash`, no `approvedBy`.
`pictures.json` adds `provenance` (title/artist/source — attribution, not
approval). Source scan docs carry `pin_hash`, which proves the tracer is
deterministic; it does not encode that a human approved the result.

So the answer to "any asset with no approval record" is **all 394**. F7 is not
repairing a broken mechanism; it is building the first one. F4 and F6 both depend
on a `status` field that does not exist yet either.

## §5 Did the eval pass these?

The gate runs `tools/density_check.py` (density floor + bank-wide ratchet) and
`tools/masterpiece_lint.py` (outline hierarchy). Both are green on this asset set.

The masters floor is **500 points**, and it counts points, not content:

| subject | points | verdict |
|---|---|---|
| carp | 554 | PASSES |
| beetle | 560 | PASSES |
| rose | 643 | PASSES |
| flamingo | 752 | PASSES |
| banana | 769 | PASSES |
| kosonowl | 807 | PASSES |
| hare | 866 | PASSES |
| wing | 1015 | PASSES |

Six long smooth rails clear a 500-point floor comfortably. The gate is not broken
in the sense F7 assumes — it never measured whether a subject is recognisable,
and no gate anywhere measures the guide layer at all. **Per D8, Eric's grades are
the truth set; but here the eval and Eric are not in conflict about the same
thing.** He graded thumbnails; the eval measured carriers. Recalibrating the eval
against thumbnail grades would be fitting the wrong instrument to the wrong
symptom — fix the thumbnail first, then re-grade.

## §6 Daily Challenge audio

**What is settled:**

- **Daily has no audio path of its own.** It routes through `speak_word`
  (`src/game.rs:1011`) like every other mode. F2's "delete any Daily-local audio
  path" is a no-op — there is nothing to delete, and the single-resolver
  requirement (I2.1) already holds for word readout.
- **The audio session is already correct.** `.playback, mode: .default` is set at
  launch (`AppDelegate.swift:13`) and before each speak
  (`NativeLanguageKitPlugin.swift:91`, `:115`). F2's session item needs no work.
- **AVSpeechSynthesizer IS in the word-readout path, for every mode.** The router
  order is Pack → ServerCache → **NativeTts** (`src/api.rs:85`), and
  `play_native_tts` (`:316`) resolves to the Swift `Speaker`, i.e.
  AVSpeechSynthesizer. This is the doctrine violation F2 names, and it is not
  Daily-specific — it is rung 3 by design for the whole app.
- **No Daily pre-cache exists.** `preload_pool` warms the *tier* pool
  (`active_word_list`), never the date-seeded Daily set, and there is no nightly
  job on the server. F2's pre-cache item is real, unstarted work.
- Server audio cache currently holds **89,627** mp3 clips, so a cold English
  Daily word is possible but not obviously likely.

**What is NOT settled, and needs device evidence:** which rung actually played on
Eric's device, and the LUFS comparison §2 asks for. That cannot be established
from this repo. A second, untested hypothesis worth instrumenting: three mic
capture paths set `.playAndRecord, mode: .measurement`
(`NativeLanguageKitPlugin.swift:579`, `:813`, `:1013`), a mode that attenuates
output. They do restore `.playback` in `finish()` (`:1221`), so this only bites
if a capture ends without reaching `finish` — unproven, and it would explain
"barely audible" where NativeTts alone explains "computer voice."

Recommend landing `AUDIO_MISS(lang, word, mode)` logging (already in F2) *before*
concluding, so the next occurrence names its own rung.

## §7 Weapons scan (F5, retroactive)

3 hits across 394 pictures:

| id | name | pack | terms | reading |
|---|---|---|---|---|
| `maasaishield` | Maasai Shield | culture7 | shield | **real** — slated for archive |
| `arrow` | Arrow | symbols1 | arrow | false positive: directional symbol |
| `banana` | Banana Flower | worldart3 | blade | false positive: botanical leaf blade |

Two of three need `contentReview: {weapons: none, …}` with a note, exactly as F5
designs it ("lint is a flag, contentReview is the decision").

**The limitation matters more than the hits.** The blocklist searches titles,
tags, aliases, feature names and guide-anchor names. **"spear" appears nowhere in
any of them** — the Maasai Shield's spears exist only in the artwork. A text lint
cannot see a depiction. F5 will catch names, and will not catch the next
Maasai Shield.

## §8 What this changes

1. **F6 as written pulls 17 good assets.** The artwork is fine. Fixing
   `thumb_svg` to draw the guide layer likely resolves items 10, 11 and 12
   outright. Recommend: fix the thumbnail, re-render the gallery, and let Eric
   re-grade before pulling anything.
2. **rhino genuinely has no artwork** (0 guide paths) and is the one master that
   should go `pending-approval` on content grounds.
3. **F7 is still entirely warranted** — there is no approval record anywhere, and
   `authoring` is a hardcoded lie. Build it.
4. **F5's lint cannot see depictions.** Worth saying out loud before it is
   treated as coverage.

## §9 Stop-and-ask

- **D6 — "Basil".** The spec assumes Church, Taos Pueblo. There is no subject
  matching "basil" or "church" in the bank; the only Taos entry is `taos`
  ("after Ansel Adams"). Confirm `taos` is meant.
- **Great Pyramid (`pyramids`)** is *not* a master: dark canvas, 2 scan paths, 11
  and 7 points, `authoring: "suggested-then-approved"`. Its thumbnail problem is
  not the guide-layer bug — it is genuinely thin. It needs its own look.
- Whether to fix the thumbnail before executing F6, per §8.1.
