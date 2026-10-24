# TEST-REPORT — Web E2E (app build)

**522/522 passed** across 63 areas.

## drawer — 21/21
- ✅ every drawer row presses something that can receive it (en)
- ✅ every drawer row presses something that can receive it (ar)
- ✅ every drawer row presses something that can receive it (Jr)
- ✅ drawer: opens from the meta corner and renders registry rows
- ✅ drawer: every mode row has a destination that exists
- ✅ drawer: word_stories is never rendered (F8 hard gate)
- ✅ drawer: a hidden mode cannot be flagged back on
- ✅ home: the empty chains line reads as a sentence, not a column
- ✅ drawer: rows are localized with no new copy (es)
- ✅ drawer: scrolling the panel never scrolls the page behind it
- ✅ drawer: A2.3 — no locks, no upsell (Full-tier gating still UNPROVABLE)
- ✅ drawer: A2.2 — Little Speller sees only kidSafe rows, zero upsell
- ✅ placement: offered once, skip stands, no re-offer
- ✅ placement: Try serves the set through the real session
- ✅ placement: the offer never interrupts a live run
- ✅ placement: a COMPLETED probe never re-offers, across a reload
- ✅ learner: a NEWER build's stored state survives an older build, byte for byte
- ✅ learner: unreadable stored bytes are backed up exactly, then play recovers
- ✅ learner: a stored v1 state migrates to v2 on the next write, losing nothing
- ✅ learner surfaces render through LearnerQuery: Reports, Stats, Guardian Dash, Calendar
- ✅ learner: "reset this language" deletes its learner state and backup, only for that language

## stats-sheet — 1/1
- ✅ stats: the guardian section and Guardian Dash door survive boot and a stats render

## review-queue — 3/3
- ✅ review: a miss enters the queue on the review rule, due now
- ✅ review: a pre-R2 queue entry carries over and keeps its due time
- ✅ review: a correct answer after a replay grades Hard and comes back sooner than a clean one

## inspector — 1/1
- ✅ inspector: shows the review queue, next-review dates and why the word was served

## audio-rescue — 4/4
- ✅ rescue: offers only the steps that exist
- ✅ rescue: showing the word voids the round and leaves nothing behind
- ✅ rescue: sends nothing
- ✅ rescue: revealing everything scores nothing and hits its cap

## hub-inventory — 5/5
- ✅ hub: nothing above the orb but the brand, the burger and the session pill
- ✅ hub: A2 — no quick-play tile and no Misses chip renders
- ✅ hub: A3 — the burger is 88x44, top-trailing, centred on the wordmark
- ✅ hub: A7 — the burger frame does not move with the Misses count
- ✅ hub: A12 — Spell Jr is never nudged, at any miss count

## drawer-close — 12/12
- ✅ close: T1 — the bottom X dismisses at every scroll offset (SE)
- ✅ close: T1 — the bottom X dismisses at every scroll offset (16)
- ✅ close: T1 — the bottom X dismisses at every scroll offset (ProMax)
- ✅ close: T2 — nothing closes the drawer from its top half
- ✅ close: T4 — a scrim tap dismisses
- ✅ close: T5 — 56x56, 16pt inside the trailing edge (SE)
- ✅ close: T5 — 56x56, 16pt inside the trailing edge (16)
- ✅ close: T5 — 56x56, 16pt inside the trailing edge (ProMax)
- ✅ close: T6 — the last row scrolls clear of the footer
- ✅ close: T7 — no row draws outside the panel at the top
- ✅ close: T8 — every close path leaves the nudge watermark identical
- ✅ close: T9 — named for a screen reader, and focus goes back to the burger

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

## telemetry — 11/11
- ✅ standard_player_sends_valid_batches_with_perf_and_no_text
- ✅ jr_sends_only_one_daily_aggregate
- ✅ unknown_age_sends_only_one_daily_aggregate
- ✅ kill_switch_sends_nothing_and_clears_the_queue
- ✅ endpoint_down_is_invisible_and_keeps_the_queue
- ✅ observe_never_act_replay_is_identical_on_and_off
- ✅ endpoint_down_costs_no_round_latency
- ✅ settings_effect_telemetry
- ✅ settings_effect_telemetry_jr_needs_a_grown_up
- ✅ settings_effect_telemetry_kill_switch_overrides
- ✅ personal_data_never_reaches_a_payload

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

## mywords-lists — 18/18
- ✅ mywords_migration_keeps_every_saved_word
- ✅ mywords_migration_runs_once_across_boots
- ✅ mywords_migration_of_an_empty_set_makes_no_list
- ✅ mywords_save_sheet_offers_no_way_to_erase
- ✅ mywords_second_save_today_joins_todays_list
- ✅ mywords_choosing_new_list_leaves_the_other_alone
- ✅ mywords_save_button_follows_the_destination
- ✅ mywords_empty_screen_offers_both_doors
- ✅ mywords_cards_are_newest_first_with_counts
- ✅ mywords_open_action_lands_in_the_saved_list
- ✅ mywords_an_opened_list_can_be_fixed_and_undone
- ✅ mywords_deleting_a_list_asks_first_and_can_be_undone
- ✅ mywords_spell_jr_cannot_delete_without_a_grown_up
- ✅ mywords_playing_a_list_serves_only_its_words
- ✅ mywords_play_falls_back_when_the_chosen_list_is_deleted
- ✅ mywords_in_my_order_serves_the_players_order
- ✅ mywords_arabic_list_gets_arabic_keyboard_and_joined_letters
- ✅ mywords_browser_voice_only_language_keeps_the_english_keyboard

## pick-queue — 39/39
- ✅ pick_queue_flag_off_is_todays_app
- ✅ pick_queue_zero_picks_is_the_normal_queue
- ✅ pick_queue_misses_lineup_numbers_persists_and_serves_first
- ✅ pick_queue_unpicked_words_follow_and_picks_do_not_repeat
- ✅ pick_queue_not_due_pick_keeps_its_schedule_when_answered
- ✅ pick_queue_end_lineup_clears_it
- ✅ pick_queue_reset_this_language_clears_its_lineup
- ✅ pick_queue_my_words_lineup_numbers_persists_and_serves_first
- ✅ pick_queue_removing_a_my_words_word_unlines_it
- ✅ pick_queue_a_lineup_never_mixes_two_languages
- ✅ pick_queue_spell_jr_caps_at_five_and_refuses_the_sixth
- ✅ pick_stars_flag_off_has_no_star_control_and_writes_nothing
- ✅ pick_stars_misses_star_leads_every_session_and_persists
- ✅ pick_stars_picks_lead_then_stars_then_the_rest
- ✅ pick_stars_not_due_star_keeps_its_schedule_when_answered
- ✅ pick_stars_the_eleventh_star_is_refused_with_a_visible_message
- ✅ pick_stars_reset_this_language_clears_its_stars
- ✅ pick_stars_a_newer_blob_is_kept_byte_for_byte
- ✅ pick_stars_my_words_star_reuses_the_entry_star_and_leads
- ✅ pick_stars_my_words_eleventh_star_is_refused_with_a_message
- ✅ pick_stars_spell_jr_has_no_stars_and_ignores_persisted_ones
- ✅ pick_stars_everything_off_keeps_the_in_my_order_star
- ✅ pick_requeue_misses_missed_pick_returns_three_to_five_later
- ✅ pick_requeue_just_these_keeps_a_miss_inside_the_session
- ✅ pick_requeue_my_words_missed_pick_returns_three_to_five_later
- ✅ pick_requeue_flag_off_is_todays_app
- ✅ pick_requeue_spell_jr_missed_pick_returns
- ✅ pick_queue_rtl_misses_sheet_is_rtl_in_arabic_and_ltr_in_english
- ✅ pick_queue_rtl_my_words_footer_is_rtl_in_arabic_and_ltr_in_english
- ✅ pick_help_flag_off_is_the_phase_a_surface
- ✅ pick_help_flag_c_alone_does_nothing
- ✅ pick_help_defaults_show_todays_order_and_write_nothing
- ✅ pick_help_sort_changes_display_only_not_the_serve_order
- ✅ pick_help_search_hides_rows_but_hidden_picks_stay_lined_up_and_counted
- ✅ pick_help_search_folds_diacritics_and_non_latin_and_keeps_focus
- ✅ pick_help_toughest_five_fills_the_lineup_and_is_served_in_order
- ✅ pick_help_toughest_keeps_existing_picks_and_never_mixes_languages
- ✅ pick_help_my_words_sort_search_and_toughest
- ✅ pick_help_spell_jr_has_no_sort_search_or_toughest

## spelldoku — 35/35
- ✅ spelldoku_every_language_opens_a_board
- ✅ spelldoku_mandarin_needs_its_tone
- ✅ spelldoku_vietnamese_uses_the_tone_row
- ✅ spelldoku_spelling_commits_and_unlocks_a_chip
- ✅ spelldoku_a_board_can_be_solved_by_spelling
- ✅ spelldoku_hard_waits_for_check_board
- ✅ spelldoku_hints_never_show_a_letter
- ✅ spelldoku_spell_jr_sees_only_its_column
- ✅ spelldoku_wasm_matches_the_pinned_golden_digest
- ✅ spelldoku_daily_is_personal
- ✅ spelldoku_word_mode_never_shows_a_spelling
- ✅ spelldoku_word_mode_spells_to_commit_and_feeds_missed_words
- ✅ spelldoku_word_mode_draws_my_words_first
- ✅ spelldoku_conflict_is_refused_4_easy
- ✅ spelldoku_conflict_is_refused_9_easy
- ✅ spelldoku_conflict_is_refused_9_medium
- ✅ spelldoku_conflict_is_refused_9_hard
- ✅ spelldoku_conflict_is_refused_9_expert
- ✅ spelldoku_counts_and_dimming
- ✅ spelldoku_every_path_refuses_a_conflict
- ✅ spelldoku_symbol_pref_is_per_difficulty
- ✅ spelldoku_picker_absent_cases
- ✅ spelldoku_switching_never_disturbs_the_current_board
- ✅ spelldoku_first_letters_board_explains_itself
- ✅ spelldoku_boards_are_fresh
- ✅ spelldoku_mode_chip_names_the_symbols
- ✅ spelldoku_nine_fits_a_small_phone_numbers
- ✅ spelldoku_nine_fits_a_small_phone_words
- ✅ spelldoku_no_twelve_by_twelve
- ✅ spelldoku_word_mode_definition_card_never_spells_it
- ✅ spelldoku_tier_ladder_is_legible_from_load
- ✅ spelldoku_tier_word_commits_and_recommit_is_free
- ✅ spelldoku_tier_served_words_enter_the_repeat_window
- ✅ spelldoku_tier_pencil_marks_are_free
- ✅ spelldoku_tier_jr_never_leaves_easy_medium

## spell-search — 10/10
- ✅ spell_search_opens_in_every_launch_language
- ✅ spell_search_never_opens_in_korean_or_chinese
- ✅ spell_search_a_drag_finds_a_word
- ✅ spell_search_trap_then_lock_in_scores_in_stars_only
- ✅ spell_search_spell_jr_gets_only_jr
- ✅ spell_search_wasm_matches_the_pinned_golden_digest
- ✅ spell_search_daily_is_the_same_for_everyone
- ✅ spell_search_make_a_puzzle_from_my_words
- ✅ spell_search_meaning_never_gives_the_word_away
- ✅ spell_search_twelve_word_list_builds_fast

## spell-cross — 7/7
- ✅ spell_cross_clues_are_heard_never_read
- ✅ spell_cross_checks_on_the_last_letter
- ✅ spell_cross_finishes_on_the_hidden_word
- ✅ spell_cross_offers_spell_search_when_a_list_will_not_interlock
- ✅ spell_cross_launch_set
- ✅ spell_cross_spell_jr_gets_jr_only
- ✅ spell_cross_daily_is_the_same_for_everyone

## boardgame — 41/41
- ✅ boardgame_pass_and_play_handoff_hit_and_miss
- ✅ boardgame_pass_and_play_reaches_the_podium
- ✅ boardgame_npc_run_is_fast_and_never_shows_keys
- ✅ boardgame_landing_callout_names_tier_and_trap
- ✅ boardgame_layout_every_language_every_phone
- ✅ boardgame_setup_screen_has_top_right_exit
- ✅ boardgame_board_state_fills_height_and_exit_works
- ✅ boardgame_spell_state_exit_works
- ✅ boardgame_ja_modifier_keys
- ✅ boardgame_unrolled_track_en
- ✅ boardgame_unrolled_track_ar
- ✅ boardgame_compact_strip_when_space_is_short
- ✅ boardgame_polish_A-P11_layout_375x667
- ✅ boardgame_polish_A-P11_layout_393x852
- ✅ boardgame_polish_A-P11_layout_820x1180
- ✅ boardgame_polish_A-P11_grayscale_tiers_start_finish_revealed
- ✅ boardgame_polish_tile_numbers_are_on_the_interior_side
- ✅ boardgame_polish_A-P12_6_tile_hop_within_1200ms
- ✅ boardgame_polish_A-P12_12_tile_hop_within_2000ms
- ✅ boardgame_polish_A-P12_reduce_motion_has_no_hops_and_no_ticks
- ✅ boardgame_polish_A-P12_real_move_hops_tile_by_tile_inside_the_budget
- ✅ boardgame_polish_A-P12_tap_cancels_hops_and_the_npc_run_stays_inside_the_step
- ✅ boardgame_polish_centre_stage_die_label_and_tile_line
- ✅ boardgame_polish_tier_pill_is_the_fallback_when_there_is_no_stage
- ✅ boardgame_polish_ja_stage_and_pieces_at_430x932
- ✅ boardgame_sprint_solo_defaults_to_sprint_and_can_pick_full
- ✅ boardgame_sprint_pass_and_play_opens_on_full_and_remembers
- ✅ boardgame_sprint_spell_jr_has_no_size_picker
- ✅ boardgame_sprint_ring_fit_375x667
- ✅ boardgame_sprint_ring_fit_393x852
- ✅ boardgame_sprint_ring_fit_820x1180
- ✅ boardgame_stretch_choice_precedes_every_word_and_pays_two
- ✅ boardgame_stretch_is_never_offered_from_expert_or_in_spell_jr
- ✅ boardgame_stretch_podium_counter_and_flag_off_has_none
- ✅ boardgame_boost_streak_chips_count_then_show_the_plus_one_marker
- ✅ boardgame_boost_ward_marker_and_blocked_trap_named_in_the_stage
- ✅ boardgame_boost_effects_are_named_in_the_stage_and_resolve
- ✅ boardgame_boost_grayscale_boost_diamond_vs_trap_circle
- ✅ boardgame_boost_four_chips_with_markers_fit_375
- ✅ boardgame_boost_npc_runs_stay_inside_the_budget_and_tap_applies_final_positions_effects
- ✅ boardgame_boost_npc_runs_stay_inside_the_budget_and_tap_applies_final_positions_shipped_defaults

## spelluzzle — 44/44
- ✅ spelluzzle_exit_is_top_left_and_leaves
- ✅ spelluzzle_first_board_shows_the_explainer_once
- ✅ spelluzzle_untouched_board_shows_runes_never_letters
- ✅ spelluzzle_typing_a_word_decodes_the_others_and_a_clash_is_amber
- ✅ spelluzzle_solves_and_the_secret_is_said_once_at_the_end
- ✅ spelluzzle_layout_floor_at_375x667
- ✅ spelluzzle_no_red_anywhere
- ✅ spelluzzle_jr_gets_jr_boards_only
- ✅ spelluzzle_leaving_saves_and_playing_again_resumes
- ✅ spelluzzle_a_clip_that_will_not_load_replaces_the_board_then_says_unavailable
- ✅ spelluzzle_medium_has_1_silent_words_and_listen_costs_the_star
- ✅ spelluzzle_hard_has_2_silent_words_and_listen_costs_the_star
- ✅ spelluzzle_expert_has_3_silent_words_and_listen_costs_the_star
- ✅ spelluzzle_second_explainer_shows_once_on_the_first_silent_board
- ✅ spelluzzle_expert_layout_floor_at_375x667
- ✅ spelluzzle_v11_flags_off_is_v1
- ✅ spelluzzle_v11_pencil_marks_are_notes
- ✅ spelluzzle_v11_pencil_survives_leaving
- ✅ spelluzzle_v11_strip_counts_order_and_highlight
- ✅ spelluzzle_v11_ripple_counts_cells_and_a_tap_skips_it
- ✅ spelluzzle_v11_reduce_motion_has_no_flip_and_still_counts
- ✅ spelluzzle_v11_layout_at_375x667
- ✅ spelluzzle_v11_jr_gets_pencil_strip_ripple_with_a_flat_tick
- ✅ spelluzzle_par_is_absent_with_its_flag_off_and_for_jr
- ✅ spelluzzle_par_boards_start_silent_and_one_listen_is_one_stroke
- ✅ spelluzzle_par_solved_at_par_says_par_and_over_par_says_so
- ✅ spelluzzle_par_deduced_with_no_listens_is_under_par
- ✅ spelluzzle_par_expert_is_par_3
- ✅ spelluzzle_par_layout_at_375x667_keeps_7_rows_with_the_keyboard_up
- ✅ spelluzzle_par_resumes_and_remembers_the_board_type
- ✅ spelluzzle_expedition_is_absent_with_its_flag_off_and_for_jr
- ✅ spelluzzle_expedition_carries_the_cipher_to_board_2_and_the_vault
- ✅ spelluzzle_expedition_resumes_on_the_board_you_left
- ✅ spelluzzle_expedition_vault_layout_at_375x667
- ✅ spelluzzle_my_words_builds_a_board_from_the_list_and_badges_top_ups
- ✅ spelluzzle_my_words_needs_three_matched_words_and_leaves_the_list_alone
- ✅ spelluzzle_my_words_for_jr_has_no_sound_alikes
- ✅ spelluzzle_duel_is_absent_with_its_flag_off_and_for_jr
- ✅ spelluzzle_duel_pass_and_play_scores_claims_and_shows_each_verdict
- ✅ spelluzzle_duel_two_passes_end_the_leg_and_two_legs_end_the_match
- ✅ spelluzzle_duel_against_the_computer_takes_its_turn_and_never_blocks_you
- ✅ spelluzzle_duel_hard_layout_at_375x667_with_the_keyboard_up
- ✅ spelluzzle_duel_expert_layout_at_375x667_with_the_keyboard_up
- ✅ spelluzzle_duel_resumes_a_match_in_progress

## randospell — 42/42
- ✅ randospell_exit_is_a_word_and_leaves
- ✅ randospell_the_die_shows_the_length_of_the_spoken_word
- ✅ randospell_a_right_answer_scores_the_roll_and_a_miss_scores_nothing
- ✅ randospell_sprint_ends_at_the_goal_with_stats_a_histogram_and_a_way_out
- ✅ randospell_reduce_motion_removes_the_shake_and_keeps_the_number
- ✅ randospell_is_absent_for_a_language_with_no_bank_for_it
- ✅ randospell_marathon_is_offered_only_with_its_flag
- ✅ randospell_marathon_right_answer_fills_the_pot_and_asks_bank_or_roll
- ✅ randospell_marathon_the_ward_saves_one_pot_then_a_miss_empties_it
- ✅ randospell_marathon_ends_only_on_a_bank_at_the_goal
- ✅ randospell_daily_and_h2h_are_offered_only_with_their_flags
- ✅ randospell_daily_is_the_same_game_for_everyone_and_one_attempt
- ✅ randospell_h2h_a_miss_hands_over_with_a_bonus_roll_and_a_bank_wins
- ✅ randospell_jr_is_off_without_its_flag
- ✅ randospell_jr_is_off_for_swahili_and_filipino
- ✅ randospell_jr_one_die_no_pot_no_ward_no_timer
- ✅ randospell_jr_a_miss_replays_the_word_for_one_more_try
- ✅ randospell_jr_two_misses_move_on_and_the_end_card_has_six_faces
- ✅ randospell_a15_a_full_run_changes_only_its_own_storage
- ✅ randospell_a9_the_info_sheet_lists_a_remap_where_one_exists
- ✅ randospell_d10_preview_owns_the_first_tier_and_an_unviable_bank_is_absent
- ✅ randospell_v11_the_choices_come_between_the_die_and_the_word
- ✅ randospell_v11_the_guess_toggles_and_locks_on_the_press
- ✅ randospell_a22_stretch_is_chosen_before_the_word_and_draws_a_tier_up
- ✅ randospell_a23_ear_folds_the_replay_away
- ✅ randospell_v11_heat_climbs_with_each_unbanked_word_and_resets
- ✅ randospell_a24_doubles_glow_and_pay_double_in_sprint
- ✅ randospell_table_is_offered_only_with_its_flag
- ✅ randospell_table_the_setup_picks_count_personalities_and_difficulty
- ✅ randospell_table_a_miss_hands_the_turn_to_the_computer_and_back
- ✅ randospell_table_a_human_bank_past_100_wins_the_table
- ✅ randospell_jr_table_is_one_cautious_racer_with_no_pot
- ✅ randospell_g_a_goal_bank_opens_the_final_round_instead_of_ending_the_game
- ✅ randospell_g_the_final_round_banner_is_words_not_only_a_colour
- ✅ randospell_g_steal_moves_floor_half_and_leaves_the_rest
- ✅ randospell_g_i18_steal_is_never_offered_in_solo_marathon_or_sprint
- ✅ randospell_h_daily_carries_the_bets_and_never_steal_or_a_final_round
- ✅ randospell_i_fx_draws_the_state_and_fx_off_still_shows_every_number
- ✅ randospell_a33_particles_stay_under_200_and_the_canvas_is_gone_between_turns
- ✅ randospell_a16_reduce_motion_runs_no_bursts_and_no_orbit_animation
- ✅ randospell_a34_every_state_has_a_shape_and_the_numerals_have_contrast
- ✅ randospell_i_the_lantern_never_overlaps_the_title_or_the_score

## quickbee-draw — 15/15
- ✅ quickbee_draw_is_offered_in_zh_with_its_flag
- ✅ quickbee_draw_is_offered_in_ja_with_its_flag
- ✅ quickbee_draw_is_absent_in_ko_even_with_every_flag_on
- ✅ quickbee_draw_is_absent_in_vi_even_with_every_flag_on
- ✅ quickbee_draw_is_absent_in_en_even_with_every_flag_on
- ✅ quickbee_draw_is_absent_in_es_even_with_every_flag_on
- ✅ quickbee_draw_is_absent_for_zh_with_the_flags_off_and_for_spell_jr
- ✅ quickbee_draw_a_right_cell_scores_one_and_advances_with_no_confirm_button
- ✅ quickbee_draw_a3_a_wrong_second_cell_keeps_the_first_point_and_starts_the_next_word
- ✅ quickbee_draw_an_unreadable_cell_gets_one_free_redraw_per_word
- ✅ quickbee_draw_the_clock_ends_the_run_the_pad_goes_home_and_there_is_a_way_out
- ✅ quickbee_draw_a5_no_word_repeats_within_a_run
- ✅ quickbee_draw_c_the_board_is_per_language_and_labelled_drawn
- ✅ quickbee_draw_c_a_finished_run_is_placed_on_the_board_with_its_rank
- ✅ quickbee_draw_c_spell_jr_has_no_entry_no_pad_and_no_board

## impossible — 10/10
- ✅ F7: the Daily button on the Impossible level is the Impossible Daily, with its own record
- ✅ German Impossible: a capitalised noun is typed with the Shift key and counts
- ✅ Impossible Mix: a German word brings the German keyboard and its Shift
- ✅ German outside Impossible: the Shift key is there for every word
- ✅ Mix: the keyboard follows each word into its own language
- ✅ F4: a signed kill file switches Mix off after it is cached; a tampered one does nothing
- ✅ flags OFF: no Impossible entry in the level picker
- ✅ flags ON, free English: Impossible playable, Mix locked with the Complete reason
- ✅ an Impossible word: no hints, one replay, one attempt, miss filed under the tag
- ✅ an Impossible word typed right is correct (strict grader, no server)

## translate-screen — 10/10
- ✅ translate_state_matrix_holds_the_action_row_still
- ✅ translate_no_committable_miss
- ✅ translate_no_target_answer_declines_with_a_way_out
- ✅ translate_script_primacy_and_d1_default
- ✅ translate_rtl_mirrors_the_card_not_the_spine
- ✅ translate_save_keeps_the_target_word
- ✅ translate_spell_it_serves_or_declines_honestly
- ✅ translate_spell_jr_gets_the_screen
- ✅ translate_audio_unavailable_says_so_inline
- ✅ translate_cancelling_the_destination_saves_nothing

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

## wordpic-vi-tone — 1/1
- ✅ wordpic-vi: a toned Vietnamese word can be typed and lands

## spell-keyboard — 4/4
- ✅ spell-keyboard: the main keyboard is the shared one, labelled, lowercase
- ✅ spell-keyboard: the five Vietnamese tone keys are named apart
- ✅ spell-keyboard: Bee draws shared keys and offers alternates
- ✅ spell-keyboard: a German Impossible trap word in BoardGame is typed with Shift

## audio-gate — 3/3
- ✅ audio gate: starter keeps Replay AND the slow voice
- ✅ audio gate: advanced keeps Replay but loses Slow
- ✅ audio gate: expert SHOWS Replay — F5 supersedes the listening ladder

## human-audio — 4/4
- ✅ human audio: a verified clip heads the router (D2)
- ✅ human audio: switch off plays no human clip (F6)
- ✅ human audio: a clip that fails to load falls through to TTS (I6)
- ✅ human audio: slow replay is the same clip at 0.7x, pitch kept (D8)

## finale-relaunch — 1/1
- ✅ finale: layer 1 of orion survives the kill — layer 2 unlocked on relaunch

## metadata-audit — 2/2
- ✅ metadata: the keepsake carries no EXIF, no text, no learner data
- ✅ metadata: the card carries no EXIF, no text, no learner data

## kid-reveal — 4/4
- ✅ kid reveal: Share is gone, Save stays
- ✅ kid mode: every share door is hidden and the share call is refused
- ✅ kid mode share control: an adult share click reaches the share sheet
- ✅ kid reveal control: an adult sees both actions

## feedback-holds — 2/2
- ✅ bee, kid miss: the right spelling stays on screen, then the next word comes
- ✅ impostor, right answer: the green card stays up, then the next set comes

## web-speech-fallback — 1/1
- ✅ no playable clip: the browser voice speaks the word, and no failure banner shows

## play-fold — 12/12
- ✅ AT-4 390x844 with a 34 pt home-indicator inset: Check stays beside Delete and on screen
- ✅ AT-4 440x956 with a 34 pt home-indicator inset: Check stays beside Delete and on screen
- ✅ AT-4 standard 320x568: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 standard 375x667: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 standard 390x844: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 standard 440x956: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 standard 1032x1376: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 Kid Mode 320x568: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 Kid Mode 375x667: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 Kid Mode 390x844: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 Kid Mode 440x956: spellbox, keyboard, Check and the result line fit without scrolling
- ✅ AT-4 Kid Mode 1032x1376: spellbox, keyboard, Check and the result line fit without scrolling

## script-render — 3/3
- ✅ AT-6 ar: the reveal is drawn by Noto Naskh Arabic, "It was" is not serif, nothing clips
- ✅ AT-6 hi: the reveal is drawn by Noto Sans Devanagari, "It was" is not serif, nothing clips
- ✅ AT-6 vi: the reveal is drawn by Space Mono, "It was" is not serif, nothing clips

## reduce-motion — 2/2
- ✅ I4: under Reduce Motion nothing moves through a round, the orb, the drawer and settings
- ✅ I4 control: without Reduce Motion the same tour does move (the sampler is looking)

## frame-log — 2/2
- ✅ A5: the frame logger is off by default and records nothing
- ✅ A5: switched on in the dev door, it records typing and the result, and survives a reload

## birthday-months — 3/3
- ✅ B-C2: the months are in the interface language, and the value is the month number
- ✅ B-C1: every month fits its picker at 320x568 in all 15 languages
- ✅ B-C1: every month fits its picker at 375x667 in all 15 languages

## answer-feedback — 7/7
- ✅ F4 off (default): the reveal is unchanged — no rows, coral, the orb shakes
- ✅ F4 on: a miss shows You wrote / Correct, marked, in amber, and the orb dims
- ✅ F4 on: a correct answer lifts the letters, 25 ms apart, all inside 300 ms
- ✅ F4 on, Reduce Motion: no letter moves, and the orb only dims
- ✅ AT-14: the play screen draws no letter slot before input, flag on or off
- ✅ AT-13: before an answer, nothing of the spelling is on screen, flag on
- ✅ E-Q7: DefMatch, Spelldoku, Spell Cross and Impostor mark a wrong answer amber, not red, flag on

## orb-states — 7/7
- ✅ F3 off (default): no orb state, no listener, no side read of the clip
- ✅ F3 on: a slow clip goes fetching -> speaking -> awaiting, the corona follows the voice
- ✅ F3 on: a clip that answers inside 120 ms never shows fetching (no flicker)
- ✅ F3 on: correct flares, wrong dims, and a whole session refuses no transition
- ✅ AT-9: with Reduce Motion, no transform animation runs in any of the 7 states
- ✅ V-D23: the idle breath rests while the page is hidden
- ✅ AT-8 instrument: the frame report carries the orb sync line

## controls-color — 6/6
- ✅ Controls colour: the default is build 289's violet
- ✅ Controls colour: a swatch tints all four controls and survives a reload
- ✅ Controls colour: a custom white pick keeps the setup bar text readable
- ✅ Buttons colour: unset, primary buttons are the amber accent with dark text
- ✅ Buttons colour: a swatch fills Check and Done, a dark pick flips the text light, and it survives a reload
- ✅ Buttons colour: unset, the Japanese accent still reaches the buttons

## sound-haptics — 6/6
- ✅ Phase C off (default): no sound, no Settings rows, today's haptics uncapped
- ✅ settings_effect_game_sounds
- ✅ settings_effect_game_sounds_level
- ✅ settings_effect_haptics
- ✅ settings_effect_key_feel
- ✅ AT-17: a scripted word never gets more than 2 answer haptics; keys are exempt

## icons — 3/3
- ✅ F8 off (default): every icon slot shows its emoji, no drawn icon
- ✅ F8 on: every icon slot draws its icon, every drawer row has a real symbol
- ✅ AT-20: with icons on, no emoji icon is visible on the play screen or in the drawer

## celebrate — 4/4
- ✅ F7 off (default): a perfect run plays no celebration; achievements keep today's toast
- ✅ AT-18: a perfect run plays one big celebration, in the gap, under 2.5 s, holding the next word
- ✅ AT-18: a tap skips the big celebration at once
- ✅ F7 Reduce Motion: the big celebration is a calm card, no sparks

## cards — 1/1
- ✅ AT-26: every card mode x palette, and an Arabic card, is deterministic, 1080x1350 and matches its golden

## themes — 7/7
- ✅ F9 off (default): no data-theme, no Appearance rows
- ✅ AT-23: a new player sees System and five themes; locked ones show how, never a price or a store link
- ✅ AT-23: a player who already qualifies gets all three themes the first time, and they stay after a reset
- ✅ F9: System follows the OS; a chosen theme applies its tokens and survives a reload
- ✅ AT-21: every theme, standard and Jr, keeps the play screen's text readable
- ✅ F9: the Typewriter theme gives every key a keystrike sound; other themes do not
- ✅ F10: the language chip, progress fill and orb carry the study language's accent, and switching language changes it

## hotfix-contrast — 4/4
- ✅ I1/I2 eclipse: every registry screen and home state reads (axe + measured residue + boundaries)
- ✅ I1/I2 eclipse + Background #f4f1fb: every registry screen and home state reads (axe + measured residue + boundaries)
- ✅ I1/I2 daybreak: every registry screen and home state reads (axe + measured residue + boundaries)
- ✅ I1/I2 daybreak + Background #181818: every registry screen and home state reads (axe + measured residue + boundaries)

## hotfix-overlap — 8/8
- ✅ I4 375 pt, default text: no text or control overlaps another on any registry screen
- ✅ headers 375 pt, default text: Spell Picture's title, Gallery and Surprise share a row (Surprise unwrapped); Board Game's title and Exit show
- ✅ I4 375 pt, XL text: no text or control overlaps another on any registry screen
- ✅ headers 375 pt, XL text: Spell Picture's title, Gallery and Surprise share a row (Surprise unwrapped); Board Game's title and Exit show
- ✅ I4 430 pt, default text: no text or control overlaps another on any registry screen
- ✅ headers 430 pt, default text: Spell Picture's title, Gallery and Surprise share a row (Surprise unwrapped); Board Game's title and Exit show
- ✅ I4 430 pt, XL text: no text or control overlaps another on any registry screen
- ✅ headers 430 pt, XL text: Spell Picture's title, Gallery and Surprise share a row (Surprise unwrapped); Board Game's title and Exit show

## hotfix-glow — 2/2
- ✅ I5 eclipse: typed letters fill with ink >= 4.5:1 for all five glows; the glow is the halo
- ✅ I5 daybreak: typed letters fill with ink >= 4.5:1 for all five glows; the glow is the halo

## hotfix-diag — 3/3
- ✅ I8: with the switch off (default), a live capture paints no diag, on any route
- ✅ F4: with the switch on, the readout shows during a capture and goes when the mic stops or the route changes
- ✅ F4: the switch lives in the dev menu and defaults off

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
