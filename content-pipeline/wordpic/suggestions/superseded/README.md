# Superseded suggestions (CC-BUILD219-FIXES F3, 2026-09-03)

Eric's earlier approvals, kept intact and never deleted (the rule F4 sets for
archived assets). Moved aside rather than edited because an approval record is
a human artefact — I7.2 says no agent writes one, and quietly reversing one is
the same offence backwards.

## Why daruma was superseded

`build_scan_library.py` prefers approved suggestions over tracing the
reference: if `_approved_candidates(sub)` returns anything, the reference image
is never traced. Daruma took that side door, so its outline was never derived
from `daruma.png` at all — which is the "open runs" Eric photographed in
build 218/219.

Measured with the v7.5 three-gate eval, goldens as controls (dog 1.000/1.000,
fish 1.000/1.000, eiffel 0.967/1.000 — the apparatus is sound before any
conclusion is drawn about an asset):

| subject | source | paths | ink recall | path precision | continuity |
|---|---|---|---|---|---|
| daruma | approved suggestions | 8 | 0.343 | 0.274 | 1.08% |
| daruma | traced from `daruma.png` | 8 | **1.000** | **1.000** | **1.08%** |

Same eight strokes, same word capacity — now on the ink instead of near it.

The geometry is NOT approved by this change. F3's acceptance is Eric's
on-device re-grade, and I7.2 reserves the approval record to him.

## Why dallah IS here

Same defect: two approved strokes for an entire coffee pot, recall 0.135,
continuity 57.95%. Traced from `dallah.png` it becomes 7 paths at 1.000/1.000
with continuity 0.07%.

| subject | source | paths | ink recall | path precision | continuity |
|---|---|---|---|---|---|
| dallah | approved suggestions | 2 | 0.135 | 0.123 | 57.95% |
| dallah | traced from `dallah.png` | 7 | **1.000** | **1.000** | **0.07%** |

The first retrace REFUSED TO PLAN and would have shipped a blank card. Two of
its word paths ran 12.985 apart against the packer's 13.0 corridor floor —
found by putting the geometry through `scanlock-render`, which names the
conflicting pair, rather than by reading `spellpic::plan`'s `None` (the test
reports every packer error as an F5 coverage failure, which it is not).

The tight-run split is supposed to catch this, but it smooths runs — a run only
flips when three consecutive points agree — so a brief pinch slips through.
`build_scan_library` now demotes the shorter path of any conflicting pair to
`decorative_thin`: it keeps the ink, so the drawing and its continuity survive,
and takes it out of the packer's corridor set. Measured as a no-op for the
existing bank: 0 of 1060 shipped word paths sit under the floor.
