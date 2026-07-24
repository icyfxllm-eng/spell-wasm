# Spell Racing preset avatars (CC-SPELL-RACING D5)

**Approved D5 identity set.** Two families — 8 stars + 8 wands = **16 avatars**,
paired with **6 preset colours** → 96 identities. Identity is
`{ avatar_id: 0..15, color_id: 0..5 }` only: no names, no free text — COPPA-inert
by construction (spec Feature 1 invariant).

- **Generated**, do not hand-edit: `node scripts/gen-racing-avatars.mjs`.
- Each SVG is **colour-agnostic**: the identity shape uses `fill="currentColor"`, so
  the app applies the chosen `color_id` via CSS `color`. Fixed accents (dark eye
  marks, gold crown, white shine, grey wand stick) are not the identity colour.
- `manifest.json` is the ordered `avatar_id` → file map plus the `color_id` palette.

**Gating:** these are the resolved D5 art. The Spell Racing mode that consumes them
is **REVIEW-GATED** (`docs/CC-SPELL-RACING-PLAN.md`) and not built — locking the
avatars clears gate **G-A**, nothing more. No mode code references these yet.
