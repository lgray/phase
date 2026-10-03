   Compiling phase-engine v0.101.0 (/home/lgray/vibe-coding/dandan-run/wt-dandan/crates/engine)
warning: function `put_on_top` is never used
   --> crates/engine/tests/integration/dandan_shared_pile_storage.rs:244:4
    |
244 | fn put_on_top(state: &mut GameState, seat: PlayerId, cards: &[ObjectId]) {
    |    ^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `phase-engine` (test "integration") generated 1 warning
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 15s
────────────
 Nextest run ID b48d8963-5401-47d7-9f30-a151f343a25e with nextest profile: default
    Starting 19 tests across 1 binary (9236 tests skipped)
        FAIL [   0.081s] ( 1/19) phase-engine::integration dandan_shared_pile_storage::v14_wasm_boot_guard_reads_the_library_accessor
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v14_wasm_boot_guard_reads_the_library_accessor ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v14_wasm_boot_guard_reads_the_library_accessor

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 0.01s

  stderr ───

    thread 'dandan_shared_pile_storage::v14_wasm_boot_guard_reads_the_library_accessor' (187396) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:217:5:
    assertion failed: production.contains("seats_with_empty_library(")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        PASS [  32.879s] ( 2/19) phase-engine::integration dandan_shared_pile_storage::v1_pile_stays_in_the_lowest_seat_when_another_seat_starts
        PASS [  34.595s] ( 3/19) phase-engine::integration dandan_shared_pile_storage::v2_fixed_list_resolves_against_the_real_database
        FAIL [  35.796s] ( 4/19) phase-engine::integration dandan_shared_pile_storage::v15_surveil_keeps_cards_on_top_of_the_pile_for_the_non_canonical_seat
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v15_surveil_keeps_cards_on_top_of_the_pile_for_the_non_canonical_seat ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v15_surveil_keeps_cards_on_top_of_the_pile_for_the_non_canonical_seat

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 35.54s

  stderr ───

    thread 'dandan_shared_pile_storage::v15_surveil_keeps_cards_on_top_of_the_pile_for_the_non_canonical_seat' (187395) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:670:5:
    assertion `left == right` failed
      left: [ObjectId(1), ObjectId(2)]
     right: [ObjectId(2), ObjectId(1)]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [  35.972s] ( 5/19) phase-engine::integration dandan_shared_pile_storage::v7_conjure_into_the_library_lands_in_the_pile_only
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v7_conjure_into_the_library_lands_in_the_pile_only ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v7_conjure_into_the_library_lands_in_the_pile_only

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 35.66s

  stderr ───

    thread 'dandan_shared_pile_storage::v7_conjure_into_the_library_lands_in_the_pile_only' (187387) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:469:9:
    conjured into the top eight at random, got slot 10
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [  41.002s] ( 6/19) phase-engine::integration dandan_shared_pile_storage::v4_opening_deal_draws_both_hands_from_the_pile
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v4_opening_deal_draws_both_hands_from_the_pile ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v4_opening_deal_draws_both_hands_from_the_pile

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 40.77s

  stderr ───

    thread 'dandan_shared_pile_storage::v4_opening_deal_draws_both_hands_from_the_pile' (187394) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:290:5:
    assertion `left == right` failed
      left: 73
     right: 66
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [  41.970s] ( 7/19) phase-engine::integration dandan_shared_pile_storage::v15_scry_choice_reorders_the_pile_for_the_non_canonical_seat
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v15_scry_choice_reorders_the_pile_for_the_non_canonical_seat ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v15_scry_choice_reorders_the_pile_for_the_non_canonical_seat

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 41.70s

  stderr ───

    thread 'dandan_shared_pile_storage::v15_scry_choice_reorders_the_pile_for_the_non_canonical_seat' (187385) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:725:5:
    assertion `left == right` failed: kept on top
      left: ObjectId(1)
     right: ObjectId(2)
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [  42.112s] ( 8/19) phase-engine::integration dandan_shared_pile_storage::v5_v6_either_seat_cycles_from_and_into_the_shared_zones
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v5_v6_either_seat_cycles_from_and_into_the_shared_zones ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v5_v6_either_seat_cycles_from_and_into_the_shared_zones

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 41.98s

  stderr ───

    thread 'dandan_shared_pile_storage::v5_v6_either_seat_cycles_from_and_into_the_shared_zones' (187384) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:383:9:
    assertion `left == right` failed: PlayerId(1): the cycling cost was paid into the shared graveyard
      left: []
     right: [ObjectId(7)]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        PASS [  43.199s] ( 9/19) phase-engine::integration dandan_shared_pile_storage::v15_library_placement_by_the_zone_pipeline_targets_the_pile
        PASS [  44.100s] (10/19) phase-engine::integration dandan_shared_pile_storage::v1_non_shared_formats_keep_per_seat_libraries
        FAIL [  45.126s] (11/19) phase-engine::integration dandan_shared_pile_storage::v1_boot_loads_one_pile_into_the_canonical_seat
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v1_boot_loads_one_pile_into_the_canonical_seat ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v1_boot_loads_one_pile_into_the_canonical_seat

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 44.93s

  stderr ───

    thread 'dandan_shared_pile_storage::v1_boot_loads_one_pile_into_the_canonical_seat' (187389) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:112:5:
    assertion `left == right` failed
      left: 2
     right: 1
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [  46.006s] (12/19) phase-engine::integration dandan_shared_pile_storage::v8_zone_change_commands_record_and_replay_pile_positions
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v8_zone_change_commands_record_and_replay_pile_positions ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v8_zone_change_commands_record_and_replay_pile_positions

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 45.93s

  stderr ───

    thread 'dandan_shared_pile_storage::v8_zone_change_commands_record_and_replay_pile_positions' (187404) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:526:13:
    assertion `left == right` failed: appended to the container's end
      left: Some(ObjectId(7))
     right: Some(ObjectId(10))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [  46.945s] (13/19) phase-engine::integration dandan_shared_pile_storage::v8_library_shuffle_acts_on_the_pile_and_replays
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v8_library_shuffle_acts_on_the_pile_and_replays ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v8_library_shuffle_acts_on_the_pile_and_replays

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 46.77s

  stderr ───

    thread 'dandan_shared_pile_storage::v8_library_shuffle_acts_on_the_pile_and_replays' (187383) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:563:5:
    assertion `left != right` failed: the pile order changed
      left: [ObjectId(1), ObjectId(2), ObjectId(3), ObjectId(4), ObjectId(5), ObjectId(6)]
     right: [ObjectId(1), ObjectId(2), ObjectId(3), ObjectId(4), ObjectId(5), ObjectId(6)]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        PASS [  47.363s] (14/19) phase-engine::integration dandan_shared_pile_storage::v10_pool_resolver_maps_both_seats_to_the_pile_holder
        FAIL [  48.395s] (15/19) phase-engine::integration dandan_shared_pile_storage::v3_mulligan_by_either_seat_shuffles_the_pile
  stdout ───

    running 1 test
    test dandan_shared_pile_storage::v3_mulligan_by_either_seat_shuffles_the_pile ... FAILED

    failures:

    failures:
        dandan_shared_pile_storage::v3_mulligan_by_either_seat_shuffles_the_pile

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9254 filtered out; finished in 48.26s

  stderr ───

    thread 'dandan_shared_pile_storage::v3_mulligan_by_either_seat_shuffles_the_pile' (187390) panicked at crates/engine/tests/integration/dandan_shared_pile_storage.rs:329:9:
    assertion `left == right` failed: PlayerId(1)
      left: 73
     right: 66
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        PASS [  48.583s] (16/19) phase-engine::integration dandan_shared_pile_storage::v15_dig_choice_reorders_the_pile_for_the_non_canonical_seat
        PASS [  50.371s] (17/19) phase-engine::integration dandan_shared_pile_storage::v5_v6_standard_format_cycles_in_the_owners_own_zones
        PASS [  21.785s] (18/19) phase-engine::integration dandan_shared_pile_storage::v9_standard_shuffle_keeps_the_other_players_library_knowledge
        PASS [  25.206s] (19/19) phase-engine::integration dandan_shared_pile_storage::v9_library_boundary_by_either_seat_forgets_the_looked_at_pile
────────────
     Summary [  58.092s] 19 tests run: 9 passed, 10 failed, 9236 skipped
        FAIL [   0.081s] ( 1/19) phase-engine::integration dandan_shared_pile_storage::v14_wasm_boot_guard_reads_the_library_accessor
        FAIL [  35.796s] ( 4/19) phase-engine::integration dandan_shared_pile_storage::v15_surveil_keeps_cards_on_top_of_the_pile_for_the_non_canonical_seat
        FAIL [  35.972s] ( 5/19) phase-engine::integration dandan_shared_pile_storage::v7_conjure_into_the_library_lands_in_the_pile_only
        FAIL [  41.002s] ( 6/19) phase-engine::integration dandan_shared_pile_storage::v4_opening_deal_draws_both_hands_from_the_pile
        FAIL [  41.970s] ( 7/19) phase-engine::integration dandan_shared_pile_storage::v15_scry_choice_reorders_the_pile_for_the_non_canonical_seat
        FAIL [  42.112s] ( 8/19) phase-engine::integration dandan_shared_pile_storage::v5_v6_either_seat_cycles_from_and_into_the_shared_zones
        FAIL [  45.126s] (11/19) phase-engine::integration dandan_shared_pile_storage::v1_boot_loads_one_pile_into_the_canonical_seat
        FAIL [  46.006s] (12/19) phase-engine::integration dandan_shared_pile_storage::v8_zone_change_commands_record_and_replay_pile_positions
        FAIL [  46.945s] (13/19) phase-engine::integration dandan_shared_pile_storage::v8_library_shuffle_acts_on_the_pile_and_replays
        FAIL [  48.395s] (15/19) phase-engine::integration dandan_shared_pile_storage::v3_mulligan_by_either_seat_shuffles_the_pile
error: test run failed
   Compiling phase-engine v0.101.0 (/home/lgray/vibe-coding/dandan-run/wt-dandan/crates/engine)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3m 10s
────────────
 Nextest run ID 6e653280-8d45-4490-b3cd-c40f54e30fbc with nextest profile: default
    Starting 5 tests across 1 binary (23140 tests skipped)
        FAIL [   0.037s] (1/5) phase-engine game::engine_resolution_choices::tests::nth_from_top_pair_reads_the_shared_pile_not_the_empty_seat
  stdout ───

    running 1 test
    test game::engine_resolution_choices::tests::nth_from_top_pair_reads_the_shared_pile_not_the_empty_seat ... FAILED

    failures:

    failures:
        game::engine_resolution_choices::tests::nth_from_top_pair_reads_the_shared_pile_not_the_empty_seat

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23144 filtered out; finished in 0.00s

  stderr ───

    thread 'game::engine_resolution_choices::tests::nth_from_top_pair_reads_the_shared_pile_not_the_empty_seat' (213739) panicked at crates/engine/src/game/engine_resolution_choices.rs:13897:13:
    assertion `left == right` failed: shared=true
      left: [ObjectId(12), ObjectId(11)]
     right: [ObjectId(11), ObjectId(12)]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [   0.047s] (2/5) phase-engine game::engine_resolution_choices::tests::mixed_owner_top_batch_is_one_pass_over_the_shared_pile
  stdout ───

    running 1 test
    test game::engine_resolution_choices::tests::mixed_owner_top_batch_is_one_pass_over_the_shared_pile ... FAILED

    failures:

    failures:
        game::engine_resolution_choices::tests::mixed_owner_top_batch_is_one_pass_over_the_shared_pile

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23144 filtered out; finished in 0.00s

  stderr ───

    thread 'game::engine_resolution_choices::tests::mixed_owner_top_batch_is_one_pass_over_the_shared_pile' (213744) panicked at crates/engine/src/game/engine_resolution_choices.rs:13908:9:
    assertion `left == right` failed: the first chosen card ends on top of the one pile
      left: [ObjectId(11), ObjectId(12)]
     right: [ObjectId(12), ObjectId(11)]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [   0.047s] (3/5) phase-engine game::engine_resolution_choices::tests::library_origin_reposition_sees_one_pile_across_owners
  stdout ───

    running 1 test
    test game::engine_resolution_choices::tests::library_origin_reposition_sees_one_pile_across_owners ... FAILED

    failures:

    failures:
        game::engine_resolution_choices::tests::library_origin_reposition_sees_one_pile_across_owners

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23144 filtered out; finished in 0.00s

  stderr ───

    thread 'game::engine_resolution_choices::tests::library_origin_reposition_sees_one_pile_across_owners' (213743) panicked at crates/engine/src/game/engine_resolution_choices.rs:13938:9:
    assertion `left == right` failed: a P1-owned card is placed third from the top of the one pile
      left: [ObjectId(1), ObjectId(2), ObjectId(3), ObjectId(4), ObjectId(5), ObjectId(6), ObjectId(7), ObjectId(8), ObjectId(9), ObjectId(10)]
     right: [ObjectId(1), ObjectId(2), ObjectId(8), ObjectId(3), ObjectId(4), ObjectId(5), ObjectId(6), ObjectId(7), ObjectId(9), ObjectId(10)]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        FAIL [   0.048s] (4/5) phase-engine types::game_state::shared_zone_storage_tests::library_stamp_names_the_storage_seat
  stdout ───

    running 1 test
    test types::game_state::shared_zone_storage_tests::library_stamp_names_the_storage_seat ... FAILED

    failures:

    failures:
        types::game_state::shared_zone_storage_tests::library_stamp_names_the_storage_seat

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23144 filtered out; finished in 0.01s

  stderr ───

    thread 'types::game_state::shared_zone_storage_tests::library_stamp_names_the_storage_seat' (213741) panicked at crates/engine/src/types/game_state.rs:44022:13:
    assertion `left == right` failed
      left: PlayerId(1)
     right: PlayerId(0)
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

        PASS [   0.047s] (5/5) phase-engine types::game_state::shared_zone_storage_tests::storage_seat_resolves_only_the_shared_zones
────────────
     Summary [   0.092s] 5 tests run: 1 passed, 4 failed, 23140 skipped
        FAIL [   0.037s] (1/5) phase-engine game::engine_resolution_choices::tests::nth_from_top_pair_reads_the_shared_pile_not_the_empty_seat
        FAIL [   0.047s] (2/5) phase-engine game::engine_resolution_choices::tests::mixed_owner_top_batch_is_one_pass_over_the_shared_pile
        FAIL [   0.047s] (3/5) phase-engine game::engine_resolution_choices::tests::library_origin_reposition_sees_one_pile_across_owners
        FAIL [   0.048s] (4/5) phase-engine types::game_state::shared_zone_storage_tests::library_stamp_names_the_storage_seat
error: test run failed
done rc=100
