# Phase 14 executor report r1

Mode: implementation/fix (phase mode)
BASE_SHA = START_SHA = 2964d6818f8e0f18b4a652c21ed3fccedbedaf42
IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan

VERDICT: dealer (unit 1) and Serum Powder cut (unit 2) implemented; the five literal-4 wire assertions fixed after approval (section 9); full tree green.

## 1. Diff summary (21 modified + 2 new; delta is inside the 23-path scope, `comm` against scope.nul)
Dealer (S4c/S6):
- `types/game_state.rs`: `DrawDealer{stage,seats}`, `DrawDealerStage{Settling,Dealing}`, `DrawDealerSeat`; `DrawSequenceFrame.dealer` (`serde(default, skip_serializing_if)`), `settling_seat/begin_next_unit/dealer_deliveries`, `validate_draw_dealer`, `loop_equal` compares it; unit tests.
- `effects/draw.rs`: `consult_draw_instruction` extracted (returns `ControlFlow<DrawSequenceOutcome>`), `plan_simultaneous_draw` (gate + per-seat count in one pass), `start_simultaneous_draw`, `settle_dealer_seat`, settle step and `begin_next_unit` in the driver loop, completion via `dealer_deliveries`; unit tests.
- `effects/mod.rs`: two fan-out intercepts (layer 1 `player_scope` driver, layer 2 `resolves_for_each_target_player`), `record_player_actions_performed` and `bind_scoped_seat` extracted from the existing three-call sites.
- `effects/scoped_library_search.rs`: `has_no_resolution_riders` extracted (shared with `is_plain_parent_target_delivery`); unit test.
- `types/resolution.rs`: `RESOLUTION_STATE_WIRE_VERSION` 4 -> 5, `LEGACY_DEALERLESS_RESOLUTION_STATE_WIRE_VERSION = 4`; 4 and 5 both decode to `ResolutionWireV4`; tests.
- Comment-only: `game/quantity.rs`, `types/ability.rs`.
- Protocol: lobby-broker 112, server-core test renamed, client `PROTOCOL_VERSION` 112, `WIRE_PROTOCOL_VERSION` 94, two client tests, `scripts/check-protocol-version.mjs` +41/+40.
- Tests: new `dandan_simultaneous_draw.rs`; census row in `deterministic_game_state_serde.rs`; pin `== 5` in `exploit_object_filter.rs`; fixture regenerated (5025 cards, `--check` rc 0).

Serum Powder (unit 2):
- `game/mulligan.rs`: `serum_powders_in_hand` (moved) and `serum_powders_offered_to(state, seat)` (the one predicate: the seat's own pending entry in `Declare`).
- `ai_support/candidates.rs`: `candidate_actions_exact` Declare arm is Keep/Mulligan only; `candidate_actions_for_semantic_owner_with_probe` emits Powder (before FreeReveal) via the predicate.
- `ai_support/mod.rs`: `with_viewer_actions` emits Powder via the same predicate.
- New `tests/integration/mulligan_serum_powder_scope.rs` (+ mod line).

## 2. Worktree record
START clean, HEAD == START_SHA. End: `git rev-parse HEAD` == START_SHA, 0 staged, delta == authorized scope. Preparatory evidence is not completion evidence.

## 3. PREPARATORY results (direct cargo; `CARGO_TARGET_DIR=/home/lgray/vibe-coding/dandan-run/target-dandan`)
- `cargo fmt --all -- <17 authorized .rs>`: clean.
- `clippy --workspace --all-targets -D warnings`: rc 0 (fixed on the way: `result_large_err` -> `ControlFlow`, `useless_vec`, `type_complexity` in two unit tests).
- Final tree: `clippy --workspace --all-targets -D warnings` rc 0, then nextest `-p phase-engine -p phase-ai -p server-core -p lobby-broker -p phase-server -p manabrew-compat`: 36949 run, 36949 passed, 33 skipped (log `.planning/dandan-5169/p14-final.log`, `done rc=0`).
- `check-interaction-bindings.sh --check` rc 0; `node scripts/check-protocol-version.mjs` rc 0; `gen-test-fixture.py --check` rc 0; client vitest for the two touched protocol tests: 213 passed.

## 4. Parser gate
N/A (no parser files).

## 5. Discriminating-test coverage map
Revert-probes were in-place env-free mutants (file restored from copy, then touched); each row = mutant -> tests that went red. Unmutated tree green.
Dealer (`dandan_simultaneous_draw::*` unless noted):
| claim | seam | entry | mutant -> red |
|---|---|---|---|
| layer-1 draw deals one card at a time, active player first | `player_scope` intercept | Prosperity (v1, v1b), Alms Collector (v4 + control), Obstinate Familiar (v5), empty-pile loss rows (v6c), v7, nested v11 | intercept off -> v1, v1b, v4, v4_control, v5, v6c, v7, v11 (8 red) |
| layer-2 (targeted players) | `resolves_for_each_target_player` intercept | v3 (target_players draw twenty) | off -> v3 red (1) |
| shared-library gate | `plan_simultaneous_draw` | v2/v3 separate-library rows; unit refusal test | gate off -> v2, v3_separate, unit refusal (3) |
| seat switch saves/loads working copy | `begin_next_unit` | v1, v1b, v3, v5; unit | write-back removed -> 5 red |
| settle every seat before dealing | Settling pass | all rows | skipped -> 12 red |
| tail runs after the deal when a nested prompt parks | child-boundary insert | v11 | boundary replaced -> v11 red (1) |
| wire v5 and v4 reader | `resolution.rs` | `resolution_wire_*`, `the_dealerless_version_*`, `exploit_object_filter` pin | bump reverted -> 3 red; v4 arm removed -> 1 red |
Powder (`mulligan_serum_powder_scope`): v1 (Dandan, only P1 holds the real Serum Powder; oracle text asserted verbatim: "{T}: Add {C}.\nAny time you could mulligan and this card is in your hand, you may exile all the cards from your hand, then draw that many cards. (You can do this in addition to taking mulligans.)"), v2 (Standard twin), v3 (Standard holder's Powder mulligan applies through `apply`). Reach-guards: Keep present in every unscoped list, viewer list and AI list for both seats; both seats pending.
| mutant | red |
|---|---|
| exact enumerator re-emits Powder | v1, v2 |
| owner-scoped AI enumerator silent | v1, v2 |
| `with_viewer_actions` silent | v1, v2, v3 |
Base-red evidence for the dealer rows (V1, V1b, V3, V4(+control), V5, V6c, V7) was captured at base earlier in this run. Sequential Standard order and V6a/V3-Standard are the controls that stay as before.

## 6. Maintainer-simulation matrix
| seam | entry / first branch | authority | bound value, when | mode | storage | consumers | invalidation | hostile fixtures |
|---|---|---|---|---|---|---|---|---|
| dealer gate+count | fan-out layers -> `plan_simultaneous_draw`, branch `shared_zones().library == Shared`, >=2 seats, each seat bare `Draw`, no riders | seat = scoped player | per-seat count, at announcement of the instruction | snapshot (CR 121.2: each seat's instruction is its own) | `DrawSequenceFrame.dealer.seats` | draw driver | refused gate falls back to sequential fan-out | v2/v3 separate libraries, unit refusal rows |
| settle then deal | `resume_draw_sequence_outcome` Settling branch | replacement choices per seat | `applied` set per seat at consult | latched per seat | `DrawDealerSeat.applied/accumulated` | `begin_next_unit` | replacement choice parks the frame; resume re-enters Settling | v4, v5 (answers [1,0,1]), v11 |
| deal order | `mulligan::deal_sequence(apnap)` | active player | schedule at end of settling | latched | `DrawDealerStage::Dealing{schedule}` | driver | empty library mid-deal: shortfall seat attempts, SBA 704.5b decides | v6a/b/c, v7 |
| count table | completion in `draw.rs` + intercept re-install | seat | `last_effect_counts_by_player` at completion | latched | state field | "that many" tails | cleared by generic publish for Draw, re-installed by intercept | v4 (Alms), v11 |
| wire v5 | `ResolutionStateWire::from_value` | version stamp | at write | n/a | `resolution_state_version` | decoder | v4 payload decodes dealerless; 6 refused | resolution tests, exploit pin |
| Powder scoping | `serum_powders_offered_to`: own pending entry in `Declare` | holder seat | at enumeration | live predicate | `WaitingFor::MulliganDecision.pending` | viewer list, AI owner list | BottomCards phase or no pending entry -> none | v1/v2/v3 |
Serialized shape: `dealer` omitted when `None` (test `a_frame_without_a_dealer_writes_no_dealer_key`); Powder: no serialized shape change.

## 7. CR-annotation gate
Grep of every `CR n` in the diff + new files: zero `UNVERIFIED`. Verified: 103.5, 103.5b, 109.5, 121.2, 121.2a, 121.2c, 201.2, 608.2, 608.2c, 616.1g.

## 8. Judgement calls
- `consult_draw_instruction` and `settle_dealer_seat` return `ControlFlow<DrawSequenceOutcome>` (clippy `result_large_err`); the parked carrier is `Break`.
- The layer-1 intercept is skipped when `after_scope_needs_linked_exile` or `next_sub_needs_tracked_set` (those tails need the sequential path's tracked sets).
- Dropped the V3 "one chosen player" row (harness reports "Illegal target selected"); V11 uses a synthetic oracle "Each player draws two cards. You gain 7 life." with real Alms Collector/Obstinate Familiar, because no real card has a nested instruction after a shared-library scoped draw.
- Powder: predicate reuses the name-based `serum_powders_in_hand` moved from candidates.rs, unchanged semantics (CR 201.2).
- AI behavior unchanged: phase-ai still picks Powder via its own `first_serum_powder_in_hand` (search.rs, policies/mulligan) and `live_mulligan_chooser_*` / `mulligan_*` tests pass.

## 9. Stop-and-return items
None open. Resolved after coordinator approval (scope.nul now 25 paths): the five tests asserting the old wire literal 4 now compare against `RESOLUTION_STATE_WIRE_VERSION`:
- `crates/engine/tests/integration/mycoloth_devour_drain_strand.rs`: 4 assertions (+ import).
- `crates/engine/tests/integration/p1_9354_oracle_owner_contract_30.rs`: the `Some(4)` pointer assertion and the raw-wire assertion (+ import).
The "requires resolution_state_version 4" message rows and the 2/3 downgrade rows stay (delivery_owner is still required from version 4).
Whole-crates grep for `resolution_state_version` beside a literal 4 (`grep -rnE "resolution_state_version" crates --include=*.rs`, minus resolution.rs): only these two files plus the message rows; `exploit_object_filter.rs`/`game_state.rs` hits are literal 2 downgrade stamps. Nothing outside scope.nul.

## 10. CR annotations added/changed
`grep -nE "^(103.5b|121.2a|121.2c|608.2c|616.1g)" docs/MagicCompRules.txt` each hit; subjects match the annotated code (draw instruction vs individual draws 121.2/121.2a; deal order 121.2c as modified by DealOrder; mulligan Powder 103.5b; "that many" tail 608.2c; replacement drain 616.1g).

## 11. Deviations from the plan (stale premises replaced inside scope)
- `RESOLUTION_STATE_WIRE_VERSION` was already 4 (plan: 3): bump 4 -> 5, new `LEGACY_DEALERLESS_..._VERSION = 4`; no new decode mode (`ResolutionWireV4` exists and serves both).
- Protocol numerals re-derived at START_SHA: full-game 111 -> 112, P2P wire 93 -> 94, lobby unchanged 16; `MIN_SUPPORTED_PROTOCOL` assert 110 -> 111 as 112-1; script offsets +41/+40. Script was red after bumping only the constants, green after the offsets.
- The plan's F1 ("this-turn gains one entry per seat") is satisfied by the draw.rs completion recording since the fold skips Draw for the this-turn ledger.
- New-field threading sweep for `DrawSequenceFrame.dealer`: the only construction literal is the push in `game_state.rs` (`dealer: None`, defaults intentionally: the dealer is attached after the push by `start_simultaneous_draw`); every other consumer reads via serde default; `loop_equal`, `validate` thread it; `engine_replacement.rs` unedited by design (credits `frame.accumulated` where `frame.player == player_id`, kept true by the working-copy rule).

## 12. Risks / for the PR body
- **Shipped-format behavior change:** Standard (and every format) now scopes `UseSerumPowder` per viewer: unscoped enumerators no longer carry it, only `legal_actions_for_viewer`/`with_viewer_actions` and the owner-scoped AI enumerator, for the holder's own pending `Declare` entry. Surfaces that bypass the viewer entry would lose the Powder: re-walked population = phase-server per-seat sends (`legal_actions_for_seat` -> `with_viewer_actions`), engine-wasm `get_legal_actions_for_viewer_js` (viewer-scoped; unscoped `get_legal_actions_js` has no client consumer), manabrew-compat (viewer-scoped), phase-ai (own `first_serum_powder_in_hand`), server-core `legal_actions_full` uses (internal), client `GamePage.tsx` (reads the local seat's viewer-routed list). No existing test or client file asserted Powder in an unscoped list (nothing to list).
- Parked-then-resumed dealer completion skips `EffectResolved`/this_way/clause publication (parity with the existing parked sequential path; not exercised by a real card).
- S6 scope limit: deck-out rows verify the existing loss logic over the dealer (v6a/b/c, v7); the loss logic itself is unchanged and no non-shared flow was changed.
- Dealer gate refuses `starting_with`, up-to counts and riders (class-correct, unexercised by a real card).
- Scratch: `scratch/pw` deleted; `target-dandan` kept for the orchestrator (no cargo clean); `scratch/e14-*.sh` wrappers remain. A stray `wt-dandan/target` created by `check-interaction-bindings.sh` before I set `CARGO_TARGET_DIR` was deleted; the re-run under the wrapper wrote nothing there.
