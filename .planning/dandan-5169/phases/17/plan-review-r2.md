# Phase 17 plan review, round 2 (delta round, phase-plan mode, phase-fit context declared)

Reviewer model: Sonnet 5.5. Reviewed the delta `plan.r1.md` -> `plan.md` (word-level diff) plus everything it references, against the charter Phase 17 entry, `addenda/phase-17`, `phases/6/plan.md` (section 3.5) and `phases/9/plan.md` (class F rows). Code anchors re-measured at W HEAD `4e141877ca` plus the uncommitted Phase 6 work in W (`deck_loading.rs`, `game_state.rs`).

Verdict: CLEAN. 0 blocking findings (0 behavior, 0 text, 0 machinery). The delta has zero design findings, so the whole-artifact pass was also run: no finding. F1-F7 are all closed.

## F1-F7 closure

- F1 (behavior) closed. Section 8 "Provisioned fixture" plus V8/V9 now build the state through `load_deck_into_state` with a list for BOTH seats on a Dandan state; the reach-guard asserts one pool at P0, `players[1].library` empty, `library_of(P1) == library_of(P0)` non-empty, `deck_pool_of(P1)` that pool; step 0 carries the P-provision probe and the documented fallback. Checked against code, not prose: in W's working tree `load_deck_into_state` gates pool push and `load_player_library` on `zone_storage_seat(Zone::Library, seat) == seat` (matches Phase 6 plan section 3.5 bullet), `deck_pool_of(seat)` resolves through `zone_storage_seat` (`game_state.rs`), `load_player_library` takes `DeckEntry` values and needs no database, `DeckPayload`/`DeckEntry` are `pub` with `CardFace`-built entries (`determinize.rs` `deck_entry` and `session.rs` test calling `load_deck_into_state` are real precedents). V9's `GameScenario::new_with_format`, `runner.state_mut()`, `run_ai_actions_bounded`, and `AiActionResult` (no actor field, `state` field present, `AiActionsRun` derefs/iterates) all exist as the row uses them.
- F2 closed: section 0.1 and step 0 now name the five plus the four `planner/mod.rs` class-F hits; Phase 9 plan row 4 classes `planner/mod.rs` `quick_state_hash`/`search_position_hash` as F; `count_reads_lines.py HEAD '\.(library|graveyard)\b(?!\s*\()'` still lists 12 phase-ai hits, split as the plan says.
- F3 closed: V1 hostile and Standard legs use 1e-9; the two `== 0.0` assertions stay exact.
- F4 closed: `grep -n 'home/user\|worker-env\|Disk\|Memory\|CARGO_BUILD_JOBS' plan.md` returns nothing; sections 11/12 carry the common.md prefix and the `ai-gate.log` path in W. `data/card-data.json`, `suite-baseline.json`, and the `ai-gate` alias exist in W.
- F5, F6 (a to d), F7 closed with the supplied replacement text; the Standard preservation derivations are now listed in section 9.

## Residual non-blocking notes

- V9's parenthetical "(both seats drew from it)" is not asserted by the pile-shrank check alone (one seat's draw also shrinks it); the per-seat action assertion carries the liveness claim. Wording only, the executor may tighten.
- The MEASURED line "a scratch probe ... gave one pool at P0" is an unrerun claim; the code read above supports it, and step 0's P-provision probe re-establishes it at PHASE_BASE.

## Probe limits

No live engine run: the only prebuilt engine integration binary in `target-dandan` predates Phase 6's uncommitted `dandan_shared_pile_storage` test (filter returned 0 of 9236, an inert instrument), W is mid-Phase 6 with another process building, and this review is read-only. F1's provisioning claim is therefore established by reading the working-tree gate in `load_deck_into_state` and `deck_pool_of` against Phase 6 plan section 3.5, not by execution.

## Pre-existing

None new (r1's two entries stand and are not repeated).
