# TEST-REPORT — Web E2E (app build)

**124/124 passed** across 26 areas.

## playhub — 12/12
- ✅ hub: opens from the meta corner and renders registry tiles
- ✅ hub: word_stories is never rendered (F8 hard gate)
- ✅ hub: coming_soon renders as a non-tappable teaser
- ✅ home: the empty chains line reads as a sentence, not a column
- ✅ hub: tiles are localized with no new copy (es)
- ✅ hub: scrolling the hub never scrolls the page behind it
- ✅ hub: A2.3 — Full-only gating (UNPROVABLE while no live mode is Full-tier)
- ✅ hub: A2.2 — Little Speller sees only kidSafe tiles, zero upsell
- ✅ placement: offered once, skip stands, no re-offer
- ✅ placement: Try serves the set through the real session
- ✅ placement: the offer never interrupts a live run
- ✅ placement: a COMPLETED probe never re-offers, across a reload

## hub-row — 2/2
- ✅ hub row: The Climb is out, the rest stay
- ✅ hub row: the mode is untouched and the board still routes

## keyboard — 6/6
- ✅ keyboard[en]: keys visible with hit area at SE
- ✅ keyboard[en]: rapid-fire 15 keys drops nothing
- ✅ keyboard[en]: metrics hold at 320x568
- ✅ keyboard[en]: metrics hold at 375x667
- ✅ keyboard[en]: metrics hold at 390x844
- ✅ keyboard[en]: metrics hold at 428x926

## gameplay — 2/2
- ✅ gameplay[en]: correct answer is accepted
- ✅ gameplay[en]: wrong answer is rejected

## modes — 4/4
- ✅ daily: entering shows progress bar + locks language
- ✅ daily: deterministic per date+language (seam pool stable)
- ✅ h2h: start then quit mid-game returns to clean solo state
- ✅ agegate: no stored verdict shows the DOB prompt cold

## menu — 8/8
- ✅ menu: every language shows its own endonym
- ✅ menu: selecting ko switches UI chrome
- ✅ menu: selecting ja switches UI chrome
- ✅ menu: selecting zh switches UI chrome
- ✅ menu: selecting fil switches UI chrome
- ✅ rtl F1: play surface carries the WORD's language + dir from the registry
- ✅ rtl F4: the hint carries the word's direction and counts letters, not codepoints
- ✅ rtl F4: the hint count is translated, not hardcoded English

## settings-effects — 16/16
- ✅ settings_effect_big_text
- ✅ settings_effect_readable
- ✅ settings_effect_kid
- ✅ settings_effect_kid_levels_en
- ✅ settings_effect_kid_levels_ko
- ✅ settings_effect_kid_locked
- ✅ settings_effect_extra_attempt
- ✅ settings_effect_slow_rate
- ✅ settings_effect_volume_gain
- ✅ settings_effect_remind
- ✅ settings_effect_word_stories
- ✅ settings_effect_syllable_replay
- ✅ btn_hide_actually_hides
- ✅ photo_split_proposes_and_never_auto_applies
- ✅ spell_jr_shows_no_prices
- ✅ spellit_guide_shows_once_and_can_be_replayed

## front-door — 8/8
- ✅ front_door_no_credential_field_before_a_verdict
- ✅ front_door_opens_for_13_plus_and_can_be_walked_past
- ✅ front_door_never_opens_for_under_13
- ✅ front_door_never_meets_an_existing_install
- ✅ front_door_fields_wear_the_app_styling
- ✅ front_door_password_checklist_is_live
- ✅ front_door_is_the_only_sign_in_surface
- ✅ front_door_refuses_a_locked_junior_even_programmatically

## jr-climb — 2/2
- ✅ jr_climb_run_steps_to_medium_at_word_21_once
- ✅ jr_climb_leaderboard_guard_has_a_control

## translate-screen — 9/9
- ✅ translate_state_matrix_holds_the_action_row_still
- ✅ translate_no_committable_miss
- ✅ translate_no_target_answer_declines_with_a_way_out
- ✅ translate_script_primacy_and_d1_default
- ✅ translate_rtl_mirrors_the_card_not_the_spine
- ✅ translate_save_keeps_the_target_word
- ✅ translate_spell_it_serves_or_declines_honestly
- ✅ translate_spell_jr_gets_the_screen
- ✅ translate_audio_unavailable_says_so_inline

## sayit — 2/2
- ✅ say-it: launcher hidden by default (flag off = zero diff)
- ✅ say-it: still hidden with flag ON but no native bridge (not iOS)

## ghost — 1/1
- ✅ ghost: live pace marker appears and shows a delta on a Climb run

## spellaloud — 2/2
- ✅ spell-aloud: mic hidden by default (flag off = zero diff)
- ✅ spell-aloud: still hidden with flag ON but no native bridge (not iOS)

## submit-advance — 12/12
- ✅ A1: exactly one visible submit control at each width (320→1280)
- ✅ A2: the submit control sits in-viewport and never overlaps the input
- ✅ A3: submit fires exactly once — via button click and via Enter
- ✅ A4: Daily(en) correct → next word auto-loads within DELAY+500ms
- ✅ A5: orb-click ~300ms in advances immediately by exactly 1
- ✅ A6: 5 rapid orb-clicks advance by exactly 1 (no double-skip)
- ✅ A7: incorrect answer never auto-advances (wait 3×DELAY)
- ✅ A8: final word → results after DELAY; score/streak == a skip-advance run
- ✅ A9: language matrix — es is coming-soon-gated; en async path proves agnosticism
- ✅ A10: open IME composition blocks validation until compositionend
- ✅ A11: daily sequence is deterministic for a fixed date+language
- ✅ A12: solo orb-skip — the next word is not replaced when the old timer fires

## attempts-shields — 8/8
- ✅ flag OFF (explicit): extra-attempts row + shield HUD hidden
- ✅ flag ON (build-54 default): extra-attempts row revealed + toggle defaults OFF
- ✅ 3-try RETIRED: one wrong submission is immediately the miss (word revealed)
- ✅ extra-attempts ON: a wrong submission grants exactly one clean retry
- ✅ flag ON: extra-attempts toggle persists across reload (single-source pref)
- ✅ flag ON in en: toggle revealed (identical test code)
- ✅ flag ON in es: toggle revealed (identical test code)
- ✅ flag ON in ja: toggle revealed (identical test code)

## tools-hub — 4/4
- ✅ renders: Tools section + all six rows + populated hints
- ✅ toggle flips localStorage flag and persists across reload
- ✅ flipping ghost off/on changes the tool visibility in a Climb run
- ✅ Kid Mode simplifies the hub: dark/owner rows hidden, play aids kept

## finale — 6/6
- ✅ finale: reveal rests and advances at 320x568
- ✅ finale: reveal rests and advances at 375x667
- ✅ finale: reveal rests and advances at 390x844
- ✅ finale: reveal rests and advances at 428x926
- ✅ finale: rest holds a full minute — no timer may advance it
- ✅ finale: the build replays on request

## economics — 3/3
- ✅ economics: starter misses are free and unlimited
- ✅ economics: an advanced miss dims the earned stroke until it lands
- ✅ economics: an expert miss charges the stroke; erase-to-recover restores it

## audio-gate — 3/3
- ✅ audio gate: starter keeps Replay AND the slow voice
- ✅ audio gate: advanced keeps Replay but loses Slow
- ✅ audio gate: expert SHOWS Replay — F5 supersedes the listening ladder

## finale-relaunch — 1/1
- ✅ finale: layer 1 of orion survives the kill — layer 2 unlocked on relaunch

## metadata-audit — 2/2
- ✅ metadata: the keepsake carries no EXIF, no text, no learner data
- ✅ metadata: the card carries no EXIF, no text, no learner data

## kid-reveal — 2/2
- ✅ kid reveal: Share is gone, Save stays
- ✅ kid reveal control: an adult sees both actions

## picker-continue — 3/3
- ✅ in-progress lives on the tile, and the resume strip is gone
- ✅ contextual long-press: a run offers Remove, a plain tile stars
- ✅ families: shelves render; learn leads with the active script

## finale-pixels — 2/2
- ✅ finale-pixels: export raster == play frame for dog
- ✅ finale-pixels: export raster == play frame for eiffel

## platform — 3/3
- ✅ platform[app]: English always plays
- ✅ platform[app]: non-English follows the platform
- ✅ platform[app]: every language is still LISTED

## gallery — 1/1
- ✅ gallery: trophies appear per completion, in-progress stays off the shelf
