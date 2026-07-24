# Spell Racing — word-ID foundation (G-B design)

**Status: DESIGN FOR APPROVAL (REVIEW-GATED).** No code lands until this is signed
off. This resolves gate G-B of `CC-SPELL-RACING-PLAN.md` — the layer that lets a
ghost reference words by stable ID + list hash instead of raw strings (spec F1).

## The requirement (spec F1)

A ghost references words by `(languageCode, wordListHash, wordID)`, **never by raw
string**. On replay/import: matching list hash ⇒ exact reproduction; mismatch ⇒ fall
back to per-ID resolution; any unresolvable word ⇒ **abort with a clear message,
never substitute silently**. Everything must be **deterministic across devices**
(acceptance tests 1 & 2).

## What exists today

`src/word_data.rs` holds `pub const XX_TIER: &[&str]` arrays — plain strings, already
**NFC-normalized** and emitted in a byte-stable, deterministic order by
`scripts/build-wordlists.py`. `tier_for(lang, tier)` returns `&'static [&'static
str]`. There are **no IDs and no hash**. The design below adds them without touching
a single word.

---

## Decision 1 — IDs are content-derived, not positional

| Option | Stable across list edits? | Verdict |
|---|---|---|
| **Array index** (position in the tier) | ❌ inserting/removing any word renumbers everything | reject |
| **Explicit ID column** in the source lists | ✅ but hand-maintained; risks reuse/typos across 14 languages | reject |
| **Content-derived** = hash of the word itself | ✅ by construction — a word's ID never changes regardless of what else is in the list | **choose** |

A content-derived ID is a pure function of the word, so:
- It's stable across audits that add/remove *other* words.
- Nothing new needs to be stored per word — the ID is *computed*, so the word arrays
  are untouched (zero-risk, zero-diff to gameplay data).
- Two devices with the same word compute the same ID with no coordination.

**`word_id(word) = FNV-1a-64( NFC(word) as UTF-8 )`** — a fixed, seedless hash so it
is byte-identical in the Python build and the Rust/WASM runtime (Rust's default
`Hash` is seeded and MUST NOT be used here). 64-bit because at ~10k words/language a
32-bit hash has ~1% chance of an *intra-language* collision — too high to treat the
build gate as "rare." At 64-bit, collision probability is ~1e-12; the build gate is a
true backstop, not an expected event.

FNV-1a-64 (exact, so both implementations match): `h = 0xcbf29ce484222325`; for each
byte `b`: `h = (h XOR b) * 0x100000001b3  (mod 2^64)`.

## Decision 2 — `wordListHash` per (language, tier)

Tracks are drawn from one `(lang, tier)` (spec F6), so the hash is scoped there:

**`list_hash(lang, tier) = FNV-1a-64` over the tier's word IDs concatenated as
8-byte big-endian, in array order.** Captures both the *set* and the *order* (seeded
track generation depends on order, and the arrays are already order-deterministic).
Same build on two devices ⇒ identical arrays ⇒ identical `list_hash`.

## Decision 3 — the resolution API (Rust/WASM core, per F1)

New module `src/wordid.rs`:

```rust
pub fn word_id(word: &str) -> u64;               // FNV-1a-64 of NFC(word)
pub fn list_hash(lang: &str, tier: &str) -> u64; // over the tier's ordered IDs
pub fn resolve(lang: &str, tier: &str, id: u64) -> Option<&'static str>;
```

`resolve` builds an `id -> &'static str` map for that `(lang, tier)` once (cached in
a `thread_local` / `OnceCell`), then O(1). Returns `None` for an ID not in that list
— the caller (race loader) turns `None` into the abort path, never a substitution.

## Decision 4 — build-time changes (`scripts/build-wordlists.py`)

Two additions, both gates/pins — **no change to the emitted word arrays**:

1. **Collision gate.** Compute `word_id` for every word; if two distinct words in one
   language collide, **fail the build**. (Never expected at 64-bit; it's the invariant
   the determinism tests rely on.)
2. **Golden hash consts.** Emit `pub const XX_TIER_HASH: u64 = 0x…;` per tier. A Rust
   test asserts `list_hash(lang,tier) == XX_TIER_HASH`, which cross-checks that the
   Python build and the Rust runtime compute the identical hash (drift would break
   determinism silently otherwise). The Python FNV and the Rust FNV are the same spec.

## Compatibility behavior (spec F1) — where each message fires

Ghost header carries `schemaVersion`, `languageCode`, `wordListHash`, track seed,
length. On replay/import:

- **Unknown major `schemaVersion`** → *"This ghost was made in a newer version of
  Spell."* Never crash.
- **`wordListHash` matches** local `(lang,tier)` → the seed regenerates the identical
  track; race exactly.
- **`wordListHash` mismatches** (an audit changed the list) → fall back to resolving
  each stored `wordID` via `resolve`. All resolve → race proceeds on the recovered
  words. **Any `None` → abort:** *"This ghost references words that aren't in your
  word list."* Never substitute.
- **Language not installed** → the language-availability error (import validation).

Stored word IDs are the belt-and-suspenders that make the mismatch case recoverable;
without them a changed list would be unraceable. This is exactly why F1 stores IDs
even though the seed alone reproduces a matching-hash track.

## What this does NOT change

- Word arrays: byte-identical. No gameplay, `tier_for`, or selection change.
- No new authored content. No new dependency. No network. No free text (IDs are
  integers; identity is the preset avatar/color from G-A).
- The Climb, shields, Daily: untouched.

## Validated on the real word data

Ran FNV-1a-64 over every current production word (`src/word_data.rs`):
**14,368 words across 12 languages, zero ID collisions** (ru/sw at 3,200 each
included). So the collision gate passes on today's banks with room to spare, and a
sample `RU_MEDIUM_HASH` computes as `0x60c1feb4ee3f56a2`. The design works on real
data, not just in theory.

## How it satisfies the acceptance tests

- **#1 / #2 (determinism):** content IDs + ordered list hash + seeded tracks ⇒ the
  same inputs produce byte-identical IDs/hashes/tracks on any device. The golden-hash
  test pins Python≡Rust agreement.
- **#4 (no free text):** IDs and hashes are `u64`; the schema carries no string field
  — enforced by the schema test in Phase 1.
- **#10 (repeat policy):** unaffected here; lives in track generation (Phase 2).

## Open decisions — need your sign-off before Phase 0 code

1. **ID width: 64-bit** (proposed) vs 48/32 for smaller share codes. Recommend 64 —
   share-code size is dominated by keystroke timestamps, not IDs, and F4 compression
   + delta-encoding handle it. *(Your call.)*
2. **Hash: FNV-1a-64** (proposed) vs SHA-256-truncated. Recommend FNV — deterministic,
   trivially identical across Python/Rust, and this is not an adversarial keying
   (import safety comes from schema/resolvability validation, not hash secrecy).
3. **Golden hash consts in `word_data.rs`** (proposed, ~1 line per tier) vs computing
   at runtime only. Recommend emitting them — it's the only thing that catches a
   Python/Rust hash-drift regression.
4. **Source of record = the production bank** (`assets/words` → `word_data.rs`), i.e.
   ghosts only exist for playable languages. Confirm (ar is banked-but-gated, so no ar
   ghosts until it activates — consistent).

Approve these four and Phase 0 is a small, additive change: `src/wordid.rs` + the two
build-script gates + the golden consts + tests. Nothing else moves until it's landed
and soaked.
