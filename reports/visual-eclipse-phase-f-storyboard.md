# CC-VISUAL-ECLIPSE F12: the app preview, storyboard (DRAFT for Eric)

One portrait preview, about 26 s, recorded from the iOS Simulator running the
build that is submitted. App Store sizes: iPhone 6.9" 886 x 1920 and
iPad 13" 1200 x 1600, 30 fps, H.264, stereo AAC. Every frame is real app
footage, with no device frame, no hands and no captions drawn over the app,
as Apple's preview rules require. The only added audio is the app's own
sound, the voice and the game sounds, so it plays well muted too (previews
autoplay muted in the store).

The flags rule applies here as it does to the screenshots: a beat is cut only
if every feature it shows has its flag on in the submitted build. The beat
table below carries the same flag names as config/store-scenes.json.

| # | Time | Beat | What is on screen | Flags |
|---|------|------|-------------------|-------|
| 1 | 0.0 to 3.0 | Hear it | The play screen. A tap on the orb; the corona swells and follows the spoken word. | visual_orb, visual_type |
| 2 | 3.0 to 7.0 | Spell it | The word typed on the game keyboard, key by key, with the key-feel sound, then Check spelling. A green confirm and the score ticks up. | visual_feedback, visual_sound |
| 3 | 7.0 to 10.5 | Miss one | The next word, a near miss. The attempt is marked in the answer box and the right spelling sits under it in amber. Held long enough to read. | visual_feedback |
| 4 | 10.5 to 14.5 | Streak | Three quick correct words, sped up 2x, then the tenth: the orb goes into totality with sparks. | visual_celebrate, visual_orb |
| 5 | 14.5 to 18.5 | Every language | Four quick cuts, about one second each: Korean, Arabic (right to left), Hindi, Vietnamese, each with a word served and its keyboard. | visual_type |
| 6 | 18.5 to 22.0 | Modes | The menu opens; the drawn icons; a tap into Board Game, then a cut to Randospell. | visual_icons, boardgame, randospell |
| 7 | 22.0 to 26.0 | Make it yours | Settings: the theme picker flips from Eclipse to Daybreak to Chalkboard, then back to the play screen in the new theme. End on the orb, idle. | visual_themes |

The poster frame (what shows before playback) is from beat 4: the orb in
totality.

## How it is captured

1. Install the submitted build on the iPhone 17 Pro Max and iPad Pro 13"
   simulators, fresh install, age gate passed, Daily streak and theme
   counters seeded through the debug seam so the themes are earned. Nothing
   is faked on screen.
2. Run `xcrun simctl io <device> recordVideo --codec h264` for each beat; the
   beats are scripted with the simulator control taps so takes are repeatable.
3. Cut, speed up beat 4, and scale to the store sizes with ffmpeg; check
   each export with ffprobe (resolution, fps, length 15 to 30 s).

## Open for Eric

- V1. Is the beat order right, or should languages (beat 5) come earlier?
- V2. Beat 3 shows a miss. Some apps keep previews all wins; I think the
  amber diff is one of the best things to show. Keep it?
- V3. One preview, or a second one later for Spell Jr?
- V4. The voice is audible for beats 1 and 2 only and the game sounds after.
  OK, or keep the voice throughout?

Final capture waits for the build that carries the CC-ECLIPSE-HOTFIX fixes,
because the assets must match the build that is submitted.
