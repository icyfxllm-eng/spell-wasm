# CC-MYWORDS-LISTS v1 — Dated word lists instead of "Replace my current words"

**Status: REVIEW-GATED. Execute after Eric signs §6 PROPOSED items.**
Amends: CC-SNAP-LIST v1. This file resolves its D8 and changes the wording of F8
rule 7; that is the only thing it changes there.

Blast radius:

* the My Words data model
* the My Words screen
* the save sheet, which is shared by Snap a word list and, pending D5, Spell Translate
* the source picker for playing My Words

> **Census (§0) run 2026-09-15 — see `docs/census/mywords-lists-census.md`.**
> Four stop conditions fired: the free cap's declared unit is *lists* (C2), there
> is no profile system (C4), CC-SNAP-LIST v1 is not in this repo, and the §8
> commands name paths this repo does not have. Nothing below is implemented.

## 1. Intent

Parents and teachers get a new spelling list every week. Today, saving a new photo
means choosing between wiping last week's words and mixing everything into one
pile. Both are bad. Wiping destroys words a child still needs to review. Mixing
makes "this week's list" impossible to find.

The player should never lose words by saving new ones. Each save should land
somewhere they can find by date. Deleting words should only happen on purpose,
from My Words, with a way back.

When this spec has a gap, choose whatever keeps saved words findable and never
destroyed by accident. If that is unclear, stop and ask.

## 2. Ownership boundaries

* CC-BANK-TRANSLATE owns the My Words ingest path. This file changes where a save
  lands (which list), not how an entry is validated or ingested. If ingest assumes
  a flat list, stop and ask.
* CC-SNAP-LIST owns everything before Save: reading, rows, and flags.
* CC-TRANSLATE-SCREEN F9 uses the save sheet only if D5 is signed.
* CC-ONBOARD-JR owns profiles and Jr gating. Lists are scoped per profile and do
  not change that.
* Scoring, spaced repetition, and missed-words are untouched. See invariant I5.

## 0. Blocking census

Write `docs/census/mywords-lists-census.md`.

* C1 Storage. Where My Words lives (device storage, the Rust core, the Pi backend,
  iCloud). Record the exact current schema.
* C2 Free cap. The monetization plan says "custom lists beyond free cap" is
  premium. Record what the cap counts today (words, lists, or saves) and its value.
* C3 Per-word progress. Is spaced-repetition or missed-word progress keyed by
  word + language, or by position in My Words?
* C4 Profiles. Is My Words per profile, or per device?
* C5 Consumers. List every screen or mode that reads My Words, with file paths.
* C6 Existing data. Row counts on Eric's TestFlight device and in the fixture DB.
  Do entries carry any timestamp?

Stop and ask if:

* C2's cap counts something that dated lists would change. Lists must not quietly
  give away or take away premium value.
* C3 keys progress by position, since migrating would reset children's progress.
* My Words syncs across devices. That adds merge rules this file does not cover.

## 3. Data model

```
WordList {
  id: Uuid
  profileId: ProfileId
  name: String            // default = local date label at creation, see F2
  createdAt: UtcTimestamp
  updatedAt: UtcTimestamp
  source: Photo | Translate | Manual | Migrated
  entries: Vec<ListEntry> // ordered as saved
  deletedAt: Option<UtcTimestamp>   // soft delete, see F5
}
ListEntry { text: String (NFC), lang: LangCode, addedAt: UtcTimestamp }
```

## 4. Features

### F1 — Save destination replaces "Replace my current words"

**Intent.** The one checkbox that could erase everything sat directly above the
Save button, and it was checked by default. Saving should only ever add words.

**Behavior.**

1. Remove the "Replace my current words" checkbox. The save sheet has no
   destructive option of any kind.
2. Add a destination control above Save, with two choices:
   * New list · Sep 14, 2026, the default, shown with today's local date.
   * Add to a list…, which opens a picker of the profile's lists, newest first.
3. The button label follows the choice: "Save as new list" or "Add to Sep 7, 2026".
4. After saving, show a toast ("12 words saved to Sep 14, 2026") with an Open
   action that goes to that list.

**Acceptance.**

* AT1.1 Playwright: the save sheet contains no element whose text matches /replace/i.
* AT1.2 Save twice from two different photos → two lists; both entry sets are fully present.
* AT1.3 Add to an existing list → that list's count grows by the number of new
  unique entries; other lists are unchanged (hash compare).

### F2 — Date naming

**Intent.** Players find lists by when they got them ("this week's list"). The date
is the name, so no one has to type anything.

**Behavior.**

1. The default name is the local calendar date at save time, in the UI language's
   date format.
2. A second new list on the same local date is named "Sep 14, 2026 (2)", then
   "(3)", and so on.
3. `createdAt` is stored in UTC. The label is derived once, at creation. Changing
   time zones later does not rename a list.
4. Players can rename a list, trimmed to 1–40 characters. Names are unique per
   profile; a clash appends " (2)".

**Acceptance.**

* AT2.1 Unit test: create lists at 23:59 and 00:01 local → two different date labels.
* AT2.2 Three saves on the same date → base, (2), (3).
* AT2.3 Changing the device time zone after save leaves the name unchanged.
* AT2.4 Renaming to an existing name → " (2)" is appended.

### F3 — My Words screen grouped by list

**Intent.** Recent lists come first, and any list opens in one tap.

**Behavior.**

1. My Words shows one card per list, sorted newest `createdAt` first. Each card
   shows the name, word count, and language chips (distinct languages, maximum 4,
   then "+N").
2. Tapping a card opens the list: entries in saved order, each with its language
   chip, plus rename, add words, and delete list.
3. An All words view at the top shows the union of every list, deduplicated by
   text + language, sorted A→Z within each language.
4. With zero lists, the empty state offers "Snap a word list" and "Add words".

**Acceptance.**

* AT3.1 Maestro: seed 3 lists dated Sep 1, 7, and 14 → the cards appear in the order 14, 7, 1.
* AT3.2 The same word + language in two lists appears once in All words.
* AT3.3 Language chips cap at 4 with a correct "+N".

### F4 — Playing lists

**Intent.** On the night before a spelling test, a child plays this week's list,
not every word ever saved.

**Behavior.**

1. When My Words is chosen as a word source, the source picker lets the player
   select one or more lists. The newest list is preselected. An "All words" option
   is also available.
2. The selection is remembered per profile. If a remembered list is deleted, fall
   back to the newest list.
3. The selected entries are deduplicated by text + language before being handed to
   the mode. The mode receives the same entry shape it receives today (C5).

**Acceptance.**

* AT4.1 Select the Sep 7 list only → a session draws only Sep 7 entries (run 100
  draws against a mocked mode).
* AT4.2 Delete the remembered list → the next launch preselects the newest list.
* AT4.3 The payload shape handed to modes is byte-compatible with the pre-change fixture.

### F5 — Deleting on purpose, with undo

**Intent.** Deleting is still needed, but it has to be deliberate and recoverable.
A child tapping around must not wipe a parent's month of lists.

**Behavior.**

1. Delete lives only inside an opened list, never on the save sheet or on the list cards.
2. Deleting shows a confirm dialog naming the list and its word count. It then
   soft-deletes (`deletedAt` set) and shows an Undo toast for 8 s.
3. Soft-deleted lists are purged after 30 days, or through a "Recently deleted" row
   at the bottom of My Words that offers Restore and Delete now.
4. Removing a single entry needs no confirm and shows an Undo toast.
5. In Spell Jr profiles, deleting a list requires the existing parent gate.

**Acceptance.**

* AT5.1 Delete → Undo → the list is byte-identical to before.
* AT5.2 A soft-deleted list is absent from My Words, from All words, and from the play picker.
* AT5.3 With the clock mocked forward 30 days, the purge job removes it.
* AT5.4 The Jr profile delete is blocked without the parent gate (Maestro).

### F6 — Lossless migration

**Intent.** Existing players already have words saved, and none of them may
disappear or lose progress when this ships.

**Behavior.**

1. A one-time, idempotent migration moves the existing flat My Words into one list
   per profile:
   * Name: "Saved before Sep 14, 2026", using the migration date.
   * source: Migrated.
   * createdAt: the earliest entry timestamp if one exists (C6), otherwise the
     migration time.
2. Entry order and language are preserved. An empty My Words produces no list.
3. A migration version marker prevents it from running twice.

**Acceptance.**

* AT6.1 A fixture with N entries → one list with N entries, same order, same
  `(text, lang)` multiset.
* AT6.2 Running the migration twice → identical state.
* AT6.3 Per-word progress for every migrated word is unchanged (C3 keying).
* AT6.4 Empty My Words → zero lists and no crash.

## 5. Invariants

* I1 Saving never deletes. No code path under Save reduces the entry count of any
  list. [property test: random save sequences, counts are monotonic]
* I2 No hard delete without a soft delete first, except the 30-day purge and
  "Delete now". [AT5.x]
* I3 Unique within a list. No two entries share the same text + language. [unit test]
* I4 NFC entries. [property test]
* I5 Progress is independent of lists. Deleting, renaming, or moving a list changes
  no spaced-repetition or missed-word data. [snapshot compare]
* I6 Profile isolation. No list is readable across profiles. [unit test]
* I7 Migration is lossless and idempotent. [AT6.1, AT6.2]

## 6. Decisions

PROPOSED (Eric signs or edits):

* D1 The default destination is a new dated list.
* D2 A word may exist in several lists. Duplicates are removed only within a list,
  and in All words or play.
* D3 The newest list is preselected for play.
* D4 Soft delete with 8 s undo and a 30-day Recently deleted.

Open (do not guess):

* D5 Should Spell Translate's "Save to My Words" (CC-TRANSLATE-SCREEN F9) use this
  same destination sheet? The proposal is yes, with a single sheet, but that edits
  another file's feature.
* D6 Free cap (from C2): does the free tier limit the number of lists, the number
  of words, or neither? The proposal is to keep whatever unit the cap counts today
  and not introduce a list cap in this file.
* D7 Should kid profiles be able to create lists, or only play lists a parent saved?

## 7. Non-goals

* No folders, tags, colors, or sharing lists between devices or people.
* No list export or import, and no cross-device sync.
* No changes to the ingest path, the modes, scoring, or spaced repetition.
* No automatic weekly grouping. The date is the organization.

## 8. Done means

Use the exact command names recorded in CC-SNAP-LIST C6.

```
cargo test -p <core-crate> mywords_      # model, F2, F6, I1–I7
npx playwright test tests/mywords-lists/ # F1, F3.2–3.3, F4, F5.1–5.2
maestro test .maestro/mywords-lists/     # F3.1, F5.4, migration on a seeded device
```

* [ ] The census (C1–C6) is complete.
* [ ] Before/after screenshots of the save sheet and My Words are in the PR.
* [ ] D1–D4 are signed, and D5–D7 are answered or explicitly deferred.
* [ ] CC-SNAP-LIST D8 is marked "resolved by CC-MYWORDS-LISTS F1".

## 9. Stop-and-ask triggers

* Any §0 stop condition fires.
* Migration cannot preserve per-word progress.
* A signed item in CC-BANK-TRANSLATE or CC-TRANSLATE-SCREEN assumes a single flat
  My Words list.
