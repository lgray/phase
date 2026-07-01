# PR-3 implementation log (wt-combo-pr3)

Branch `feat/combo-detect-pr3` off main `5eca83b8c` (v0.7.0). Sole writer.

## Scope (from team lead + PR3-PLAN round 2 + PR3-REVIEW-r2)
1. §7 resource.rs `project_out_resources`: positional stack-id canonicalization (modulo layer only).
2. §8 loop_check.rs `live_mandatory_loop_winner(start,end,delta)->Option<PlayerId>` pub(crate).
3. sba.rs:308 `player_has_cant_lose` fn -> pub(crate). (player_has_cant_win already pub.)
4. §9 engine.rs `no_living_player_has_meaningful_priority_action(state)`.
5. §10 wiring in run_auto_pass_loop (after strict-draw return, before window push).
6. Promote DRIVEN_ROW_INDICES += 18 (and 17 iff Step-0 Sanguine auto-resolves).
7. loop_check.rs:12-19 module doc update.

## Fix-constraints (review r2)
- LOW-1: DROP CR 810.8a from §8 firewall comment. Keep CR 101.2 + 104.2b + 104.3b.
- LOW-2: §7 comment enumerate in-kind compared fields (trigger_event/subject_match_count/die_result) = fail-safe.
- LOW-3: one-line worst-case perf note at §10 find_map.
- LOW-4: doc-debt update lands in this change.

## File locations (this worktree)
- resource.rs: crates/engine/src/analysis/resource.rs ; project_out_resources @588, return @722, ObjectId imported @30.
- loop_check.rs: crates/engine/src/analysis/loop_check.rs ; detect_loop @135-173; tests mod @289; module doc 1-38.
- engine.rs: crates/engine/src/game/engine.rs ; priority_player_has_meaningful_action @453; run_auto_pass_loop @1095; strict draw block @1188-1224 (strict if return @1208, window push @1220-1223).
- sba.rs: crates/engine/src/game/sba.rs
- static_abilities.rs: crates/engine/src/game/static_abilities.rs (player_has_cant_win already pub)
- corpus_tests.rs: crates/engine/src/analysis/corpus_tests.rs ; CORPUS rows 17/18/19 @238/245/252; DRIVEN_ROW_INDICES @1927; confirmed_drivers_match_expected @1943; bucket doc @1908.

## Verified facts
- normalize_for_loop (game_state.rs:7754) zeros only revision/timestamp/next_object_id/next_pip_id + dirty flags; PRESERVES players(life/library/poison)/objects/stack(raw ids). => snapshot(prior_normalized).life is correct.
- GameState::eq (7822) compares self.stack==other.stack (incl entry.id @4813), stack_paid_facts (7835), pending_trigger_entry (7863). stack_trigger_event_batches NOT in eq.
- StackEntry derives PartialEq/Eq; fields id/source_id/controller/kind. TriggeredAbility.kind carries source_id/ability/condition/trigger_event/description/source_name/subject_match_count/die_result.
- project_out_resources only called by loop_states_equal_modulo_resources (resource.rs:561). Strict loop_states_equal compares normalize_for_loop directly => untouched by §7. CONFIRMED.

## Progress / FINDINGS

### Implemented (all compile; unit tests PASS)
- §7 project_out_resources stack-id canon (+ U-stack) — modulo comparator PROVEN to match same-phase cascade states (`loop_states_equal_modulo_resources = true`).
- §8 live_mandatory_loop_winner (+ U1-U10, U-draw) — all pass.
- §9 no_living_player_has_meaningful_priority_action (+ U-gate) — pass.
- §10 wiring in run_auto_pass_loop.
- sba.rs player_has_cant_lose -> pub(crate).
- loop_check.rs module doc updated.

### STEP-0 MEASUREMENT (the non-vacuity proof) — BLOCKER FOUND
Cascade shapes (idx 18): BP trigger=LifeGained(valid_target Controller)->LoseLife{Fixed1, scope Opponent} NON-TARGETED;
Conqueror trigger=LifeLost(opp)->GainLife{EventContextAmount} NON-TARGETED. idx17 Sanguine has a TARGET on LoseLife.
- Manual passes (no session): cascade SELF-SUSTAINS, stack=1/window, trigger_event byte-STABLE (P0:+1 / P1:-1 each cycle),
  subject_match_count=None, die_result=None, stack_paid_facts EMPTY, stack_trigger_event_batches EMPTY, pending_trigger=None. (§7 side-map "leave as-is" = no-op match CONFIRMED.)
- BUT `apply(SetAutoPass{UntilStackEmpty})` does NOT drive the cascade to the PR-3 site: the UntilStackEmpty session is
  REMOVED on the first cascade resolution because the resolution TRANSIENTLY empties the stack and
  `finish_completed_or_interrupted_until_stack_empty_sessions` removes any session whose `stack.is_empty()`.
  run_auto_pass_loop BREAKS at iteration 1 (`break: no requester`); mandatory_iters never reaches 4 (let alone FINGERPRINT_AFTER_ITERS=32).
  Terminal = Priority{P0}, NO GameOver, NO ResolutionHalted. Held even when a REAL trigger (not the GainLife seed) is on the stack before SetAutoPass.

### ROOT CAUSE / PLAN-VS-CODE CONTRADICTION
PR-3 hooks run_auto_pass_loop's fingerprint block (needs a single apply() to sustain >=32 mandatory iters).
Self-refilling same-controller trigger cascades CANNOT sustain a session there (UntilStackEmpty dies on transient empty;
UntilEndOfTurn: opponent sees controller's trigger -> opponent_on_stack -> Finish). Production drives these via
`resolve_all_fast_forward` (engine-wasm/src/lib.rs:1314) = repeated single-step apply()s with its OWN stack_resolution_limit /
ResolutionHalted — NOT run_auto_pass_loop. So PR-3's live win is UNREACHABLE for its target loops. §7 makes the comparator
ABLE to match, but the comparison site is never reached. No existing integration test reaches run_auto_pass_loop's
strict CR 104.4b draw block via a real cascade either (consistent).

### CONSEQUENCE for tests
L1 FAILS (Priority, no GameOver). L-neg-A / L-neg-B PASS but VACUOUSLY (PR-3 never fires regardless). Discriminating-test gate FAILS for the live path.

### DECISION: STOP-AND-RETURN to orchestrator (plan contradicts code; do not improvise a drive/placement redesign).
Cleaned up debug cruft; kept sound unit-tested building blocks + §10 (flagged unreachable); did NOT promote DRIVEN_ROW_INDICES.
