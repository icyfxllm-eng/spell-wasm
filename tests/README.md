# SpellGame test harness

Automated coverage that replaces most of the manual test plan. Four parts; Part 1
runs today in CI, Parts 2–4 need external tooling (Maestro/simulators, whisper.cpp,
an Anthropic key) and run on Eric's machine or manual CI jobs.

## Part 1 — Web E2E (runnable now) ✅

```bash
npm run e2e        # builds dist-test/ (with the seam) + runs the suite
```

- **Test seam** (`window.__spelltest`): compiled ONLY under `--features testseam`
  (`scripts/build-web-test.sh`). Observation-only — exposes the current word/tier/
  lang and the pool for a (lang,tier); it never types, sets an answer, or bypasses
  the profanity/answer checks. `scripts/seam-absence-check.mjs` greps the
  production `dist/` (built by `npm run build`) and fails if any seam trace ships
  — wired into `npm run build`.
- **Specs** (`tests/e2e/specs/`): keyboard (layout coverage, hit area at SE,
  rapid-fire no-drop, ko/ja composition), gameplay (read expected word from the
  seam → type on the real on-screen keyboard → assert accept/reject), modes
  (Daily start + determinism, H2H quit-mid-game clean state, age-gate cold), menu
  (endonyms + UI switches per language). Typing is real key **clicks** on the
  anti-dictation keyboard — never `input.fill`.
- Output: `tests/e2e/TEST-REPORT.md` (pass/fail by area).
- **Currently 34/34 passing.**

**Gaps flagged for Eric (Hard Review Gate 1):** the leaderboard/name-server specs
need a staging backend. This suite stubs `/api/*` at the network layer and does
NOT exercise the real Climb server. A staging `docker-compose` backend fixture is
the right long-term fix — decide before relying on server-integration coverage.

## Part 2 — iOS simulator UI (`tests/ios-ui/`) — needs Maestro + Xcode

Maestro flows (`flows/*.yaml`) for launch/age-gate/safe-area/Big-Text/zoom-symptom
/backgrounding, plus XCUITest where Maestro can't reach (native picker, WKWebView
`zoomScale`). Wire as fastlane `lane :ui_tests` (simulator matrix) and
`lane :device_tests` (Eric's M4 + a plugged-in iPhone). The zoom flow is the
**symptom** check; `scripts/viewport-check.mjs` is the **cause** check.
BrowserStack/AWS Device Farm noted as a future option — not integrated.

## Part 3 — TTS audio loopback (`tools/audio-verify/`) — needs whisper.cpp

`verify.py` runs every cached clip through whisper.cpp (offline, no API cost) and
compares to the expected word with per-language comparators (Latin casefold;
zh→hanzi via lexicon `display`; ja→kana/kanji via readings; ko/th NFC). MISMATCH
words are held out of pools (wire into the assignment generator). **Hard Review
Gate 3:** confirm Whisper's Filipino quality on a sample before trusting fil.

## Part 4 — LLM pre-screen (`tools/llm-prescreen/`) — needs ANTHROPIC_API_KEY

`prescreen.py` batches lexicon entries through the API (temp 0, committed prompt
versions, cached by input-hash) to pre-shrink the human review packet. Cost guard
confirms above $5. **Hard rule:** output is ADVISORY — nothing auto-writes to any
filter/word list; missing-term suggestions route to Eric's custom-list intake
only (Hard Review Gate 2).

## What stays human (do NOT automate)

1. **Native-speaker final review** of blocklists + Kid Mode lists (Part 4 pre-shrinks it).
2. **Feel-testing** — fun, clarity, difficulty perception (TestFlight friends/family group).
3. **One physical-device pass** by Eric before each App Store submission.
4. **Kid-facing content sign-off** — always Eric.

## Baseline-update workflow (visual specs)

Visual/screenshot baselines are regenerated with `scripts/screenshots.mjs`
(per-locale). Review the diff before committing new baselines; never overwrite
blind.
