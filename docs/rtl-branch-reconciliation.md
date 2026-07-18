# Reconciliation plan — `build-54` ⇄ `feature/rtl-feedback`

**Status: NOT DONE — this is a recipe, not a merge.** Merging is a pipeline-freeze
operation (no merges without Eric's approval), and the two branches have not
converged. This documents exactly what the merge will take so it is a known,
low-risk quantity the moment the freeze lifts. Assessed with `git merge-tree`
(pure computation — no working tree touched).

## The shape of it

- Merge base: `868fbd9`. `build-54` is +15 commits, `feature/rtl-feedback` +27.
- **Everything auto-merges cleanly except two files:** `src/consts.rs` and
  `tests/e2e/specs/menu.mjs`. `src/game.rs`, `index.html`, `privacy.html`, etc.
  merge with no conflict.
- The cross-branch doc references resolve on merge with no work: the RTL docs
  (`docs/rtl-keyboard-split.md`, `spike/urdu-nastaliq/`) live on
  `feature/rtl-feedback`; `docs/DECISIONS-PENDING.md` (with §5/§5.1/§5.2 pointing
  at them) lives on `build-54`. After the merge both paths coexist and the
  references are valid.

## ⚠ Conflict 1 — `src/consts.rs`: keep BOTH changes (one is a security fix)

The two sides edited adjacent lines, which is why git can't auto-merge, but the
changes are about **different things and both are wanted**:

- **`build-54` deleted `def_lang`** — a privacy fix (DECISIONS-PENDING §10). That
  function mapped languages to `dictionaryapi.dev`, which the browser called
  *directly*, sending a child's word + IP to a third party (the service is
  English-only, so every non-English call 404'd for nothing).
- **`feature/rtl-feedback` rewrote `BUILTIN_LANGS`** — the 15-language registry
  with `Direction`/RTL, right below where `def_lang` was.

**Resolution: take `feature/rtl-feedback`'s registry AND keep `def_lang` DELETED.**
The two do not actually conflict in meaning; they are just neighbours.

**The trap to avoid:** `feature/rtl-feedback` still HAS `def_lang` (and still calls
it at `game.rs:660`) — it never got the privacy fix. A careless resolution that
favours the RTL side wholesale would REINTRODUCE the leak. Do not.

Good news, verified: `src/game.rs` auto-merges to **drop the `def_lang` caller**
(build-54's deletion there doesn't overlap the RTL edits), so the merged
`game.rs` has no `def_lang` reference. That means the only hand-work is
`consts.rs`: delete `def_lang`, keep `BUILTIN_LANGS`. After that the tree compiles
(no dangling call). Confirm with `grep -rn def_lang src/` returning nothing.

## Conflict 2 — `tests/e2e/specs/menu.mjs`: a judgment call, both sides valid

Both branches independently reworked the endonym test after Thai was cut (the old
hardcoded `th: 'ไทย'` had rotted the suite), but took **different approaches** —
both green on their own branch:

- **`build-54`:** an invariant ("every option renders a real name, never a bare
  code") plus spot-checks against an `ENDONYMS` map, skipping codes not present.
- **`feature/rtl-feedback`:** parses `BUILTIN_LANGS` from `consts.rs` (drift-proof:
  a cut language can't rot it) and intersects with a hardcoded `KNOWN_ENDONYMS`
  table (independent correctness check), with a guard so the intersection can't
  silently empty.

Neither is wrong. **Recommendation: keep the `feature/rtl-feedback` version** — it
is the stricter of the two (independent endonym table + registry-derived
membership + an anti-tautology guard, all negative-controlled), and it subsumes
build-54's "no bare code" intent. Whoever merges should confirm this rather than
auto-resolve; it is the one real choice in this merge.

## Recipe (when the freeze lifts)

1. On the target branch, `git merge <other>`.
2. `consts.rs`: keep the RTL `BUILTIN_LANGS`; ensure `def_lang` stays gone.
3. `menu.mjs`: take the `feature/rtl-feedback` version (or deliberately blend).
4. Verify: `grep -rn def_lang src/` is empty; `cargo test --lib`; the e2e suite;
   the four gates (css-logical, font-selfhost, i18n-parity, modes).
5. Fold `docs/rtl-branch-reconciliation.md` (this file) away once merged.

Only two files, one of them a mechanical "keep both", the other a one-line choice.
The risk in this merge is entirely the §Conflict-1 trap — do not let the privacy
fix get reverted.
