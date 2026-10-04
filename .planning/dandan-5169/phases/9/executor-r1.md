# Phase 9 executor report r1 (S2b-2 read sweep: analysis / ai_support / cross-crate)

MODEL: claude-sonnet-5-5. Mode: implementation/fix, phase mode. Not committed.
BASE_SHA = START_SHA = `e35fc63606a4ca32ed108a36cfbbe1a0d143445b`. IMPLEMENTATION_WORKTREE = `/home/lgray/vibe-coding/dandan-run/wt-dandan`.

## Verdict
Sweep done; every S/S-guard row red at base and green after; the F pins are green at base and red under a swap; workspace clippy clean. No stop-and-return item. One plan premise measured false and replaced in scope (section 8).

## 1. Diff summary
- engine: `analysis/resource.rs` (`snapshot`, `seat_headroom_bound` gains `state: &GameState`, four test call sites forced; tests V1, V2, V3), `ai_support/candidates.rs` (two graveyard loops, `card_name_choice_candidates`), `ai_support/mod.rs` (graveyard loop), `types/game_state.rs` (test only, V4).
- phase-ai: `combo/detection.rs`, `policies/fetch_land_patience.rs` (helper `verdict_for_seat` added to its tests), `determinize.rs` (`unknown_slots`: library by `library_of`, owner filter dropped on library slots only), `planner/mod.rs` (test only, V9).
- phase-llm: `render/game.rs` (`push_players`, `push_graveyards`, test V12).
- tests: new `crates/engine/tests/integration/dandan_analysis_ai_support_reads.rs` (5 rows) + `mod` line; fixture `integration_cards.json.gz` regenerated (`gen-test-fixture.py --check`: ok, 4965 cards).
- `git diff --shortstat -- crates`: 11 files, +440 / -39 (plus the new untracked test file).

## 2. Worktree record
Start: HEAD == START_SHA, clean, nothing staged. End: HEAD unchanged, 0 staged entries. Changed set (tracked diff + untracked) is byte-equal to `phases/9/scope.nul` (sha256 `13066eb02fd6a8d51c20a7067685aab3914cac14566299247775fddf38fc5498`). Step 0 at START_SHA: accessors present; census equals the plan table (engine-wasm lists no read; manabrew-compat check unchanged: one production `PreparedManabrewSnapshot {` constructor; `InGraveyard` has no constructor; no Phase 4b-8 plan names `determinize.rs`). All results below are PREPARATORY, not completion evidence.

## 3. Preparatory results (isolated env, no Tilt)
- `cargo clippy --workspace --all-targets -- -D warnings`: rc 0 (`phases/9/clippy.log`). `cargo fmt` on the exact changed `.rs` paths; `--check` clean.
- Default fixture mode (no `FORGE_TEST_FULL_DB`): all 15 new rows pass (`green-final.log`, after a rebuild of the restored tree).
- V14: engine `analysis|ai_support|types::game_state` lib tests, all phase-ai and phase-llm tests, integration filter `dandan|ai_decision|ability_cost_block|candidate|loop|graveyard_permission|erratic|bending`: 4527/4527 pass (`v14.log`). Not the full suite.
- V13 census (snapshot of the working tree through `count_reads_lines.py`): non-`game/` residual = `game_state.rs` 8 (Phase 6 accessors + `loop_fingerprint`), `resource.rs` 4 (C1b residual), `planner/mod.rs` 4 (hashes), and the five Phase 17 files (`deck_knowledge`, `mill_targeting`, `payoff`, `search`, `zone_eval`). Exactly the plan's F and P17 sets.

## 4. Parser gate
No `parser/` path touched; not applicable.

## 5. Production-path coverage map
Base-red evidence: all S/S-guard swaps reintroduced raw in one mutation (`mut-base.log`): 10 of 15 rows red; the five green are the designed controls (V3 pin, V4, V9, the per-seat determinize control, the empty-pile (d) row). F-site swap mutation (`mut-fpins.log`): V3, V4, V9 red, everything else green. Owner-filter mutation on the library slots (`mut-owner.log`): `unknown_slots_select_the_shared_pile_by_storage` red.

| Claim / seam | Production entry | Test | Assertion that flips |
|---|---|---|---|
| `snapshot` per-seat library size | `ResourceVector::snapshot` | `snapshot_reads_the_shared_pile_for_every_seat` | `library_delta[P1] == 5` (0 at base); Standard control (3, 5) |
| `seat_headroom_bound` | `elimination_bounds` -> `seat_headroom_bound(state, ..)` | `seat_headroom_reads_the_shared_pile_for_every_seat` | P1 `Some(5)` (`Some(0)` at base); Standard `Some(4)`. Private fn, called directly (unit) |
| graveyard non-mana activation | `candidate_actions` | `v5_a_graveyard_activated_ability_is_offered_from_the_pile` | Blessed Ghoul offered for P1 exactly when payable; P0/Standard legs are the controls; empty-pile row `v5_an_empty_pile_graveyard_offers_no_activation` |
| graveyard mana ability | `candidate_actions` | `v6_a_graveyard_mana_ability_is_offered_from_the_pile` | Jack-o'-Lantern offered for P1 |
| block read-out graveyard loop | `activation_block_reasons` | `v7_the_block_read_out_covers_the_pile_graveyard` | Bloodsoaked Champion entry `[CostNotPayableNow]` with no mana, absent and offered with mana |
| card-name candidates | WaitingFor::NamedChoice -> `candidate_actions` | `v8_card_name_candidates_come_from_the_pile` | names `[Island, Brainstorm]`, not the fallback; prompt reach asserted |
| combo piece | `piece_present` | `graveyard_piece_reads_the_shared_pile_from_either_seat` | true for P1 over the pile; absent seat false |
| fetch patience | `FetchLandPatiencePolicy::verdict` | `untapped_fetchland_reads_the_shared_pile_from_either_seat` | preferred iff the pile holds a land, for P1 and P0; dry pile not preferred |
| render | `render_board` over `filter_state_for_viewer` | `the_shared_pile_renders_for_every_seat_through_the_viewer_filter` | "3 cards in library", "You: Memory Lapse" (0 / empty at base); filtered state keeps the pile |
| unknown slots | `unknown_slots` | `unknown_slots_select_the_shared_pile_by_storage`, `..._in_a_per_seat_format_stay_per_seat` | `[hand, mine, theirs]` for both seats; red at base and under the owner-filter mutation; token and `known` card skipped; borrowed hand card present (reach) |
| F: C1b residual | `certify_instructed_opponent_library_departure` | `certificate_residual_compares_stored_containers` | stray id in the non-holder container refuses; red under accessor swap |
| F: fingerprint | `loop_fingerprint` | `loop_fingerprint_folds_the_shared_pile_once` | Standard-vs-Dandan equal; red under swap; library and graveyard reach-guards |
| F: planner hashes | `quick_state_hash`, `search_position_hash` | `position_hashes_fold_the_shared_pile_once` | equal across the format flip; red under swap; order/graveyard reach-guards |

No test is shape-only; the unit rows (resource, determinize, detection, fetch, planner, game_state) are unit because the seams are private, hash-valued, or reached only through a resampling path whose pool lookup is Phase 17.

## 6. Maintainer-simulation matrix (rows condensed; all hostile fixtures place the acting seat's card behind the other seat's in a mixed-owner pile)
- Selected authority: the acting/read seat; bound live from the format axis inside `library_of`/`graveyard_of` at every call (no latch). Storage: the canonical seat's container. Consumers: the listed functions. Invalidation: none (reads). Serde/protocol/card-data: none changed; fixture regenerated for card names only.
- Owner gates kept: graveyard activation loops keep `obj.owner == player` (CR 602.2, CR 108.4); other-owned pile card activation is `DEFERRED(phase 11)`. No row asserts it.
- `unknown_slots`: hand filter keeps its owner test; library slots drop it (CR 401.2/401.3). End-to-end resampling of the pile for the non-canonical opponent `DEFERRED(phase 17)`.
- Loop-analysis end to end for Dandan after the `snapshot` swap: not claimed (plan 4.2).
- Receiving hands `DEFERRED(phase 11)`.

## 7. CR gate
Added CRs: 104.3c, 121.4, 400.1, 401.2, 401.3, 605.1a; all present in `docs/MagicCompRules.txt`, zero `UNVERIFIED`. Subjects checked against the rule text (104.3c/121.4 decking headroom, 400.1 zones, 401.2/401.3 hidden library and public count, 605.1a mana ability).

## 8. Plan premises found false (replaced inside scope)
1. V7: `activation_block_reasons` only reports `CostNotPayableNow`; a failed activation restriction (raid) yields no entry. The row now meets raid with a real attack-declaration record (`attacker_declarations_this_turn`, not `players_attacked_this_turn`) and asserts `[CostNotPayableNow]` without mana, absent-and-offered with mana.
2. V8 needs blue mana for Predict; pool is Black, Blue, Colorless.
3. V10/V11/V12/V15/V3 built with `GameState::new(FormatConfig, 2, seed)` and `create_object` (routed), as planned; V3 uses the routed pile rather than `cert_library_card`.

## 9. Judgement calls
- Tests for `loop_fingerprint` landed in the existing `shared_zone_storage_tests` module of `game_state.rs` (the tail of the file) rather than a new module.
- The determinize doc comment was rewritten to state the library/hand asymmetry.

## 10. Stop-and-return items
None.

## 11. Risks
- Cargo mtime trap during mutation runs: restoring with `cp -p` leaves a mutated artifact "fresh"; the first owner-filter run saw a stale engine (fpins) and its engine failures are void; the owner-filter conclusion rests on the phase-ai row only, and the final green run followed a forced rebuild.
- `v14.log` is a filtered regression, not the full suite.
