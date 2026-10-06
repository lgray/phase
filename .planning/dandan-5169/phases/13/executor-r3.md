# Phase 13 executor report r3 (fail-closed per-seat FreeReveal)

Mode: implementation/fix (phase mode). BASE_SHA = 9ca36e7c; START_SHA = 25606d19; IMPLEMENTATION_WORKTREE = wt-dandan. Start clean, HEAD == START_SHA; end: HEAD unchanged, nothing staged, 12 modified paths. All evidence PREPARATORY.

## Step 0 (r2 finding reproduced at START_SHA)
New test v8 written first and run against the unmodified engine: `dandan_free_reveal::v8_...` FAIL "legal_actions_full must not carry a seat-specific FreeReveal" (P0 0/7, P1 3/4). `get_legal_actions_js` returns `legal_actions_full(state)` verbatim, so a non-qualifying local seat received the action. Log p13r3-base.log. Probe is at the engine entry the wasm export wraps, not through a wasm instance.

## Cut
- Engine: no unscoped enumerator carries FreeReveal (removed from the `candidate_actions_exact` Declare arm). One predicate, `mulligan::free_reveal_offered_to(state, seat)` (seat's own pending entry through `free_reveal_offered`), has two emitters, both seat-scoped:
  - `with_viewer_actions(state, viewer, actions)` (`ai_support/mod.rs`, replaces the r2 filter `actions_visible_to_viewer`; no filter+emitter pair): appends FreeReveal. Used by `legal_actions_for_viewer` and phase-server `legal_actions_for_seat`.
  - `candidate_actions_for_semantic_owner_with_probe` (candidates.rs): pushes the candidate for the owner. This is the single enumerator under `AiDecisionContract::issue`, `validated_candidate_actions_for_semantic_owner` and `build_decision_context_for_semantic_owner`.
- AI walk (population: every `AiDecisionContract::issue` / `validated_candidate_actions_for_semantic_owner` / `choose_action*` caller in engine-wasm get_ai_*, phase-ai search/auto_play, server-core `run_ai_actions`, phase-llm): all obtain candidates through the owner-scoped enumerator, so phase-ai needed no edit; keep > FreeReveal > Powder > Mulligan unchanged. Tests `mulligan_*`/`fallback_takes_*`/`ai_pair_leaves_*` green.
- phase-server: five per-seat sends already go through `legal_actions_for_seat`; it now calls `with_viewer_actions`. No logic added.
- engine-wasm `get_legal_actions_js` unchanged (seat-agnostic, now carries no seat-only action); one `//` comment. No wasm doc comment touched, so no .d.ts regen.
- Client: local seat = `PLAYER_ID` (`constants/game.ts`), which `resolveLocalSeat`/`getPlayerId()` returns for every mode that constructs a WasmAdapter (`seatSource` = "seat-zero": ai, local, p2p-host). Cut at the in-browser engine boundary: `WasmAdapter.getLegalActions/getSnapshot/resumeRestoredGameState/resumeMultiplayerHostState` pass `PLAYER_ID`; worker messages, `EngineWorkerClient` methods and the main-thread fallback runtime take `viewerId` and call `get_legal_actions_for_viewer_js`. Nothing in the client reads `get_legal_actions_js` any more. The client passes the seat and renders; no filtering.
  - gameStore atomic pair and p2p host own seat need no edit: gameStore calls `adapter.getSnapshot()` (the WasmAdapter method above); `P2PHostAdapter.getLegalActions/getSnapshot` call `this.wasm.*` (same WasmAdapter) or `nativeBridge` (per-seat WebSocketAdapter sockets to the sidecar server, i.e. the phase-server per-seat sends). gameStore.ts and p2p-adapter.ts are unmodified.
  - Hot-seat check: `local` is seat-zero in `GAME_MODE_TRAITS`, no navigation sets `mode=local`, and `gameLoopController` starts no controller for a second local seat: one local recipient per surface. Nothing to stop on.
  - Behavior note: the single-player legal-actions list is now empty while seat 0 is not acting (as online/P2P-guest already are), where the unscoped export returned the acting AI seat's list. Client adapter/stores/game vitest suites (91 files, 2027 tests) green.

## Walk (population command + predicate)
`rg -n "legal_actions_full|legal_actions_for_viewer|get_legal_actions_js|get_legal_actions_for_viewer_js|with_viewer_actions" crates` minus tests/ and engine-internal ai_support; and `rg -n "getLegalActions\b|get_legal_actions_js|get_legal_actions_for_viewer_js|getLegalActionsForViewer" client/src` minus tests. Predicate: a site that hands a legal-action list to a seat must get FreeReveal only from `with_viewer_actions`/the owner enumerator. Result: crates: engine-wasm `legal_actions_result_for_viewer` (scoped), manabrew-compat (scoped, Dandan refused), phase-server 5 sends (scoped via helper), `preview.rs`/`analysis/resource.rs` (viewer-scoped, unaffected), server-core/phase-ai internal `legal_actions_full` uses are all-seat consumers that no longer see FreeReveal (crew_timing, payment_selection, mulligan policy, benches: none read it). Client: wasm-adapter + worker (edited), p2p host (delegates), ws/draft/replay adapters (server per-seat or empty).

## Tests (red on revert, shown)
- Engine `dandan_free_reveal::v8_the_free_reveal_is_only_in_the_lists_scoped_to_its_own_seat`: unscoped `legal_actions_full`, `legal_actions`, `candidate_actions`, `candidate_actions_exact`, `validated_candidate_actions` lack FreeReveal (reach: each holds Keep); viewer P0 has it, viewer P1 not; AI contract for P0 has it, P1 not (Keep in both). v7 and `offered` now read the viewer list. candidates.rs unit tests moved to the owner enumerator plus an all-seat negative.
- phase-server `free_reveal_reaches_only_the_seat_whose_hand_qualifies`: all-seat list holds Keep and not FreeReveal; per-seat sends as r2.
- Mutants (one env-gated build, control NONE 176/176 green): MA unscoped exact arm re-emits -> 5 red (candidates unit, phase-server, v1, v7, v8); MB owner enumerator emits nothing (AI loses it) -> 6 red (candidates unit, v8, four phase-ai mulligan/fallback/pair tests); MC `with_viewer_actions` emits nothing -> 8 red (phase-server, v1/v2/v5/v6/v7/v8 ...). Restored: `grep P13R3` = 0. Log p13r3-mut.log.
- vitest (one per routed surface): `engine-worker.test.ts` (4 messages answer from `get_legal_actions_for_viewer_js(viewerId)`, seat-agnostic export not called), `engine-worker-client.test.ts` (4 requests post the viewer id), `wasm-adapter.test.ts` (worker path passes `PLAYER_ID` on all four; main-thread fallback reads the viewer export 4x, never `get_legal_actions_js`). Mutants (worker uses unscoped export; fallback uses unscoped export; adapter drops `PLAYER_ID`; client drops `viewerId`): 9 red; restored (grep 0, suites 101/101).

## PREPARATORY results
fmt (rustfmt on the 6 changed .rs paths, --check rc 0); `cargo clippy --workspace --all-targets -D warnings` rc 0; nextest phase-engine+phase-ai+server-core+phase-server+engine-wasm+manabrew-compat: 36684 run, 36683 passed, 1 failed `phase-ai search::tests::untapped_fetchland_outscores_passing_on_its_own_turn` (13 s deadline-scored search under full-suite load, 25.2 vs 34.0; passes in isolation 3.0 s; no mulligan path); `check-interaction-bindings.sh --check` rc 0; `tsc -b --noEmit --force` rc 0 (live control: planted type error reported); eslint on the 6 touched TS files rc 0; vitest adapter+stores+game 91 files green. Logs p13r3-*.log. Serialized shape unchanged (no type/enum/action/state edit); no protocol constant touched.

## Production-path coverage map
| claim | seam | entry | test | flips on revert | siblings |
|---|---|---|---|---|---|
| unscoped lists never carry FreeReveal | exact arm | `legal_actions_full`, `legal_actions`, `candidate_actions*`, `validated_candidate_actions` | v8, candidates unit | MA | Keep present in each |
| viewer list carries it for the qualifying seat only | `with_viewer_actions` | `legal_actions_for_viewer` | v7, v8 | MC | both/neither qualify (v7) |
| AI still issues it for its own seat | owner enumerator | `AiDecisionContract::issue` / `build_decision_context_for_semantic_owner` | v8, phase-ai `mulligan_*`, `fallback_*`, `ai_pair_*` | MB | non-qualifying owner none |
| server per-seat send | `legal_actions_for_seat` | `build_state_update_message` | phase-server test | MC | other 4 sends share the helper (GameStarted + 3 async broadcasts not directly tested, as r2) |
| client local seat uses the per-viewer call | WasmAdapter, worker, worker client, fallback | adapter methods / worker onmessage | three vitest files | 4 client mutants | - |

## Maintainer-simulation matrix
| seam | entry / first branch | authority | bound value, when | mode | storage | consumers | invalidation | hostile fixtures | serde |
|---|---|---|---|---|---|---|---|---|---|
| FreeReveal emission | `free_reveal_offered_to`: `waiting_for` MulliganDecision, pending entry with `entry.player == seat` | the seat's own entry, hand, count, phase | per call | live predicate | none | `with_viewer_actions`, owner enumerator | non-pending seat / BottomCards / count>0 / non-Dandan format: not offered | v7, v8, candidates unit | none |
| client seat | `WasmAdapter.*` -> `PLAYER_ID` | `GAME_MODE_TRAITS.seat == "seat-zero"` | each read | live | none | gameStore commit, p2p host | non-acting seat 0: empty list | wasm-adapter vitest | none |
No row incomplete, no DEFERRED.

## CR-annotation diff gate
UNVERIFIED count 0. Added: CR 103.5 (`grep -n "^103.5" docs/MagicCompRules.txt`; mulligan procedure) in the owner-enumerator comment.

## Judgement calls
- Owner enumerator (`candidate_actions_for_semantic_owner_with_probe`) is the "viewer-scoped function for the AI's own seat": it is the owner-scoped seam every AI entry already uses, so phase-ai/engine-wasm/server-core needed no change. `get_legal_actions_js` kept, unscoped; no client reader remains.
- Client seat comes from `PLAYER_ID` at the adapter boundary, not threaded from gameStore through ~15 `getSnapshot()` call sites; same value `getPlayerId()` resolves for every WasmAdapter mode, no new derivation.
- Serum Powder: NOT covered by this cut. Powder candidates stay in the exact arm (owner-tagged, all-seat in unscoped lists, pre-existing). Moving them is the same shape (~15 lines: emit from `serum_powders_in_hand(viewer)` in both emitters) but changes shipped-format behavior and its tests; not widened.

## Stop-and-return
None. Deviation from the brief: gameStore.ts, p2p-adapter.ts untouched (see Cut). Pre-authorized extra paths: client/src/adapter/__tests__/{engine-worker,engine-worker-client,wasm-adapter}.test.ts.

## Risks
Single-player store `legalActions` is now empty during AI turns; `viewerInteraction`/`autoPassRecommended`/`activationBlockReasons` for the single-player surface now come from the viewer-scoped entry too (they were the active-player/unscoped variants). A native-ai/real-browser Dandan mulligan was not driven end to end.
