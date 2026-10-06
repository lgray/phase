# Phase 13 executor report r2 (FreeReveal viewer scoping)

Mode: implementation/fix (phase mode). BASE_SHA = 9ca36e7c; START_SHA = 64a7cbf6; IMPLEMENTATION_WORKTREE = wt-dandan. Start clean at START_SHA; end: HEAD unchanged, nothing staged, delta = 2 paths, both in scope.nul. All evidence PREPARATORY.

## Step 0 (premise re-measured; one part of the brief is FALSE)
- Leak reproduced: `legal_actions_for_viewer(P1)` contained FreeReveal (v7 red with the filter disabled).
- Engine acceptance: `apply` passes the submitter straight to `handle_mulligan_decision(actor)`, which matches `entry.player == actor` (no controller remap), so the literal viewer==entry.player test IS the engine's predicate; turn control cannot apply at mulligan.
- Callers: `legal_actions_for_viewer` is used by engine-wasm (get_legal_actions_for_viewer_js, P2P host) and manabrew-compat only. **phase-server and server-core do NOT use it**: they call `legal_actions_full` once and fan the same Vec to every `is_acting` seat. Fan-out sites in `crates/phase-server/src/main.rs` (out of scope.nul): `build_game_started_message` (`legal_actions: if is_actor {legal_actions}`), the StateUpdate builder (`legal_actions.clone()`), the AI-run broadcast (`player_legals`, ~7192), the snapshot broadcast (~7331), and the post-action broadcast (~7898). All-seat enumeration (`candidate_actions`, `legal_actions`, phase-ai search, server-core session AI/broadcast snapshots) legitimately keeps the full list.

## Change
- `crates/engine/src/ai_support/mod.rs`: new `pub fn actions_visible_to_viewer(state, viewer, actions) -> Vec<GameAction>` drops `MulliganDecision{FreeReveal}` unless the viewer's own pending entry satisfies `mulligan::free_reveal_offered`; `legal_actions_for_viewer` applies it. Grouped map unaffected (FreeReveal has no source_object). Powder untouched.
- `crates/engine/tests/integration/dandan_free_reveal.rs`: `v7_a_viewer_is_offered_the_free_reveal_only_for_their_own_hand`.

## Discrimination
v7 entry: `legal_actions_for_viewer` via both pending viewers. Reach-guards: both viewers' lists hold Keep; `legal_actions_full` (all-seat) still holds FreeReveal; P0 (0/7) sees it, P1 (3/4) does not and `apply(P1,FreeReveal)` errs; both-qualify -> both see; neither -> neither. Revert (`if false && !offered`): v7 FAIL at the P1-not-offered assertion, other 8 green; restored (grep 0, diff = 2 files).

## Preparatory results
fmt (rustfmt on the 2 paths); `cargo clippy --workspace --all-targets -D warnings` rc 0; nextest phase-engine+phase-ai+server-core+engine-wasm+manabrew-compat 36369 passed / 33 skipped; `check-interaction-bindings.sh --check` rc 0; vitest GamePage.freeRevealMulligan 4 passed. No serialized-shape change. CR gate: only CR 103.5 added, exists (mulligan procedure). Logs: p13r2-test.log, p13r2-prep.log.

## Scope extension (approved): phase-server
- `crates/phase-server/src/main.rs`: new `legal_actions_for_seat(state, player, &[GameAction])` = empty unless `is_acting`, else `engine::ai_support::actions_visible_to_viewer` (one engine call per acting seat; no FreeReveal knowledge in phase-server). Used at all five per-seat sends: `build_game_started_message`, `build_state_update_message`, AI-run broadcast (`player_legals`, kept `is_last` gate), snapshot broadcast, post-action broadcast (kept `ai_results.is_empty()` gate). Re-grep of `legal_actions` in phase-server/server-core/lobby-broker shows no other per-viewer send (spectator/other sites send empty). All-seat enumerations (server-core session snapshots, AI search, `candidate_actions`) stay full.
- Test `state_transport_derived_tests::free_reveal_reaches_only_the_seat_whose_hand_qualifies` (Dandan, P0 0/7, P1 3/4, real `build_state_update_message` + the helper). Reach-guards: all-seat list holds FreeReveal; Keep reaches both seats; P0 receives FreeReveal. Revert of the StateUpdate site to `if is_actor { legal_actions.clone() }`: test FAIL (P1 receives FreeReveal); restored (grep 0). Not directly tested: the GameStarted builder (needs a `GameSession`) and the three async broadcast sites; all route through the same helper.
- Preparatory (final tree): rustfmt --check ok; `cargo clippy --workspace --all-targets -D warnings` rc 0; nextest phase-server+phase-ai+server-core+engine-wasm+manabrew-compat: one failure, `phase-ai search::tests::prospective_fetch_choice_survives_to_the_real_search_prompt` (fetch-land search timing, no mulligan path), passes in isolation (4.3 s) under no load; engine integration `dandan`: 150 passed; earlier full phase-engine run (before the phase-server edit; engine unchanged since): 36369 passed. Logs p13r2-prep2.log, p13r2-flaky.log.
- Delta paths: ai_support/mod.rs, dandan_free_reveal.rs, phase-server/src/main.rs (all in scope.nul). HEAD == START_SHA, nothing staged.

## Stop-and-return
None. Powder leak unchanged (pre-existing).
