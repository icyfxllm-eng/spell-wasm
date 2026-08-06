# CC-BANK-COMPLETE — Bank Completeness: "Every Word That Matters"
[stashed verbatim from Eric 2026-08-03 — full text as pasted in chat]
Status: REVIEW-GATED. Features 1-2 executable on file greenlight; Features 3-7 blocked until D-FLOOR + D1-D5 signed. Hard dependency: the sampled-audit amendment (separate signature) blocks UNMUNCH/GENERATE waves regardless. Sequencing per Eric: "when the previous stuff is done" — after the BD ladder (post-126).
Authority chain (files NOT yet in hand): CC-BANK-EXPANSION, CC-WORDLIST-SOURCES, CC-RAFU-SOURCES, CC-SWAHILI-WORDBANK, CC-DEF-PRECHECK, CC-DEFS-DARK, CC-DEF-DICTMATCH, CC-HINDI-PHASE0.

---
DECISIONS SIGNED (Eric, 2026-08-03): "D-FLOOR as proposed, D1-D5 as proposed."
* D-FLOOR: all 15 rows as tabled (en frozen-reference; hi design-ahead; sw 3-tier structural).
* D1 form-level ranking / D2 hash-pin law / D3 empty allowlist / D4 rank-decides compounds / D5 en-frozen+hi-design-only — all as proposed.
* Features 3-7 signature-unblocked. STILL BLOCKED: Features 1-2 build on CC-BANK-EXPANSION + CC-WORDLIST-SOURCES being in hand (no-inference law); the sampled-audit amendment remains UNSIGNED and keeps UNMUNCH/GENERATE waves blocked regardless.

## GATES 4 + 5 BUILT 2026-08-05
The U-engine's pending pair are now real code. Gates 4 (TTS) and 5
(DEF-PRECHECK) judge against an explicit `Evidence` input — audited
word lists supplied by the caller — so the engine keeps its F3 purity
(no clock, no network, no RNG) while the gates become testable law.
`Evidence::pending()` reproduces the shipped behaviour exactly: with no
snapshot loaded every gate-1-3 survivor stays PROVISIONAL. With
evidence loaded a word reaches the new `Verdict::Full` only when it has
BOTH audio and a clean definition, and an exclusion names the FIRST law
it broke (gate order is asserted). UNMUNCH/GENERATE waves stay blocked
on the unsigned sampled-audit amendment — untouched by this.

## SAMPLED-AUDIT MACHINERY BUILT 2026-08-05 (rule still Eric's)
Eric opened the UNMUNCH/GENERATE lane; the amendment itself is still
unsigned, so the machinery ships with the RULE AS A PARAMETER — the
same discipline as gates 4/5. `SampleRule` has NO Default and carries a
`signed` marker; `audit_sample` returns nothing and `audit_verdict`
returns `Unsigned` unless a signed rule is loaded, so an unaudited
expansion cannot reach a bank by forgetting to configure one. The draw
is a seeded stride over the alphabetized population: reproducible from
the record (an auditor can regenerate exactly what was read) and spread
across the whole list rather than the easy head. Over-limit defects
reject the WHOLE batch, not the defective words — a bad sample means a
bad expansion.
WHAT ERIC STILL OWES for this lane to run: the amendment's numbers —
sample size, max defects, and whether the rule varies by source class
(hunspell unmunch vs morphological generator). Say them and the lane
runs; the code is waiting.
