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

## Signed decisions (Eric, 2026-09-17: "Use the recomendations")

App only. Recorded before any code. Where a ruling changes text below, the
changed line says so.

| # | Ruling |
|---|---|
| **D1** | **SIGNED, amended.** The save sheet's default follows the day: if a list was already created today, it defaults to **Add to that list**; otherwise to **a new dated list**. Either is one tap to switch. A worksheet photographed in two pages lands in one list; next week's photo still gets its own. Amends F1 behavior 2. |
| **D2** | **SIGNED, extended.** A word may live in several lists, never twice in one. Inside an opened list every entry is editable: **fix its text, change its language, remove it (with Undo), add words, copy it to another list.** Editing a word's text starts that word's progress fresh, because it is a different word (C3 keys progress by word + language). Extends F3 behavior 2. |
| **D3** | **SIGNED, extended.** The newest list is preselected for play. Each list also carries a switch: **Mixed** (default; the existing adaptive order, misses weighted) or **In my order** (entries played in the list's order, which the player sets by dragging; a starred entry moves to the front). **Amends §7:** "In my order" is the one permitted change to how a mode draws words. |
| **D4** | **SIGNED, extended.** As proposed: whole-list delete only inside an opened list, confirm naming the list and its word count, 8 s Undo, 30-day Recently deleted with Restore and Delete now; single-word removal needs no confirm, only Undo; in Spell Jr, deleting a list needs the parent gate. **Added:** lists older than a few weeks fold into an **Older lists** section, so the screen stays tidy without deleting. |
| **D5** | **SIGNED.** Translate's "Save to My Words" uses the same sheet, opened already set to today's list (D1), so it is a one-tap confirm with Cancel always present. Nothing reaches My Words from Translate unless the player taps Save; lookups, swap and Now spell it never add a word. |
| **D6** | **SIGNED in part.** **No cap on lists or words for anyone who has paid.** Organizing words is not a second thing to pay for; the photo feature is already gated on its own (`photo_ocr`). If the free tier needs a limit, it counts **total saved words**, never lists. **Still open:** whether the free tier is limited at all, and at what number. Until then `FREE_CUSTOM_LISTS_CAP` (declared in lists, enforced nowhere) must not be enforced, and is to be removed or re-pointed at words before this ships. |
| **D7** | **SIGNED.** Players, Spell Jr included, can create lists and add words. They cannot delete a list without the parent gate (D4). Kids keep their own discoveries; parents' lists stay safe. |
| **Scope** | **SIGNED: per device.** There is no profile system (census C4). Lists, the remembered play selection and Recently deleted are per device, like progress and misses already are. The data model keeps room for a profile id, so if profiles ship, the device's lists move into the first profile with nothing lost. Amends §2, F2 behavior 4, F4 behavior 2, and I6. |
| **§8** | **Map to repo equivalents**, as signed for CC-TRANSLATE-SCREEN: `cargo test --lib mywords_` on the `spell_wasm` crate, browser specs in `tests/e2e/specs/*.mjs` through `tests/e2e/run.mjs`, and device checks on the iOS Simulator in place of Maestro. |

**Still open before build:** D6's free-tier limit (if any), and CC-SNAP-LIST v1,
which is not in this repo, so its D8 cannot be marked resolved there.

## Build notes

**Phase 4 (2026-09-17), Eric: "build it the simple way".**

- **The screen is the picker.** F4.1 describes a source picker that selects one
  or more lists; the app has no such control — choosing My Words is one entry in
  the language dropdown. So playing starts from the My Words screen: *Play this
  list* inside an opened list, *Play all words* on the All words card. The choice
  is remembered per device and falls back to the newest live list when the
  remembered one is deleted, exactly as F4.2 asks.
- **Not built: an arbitrary subset.** One list, or everything. Signed as the
  simple way; a multi-select would need a control this app does not have.
- **Reordering is buttons, not dragging.** D3's order is set with up/down and a
  star that leads; a drag handle needs pointer work the screen does not yet do.
  The rule is the same either way, and the star is also the accessible route.
- **Mixed is untouched play.** A list in Mixed hands its words to the existing
  adaptive draw and its no-repeat deck. Only *In my order* changes the draw, the
  one signed exception to the non-goal on modes.

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
* CC-ONBOARD-JR owns profiles and Jr gating. Lists are scoped per device (signed
  Scope ruling: there is no profile system) and do not change that.
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
     *(Amended by D1: when a list was already created today, the default is Add
     to that list instead.)*
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
   device (Scope ruling); a clash appends " (2)".

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
   chip, plus rename, add words, and delete list. *(Extended by D2: each entry can
   be edited, re-languaged, removed with Undo, or copied to another list. Extended
   by D3: the list's Mixed / In my order switch, drag to reorder, star to front.
   Extended by D4: lists older than a few weeks fold into Older lists.)*
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
2. The selection is remembered per device (Scope ruling). If a remembered list is
   deleted, fall back to the newest list.
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
* I6 Device scope (amended by the Scope ruling). Lists live on the device; the
  model keeps a profile slot so a future profile system can adopt them without
  loss. [unit test]
* I7 Migration is lossless and idempotent. [AT6.1, AT6.2]

## 6. Decisions

PROPOSED (Eric signs or edits): *all signed 2026-09-17 — see the table at the top.*

* D1 The default destination is a new dated list.
* D2 A word may exist in several lists. Duplicates are removed only within a list,
  and in All words or play.
* D3 The newest list is preselected for play.
* D4 Soft delete with 8 s undo and a 30-day Recently deleted.

Open (do not guess): *answered 2026-09-17 — see the table at the top; D6's free-tier number remains open.*

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
* No changes to the ingest path, the modes, scoring, or spaced repetition — except
  D3's "In my order", the one signed change to how a mode draws words.
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
