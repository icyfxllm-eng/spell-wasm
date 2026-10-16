# CC-EN-EXPERT-GROWTH build tools

Build-time only; nothing here ships in the app. Inputs live in the gitignored `.corpus-cache/`.

    python3 -m venv .corpus-cache/wf/v && .corpus-cache/wf/v/bin/pip install wordfreq==3.1.1
    .corpus-cache/wf/v/bin/python tools/en_growth/build_pron.py --extra <words>   # F1  -> assets/words/en/en_pron.json
    .corpus-cache/wf/v/bin/python tools/en_growth/score.py      --extra <words>   # F2  -> assets/words/en/en_difficulty.json
    .corpus-cache/wf/v/bin/python tools/en_growth/test_phase_a.py                 # A1-A3

Licences: wordfreq code Apache-2.0, its data CC BY-SA 4.0 (credits: Google Books Ngrams, Wikipedia, OpenSubtitles, SUBTLEX with credit condition). Used to compute a band and a rank only; no Zipf value or wordfreq word list is shipped. Wiktionary (kaikki) is CC BY-SA 4.0 and its attribution approach is still unapproved; it is a cache input for Phase B and ships nothing.

`calibration/en_expert.csv` is a DRAFT. Do not tune against it until it says `STATUS: APPROVED`.
