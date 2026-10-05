# The /b/ → /ð/ mishearings are not a padding artifact

Run 2026-10-04. Closes a line of investigation so nobody opens it again.

## The suspicion

Four `/b/`-initial words come back from BOTH recognizers as `/ð/`-initial:

    ban  -> "then"     bear -> "There"
    bay  -> "They"     ben  -> "Then"

Four for four on one consonant is not four separate word defects. F1 had
added a 200 ms lead `<break>` to every clip precisely because MP3 encoder
priming eats the start of a file, so the obvious question was whether 200 ms
is enough — an onset half-swallowed is exactly how a plosive stops sounding
like a plosive.

No direction was predicted. More lead helping would mean the padding is short
and one constant fixes all four; no difference would mean the onset is a
property of the voice, and the fix is a signed IPA row per word.

## The method

A disposable copy of `backend/` with `PAD_LEAD_MS` read from the environment —
the live tree was not touched — run at **0, 200 and 400 ms**, each with its own
cache directory so clips could not collide. Four failing words and four
`/b/`-initial controls that pass under the shipped voice. `afconvert` and the
same two recognizers the F2 and bake-off harnesses use, so transcripts here are
comparable with the ones on record.

## The result

| lead | ban | bay | bear | ben | bad | bed | big | box |
|---|---|---|---|---|---|---|---|---|
| 0 ms | then / then | Bay / they | There / — | then / then | ok | ok | ok | ok |
| 200 ms | then / then | They / — | There / there | Then / then | ok | ok | ok | ok |
| 400 ms | then / then | Bay / they | There / there | Ben / then | ok | ok | ok | ok |

**The padding does nothing.** `ban` and `bear` fail identically at every value.
`bay` and `ben` flicker, but Google synthesis is non-deterministic, so each
cell is a DIFFERENT clip and one sample per cell cannot separate a padding
effect from ordinary clip-to-clip variation. The flicker is the size of that
noise and should not be read as a trend.

The controls are what settle it. `bad`, `bed`, `big` and `box` are perfect on
both recognizers at **0 ms** — no lead silence at all. If encoder priming were
eating onsets, that is the cell where it would show, and it does not.

## What follows

The four words stay in the auditor's worklist: a signed IPA row is the right
instrument for them, and no constant will do it instead.

One observation, offered as a question and not a finding: the four that fail
have a nasal coda, no coda, or `/r/` (ban, ben, bay, bear), while all four
controls end in a stop or fricative (bad, bed, big, box). Whether the voice's
`/b/` is weak only outside that environment is not something this run can say;
it would need the same sweep over a proper set, and it changes nothing about
the fix either way.
