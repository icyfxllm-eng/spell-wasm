# CC-MYWORDS-LISTS — Amendment KC-1: keyed create (idempotent Add)

**Status:** DECISIONS SIGNED. D-KC1 to D-KC4 were signed as recommended (Eric, 2026-10-26: "I like the recommendations"). Under D-KC4, KC-O1 is now in scope as its own small change, to land before GUARDIAN-REPORT Phase C. Nothing is built yet.
**Raised by:** CC-GUARDIAN-REPORT v1 §0 C6 (HALT on the idempotency key). Base: origin/main `8a33d1a6`.
**Amends:** §3 Data model, plus one new function in `src/word_lists.rs`. No existing function changes behaviour.
**Carries forward unchanged:** I1–I7, D1 (today's-list default), F2 naming, F5 soft delete and the 30-day purge.

---

## Why

GUARDIAN-REPORT F6/I9 says a second "Add to My Words" writes nothing and opens the list the first one made. The key is `(profile_id, week_start, language)`.

`word_lists::create` (`word_lists.rs:173`) always mints a new `l{n}`, and on a name clash it makes "Week of Oct 19 (2)" (`unique_name`, `:113`). The nearest thing to idempotency is D1's `default_destination` (`:159`). That matches by **name**, which is fine for its job but not usable as a key here:
- the name is localized;
- a parent can rename the list (F2.4);
- the parent's own "Week of Oct 19" list would match by accident.

Adding entries is already idempotent: `add_entries` applies NFC and skips any (text, language) the list holds (I3, I4). So GUARDIAN A17's "no duplicate words" and A19's "NFC form" are already true. The gap is the list itself.

---

## Change

### KC-F1 — `WordList.key`

```rust
/// KC-1: a caller-owned identity for lists that must not be created twice.
/// None for every list a player makes. Never shown, never edited.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub key: Option<String>,
```

- **Format:** `<owner>:<profile>:<scope...>`. GUARDIAN-REPORT uses `guardian:0:2026-10-19:en`; profile 0 is the device until profiles exist (I6).
- **Survives:** renames (F2.4), moving entries and starring.
- **Not shown or editable** anywhere.

### KC-F2 — `create_keyed`

```rust
pub enum Keyed { Existing(String), Created(String) }

/// KC-1: the live list holding `key`, or a new one carrying it.
pub fn create_keyed(lists: &mut Lists, key: &str, name: &str, source: ListSource, now: f64) -> Keyed;
```

- A **live** list with this key returns `Existing(id)` and writes nothing.
- Otherwise it calls `create` as today, with the same `unique_name` behaviour, sets `key` on the new list, and returns `Created(id)`.
- `Existing` is how GUARDIAN shows "Already in My Words" and opens that list.

### KC-F3 — trash and restore

- **A trashed list doesn't count** (`live()` is false). Add after a delete makes a fresh list, which respects what the parent did (F5).
- **Restoring from the trash** can therefore produce two live lists with one key. `create_keyed` returns the **oldest live** one, by `created_at` then id, so the result is still deterministic, and nothing is merged or deleted (I1, I2).
- **The 30-day purge** removes keyed lists exactly like any other.

### KC-F4 — source

- Keyed lists use **`ListSource::Manual`**. KC-1 adds **no `ListSource` variant**, and **no `VERSION` bump.** See the next section for why; it's the main constraint on this amendment.

---

## Ship order

`load()` (`word_lists.rs:503`) returns an **empty** `Lists` when the stored JSON doesn't parse or `v != VERSION`. The next `store()` then overwrites the stored bytes. So on an **older** build:

| Change | Effect on an older build | Verdict |
|---|---|---|
| A new optional field, `key` | serde ignores unknown fields (no `deny_unknown_fields`). The lists load normally. If that build saves, it drops `key` from every list. | **Safe.** The worst case after a downgrade and upgrade is one extra "Week of Oct 19 (2)" on the next Add. |
| A new `ListSource` variant | An unknown variant fails the whole parse, so the build loads an empty store and the next save **wipes every list the player has.** | **Forbidden** |
| `VERSION` 1 → 2 | Same: unknown version, empty store, overwrite. | **Forbidden** |

**A pre-existing hazard, out of scope here:** a parse failure from *any* cause, such as corruption or a future careless change, silently wipes the player's lists on the next save. The learner store has the C10 pattern for exactly this: back up unreadable bytes, and never write over a newer version (`learner.rs` `resolve_load`). My Words has neither. That deserves its own change (KC-O1).

---

## Invariants (additions)

- **KC-I1.** At most one live list is *created* per key. `create_keyed` never mints a list while a live one holds the key.
- **KC-I2.** `create_keyed` is deterministic. The same `Lists` and the same key return the same id.
- **KC-I3.** Lists without a key serialise byte-for-byte as before.
- **KC-I4.** No `ListSource` variant is added and `VERSION` stays 1. A test pins both.

## Acceptance (CI)

1. **Idempotent:** `create_keyed` twice with one key gives `Created(a)`, then `Existing(a)`, with one list and an unchanged `next_id` (GUARDIAN A17).
2. **Rename-proof:** rename the keyed list, and `create_keyed` still returns `Existing` for it.
3. **No name match:** a player's own list named "Week of Oct 19" with no key is ignored. The keyed create makes "Week of Oct 19 (2)" and leaves the player's list untouched.
4. **Trash:** trash the keyed list, and `create_keyed` returns `Created(new)`. Restore the old one, and `create_keyed` returns the older id.
5. **Unkeyed bytes:** a fixture `Lists` with no keys round-trips byte-identically (KC-I3).
6. **Older-build shape:** CR-1-style. Deserialise keyed bytes through the pre-KC-1 struct shape, and it must succeed. Re-serialise, reload, and the lists are intact, with only the key lost.
7. **Pins:** `VERSION == 1`, and the `ListSource` variant set is exactly {Photo, Translate, Manual, Migrated} (KC-I4).
8. **Entries, unchanged:** after two Adds of the same words, the list holds each (NFC text, language) once (GUARDIAN A17, A19), through the existing `add_entries`.

## Decisions (signed as recommended, Eric 2026-10-26)

| # | Decision | Signed |
|---|---|---|
| **D-KC1** | How a list is identified | **An optional `key` field.** Matching by name breaks on localization, renames and the player's own lists. |
| **D-KC2** | Add after the parent trashed the week's list | **Make a fresh list.** The delete is respected, and the trashed one stays restorable. |
| **D-KC3** | The source for keyed lists | **`Manual`.** A new variant would wipe lists on an older build. |
| **D-KC4** | Whether to scope the backup-on-unreadable fix (KC-O1) now | **Yes, as its own small change before GUARDIAN-REPORT Phase C ships writes.** Phase C is the first feature that writes lists from a new path. |

## Open

- **KC-O1.** My Words gets the learner store's C10 treatment: back up unreadable bytes before any write, and never overwrite bytes from a newer `v`. This is separate from KC-1, but it closes the hazard the ship-order table found.

## Non-goals

- No UI change in this amendment. GUARDIAN-REPORT F6 owns the screen.
- No profile system.
- No change to D1's default destination.
- No merging of duplicate keyed lists.
