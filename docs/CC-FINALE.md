# CC-FINALE — Completion Moment, Save & Share for Spell Picture

**Status: GREENLIT by Eric. REVIEW-GATED** — Eric reviews D1–D6 **before code**.
Then EXECUTABLE on the shipping curated Spell Picture mode. Photo-upload pieces
inherit this feature automatically when CC-PHOTO-PICTURE unblocks; nothing here
touches that gating. Parent files: CC-PICTURE-BANK (manifests, tiers, Kid Mode
flags), CC-WORD-PICTURE v6/v8.1/v8.2 (the renderer being celebrated),
CC-PHOTO-PICTURE v2 (privacy posture, inherited). Authority order:
**trust > layout law > lifelikeness > speed.**

## Intent

Right now the last correct word likely rolls straight into the next picture.
That throws away the emotional payoff the whole mode builds toward: the player
just made something — spelled thirty-one words into a turtle — and never gets to
hold it. This file adds the finale: a reveal moment to look at the finished
piece, the ability to save it to Photos, and a share image. The finished render
is also SpellGame's best organic marketing — the thing people screenshot and
send unprompted — so **the export must look better than a screenshot ever
could**.

## Features

**1. The reveal moment.** Let the player hold what they made; never rush them,
never trap them. On final-word success the HUD — progress counter, keyboard,
buttons — fades out; the completed piece animates through its build, strokes
re-drawing in completion order over a few seconds; then it rests fullscreen with
a soft ambient treatment in the existing glow language. **The rest state is
indefinite**: pan/zoom active where available, no countdown, no auto-advance. A
single unobtrusive Continue affordance plus Save and Share actions. Skippable at
any time — a tap during the animation jumps to the rest state, and Continue is
always one tap away. Replay-the-build is available from the rest state; it cost
seconds to make, so let them watch it again.

**2. Clean export renderer (shared by Save and Share).** The artifact must beat
a screenshot. Exports are **re-rendered from the completed piece's data at
export resolution — never screen-captured** — so there is no HUD, no notch
cropping, and quality is device-independent. Two products from one renderer:

- **Keepsake (Save):** the piece alone on its background, full resolution.
- **Share card:** the piece plus a tasteful frame with picture title, tier mark,
  language, and a small SpellGame wordmark (D2). Masterpiece-tier cards include
  the artwork attribution line from the manifest's provenance data (e.g. *after
  Hokusai*) — honest and classy.

**No player name, no photos of the player, no identifying data on either
product, ever.**

**3. Save to Photos.** It lives in their camera roll like any art they made.
Add-only Photos permission (`PHPhotoLibrary` add-level, **never** full library
access). Permission denied → a friendly audited string, and the share sheet
still works, because sharing doesn't require Photos access. The saved file
carries no metadata beyond standard image data — no location, no learner info.

**4. Share sheet.** One tap to send the share card anywhere. The standard system
share sheet with the share-card image. No built-in social integrations, no share
tracking, no URLs with identifiers — **the image is the whole payload**.

**5. Gallery (look at it later).** Finished pieces are trophies; losing them on
Continue would sting. A finished-pieces gallery per profile: thumbnail grid by
tier, tap to view fullscreen with the same rest-state treatment, Save/Share
available from here too. It stores the compact piece data — words, paths,
styling seed, kilobytes — and re-renders on view; it never stores rendered
bitmaps. Included in the existing progress-reset flow. Multi-session expert
pieces appear here only when complete; in-progress state stays where
CC-PICTURE-BANK D4 put it.

**6. Kid Mode behaviour.** Celebration for everyone; sharing surface only where
appropriate. Kid Mode gets the full reveal, the gallery, and Save to Photos. The
Share action is **hidden** in Kid Mode, consistent with the no-share-cards
posture, unless Eric decides otherwise in review (D4). All finale strings,
including Kid Mode variants, route through the audit gate.

## Decisions

| # | Decision | Status |
| --- | --- | --- |
| D1 | Reveal animation length | **DECIDED (Eric, 2026-07-31)**: build-replay scaled to piece size, 3s (starter) to 6s (masterpiece), always skippable. Timing is taste and Done #7 still defers to Eric's eye on device -- this signs off the range, not the final feel. |
| D2 | Wordmark on share card | **DECIDED (Eric, 2026-07-31)**: small "SpellGame" text wordmark, corner placement, on share cards only — never on the saved keepsake (their art, not an ad). |
| D3 | Export resolution | **DECIDED (Eric, 2026-07-31)**: keepsake at 3× piece canvas, capped near 4096px longest side; share card at 2048px longest side. |
| D4 | Kid Mode share | **DECIDED (Eric, 2026-07-31)**: hidden in Kid Mode, Save allowed — consistent with the standing no-Kid-Mode-share-cards posture. |
| D5 | Gallery storage form | **DECIDED**: piece data + seed, re-render on view. The deterministic renderer makes this exact — the same piece every time — keeps storage tiny, and lets old trophies inherit every future renderer improvement. |
| D6 | Sound | **DECIDED (Eric, 2026-07-31)**: one soft completion chime, respecting existing sound settings; no fanfare loop. |

## Constraints and non-goals

- **No renderer changes.** The finale consumes v6/v8.1/v8.2 output exactly as
  rendered in play; export uses the same solver output at higher resolution,
  with zero layout differences (verified by Done #1).
- **No social features**: no feeds, no leaderboards, no friend systems, no
  server anything. The share sheet is the entire social surface.
- **No engagement hooks**: no "share to unlock," no streak prompts on the reveal
  screen, no interstitials between reveal and Continue.
- Privacy posture inherited wholesale: nothing leaves the device except the
  image the player explicitly saves or shares; exports carry no metadata, no
  identifiers, no learner state.
- Do not touch scoring, word selection, the Learner Model, or CC-PICTURE-BANK
  manifests beyond reading provenance/title fields.
- Uploaded-photo pieces (future): share cards remain **OFF** for photo pieces
  per CC-PHOTO-PICTURE until Eric revisits — curated pieces only for now, and
  the manifest tier field gates this automatically.

## Done when

1. **Pixel-diff test**: export renderer output at 1× matches the in-play final
   frame's piece region exactly (HUD excluded) for the dog and Eiffel goldens —
   proof of "same solver output, no layout drift."
2. **Reveal flow UI test**: final word → HUD fades → animation plays → tap skips
   to rest → rest persists ≥60s with no auto-advance → Continue advances. On all
   four device classes.
3. **Permission matrix test**: Photos granted/denied × share available — all
   four paths behave per spec; the denied-save string passes the audit gate.
4. **Metadata audit**: exported files inspected by test — no EXIF location, no
   identifiers, no learner data; the masterpiece card shows the attribution
   line; the keepsake carries no wordmark.
5. **Gallery**: complete three pieces across tiers → the grid shows three;
   viewing re-renders byte-identical to original completion (D5 determinism);
   progress-reset empties it; an in-progress expert piece is absent until
   complete.
6. **Kid Mode**: Share hidden (per the D4 outcome), Save works, strings audited,
   and Eric reviews the Kid Mode reveal screen personally.
7. **Eric's sign-offs** on D1–D4 and D6 recorded before code; his device check on
   the reveal feel — animation timing is taste, and this file defers to his eye —
   before ship.

---

## Status against this file (2026-07-31)

No code written; D1–D4 and D6 are unsigned and Done #7 makes that the gate.

Four things that change the shape of the work:

- **The premise is half true, and the better half is already there.** The last
  word does *not* roll straight into the next picture: `show_done` in
  `src/wordpic_screen.rs` raises a completion card with a word count. So there is
  a completion hook and a place to hang the reveal — what's missing is the
  reveal itself. Feature 1 replaces a text card, it doesn't invent a moment from
  nothing.
- **The share sheet is already a dependency.** `@capacitor/share` is installed,
  so feature 4 is plumbing rather than a new capability. **Photos is not**:
  there is no photo-library plugin, so feature 3's add-only `PHPhotoLibrary`
  access is genuinely new native surface and is the largest single piece of work
  in this file.
- **Feature 1's pan/zoom is the same pan/zoom CC-PICTURE-BANK D3 gates on, and
  it does not exist.** "Pan/zoom active where available" resolves to "not
  available" today, which is fine as written — but the rest state's fullscreen
  treatment should be designed so it degrades honestly rather than assuming the
  gesture.
- **D5's determinism claim is currently true and worth protecting.** The
  renderer is deterministic — the render sweep asserts it across seeds — which
  is exactly what makes "store the seed, re-render on view" safe. If that ever
  stops holding, gallery trophies change under the player, so D5 should be read
  as a standing constraint on the renderer and not only as a storage choice.
