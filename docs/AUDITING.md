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

## What must be true (and is)

- **Nothing here ships.** The production build has `audit_preview` off, so
  `RTL_SUPPORTED` is false, the languages stay gated, and the draft banks are not
  compiled in. Verified at the byte level: the draft words are absent from the
  production wasm.
- **The draft content is copyleft** (OpenSubtitles / CC BY-SA — see
  `assets/words-draft/README.md`). Using it in a non-shipping audit build sidesteps
  the §4 copyleft *shipping* decision; shipping any of it still needs that call.

## Scope

ar / fa / ur / ru **and Hindi** (hi). Hindi is registered only in the audit build
(its keyboard charset, Devanagari font, and draft bank are wired here); production
registers no Hindi (CC-HINDI-PHASE0 D8). Its hard/expert tiers are thin (17/4) —
the Hindi subtitle corpus is small and code-mixed; a monolingual source fills them.
