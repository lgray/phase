# Phase 15 impl review r3 (delta ad382d02f3..bbce5c6bdab3, final round)
MODEL: claude-sonnet-5-5
Verdict: CLEAN. behavior 0, text 0, machinery 0.

## (1) Wheel gate (read)
`has_no_resolution_riders` reads: sub_ability, else_ability, duration, condition, optional_targeting, optional, optional_for, optional_player, target_constraints, target_choice_timing, target_selection_mode, target_chooser, repeat_for, min_x_value, cant_be_copied, forward_result, unless_pay, distribution, repeat_until, replacement_applied, sub_link, modal, mode_abilities.
Chain-position fields: sub_ability, sub_link only; `standalone_view` normalizes exactly those. Mid-chain links stay enforced by `continuation()`. `optional_player` lives once, in the shared predicate.
Census (re-run by reviewer: `jq -c -f scratch/census15r3.jq client/public/card-data.json`, output scratch/census15r3-rerun.out): 10 printed wheel heads; r1_split true 10 (control nonzero); final_accept true 10; r1-accepted-but-final-refused: []. Heads carrying SequentialSibling (exercise the sub_link normalization): Once More with Feeling, Time Spiral. Hostile shapes refused by the 17-row `shared_library_wheel_split_refuses_every_other_shape`.

## (2) Probes (log scratch/probe15r3.log)
- control-before: 5 tests, 5 passed.
- mut1 old gate: FAIL shared_library_wheel_split_refuses_every_other_shape (4 passed, 1 failed).
- mut2 sub_link normalization removed: FAIL shared_library_wheel_split_ignores_the_heads_own_link (4 passed, 1 failed).
- mut3 optional_player removed: FAIL has_no_resolution_riders_accepts_a_bare_effect_and_refuses_each_rider and refuses_every_other_shape (3 passed, 2 failed).
- control-after restore: 5 tests, 5 passed.

## (3) Other callers
draw.rs dealer gate and scoped_library_search delivery gate pass the ability unnormalized; the predicate delta is the added `optional_player.is_none()` conjunct, so they only refuse more (no printed wheel shape flips per the census above). Integration run (dandan, dandan_simultaneous_draw, cane): 182 passed, 0 failed.

## (4) scope_player_for_member removal / zone_axis_admits
Non-battlefield matcher is `owner == seat`. Old: shared container -> owner==owner (true), else owner==player. New: holder, then every other seat sharing holder's container; in a shared zone the owner is such a seat, in a per-seat zone none is. Same result for every non-battlefield object; battlefield passes None -> holder only, as before. Dandan Cane + Standard twin green (182/182 above).

## W state
After probes: HEAD bbce5c6bdab3c24a727d5d5f776cda093c46b15a; tracked porcelain empty.
