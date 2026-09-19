# App Store privacy label: answers for CC-TELEMETRY-FOUNDATION v1.1

**For Eric to enter by hand** in App Store Connect → App Privacy. REVIEW-GATED:
enter these only when the telemetry build is submitted, not before. Until the
Worker is live and the build ships, the current label is still accurate for
what ships.

*Not legal advice. Apple's definitions are the ones in App Store Connect's
App Privacy help; re-read them there before entering.*

## What telemetry adds

| App Store data type | Collected? | Linked to the user? | Used for tracking? | Purpose | What it is in SpellGame |
|---|---|---|---|---|---|
| **Diagnostics → Crash Data** | **Yes** | **No** | **No** | App Functionality | Error code (`js_uncaught`, `wasm_panic`, `native_crash`, …), a 64-bit hash of where the code broke, build stamp, platform, study language, last mode entered |
| **Diagnostics → Performance Data** | **Yes** | **No** | **No** | App Functionality | Bucketed start-up time, tap-to-audio time, and audio-played/unavailable counts per language |
| Diagnostics → Other Diagnostic Data | No | — | — | — | — |
| Usage Data (any) | **No** | — | — | — | No mode starts, completions, funnels, taps, words, answers or outcomes (I3) |
| Identifiers (any) | **No** | — | — | — | The `session_id` is random per app launch, held only in memory, and never reused (D4). It can't identify a user or device across launches. |

### Why "Not Linked to You"
- The telemetry store has no account ID, email, device ID or IP address, and no field that could join to the account system (I5).
- The Worker never reads the IP, User-Agent or location (tested).
- Jr, not-yet-answered-age and Education devices send only daily counts with no identifier at all (F6, D7).

### Why "Not Used for Tracking"
- There's no third-party SDK, no ad attribution, no IDFA and no ATT prompt.
- The data never leaves SpellGame's own endpoint and is never combined with other companies' data.

### Purpose: App Functionality
Apple's App Functionality purpose includes minimizing app crashes and
improving performance, which is the whole of F1 and F5. Nothing is used for
Analytics in the product-analytics sense, for personalization or for
advertising.

## Check the rest of the label while you're there

The census found flows outside telemetry that the label may or may not
already cover. They're listed so the label is right as a whole. This document
doesn't decide them.

| Flow (census §1) | Question for the label |
|---|---|
| The Climb account: email, username, optional phone | Contact Info → Email Address / Phone Number, and User Content or Other for usernames. Linked; App Functionality. Presumably already declared; confirm. |
| `/api/notify`: a persistent random install id per language interest | Could count as Identifiers → Device ID ("other device-level ID"). Linked to nothing else. Decide. |
| `/api/stt`: consented mic audio for Spell It Out Loud (server rung) | Audio Data only counts as "collected" if it's kept longer than needed to answer the request. Check the backend's retention before answering. |
| `/api/check`: the typed English answer | Same test: collected only if it's retained. The backend compares and returns; confirm nothing logs it. |

The R7 task fixes `privacy.html` for the same flows.
