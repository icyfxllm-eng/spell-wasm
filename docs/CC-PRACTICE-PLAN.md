# CC-PRACTICE — guided first-contact spelling (20 + 5 per language)

**IMPLEMENTATION STATUS (2026-07-27): DATA LAYER LANDED; engine + UI next.**

| Piece | State |
|---|---|
| Trap classes ×15 languages | ✅ drafted (in `scripts/build-practice-curricula.py`; ru/ar align with `config/trap-registry.json`) |
| Curricula 20+5 + intro cards | ✅ `config/practice/<lang>.json` ×15 — all words from the language's own bank, intros ≤90 chars, one per trap (D4 agent-draft path, pending audit round) |
| Schema gate (I3/I6) | ✅ `node scripts/practice-check.mjs` — 20+5 exact, bank membership NFC-compared, one intro per trap, cap, ordered firstWord |
| Core module (phases D2, failure-proof D3, resume, I2 store isolation) | ⏳ next |
| Screen (path-of-20, intro cards, ceremony, graduation, what's-next, share card D8) | ⏳ next |
| Hub first-position entry (D9), availability gate (D7) | ⏳ next |
| Playwright runs, store-diff test, coverage report, screenshots | ⏳ with engine/UI |

**Survey findings / deviations to review:**
- **Script Paths (D1) does not exist in this repo** — no CC-SCRIPT-PATHS code,
  docs, or content. No collision is possible; the graduation "what's next"
  screen simply omits that door until Script Paths exists.
- **Trap registry** covered only ru/ar; the other 13 languages' trap classes
  are drafted alongside the curricula (registry-extension material, data-only).
- **D7 audit gate:** no audit rounds have run for anything in this repo.
  Following the interim posture used everywhere else (Eric's standing rulings),
  the plan is: ship English visible per the explicit exemption, and hold the
  other 14 behind the registry flag until Eric rules on the interim-content
  question for Practice strings (ask attached to the engine/UI checkpoint).
- Draft-quality note: word picks are heuristic (e.g., es drew "ha" — weak);
  the curricula are explicitly REVIEW MATERIAL per D4.

*(Original spec provided by Eric 2026-07-27; decisions D1–D9, features 1–6,
invariants I1–I6, and the behavioral eval note govern as written.)*
