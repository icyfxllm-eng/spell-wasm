# App Store — Review Notes (paste into App Store Connect → App Review Information)

## Notes for the reviewer

Spell is a native iOS app, not a wrapped website. All content and game logic
ship inside the app bundle and the game is fully playable offline. Native
capabilities include:

- **Native audio** — word clips are downloaded once and played through the OS
  audio path; the daily set and recently-played words work with no network.
- **Haptics** — tactile feedback on correct/incorrect spellings.
- **Local notifications** — an optional once-a-day practice reminder at a
  user-chosen time (off by default).
- **Native share sheet** — shares a rendered result card image.
- **Universal Links** — spellgame.net/app and /challenge links open the app.

No account, login, or personal information is required — just open the app and
press the glowing orb to hear a word, then spell it.

## How to test

1. Launch the app. Tap the orb ("tap to hear a word") to hear a word spoken.
2. Type the word into "your spelling" and tap "Check spelling."
3. Correct answers grow your chain (streak); try "Definition"/"Sentence" hints.
4. Offline: enable Airplane Mode and keep playing — recently-heard words still
   play from the on-device cache.
5. Optional: Settings (gear) → toggle "Daily reminder" to schedule a local
   notification.

No demo account needed.

## Data / privacy

No personal data is collected. Progress and any custom word lists are stored
only on the device. The app requests dictionary-word audio/definitions from the
developer's own server; these requests are not tied to any identity and are not
used for tracking or ads. See the privacy policy at https://spellgame.net/privacy.

## Notes

- The "speak your answer" microphone feature relies on the Web Speech
  recognition API, which iOS's web view does not provide, so it is inert on
  iOS; typing and handwriting input are the supported answer methods.
- Category: **Education**, age rating **4+**. (Not listed in the Kids category
  for v1 — the share sheet's external-app action would require a parental gate
  under Kids; revisit later.)
