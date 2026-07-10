# Google Play — Data Safety form (click-ready answers)

Screen-by-screen answers for Play Console → **App content → Data safety**.
Verified against the code (`backend/db.py` users/leaderboard schema,
`backend/climb.py`, and a scan confirming **no analytics/ads/tracking SDKs and no
mic/camera access**). Answer truthfully; this is a binding listing asset. Mirrors
the iOS App Privacy answers.

**Golden rule that shapes every answer:** the base game collects **nothing** off
the device. The *only* data collection is **The Climb** — an **optional** online
leaderboard with accounts — and it is **gated OFF for under-13 / Kid-Mode users
by the age gate**, so children never provide personal data.

---

## Screen 1 — Data collection and security

| Question | Answer |
|----------|--------|
| Does your app collect or share any of the required user data types? | **Yes** — only when a user opts into a Climb account. |
| Is all user data collected by your app encrypted in transit? | **Yes** (HTTPS end-to-end; Cloudflare edge → Caddy → backend). |
| Do you provide a way for users to request their data be deleted? | **Yes** — in-app **Settings → Your account → Delete my account** (permanent, cascades leaderboard entries); also by emailing the support address. |

---

## Screen 2 — Data types collected

Check ONLY these. For every one: **Shared = No**, **Processed ephemerally = No**,
and none is used for advertising/analytics/tracking.

### Personal info

| Type | Collected | Required/Optional | Purposes (check these) |
|------|-----------|-------------------|------------------------|
| **Name** (username / display name) | Yes | **Optional** (only if creating an account) | App functionality; Account management |
| **Email address** | Yes | **Optional** (account creation) | Account management; Fraud prevention, security & compliance (email verification + password reset) |
| **User IDs** (account + session id) | Yes | Optional | App functionality; Account management |
| **Phone number** | Yes | **Optional** (user may provide it for SMS password reset; never displayed) | Account management |

> The username is shown publicly on the leaderboard, but that is in-app display,
> **not "sharing"** in Play's sense (no transfer to a third party). Answer
> Shared = No.

### App activity

| Type | Collected | Required/Optional | Purposes |
|------|-----------|-------------------|----------|
| **Other actions** (leaderboard best-chain scores + run metadata: word count, duration) | Yes | Optional (only for account holders) | App functionality (ranking + anti-cheat) |

### Do NOT check any of these (not collected)

Location · Financial info · Health & fitness · Messages · Photos & videos ·
**Audio** (no recording — no mic/camera access) · Files & docs · Calendar ·
Contacts · Web browsing history · Installed apps · **Device or other IDs** (no ad
ID; no analytics) · **Crash logs / Diagnostics** (client-side console only; nothing
sent). Passwords are stored as **salted bcrypt hashes** but are authentication
credentials, not a listed Data-safety data type.

**Local-only, therefore NOT declared** (Play only asks about data that leaves the
device): game progress, accuracy stats, Misses queue, imported "My Words", and
all settings live in device localStorage and never transmit.

---

## Screen 3 — Security practices

- **Encrypted in transit:** Yes.
- **User can request deletion:** Yes (in-app + email).
- **Follows the Play Families Policy:** **Yes** (see target-audience section).
- Independent security review: No (not claimed).

---

## Screen 4 — Target audience & content (Families Policy) ⚠️ ERIC'S DECISION

This is the compliance-sensitive part for a kids-adjacent app. It is a **hard
review gate** — Eric decides, and it may warrant a quick legal read.

**How the app is built to make this safe:**
- On first launch an **age gate** asks date of birth; only a pass/fail *verdict*
  is stored locally (the DOB itself is never sent anywhere).
- Users **under the COPPA cutoff (13)** are locked into **Kid Mode**, and **The
  Climb — accounts, email, leaderboard, all data collection — is disabled** for
  them. So a child cannot create an account or provide any personal data.
- No ads, no third-party analytics, no tracking of any user.

**Recommended answers (confirm with Eric):**
- **Target age groups:** include under-13 only if you want the app listed for
  children — doing so triggers **Families Policy** (which this design supports:
  neutral age screen, no data collection from children, no ads). If you'd rather
  avoid Families Policy scrutiny, target **13+** and the age gate still protects
  younger users who install it.
- **Is your app designed for children (primarily child-directed)?** The app is
  **mixed-audience** (educational, playable by adults; Kid Mode for children).
  Most consistent answer: **"No, not primarily directed at children"**, while
  still following Families Policy where under-13 users are supported.
- **Ads:** No.
- **Content rating questionnaire:** no violence/sexual/gambling/drugs → **Everyone
  / PEGI 3**.

**→ Do not submit this screen without Eric's explicit sign-off** (kid-facing
compliance is always Eric's call). Keep this answer identical to the iOS
submission so the two stores are consistent.

---

## Consistency check before submitting

- [ ] Privacy policy published at https://spellgame.net/privacy (required).
- [ ] Answers identical to the iOS App Privacy declaration.
- [ ] Delete-account flow verified working on the shipping build.
- [ ] Target-audience / Families answer signed off by Eric.
