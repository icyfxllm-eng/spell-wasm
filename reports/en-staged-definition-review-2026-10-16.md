# Staged-row definition review, 200-row sample (2026-10-16)

Source: review pasted by Eric on 2026-10-16 against `reports/en-staged-review-sample.csv` (single Wiktionary gloss per word).
Status: **auditor notes and flags only.** The "suggested fix" wording below is not an approved definition; definitions enter only through the audited spreadsheet path, and LLM-drafted wording is rejected.

Result: 29 of 200 rows (14.5%) need attention. 171 match their word and part of speech.

## Valid but the wrong sense for learners (16)

- esquire: "a lawyer" is a narrow US usage.
- cairn: limited to ancient British burial mounds; a cairn is any pile of stones used as a marker.
- flume: "ravine" sense; the expected sense is a water channel or water slide.
- connector: "networker" sense; the expected sense is a device that joins things.
- yeoman: royal-household sense; the expected sense is a farmer who owned his land.
- lowly: "low in place"; the expected sense is humble in rank.
- deciduous: generic "part that falls off"; the expected sense is trees that shed their leaves.
- scratchy: "having scratches"; the expected sense is rough or itchy.
- rammed: British slang for "packed full".
- bleacher: "one who bleaches"; the expected sense is stadium seating.
- sympathetically: mentions occult correspondences.
- gracious: vague "of grace"; the expected sense is kind and courteous.
- irritable: "capable of being irritated"; the expected sense is easily annoyed.
- furrow: "found on wood or metal"; the expected sense is a groove cut by a plow.
- bulldog: the Pug-cross, "ladies' companion" breed history is questionable.
- aloud: listed below.

## Mismatches and wrong part of speech

| word | problem | direction of fix (flag, not approved text) |
|---|---|---|
| counteract | Tagged noun; it is a verb | verb: to act against something and reduce its effect |
| jell | Tagged noun; it is normally a verb | verb: to set firmly, or to take clear shape |
| elixir | Describes the philosopher's stone | a magical or medicinal drink |
| melancholy | Archaic "black bile" sense | a deep, lasting sadness |
| trolley | Obscure "trolley pole" sense | a cart with wheels, or a streetcar |
| whet | Boar sharpening its tusks | to sharpen; to stir up (as in "whet your appetite") |
| oeuvre | Says a single work | an artist's complete body of work |
| outrage | "Atrocity" sense | strong anger at something unjust |
| aloud | Says "loudly"; the word means audibly, not loudly | out loud, so others can hear |

## Review conventions applied

- "Match" means the definition fits the tagged part of speech and gives the sense a learner would expect from hearing the word. A rare, archaic or regional sense counts as a mismatch for a spelling app.
- Circular definitions ("unmet: Not met", "loosely: In a loose manner") count as matches: low quality, not wrong.
- Age-appropriateness of words like womanizer or whipping was out of scope.

## What changed in the tooling as a result

`tools/en_growth/admit.py` now offers up to three candidate senses per word, from the part of speech with the most senses, with regional/register senses ranked lower. This fixes counteract (now a verb) and moves the learner sense into the top three for flume, bleacher, irritable, furrow, deciduous, elixir, melancholy. It does not fix jell, trolley, cairn or outrage; those need the human audit.

## Word verdicts and tier grading (2026-10-16, later the same day)

Eric reviewed the graded copy of the 200-row sample (`reports/en-staged-review-sample-graded.csv`; the grader of the est_tier, spelling_traps and flag columns is not recorded) and ruled: **keep all 200 words, and agree with the estimated tiers.** Keep rate 200/200, so D10's 95% bar is met for the sample.

Estimated tiers on the sample: Easy 50, Medium 92, Hard 47, Expert 11. Only 5.5% of the staged Expert-band rows are Expert by this grading, so the F2 Expert band (score >= 0.702) is not a usable tier assignment for staged rows. F2 score separates the graded tiers weakly (AUC 0.71 for Expert vs the rest).

Eleven rows carry a homophone or variant flag and need context sentences when served: aloud, elicit, hoarse, jell, licorice, lite, peek, shoo, thyme, whet, wretch. Before any staged row goes live, `assets/words/en/homophones.txt` must be regenerated to include staged words, or learners are marked wrong for valid homophones.
