# Phase 9 executor report r2 (fixes F1-F3)

MODEL: claude-sonnet-5-5. Mode: implementation/fix, phase mode. Not committed.
BASE_SHA = e35fc63606; START_SHA = 761f5ff216; IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Start: HEAD == START_SHA, clean, nothing staged. End: HEAD unchanged, 0 staged, unstaged delta = 4 paths (below). All results PREPARATORY, not completion evidence.

## Diff
- `analysis/loop_check.rs`: `classify_win_kind(controller, delta, state: Option<&GameState>)`; Decking arm is `n < 0 && holder(pid) != holder(controller)` with `holder = zone_storage_seat(Library, _)` (identity when `state` is `None`). Two internal callers pass `Some(cycle_end)`; new unit test `shared_pile_drain_is_advantage_per_seat_drain_is_decking`; existing test calls pass `None`.
- Compiler-forced callers: `analysis/ability_graph.rs` (2, static analysis, `None`), `game/engine.rs` (3, `Some(state)`).
- `tests/integration/dandan_analysis_ai_support_reads.rs`: V8 stages a pile graveyard card (Memory Lapse, in the DEFAULT fixture, name added to `all_card_names`) and expects `[Memory Lapse, Island, Brainstorm]`.

## F1 (red/green)
Revert of only the graveyard statement in `card_name_choice_candidates` to `controller.graveyard.iter()`: V8 FAIL (r2-red.log); restored: V8 PASS (r2-int.log, 5/5). Mutation applied in place, restored from a saved copy and touched; clippy/tests that follow ran on the restored tree.

## F2
Rules basis: CR 104.3c (grepped, "If a player is required to draw more cards than are left in their library, they draw the remaining cards and then lose the game"): under a shared pile the controller draws from the drained pile, so it is no opponent decking. Cause-level: keyed on `zone_storage_seat`, no Dandan/bool guard; per-seat formats resolve each seat to itself.
Test discrimination: with the Decking arm reverted to `*pid != controller`, the new test FAILS (r2-red.log); green with the fix. Assertions: Dandan, controllers P0 and P1, both seats -2 => Advantage (flips); reach-guards: Standard state => Decking, `None` => Decking (opponent-pile decking still classifies Decking), plus `detect_loop` on a Dandan state => Advantage. The existing Decking/self-mill tests (`None`) and the paired Standard row cover the unchanged non-shared behavior. Note: a Dandan opponent-pile decking cycle cannot exist (one pile); the reach-guard is the per-seat pair.
Not touched: the live_mandatory_loop_winner second-loss firewall (conservative `None`, outside F2).

## F3
`grep -rn "DEFERRED(phase 1[17])" crates` = 0 (the labels live only in plan.md, 7 hits as the positive control; plan/report text not edited per brief). Nothing in code to replace; no DROPPED sentence added to code.

## Checks (PREPARATORY)
- fmt: `cargo fmt --all --check -- <4 paths>` rc 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: rc 0 (r2-clippy.log).
- nextest `analysis::loop_check`: 43/43 (r2-unit.log); `-p phase-engine --lib analysis ai_support`: 747/747 (r2-unit2.log); integration `dandan loop` filter (incl. new file, default fixture mode, loop_shortcut): 390/390 (r2-int2.log).
- CR gate: only CR 104.3c cited (pre-existing text, verified present).

## Judgement calls / risks
- `Option<&GameState>` over a closure/bool: smallest signature carrying the storage authority; `None` is only the stateless static analysis (ability_graph), where seats are synthetic.
- engine.rs / ability_graph.rs are outside the Phase 9 scope list but are the compiler-forced callers the brief allows; no other path touched.
- Not run: workspace-wide nextest (only the filters above).
