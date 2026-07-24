# Ghost schema v1 (CC-SPELL-RACING, D4)

The on-disk / on-wire format for a Spell Racing ghost. Committed alongside the code
(`src/racing/format.rs`) per D4. **Versioned from v1**; readers reject a newer major
version rather than misrace.

A ghost is *just data* — which words, and the timing of how they were spelled. That
is the whole insight behind the mode: no authored content, works in every language,
and (because it carries no free text) needs no moderation and is COPPA-inert.

## Fields

`RaceGhost` (JSON; serde field order is fixed, so encoding is byte-deterministic):

| JSON key | Field | Type | Meaning |
|---|---|---|---|
| `v` | `schema_version` | u32 | Format version. Current = **1**. |
| `language` | `language` | string* | Enumerated language code (e.g. `"en"`). Validated on load. |
| `tier` | `tier` | string* | Enumerated tier: `easy`/`medium`/`hard`/`expert`. Validated. |
| `h` | `word_list_hash` | u64 | `wordid::list_hash(language, tier)` at record time. |
| `id` | `identity` | object | Preset identity — see below. |
| `e` | `events` | array | Ordered `WordEvent`s. This ordered list **is the track**. |

`Identity`:

| key | type | range | meaning |
|---|---|---|---|
| `avatar_id` | u8 | 0..16 | index into `assets/racing/avatars/manifest.json` |
| `color_id` | u8 | 0..6 | index into the manifest colour palette |

`WordEvent` (one per raced word):

| JSON key | field | type | meaning |
|---|---|---|---|
| `w` | `word_id` | u64 | stable content ID (`wordid::word_id`) |
| `s` | `start_ms` | u32 | ms from run start when the word began |
| `f` | `finish_ms` | u32 | ms from run start when it was answered |
| `k` | `keystrokes_ms` | u32[] | elapsed-ms of each keystroke |
| `c` | `correct` | bool | spelled correctly |

\* **The only two string fields**, and both are *enumerated identifiers*, not free
text. This is an invariant, not a convention — the test
`no_free_text_only_enumerated_identifiers` fails if any other string field is added.
No names, no messages, no custom strings ever enter a ghost (spec F1).

## Versioning & compatibility (spec F1)

On `load(json)`:

1. **Parse fails** → `Malformed`.
2. **`schema_version > 1`** → `NewerVersion` → *"This ghost was made in a newer
   version of Spell."* Never crash, never guess.
3. **Identity out of preset range** → `BadIdentity`.
4. **Language not registered/playable here** → `UnknownLanguage`.
5. **Tier unknown** → `UnknownTier`.
6. **Resolve every `word_id`** via `wordid::resolve(language, tier, …)`:
   - all resolve → the ghost is playable; `list_matches` reports whether
     `word_list_hash` equals the local hash (exact reproduction vs recovered-by-ID).
   - **any `word_id` unresolvable** → `UnresolvableWord` → abort. A word is **never**
     silently substituted.

The stored `word_id`s are what make the list-changed case recoverable: after an audit
edits a word list the hash won't match, but every word that still exists resolves by
ID; only genuinely-removed words abort the race. Error strings are wired to the
locales in Phase 5 (`LoadError::message_key`).

## Determinism

serde preserves declaration order, so `encode → decode → encode` is byte-identical
(`encode_decode_is_byte_stable`), and two logically-equal ghosts encode identically.
Combined with content-derived word IDs (Phase 0) and seeded tracks (Phase 2), this is
the basis for acceptance test #1 (same ghost + same list ⇒ identical race).

## Not in v1

No track seed field (the `events` word-ID sequence *is* the track for a recording;
seeds matter for generating *new* tracks and pace ghosts — Phase 2/4). No sharing
envelope (compression/QR — Phase 6 wraps this format, it does not change it). Adding
either is a v1-compatible additive change or a version bump, per this doc.
