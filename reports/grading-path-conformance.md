# Grading-path conformance: which paths call the shared accept function

Read-only audit, 2026-11-04. Nothing was patched inside any mode (your rule). "Shared accept function" means `norm::same_answer` / `norm::answer_matches_in` (which now include `accepted::accepts`, the one reader of the variants files) or `boardgame_grade::grade`, which wraps them.

| # | Path | Where it grades | Calls the shared function? |
|---|---|---|---|
| 1 | Randospell | `randospell_screen.rs:1293` -> `boardgame_grade::grade` -> `answer_matches_in` + `variants::accepts` | **Yes** |
| 2 | Spelluzzle | `spelluzzle/duel.rs:333`: `typed == slot.answer` (an exact char-vector compare). No call into `norm`, `variants` or `accepted` anywhere under `src/spelluzzle/`. | **No** |
| 3 | Quick Bee | `bee_screen.rs:376` -> `norm::same_answer` (zh goes through the pinyin matcher) | **Yes** |
| 4 | Word-cross | `wordcross_ui.rs:464` and `:533` -> `norm::answer_matches(typed, want, false)`, the language-blind form: no language, so no variants, no ja key | **No** |
| 5 | Word-search | `wordsearch_ui.rs:508` -> `norm::answer_matches_in` | **Yes** |
| 6 | Practice | `practice.rs:~388`: letter-by-letter prefix compare; only Vietnamese calls `same_answer` | **No** |
| 7 | Server, Climb | `backend/climb.py:249` `submit_chain` checks chain length and timing only. It never sees a spelling. Climb's spelling is graded on the client by the base game (`game.rs`), which does call the shared function. | **N/A: no server spelling check exists** |
| 8 | Server, Daily | There is no Daily route. Daily is graded on the client by the base game. The only server check is `/api/check` (`backend/app.py:847`), `answer.lower() == word.lower()`, used by `game.rs` for English. | **No, but never reached by a variant**: `game.rs:2268` resolves a listed variant locally and skips the server call. |

## Paths that do not conform, and what each one means

- **Spelluzzle (2):** its answers are fixed letter sequences per board slot, not typed words. A variant would have to be a second letter sequence for the same slot. That is a design question, not a one-line call. It may be out of scope for variants.
- **Word-cross (4):** a clear gap. It uses the language-blind matcher, so an English British spelling, a German double spelling and a ja half-width katakana answer are all graded wrong there. The fix is one call, `answer_matches_in(lang, ...)`, but it is inside a mode, so it waits for you.
- **Practice (6):** typing practice with letter-by-letter progress, not a scored verdict. Variants arguably do not apply. Your call.
- **Server Climb (7) and Daily (8):** the conformance test cannot "call the shared function" on a server route that does no spelling check. If the intent is that these two should verify spellings server-side, that is a new feature.
- **Server `/api/check`:** exact lowercase equality. It also knows no apostrophe fold and no accent forms, so it would fail a curly apostrophe or an accented form if the client ever sent one. Today the client folds the apostrophe at the key filter and settles variants locally first, so it is not reached.

## Proposed conformance test (not written, awaiting your word)

A table-driven Rust test that runs every pair in `data/{en,de,ja}/accepted-variants.tsv` through each client path's grading function. It would pass for 1, 3 and 5, and fail for 2, 4 and 6, until you decide to patch or exempt them. 7 and 8 have no function to call, so the test would list them as "no grader" instead of passing or failing them. You asked for all 8 green; as it stands that is not reachable without either a decision about 2, 6, 7 and 8 or patches in modes.
