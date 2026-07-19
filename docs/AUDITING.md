# Putting a language in front of a native-speaker auditor

The `audit_preview` build unlocks the ComingSoon languages (ar/fa/ur/ru today, plus
the already-banked es/fr/de/…) so a native speaker can **play them and review what's
there**. It is unverified and must stay private; it is never a release.

## Three commands

```sh
bash scripts/build-web-audit.sh      # -> dist-audit/  (banner: "UNVERIFIED PREVIEW")
node scripts/serve-audit.mjs         # serves it behind a password; prints URL + pass
# to reach a REMOTE auditor, expose the local port over a tunnel:
cloudflared tunnel --url http://localhost:8140      # (or: ngrok http 8140)
```

Send the auditor the tunnel URL **and** the password. The password stops the
unverified build being stumbled on; the tunnel gives them HTTPS. Stop the server
(Ctrl-C) when done — don't leave it exposed.

## What the auditor is reviewing

Two separate things, and it helps to say which you want:

**The engine** — does the language *work*?
- Does the word render correctly? (Arabic/Persian in Naskh, Urdu in Nastaliq — the
  join intact, not shattered into isolated letters.)
- Does the on-screen keyboard have every letter, in a sane arrangement?
- Does per-letter feedback colour the right letters after a miss?
- Does the audio read the word correctly?

**The words** — are these the *right* words? This is where the drafts need the most
help. They are corpus-attested and correctly spelled, but nothing has judged:
- **Register / naturalness** — the frequency top is heavy with function words
  (particles, pronouns) rather than the concrete nouns a speller wants.
- **Appropriateness** — subtitle corpora carry proper nouns and adult vocabulary. A
  kids' spelling game must cut those. Mark them.
- **Orthography** — e.g. Persian should use ی/ک (Persian), not ي/ك (Arabic); the
  drafts are folded to Persian forms, but confirm.

The auditor's output is a red-pen pass: which words to drop, which to keep, any
spelling/orthography corrections. That review is what turns a draft into a bank.

## Flagging words as you play

The build carries a floating **⚑ Flag word** control (bottom-right). After a word
is revealed, tap it to mark that word for cutting; **Flags** opens the list, lets
you remove mistakes, and **Export** downloads `audit-flags.txt`. Send that file
back. It is machine-usable, not prose:

```sh
python3 scripts/ingest-audit-flags.py audit-flags.txt   # -> assets/words/exclusions/<lang>.txt
python3 scripts/build-draft-banks.py                     # drops the flagged words from the drafts
```

The exclusion list is the SAME one production honours, so a flagged word is gone
everywhere on the next rebuild — draft and, once promoted, production. The widget
lives only in this bundle (injected by `build-web-audit.sh`); it reads the word
from a `data-audit-word` attribute set only under the `audit_preview` feature.

## What must be true (and is)

- **Nothing here ships.** The production build has `audit_preview` off, so
  `RTL_SUPPORTED` is false, the languages stay gated, and the draft banks are not
  compiled in. Verified at the byte level: the draft words are absent from the
  production wasm.
- **The draft content is CC BY** (Leipzig Corpora — see
  `assets/words-draft/README.md`). Attribution-only, no share-alike: promoting a
  reviewed draft to production needs only the attribution already in `NOTICES.md`,
  not a §4 copyleft decision. (The earlier OpenSubtitles CC BY-SA drafts did.)

## Scope

ar / fa / ur / ru **and Hindi** (hi). Hindi is registered only in the audit build
(its keyboard charset, Devanagari font, and draft bank are wired here); production
registers no Hindi (CC-HINDI-PHASE0 D8). Its bank is drawn from a monolingual Leipzig news corpus (CC BY), so all four
tiers are full — the expert tier carries real long words (प्रधानमंत्री, राष्ट्रपति).
