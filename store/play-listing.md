# Google Play — Store Listing & Data Safety (Spell)

Draft copy + form answers for the Play Console listing. Field limits noted.
Screenshots come from real devices/emulators (`fastlane snapshot` later).

---

## Core identity

| Field | Value |
|---|---|
| App name (≤30 chars) | `Spell — hear it, spell it` (25) |
| Package | `net.spellgame.app` |
| Default language | English (US) |
| Category | **Education** (see Kids-category decision below) |
| Tags | Education, Word, Family |
| Content rating | Everyone / PEGI 3 (see rating questionnaire below) |
| Contact email | icyfxllm@gmail.com |
| Website | https://spellgame.net |
| Privacy policy URL | https://spellgame.net/privacy *(must be published before submit)* |

## Short description (≤80 chars)

```
Hear a word, then spell it by typing, speaking, or writing. Spelling by ear.
```
(76 chars)

## Full description (≤4000 chars)

```
Spell is a spelling game you play with your ears. Hear a word, then spell it —
by typing, speaking it aloud, or writing it by hand. Keep your chain going and
climb from easy words to expert ones.

WHY SPELL
• Learn by listening. Every word is spoken clearly, so you practice going from
  sound to spelling — the skill that real spelling tests measure.
• Three ways to answer. Type it, say it, or draw the letters with your finger.
• Climb the chain. Words get harder as your streak grows; miss one and review
  it later, spaced out so it sticks.
• Play anywhere. Your daily words and the last words you practiced work fully
  offline — no signal needed.

MADE FOR FOCUS
• No ads. No accounts. No tracking. Your progress stays on your device.
• Kid Mode: bigger text, friendly words, and a calmer feel for younger spellers.
• Readable Mode: larger, spaced, dyslexia-friendly type.
• A gentle once-a-day reminder you can set to any time (off by default).

BRING YOUR OWN WORDS
Studying for a spelling test or a new language? Paste any list under “My words”
and practice exactly what you need.

Spell is free. Hear it, spell it, keep the chain.
```

## Graphics checklist (produce before submit)

- [ ] App icon 512×512 (from `assets/icon.png`)
- [ ] Feature graphic 1024×500
- [ ] Phone screenshots ×3–8 (2:1 story: “hear it” → “spell it” → “keep the chain”)
- [ ] 7" / 10" tablet screenshots (Amazon Fire tablets share these)

---

## Data safety form (answer truthfully — this is a listing asset)

> UPDATED 2026-07-10: "The Climb" added an **optional** online leaderboard with
> accounts, so the app now DOES collect data — but only for users who opt in.
> The base game still collects nothing. Answer per the tables below.

**Does your app collect or share any of the required user data types?**
→ **Yes** (only if the user creates an optional Climb leaderboard account).

**Data collected** (Collected = yes; Shared = no for all; not used for
advertising/tracking for any):

| Data type | Collected | Purpose | Optional? |
|---|---|---|---|
| Name (username / display name) | Yes | App functionality (leaderboard identity — shown publicly) | User-provided at signup |
| Email address | Yes | App functionality, Account management (verify + password reset) | Required only if creating an account |
| Phone number | Yes | Account management (optional SMS password reset) | User-provided, optional |
| User IDs | Yes | App functionality (account + session) | — |
| App activity (leaderboard scores + run metadata) | Yes | App functionality (ranking, anti-cheat) | — |

- **Sharing:** No data is shared with third parties. (Email delivery and the
  Cloudflare Turnstile anti-bot check are *service providers/processors*, not
  "sharing" in Play's sense — they process the minimum to perform the function.)
- **Purpose:** App functionality + account management only. **Not** used for
  advertising, marketing, analytics, or tracking. No ad/analytics SDKs bundled.
- The base game (no account) still stores gameplay progress **only in device
  local storage**; the dictionary word sent to fetch audio/definitions is not
  tied to any identity.

**Data encrypted in transit:** Yes (HTTPS).
**Password storage:** salted bcrypt hash; plaintext never stored or logged.
**Users can request deletion:** **Yes** — in-app "Delete account" permanently
removes the account and all associated data (CASCADE), or by emailing
icyfxllm@gmail.com. Declare the in-app deletion path + the email in the form.

---

## Content rating questionnaire (expected answers)

- Violence / sexual / profanity / drugs / gambling: **None**
- User-to-user communication / shares location: **No**
- Digital purchases: **No**
→ Expected: **Everyone / PEGI 3**.

## Kids-category decision (from CLAUDE.md Phase 2/4)

The native **Share sheet** exposes an OS-level external action (choosing another
app to share to). Listing under **Google Play “Designed for Families / Kids”**
triggers stricter review and would require a parental gate before that external
action.

**Decision: list under _Education_ with an _Everyone_ rating** (no families
program) for v1, so the share flow needs no parental gate. Re-evaluate the
Kids/Families program later only if we add a parental gate on Share.

---

## Store presence rollout (Play Console)

1. Internal testing track → upload `app-release.aab`.
2. Closed testing: **recruit 12+ testers, 14-day clock** (new personal dev
   accounts require this). START RECRUITING EARLY — schedule long pole.
3. Production.

## Amazon Appstore (same AAB)

Reuse `app-release.aab` (or a universal APK). Free listing; reuse this copy and
the tablet screenshots. Fire tablets are a real kids install base.
