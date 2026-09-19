# privacy.html: draft telemetry section (REVIEW-GATED, do not publish)

**Status:** a draft for Eric. It isn't in `privacy.html` and mustn't be
published until:
1. the R7 corrections (audio, typed answer, install id) have landed, since
   this text assumes the rest of the page is true;
2. the telemetry build is actually shipping;
3. Eric has reviewed it.

*Not legal advice. Check the COPPA amended rule (compliance date April 2026)
and Apple's Kids Category guidance before publishing.*

---

## Where it goes

1. **The short version** (top of the page). The sentence "We never show ads,
   and we use no third-party analytics or tracking." stays true. Add after it:

   > If you leave "Help improve SpellGame" on, the app also sends us anonymous
   > crash and speed reports. They never include your words, your answers, or
   > anything about you, and you can turn them off in Settings.

2. **A new section** after "What the app sends to our server to play":

---

### Crash and speed reports ("Help improve SpellGame")

To find problems before players do, the app can send us short, anonymous
reports when something breaks or runs slowly. This is on by default for
players 13 and over, and you can turn it off at any time in Settings → Help
improve SpellGame. Turning it off also deletes any reports still waiting on
your device.

**What a report contains:**
- The type of problem (for example "the app crashed" or "a sound couldn't play"), and a code for where in our program it happened.
- The app version, whether you're on iPhone, Android or the web, the language you were studying, and which game mode you were last in.
- How long some things took, in broad ranges (for example "under a quarter of a second"), such as starting the app or playing a word's sound.
- For players 13 and over, a random number that changes every time the app starts, so reports from one session can be grouped. It's never saved on your device and can't be used to recognize you later.

**What a report never contains:** the words you practise, what you type,
your answers or scores, your recordings or photos, your word lists, your
account, email or username, your device's advertising ID, or your IP address.
We don't record your IP address when a report arrives, and reports are
stored separately from Climb accounts with no way to connect the two.

**Spell Jr and younger players.** On a device in Spell Jr, or before the
birthday question has been answered, the app never sends individual reports.
It keeps a simple tally of how many times each kind of problem happened and
sends it at most once a day, at a random time, with no identifier of any
kind. A grown-up can turn this off through the parent check in Settings.

**Schools.** In SpellGame for Education, reports are off until the school
turns them on, and even then only the daily tally is sent.

**How long we keep them.** Individual reports are deleted after 90 days. We
keep daily totals (for example "12 crashes on version X in Russian"), which
contain nothing about any person.

**Who handles them.** Reports go to our own service, which runs on Cloudflare,
our hosting provider. No analytics or advertising company receives them.

---

## Checks before publishing

- **Cloudflare's own logs:** the "we don't record your IP address" sentence depends on the zone check in `workers/telemetry/README.md` (Logpush, Security Analytics). If Cloudflare's dashboard keeps sampled IPs that can't be switched off, reword that sentence to say what's true (for example "our hosting provider may briefly keep network logs for security, which we don't use or access for reports"). Under the spec that's Eric's stop-and-ask decision.
- **"13 and over"** matches the age gate's cutoff (`agegate::MIN_FULL_APP_AGE`). Confirm the constant is still 13.
- **The Children's privacy section** currently says "no personal information is collected from them". That stays true (the tally has no identifier and isn't linked), but consider adding a sentence that points to the Spell Jr paragraph above.
