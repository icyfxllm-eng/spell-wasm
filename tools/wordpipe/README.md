# Word-List Expansion Pipeline (mechanical gates)

Turns a large candidate word list into a small, **staged** batch that has cleared
every mechanical quality gate — then **stops** for human (Eric) review. Nothing
this pipeline emits is live. Promotion to the shipping word pool is a separate,
deliberate step after the listen-page review and (once the backend exists) the
TTS→STT gate.

```
python3 tools/wordpipe/pipeline.py
```

Outputs land in `tools/wordpipe/out/`:

| file | what |
|--|--|
| `batch_report.md` | human-readable gate summary — counts, exclusions by gate, curve table, seeded-bad demo, homophone flags, sample accepted |
| `review-listen-page.html` | one row per accepted word with a ▶ hear / ✓ / ✗ control — the manual TTS audition (Gate 5 stand-in until the STT route exists) |
| `batch-en-001.json` | the staged batch, `state:"staged"`, `tts_verified:false` — the promotable artifact, still gated |

## The gates (cheapest first; a failure EXCLUDES, never patches inline)

1. **Schema** — clean `[a-z]{3,15}` word; assigns the record's required fields.
2. **Duplicate / homophone** — exact match against the live pool → reject;
   same CMUdict pronunciation as a pool word → **flag** for a deliberate yes
   (e.g. `son`/`sun`, and US/UK pairs like `favourite`/`favorite`). Homophones
   are not auto-rejected — a spelling game may *want* them — but they never pass
   silently.
3. **Appropriateness** — LDNOOBW blocklist **plus** a `MILD_BLOCK` supplement
   (`damn`, `hell`, `crap`, weapons, alcohol…) the LDNOOBW list omits, plus a
   brand/proper-noun starter set. Kids' app → we err strict (false positives are
   cheap, a bad word live is not). `MILD_BLOCK` mirrors the runtime
   `src/kid_filter.rs` intent; extend both together.
4. **Difficulty + curve guard** — see below.
5. **TTS→STT round-trip** — **STUBBED** (needs the backend). See proposal below.

## Difficulty formula (Gate 4, English)

Difficulty answers "how hard is this to spell *from hearing it*", so it weights
irregularity and rarity over raw length:

```
freq_norm = min(log10(rank) / log10(60000), 1)     # rarer word  → harder
len_norm  = min(len(word), 15) / 15                 # longer      → slightly harder
irr_norm  = min(len(en_irregularity(word)) / 3, 1)  # silent/doubled/suffix/loan traps
raw       = 0.40*freq_norm + 0.20*len_norm + 0.40*irr_norm
level     = clamp(1 + int(raw*5), 1, 5)             # 1..5
```

`en_irregularity` is the **shipped** extractor reused from
`tools/difficulty-score/extractors.py` (silent letters, doubled consonants,
tricky suffixes, loanword patterns) — the pipeline does not invent a second
notion of English difficulty.

### Curve guard

A batch must not quietly shift the pool's difficulty mix (that's how "medium"
silently drifts harder). After gating, `select_balanced` picks ~500 words whose
per-level shares mirror the **pre-batch** pool shares; the guard then asserts
every level stays within **±10%** of its pre-batch share, or the batch is
rejected. Gate-passing words beyond the per-level quota are **deferred** (held
for a later batch), not dropped. A deliberately top-heavy batch is what the
guard exists to reject.

## Candidate source

OpenSubtitles English frequency list, ranks **150–45000** (skips the ~150 most
common words already in the pool; the wide tail supplies genuine level-3/4/5
material). Seeded ahead of the real candidates are four known-bad demo words —
`damn` (mild profanity), `cat` (already in pool), `google` (brand), `recieve`
(misspelling) — so each gate's rejection path is visible in every report.

## Sources (open corpora, gitignored under `sources/`)

| file | corpus | used by |
|--|--|--|
| `freq_en.txt` | OpenSubtitles `en` FrequencyWords (top 50k) | difficulty `freq_norm`, candidate assembly |
| `cmudict.dict` | CMU Pronouncing Dictionary (~135k) | homophone detection, real-word check |
| `blocklist_en.txt` | LDNOOBW English blocklist (~400) | appropriateness gate |

Not committed (public, large, regenerable). Re-fetch with the notes in
`sources/FETCH.md`.

## Gate 5 (TTS→STT) — proposal, not yet built

The point: a word only ships if its **generated audio is recognizably that
word** — otherwise a player hears mush and is marked wrong for spelling exactly
what they heard. This needs the Flask/Google backend, which this pipeline
deliberately does not touch.

Proposed flow, to run as a batch job against a staging backend:

1. For each staged word, call the existing `/api/speak?word=…&lang=en` TTS route,
   capture the audio.
2. Feed that audio to an STT engine (Google Speech-to-Text, same cloud creds).
3. Accept only if the transcript normalizes to the word (case/punct-folded);
   near-misses (homophone transcript, plural drift) → **flag**, not auto-pass.
4. Write `tts_verified:true` + the transcript into the batch JSON; only then is a
   word promotable.

Until that route exists, `review-listen-page.html` is the manual stand-in: a
human auditions each word and records ✓/✗. **No word promotes on mechanical
gates alone.**

## Promotion (separate, deliberate — NOT this script)

After Eric's listen-page pass and (eventually) Gate 5, a promotion step appends
`state:"live"` words to `assets/words/en/{tier}.txt`. That step is intentionally
not automated here: the pipeline's job ends at a reviewed, staged batch.
