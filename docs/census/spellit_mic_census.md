# CC-SPELLIT-MIC-FIX v1.1 §0 census

Run 2026-10-03 against `main` at 5eae257f (build 261). Read-only.

**Two stop-and-ask conditions in the file itself have fired, so Phase A does
not start.** Separately, several items the file treats as open are already
answered in the tree, one of them shipped three hours ago.

The census items that need the device (C1–C5, C7, C8, C9) are not answered
here — they need the extended diag line on a build. But **one number that
already exists discriminates between the two main branches of the decision
table, and the file's evidence table omits it.** See "the fastest question".

---

## Stop and ask 1 — voiceSpell is not English and Spanish

The file's trigger: *"A language other than English or Spanish already has
`voiceSpell: true` or a lexicon file."* Both halves are true.

```rust
pub const VOICE_SPELL_LANGS: [&str; 14] =
    [EN, ES, FR, DE, PT, PL, VI, JA, FIL, ZH, RU, AR, SW, HI];
```

Fifteen letter-lexicon files exist, one per registered language.

F4 item 4 reads "v1 languages: English and Spanish only, as CC-SPELL-ALOUD
specified." That was true until **Eric's mic-everywhere ruling of 2026-07-27**,
which opened the gate to every registered language precisely because every one
had acquired a lexicon. Implementing F4.4 as written would **withdraw the mic
from twelve languages** on the strength of a sentence describing the world
before that ruling.

This is the same stale premise that `consts.rs`, `play_hub.rs` and
`config/modes.json` all carried until yesterday, when they were corrected —
the comments had said en/es for over two months after the ruling. It is a
reasonable thing for a spec author to have believed.

**Ruling needed.** Three readings:

- (a) F4.4 is stale; keep the fourteen. The Voice Completeness test (F4.3) then applies to all of them, and six would fail today — see the next section.
- (b) F4.4 is deliberate: narrow to en/es until each language passes F4.3, then re-open per language. This is defensible and is the strictest reading of I-M4, but it is a visible removal for twelve languages and reverses a signed ruling.
- (c) Middle: keep the fourteen, make F4.3 advisory per language until the native review lands.

### What F4.3 would do today

The Voice Completeness test — *every bank answer must be producible from
lexicon letter names* — is close to the gate added yesterday
(`scripts/letter-lexicon-check.mjs`), but stronger: that one checks every
required character is reachable, F4.3 checks every whole word round-trips.

Character-level coverage already fails for six languages, so the stricter
word-level test cannot pass for them either:

| | gap |
|---|---|
| vi | 63 characters. `diacritics` is **empty**, so no toned vowel is sayable |
| ar | 4 hamza carriers; `أ` alone occurs 586 times |
| ja | 7 small kana, no convention drafted |
| hi | 2 nasals · fr | 1 ligature |
| zh | 1, and not real: players type `v`, the grader converts |

Under reading (a), F4.3 fails the build on day one for ar, fr, hi, ja, vi.
Under (b) it passes, because only en and es remain. That is the practical
difference between the two rulings.

---

## Stop and ask 2 — there is more than one session owner (C6)

The file's trigger: *"C6 finds more than one session owner and consolidating
would change how word audio behaves."* Both halves are true.

| file:line | what it sets |
|---|---|
| `AppDelegate.swift:13-14` | `.playback`, `setActive(true)` at launch |
| `NativeLanguageKitPlugin.swift:101` | `.ambient` |
| `NativeLanguageKitPlugin.swift:103` | `.playback` |
| `NativeLanguageKitPlugin.swift:134-135` | `.playback`, `setActive(true)` |
| `NativeLanguageKitPlugin.swift:158-159` | `.playback`, `setActive(true)` |
| `NativeLanguageKitPlugin.swift:615-624` | `setActive(false)` → `.playAndRecord, mode: .measurement` → `setActive(true)` |
| `NativeLanguageKitPlugin.swift:854-855` | `setActive(false)` |

Five distinct configuration sites across two files, so **I-M5 is violated
today** and F2 item 1 is a real change rather than a tidy-up.

The second half of the trigger is the part that matters: the `.ambient` /
`.playback` split at lines 101–103 **is** CC-FEEDBACK D4. Ambient follows the
silent switch, playback ignores it — that is how feedback sounds go quiet while
word audio keeps playing. A naive consolidation onto one category would change
exactly that behaviour, which F2 is forbidden to do. Any owner module has to
keep both categories and the rule for choosing between them.

---

## Already true, and not open

- **D6 — Korean mic absent: DONE, and shipped.** `voice_spell("ko")` is false as of 88c53d52, in **build 261**, submitted for external review three hours ago. Pinned three ways: a `play_hub` test asserting ko reports unavailable, and two laws in the lexicon gate. F4 item 4's Korean clause needs no work.
- **C10a — why a Korean voiced answer grades wrong: settled 2026-10-03.** The lexicon's values are compatibility jamo (U+3131 block), which neither compose nor match; a player who spoke every letter correctly was still marked wrong. Proven in `norm.rs::ko_voice_spelling_needs_positional_jamo_not_a_lexicon_edit`, which also proves the obvious data fix fails: the same letter is a different codepoint as onset (U+1100) and coda (U+11A8), and a speaker does not say which. That is why D6's successor file is needed and why nothing smaller works.
- **G1 — both permission strings survive.** `NSMicrophoneUsageDescription` and `NSSpeechRecognitionUsageDescription` are present. CC-HUB-DEADROWS resolved its D4 **against** removal for exactly G1's reason: Say It was not the sole consumer, and the shipped permission text covers both features in fifteen languages ("so you can say or spell a word out loud"). The guard G1 asks for does not exist yet and is worth adding.
- **Target build 220** — TestFlight is at **261**. Builds 220 through 261 have shipped since that number was written.

---

## The fastest question, and it is free

The existing diag line (`NativeLanguageKitPlugin.swift:693-701`) already prints:

```
sr=<rate> ch=<channels> buf=<tapCount> onDev=<bool> peak=0.000
```

So `sr`, `ch` and part of C2 are **already instrumented**, and so is a counter
the file's evidence table does not mention: **`buf=`, the number of buffers
that reached the tap.** That single number splits the decision table:

- **`buf=0`** — no audio is routing. Session fault → **F2**.
- **`buf>0` with `peak=0.000`** — buffers arrive and measure as digital zero → **F3**.

Eric's screenshot of build 219 very likely already contains it. Reading it
costs nothing and may remove the need for half the instrumentation.

### A specific candidate for `buf>0, peak=0.000`

`NativeLanguageKitPlugin.swift:438-445`:

```swift
private static func rms(_ buffer: AVAudioPCMBuffer) -> Float {
    guard let ch = buffer.floatChannelData?[0] else { return 0 }
```

`floatChannelData` is nil whenever the buffer's `commonFormat` is not
`pcmFormatFloat32`. The peak would then be **exactly 0.000 no matter how loud
the room**, and `NO_SPEECH` would follow. This is precisely what census item
C5 (*"the sample format the peak is computed from"*) exists to settle, and it
matches the decision-table row *"peak=0.000 but C9 playback has audible speech
→ peak measured from the wrong format → F3"*.

It is a candidate, not a finding: `inputNode.outputFormat(forBus: 0)` is
normally float32 on iOS, so this only bites if the delivered format differs.
One line of diag (`fmt=\(buffer.commonFormat.rawValue)`, or simply whether
`floatChannelData` is nil) settles it, and it is cheaper than C9's playback
button.

### What the capture path already does right

Worth recording so Phase B does not rebuild it: F3 items 1–3 **already hold**.
The tap is installed on `audioEngine.inputNode` bus 0 using that node's own
`outputFormat(forBus: 0)`, read **after** the session is made record-capable,
with the engine stopped and reset first; the same buffers feed the recognizer.
The session is deactivated before the category change and reactivated after,
which is F2 item 2's shape. If the fault is in here it is subtler than "the
tap is on the wrong node".

---

## F1 — a real defect, and it is ordering, not a missing exit

The file reads the symptom as a listening state with no exit. There **is** an
exit: `spell_aloud.rs:930` does `if LISTENING { stop_session(); return; }`, and
`stop_session` clears the flag and calls `native_lang::stop_letter_capture()`.

But it is **fifth** in `mic_tap`. Four capability branches run first, and two of
them return:

1. `!enabled()` → return
2. `DOWNLOADING` → return
3. `CAP_STATE == "downloadable"` → start a voice-pack download, return
4. `CAP_STATE == "server"` and not yet consented → show the consent card, return
5. `LISTENING` → **stop**

So in the `downloadable` and `server` rungs, **every tap takes an earlier
branch and the stop is unreachable** — a mic that cannot be switched off by
tapping, which is exactly I-M1's prohibition. On an English device with the
voice pack installed `CAP_STATE` is `installed`, so this is **not** proven to
be Eric's build-219 symptom; it is a separate latent fault on the other two
rungs.

The fix is F1 item 2 taken literally: the stop check moves to the top of
`mic_tap`, before every capability branch. A tap while listening stops the mic,
always, whatever rung the language is on.

---

## What I recommend

1. **Read `buf=` off the existing screenshot** before building any instrumentation. It may decide F2 vs F3 outright.
2. **Rule on stop-and-ask 1.** Everything in F4 depends on it, and reading (b) is a visible removal for twelve languages that reverses a signed ruling — I would not implement it from a sentence written before that ruling.
3. **Rule on stop-and-ask 2**, specifically: the one audio-session owner must keep the `.ambient` / `.playback` split, because that split is CC-FEEDBACK D4.
4. **Let me ship the two unambiguous pieces now** — F1's ordering fix, which is small and closes a real I-M1 hole on two rungs, and G1's build guard. Both are independent of the census and of both rulings.
5. Add the one-line format diag with F1, so the next build answers C5 without the C9 playback button and its I5 exception.

Phases A–C otherwise stay blocked.

---

## Addendum, 2026-10-03: `buf=0`, and the mechanism

Eric read the diag: **`buf=0`**. No buffer ever reached the tap. Per the
decision table that is the session branch — **F2**, not F3 — and the cause is
visible in the tree.

`.playback` is an **output-only** category: a session in `.playback` has no
input route. Three independent paths assert it, each followed by
`setActive(true)`, none of them aware of a live capture session:

| path | file:line | when it fires |
|---|---|---|
| native TTS | `NativeLanguageKitPlugin.swift:134-135` | `speak()` — every spoken word |
| syllable replay | `NativeLanguageKitPlugin.swift:158-159` | `speakSyllables()` |
| **cached audio** | `ios/App/App/public/audio-native.js:83` | `configure({focus:true})` — the normal orb playback path, and its own comment says it sets `.playback` |

AVAudioSession is one object per process — the plugin says so itself at line 18.
So any of these firing while the mic is listening **switches the shared session
out of `.playAndRecord`, the input route disappears, and the tap stops receiving
buffers**. `buf=0`, then `NO_SPEECH`, which is the recognizer truthfully
reporting that it was given nothing.

`beginCapture` already handles the *other* direction: lines 614-624 deactivate,
set `.playAndRecord`, reactivate, with a comment explaining that switching
category on an already-active session does not reliably route the mic. What
nothing handles is the reverse — a `.playback` assertion arriving *after* the
mic is live. There is no guard, no ordering, and no owner.

This is also why the earlier fix attempts passed the suite and changed nothing
on the phone: the suite feeds synthesized audio to the parser, and the fault is
two layers below that, in which category the OS session happens to be in.

It matches the file's own unexplained observation — *"Orb reads 'listen…' while
the mic ring is active"* — which C8 was written to confirm. The overlap is
real, and the overlap is the bug.

### What this means for F2

F2's shape is right, and item 5 ("word audio and the mic are mutually
exclusive") is the operative clause rather than item 2. The owner module has to
intercept all three paths, not just the two in Swift — the cached-audio path
goes through JS and is the one that plays most word audio.

**Stop-and-ask 2 still stands and now has teeth.** The `.ambient` / `.playback`
split at lines 101-103 is CC-FEEDBACK D4: ambient follows the silent switch,
playback ignores it. A single owner must keep both categories and the rule for
choosing, so "one owner" cannot mean "one category".

The minimal change consistent with every signed decision looks like: one module
owns category and activation; `speak`, `speakSyllables` and the JS `configure`
route their requests through it; while the state is not `idle` those requests
are **held, not applied** (F2 item 4), and a mic tap during word audio waits for
the audio to end (D3a). None of that is written yet — F2 is Phase B and blocked
on Eric's ruling.
