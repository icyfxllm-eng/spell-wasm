# CC-MASTER-PARITY — Cut executed, Arabic rendering, full banks for ru/ar/sw

**Status: REVIEW-GATED.** This is an **orchestrator**: it sequences and gates the
subordinate files below; it does not replace them. Where this file and a subordinate
file conflict, **STOP AND ASK** — never infer. Subordinate files remain individually
review-gated on their own terms.

**Subordinates:** CC-LINEUP-TRIM, CC-RTL (Arabic rescope), CC-NEW-LANG-CONTENT,
CC-WORDLIST-RAFU, CC-RAFU-SOURCES, CC-SWAHILI-WORDBANK, CC-WORDLIST-SOURCES
(license gate), CC-DEF-PRECHECK, CC-DEV-PREVIEW.

## Intent

Bring Russian, Arabic, and Swahili to full parity with the eleven ready languages —
playable in all modes, large audited word banks, definitions, TTS — after removing
Urdu and Persian. "Parity" is defined checkably in the Parity Bar below; a language is
done when its column is all green, not when it feels done. Hindi is explicitly OUT of
this file's scope (its own Phase 0 track governs it). The point of a master file: one
place that knows the order, the gates, and what "up to snuff" means, so no track
silently waits on another or jumps a gate.

## Gates (blocking, in order encountered)

- **G1 — Sampled-audit amendment sign-off (ERIC, on paper, first).** The
  generated/derived bank machinery (Swahili's morphological generator, any UNMUNCH
  expansion) is blocked until the proposed amendment — verdicts attach to lemma base +
  mechanism rules + a 500-form (or 5%) sample, >2% reject fails the whole set — is
  explicitly signed. No generator runs before G1. If unsigned, Track S halts at
  lemma-base collection and says so rather than proceeding.
- **G2 — CC-RTL Phase 0 findings approved (ERIC).** No RTL feature work merges before
  it (per CC-RTL D4).
- **G3 — License gate (CI, standing).** Every word-list source clears
  CC-WORDLIST-SOURCES license checks before ingestion. No exceptions in any track.
- **G4 — Audit-before-activation (standing, non-negotiable).** No language activates
  for users without its Gig A + Gig B verdicts, regardless of how finished it looks.
  `rtlSupported` makes Arabic eligible, never live.

## Phase A — Execute the cut (first, alone)

Run CC-LINEUP-TRIM to completion: fa/ur removed with archived treatment, country map
updated, RAFU descoped, F0 verification report produced, full suite green with zero
weakened tests. **Phase B does not start until the F0 report exists** — building on a
registry that still contains ghost languages is how orphaned work items happen.

## Phase B — Three parallel tracks

Tracks share nothing but the gates; run them concurrently.

### Track R — Russian (no blockers, shortest path)

- **R1.** RAFU-SOURCES ingestion to the `ru` pool floor (1500), gap-report-first, G3
  enforced per source. OpenCorpora unmunch permitted per the expansion profile (G1
  applies to the unmunched portion; the curated base can proceed regardless).
- **R2.** Trap-class tiering per CC-NEW-LANG-CONTENT (soft signs, vowel reduction) with
  per-tier quotas; ё policy as decided (accept е, display ё).
- **R3.** Definitions authored per current policy (LLM drafting stays rejected),
  CC-DEF-PRECHECK pass, flags resolved before Gig A export.
- **R4.** TTS synthesis gate on every merged word; staging empties into the
  source-of-record.
- **R5.** Fully playable in all modes under `DEV_PREVIEW`; dev-preview-status row green
  except the audit column.

### Track S — Swahili (gated on G1 for generation)

- **S1.** TTS variant check decides sw-TZ vs sw-KE by voice quality (carried over from
  the trigger file; the trigger condition itself is reversed — Swahili is
  unconditional).
- **S2.** Licensed lemma base assembled (Hunspell sw_TZ + Wiktionary + Leipzig; TUKI
  reference-only), G3 per source. This may proceed before G1.
- **S3. AFTER G1:** morphological generator runs — capped 60 forms/lemma, curated
  TAM/class slots, provenance metadata on every form; 500-form sample drawn for the
  audit sheet per the amendment.
- **S4.** Tiering on the phonemic axes, easy/medium/hard only (no expert tier, as
  decided).
- **S5.** Definitions attach at the lemma level (Gig A stays O(lemmas), forms inherit);
  CC-DEF-PRECHECK at TIER-C look-here-first confidence.
- **S6.** TTS gate, staging empty, all modes under `DEV_PREVIEW`.

### Track A — Arabic (two independent halves)

**Content half (starts immediately — needs no rendering):**

- **A1.** RAFU-SOURCES ingestion to the `ar` pool floor (1000): Wiktextract backbone,
  lemma-only collection per COLLECT profile, Lane for classical backfill with the
  flag-and-modernize register policy, G3 per source.
- **A2.** Trap-class tiering (hamza seats per CC-NEW-LANG-CONTENT), unvocalized-spelling
  CI check on every merged word.
- **A3.** Definitions + CC-DEF-PRECHECK; TTS gate.

**Rendering half (CC-RTL, sequenced by G2):**

- **A4.** Phase 0 prototype → findings report → **STOP AND ASK** (G2).
- **A5.** F1–F7 per CC-RTL through its done-criteria; `rtlSupported` flips in the
  checklist-linked commit.
- **A6. Join point:** full Arabic playthrough under `DEV_PREVIEW` requires BOTH halves;
  neither half waits for the other before its own work.

## Phase C — Audits and activation

- **C1.** Wave 1 Gig A + Gig B: eleven ready languages + Russian + Swahili, per the
  standing two-auditor model (decoy rows, locked-sheet ingest, checklist-gated
  compensation).
- **C2.** Arabic Gig A (definitions sheet — no app needed) may run with wave 1 once
  A1–A3 are merged. Arabic Gig B runs only after `rtlSupported` flips (wave 2).
  Auditing broken rendering is a paid bug report; do not schedule Gig B early.
- **C3.** Verdicts ingest → activation per language per G4. Season/entitlement behavior
  on activation follows existing files; nothing here changes monetization.

## Parity Bar — "up to snuff" defined

A language reaches parity when ALL of the following are true, verified by the
dev-preview-status report plus CI, and only then leaves this file's scope:

| Criterion | ru | ar | sw |
| --- | :--: | :--: | :--: |
| Pool floor met (1500 / 1000 / generator set) | ☐ | ☐ | ☐ |
| Trap-class or phonemic tiering complete, quotas | ☐ | ☐ | ☐ |
| Definitions authored, pre-checked, flags closed | ☐ | ☐ | ☐ |
| TTS gate passed for every word | ☐ | ☐ | ☐ |
| Profanity filter seeded for the language | ☐ | ☐ | ☐ |
| All modes load and complete a round (Playwright) | ☐ | ☐ | ☐ |
| Rendering correct (ar only: CC-RTL done-list) | n/a | ☐ | n/a |
| Gig A verdicts clear | ☐ | ☐ | ☐ |
| Gig B verdicts clear | ☐ | ☐ | ☐ |
| Activated | ☐ | ☐ | ☐ |

## Decisions

- **D1:** Hindi is out of scope here. Its Phase 0 and content un-freeze are governed by
  their own files; this master file grants them nothing.
- **D2:** Track order inside phases is fixed as written; tracks across Phase B are
  concurrent. Do not serialize B to "reduce risk" — the tracks are independent by
  construction.
- **D3:** G1 sign-off is a distinct artifact (a signed line in the amendment file), not
  inferred from this file's approval. **Approving CC-MASTER-PARITY does NOT sign the
  amendment.**
- **D4:** If any subordinate file's constraint blocks a step here, the subordinate wins
  and the conflict is reported — this file has no override authority.

## Constraints and non-goals

- No scoring, entitlement-resolver, monetization, or edition changes.
- No new languages, no lineup changes beyond CC-LINEUP-TRIM.
- No LLM-authored definitions (standing rejection).
- No audit checklist modifications for the ready eleven.
- `DEV_PREVIEW` remains dev-only per its own file; nothing here widens it.

## Done = these pass

1. **Phase A:** F0 verification report exists; suite green, zero weakened tests.
2. **G1 artifact exists** before any generator output enters staging (CI guard:
   generator refuses to run without the signed-amendment marker).
3. **Parity Bar:** every cell green for `ru` and `sw` at wave-1 close; every cell green
   for `ar` at wave-2 close.
4. **dev-preview-status** shows `ru`/`ar`/`sw` rows matching the Parity Bar state at all
   times (spot-checked).
5. **Three activation commits**, each linking its language's completed Parity Bar
   column.
