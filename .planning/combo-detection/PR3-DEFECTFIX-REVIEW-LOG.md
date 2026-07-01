# PR3 Defect-Fix Review — investigation log

Adversarial review of `PR3-PLAN-DEFECTFIX.md`. Findings-only. Re-measuring every load-bearing claim against actual code in `/home/lgray/vibe-coding/wt-combo-pr3`.

## Make-or-break questions
1. Is `SimulationFilter::accept` the ONLY nested-apply re-entry path into `reconcile_terminal_result`?
2. Does `resolved_this_beat` correctly sample resolutions while leaving handoffs intact across all cases?

## Status: IN PROGRESS

### Setup verified
- Worktree branch `feat/combo-detect-pr3`, HEAD `5eca83b8c` (v0.7.0). Diff is UNCOMMITTED across 6 files, +635 lines. Matches IMPL-REPORT.

## Findings (running)
- MED-1: priority.rs §1.3 deletion → clippy let_and_return; plan claim ":99-106 pre-PR-3" FALSE (git HEAD = trailing expr).
- MED-2: §3-at-:200 auto-pass coverage conditional (reconcile only at :196/:200, not inside run_auto_pass_loop).
- LOW-1: §7 canon cited ~:742, actually resource.rs:751.

## Re-measured CORRECT (running)
- MB#1: SimulationFilter::accept (filter.rs:113) is the ONLY production nested-apply reentry. Exhaustive scan done.
- MB#2: resolved_this_beat sound — fresh monotonic ObjectId (triggers.rs:3894); handoff leaves stack untouched (priority.rs:136-147).
- §7 canon on CLONE (resource.rs:719 project_out_resources returns new GameState); live ids untouched.
- §8 live_mandatory_loop_winner present w/ single-faller + cant-lose/cant-win + WinKind::LethalDamage.
- thread_local precedent confirmed: perf_counters:19, speed:140, quantity:1029, layers:1369.
- apply path synchronous (no .await), no rayon/par_iter/thread::spawn.
- loop_detect_ring serde-skip + manual-eq excluded (eq at game_state.rs:7869, ring absent).
- reconcile_terminal_result production callers ONLY engine.rs:196/200.
- handle_priority_pass_with_limit ONLY non-test caller = engine.rs:455 (priority.rs tests start :189).
- 3 drivers confirmed: 1252 (auto-pass), 1742 (per-beat arm), 4630 (SetAutoPass); resolve_all_fast_forward routes via 1742.
- All CR numbers grep-verify.
- CR 104.4b DRAW block uses STRICT loop_states_equal (life-sensitive) → never fires on net-progress drain → no draw/win conflict.
