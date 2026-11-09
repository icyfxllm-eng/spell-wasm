# Gig A brief: rate English definitions and sentences (DRAFT)

Status: draft written from the rev 3 decisions. Eric's July Gig A template is
not on this Mac yet; when it is pasted, its terms, pay and wording win and this
file is folded into it. Eric approves the brief before it is posted.

## Who

A native English speaker with teaching or editing experience (rev 3).

## The job

Spell is a spelling game for children aged 5 and up. Each row is one word with a
short **definition** (the clue a child reads to find the word) and an example
**sentence**. You rate each one separately. About 3,180 rows, 6,330 texts.

Open `sheet.csv` in Excel, Numbers or Google Sheets. Fill ONLY these columns:

- `definition_severity`: 0, 1, 2 or 3
- `sentence_severity`: 0, 1, 2 or 3
- `note`: optional, a few words on what is wrong

Leave every other column exactly as it is: do not edit, sort, add or delete
rows, or change the text. The file is locked and an edited file is rejected
whole. If a text cell is empty, leave its severity empty.

**Flag, don't rewrite.** Never type a new definition or sentence. Rate it and,
if you like, say what is wrong in `note`. Fixes are made separately.

## The scale

| Score | Meaning |
|---|---|
| 0 | Fine for a 5-year-old, and it fits the word. |
| 1 | Not for a young child (violence, death, injury, drugs or alcohol, crime, bodily or medical detail, frightening or adult themes), **or** wrong for the word, misleading, confusing, or bad English. |
| 2 | Sexual reference, slur or hateful framing, graphic violence, or self-harm. |
| 3 | Explicit sexual content, a slur used as one, or anything unfit for any player. |

When unsure between two scores, pick the higher one. Judge the text you see,
not other meanings the word might have.

## Checks

- The sheet contains a small number of planted test rows. Rating them correctly
  is part of the job; missing them can mean the pass is not accepted.
- A second reviewer independently rates many of the same rows. If the two
  ratings disagree on more than 5% of texts, the pass is redone.

## Delivery

Return the completed `sheet.csv` with the same file name. Partial files are not
accepted.

## For Eric (not in the posted brief)

- Export: `python3 tools/defs/gen_sheet.py build/defgen/<run>`; the decoy key goes to `keys/`, never to the rater.
- Eric rates every Kid-visible row on his own copy of the same sheet (only those rows need filling).
- Ingest: `python3 tools/defs/gen_ingest.py <rater sheet> --eric <Eric's sheet> [--apply]`.
