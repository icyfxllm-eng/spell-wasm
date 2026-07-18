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

## Conflict 1 — `src/consts.rs`: take the RTL registry (privacy trap now GONE)

**Update (`66ea1f2`): the security trap this section warned about is resolved.**
build-54's `def_lang` deletion (the privacy fix — the browser called
`dictionaryapi.dev` directly, sending a child's word + IP to a third party) has
been ported onto `feature/rtl-feedback`, so **both branches now delete `def_lang`**
with the same replacement comment. The merge can no longer reintroduce the leak.

What remains is an ordinary conflict: the two branches carry **different
`BUILTIN_LANGS`** — build-54's older lineup vs `feature/rtl-feedback`'s 15-language
registry with `Direction`/RTL. This is not a two-sided change to reconcile; the
RTL branch's registry is the newer, authoritative one.

**Resolution: take `feature/rtl-feedback`'s `consts.rs` for the registry region.**
`def_lang` is absent on both sides, so it stays gone either way. Confirm with
`grep -rn def_lang src/` returning only the explanatory comments, never code.
`src/game.rs` also no longer conflicts on this — both sides dropped the caller.

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
