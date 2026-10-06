# Phase 15 executor report r1

Mode: implementation/fix (phase mode). BASE_SHA = START_SHA = 236a35f0b5189161a7513d38261751b0238f5069. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Start check: clean, HEAD == START_SHA, nothing staged. End check: HEAD == START_SHA, nothing staged, unstaged delta = `effects/mod.rs`, `effects/change_zone.rs`, `tests/integration/main.rs` (mod lines) + two untracked new test files; all in scope.nul. `integration_cards.json.gz` unchanged (every card needed is in it). No `types/` path touched; `node scripts/check-protocol-version.mjs` rc 0.

PREPARATORY evidence only; none of it is completion evidence.

## Diff summary
- `effects/mod.rs`: `is_scoped_pile_return` (extracted; the `(pile_return, pile_return)` and `(pile_return, Shuffle)` arms of `is_player_scope_local_continuation` now use it, behavior-identical); `shared_library_wheel_split` + one call line in the `driver_scope` block of `resolve_chain_body` (shadows `ability` before `split_player_scope_chain`); 3 unit tests (accept, Standard refuse, 10 hostile mutations).
- `effects/change_zone.rs`: `scope_player_for_member`, used by the player-scoped mass scan in `resolve_all`.
- `tests/integration/dandan_wheel_split.rs` (new, 11 rows), `dandan_scoped_pile_mass_move.rs` (new, 2 rows), `main.rs` mod lines.

## Design notes
- Wheel split: re-tags exactly the shuffle node `ContinuationStep -> SequentialSibling`, gated on `shared_zones().library == Shared`, a Fixed `ScopedPlayer` draw carrying the root's own scope, plain (non-optional, no condition) nodes. The draw keeps `ContinuationStep` so Phase 14's `has_no_resolution_riders` accepts the seat; the dealer engaged with no edit to `draw.rs`/`library.rs` (plan P1/P7 measured by W1: dealt interleave reached).
- Scoped pile move (item 2): CR reading derived from the card, not assumed. Brief line 30 of `issue-5169.md`: "Anything referring to 'your' graveyard or library is referencing the shared zone"; Feldon's Cane (`{T}, Exile this artifact: Shuffle your graveyard into your library.`, parsed `ChangeZoneAll{origin Graveyard, target Controller, TerminalShuffle}` + `Shuffle{Controller}`) therefore moves the whole pile. Phase 10's rule for Haunted Fengraf is the same reading (charter). Implemented as: for a non-battlefield card, the scoped player is tested as the card's owner when `zone_storage_seat(zone, owner) == zone_storage_seat(zone, player)`; per-player zones (hand, standard graveyard/library) reduce to `owner == player` unchanged. Each-player scopes: the first seat's pass takes the whole pile, later seats find nothing, so no card moves twice (W1 move-event count equals hand + graveyard population).
- The predicate `change_zone_all_player_scope_member_matches` keeps its signature: its other caller (`engine_resolution_choices.rs::legacy_mass_library_order_prompt_is_current`) validates pre-identity archived prompts (`PendingMassLibraryOrderBatches::Legacy`), which predate the format and require `owner == expected_owner` anyway; changing it would have put an out-of-scope path in the delta.

## Verification (PREPARATORY)
- Base red (no production edit): `.planning/dandan-5169/p15-base.log`: W1, W1b, W4, W5, W6 red (two shuffle events; draw order [P0x7, P1x7]; W6 P0 hand 7), Cane Dandan row red (`Graveyard` left behind); W1e, W2, W3 x2, Cane Standard twin green.
- Green with the change: 16/16 of {dandan_wheel_split, dandan_scoped_pile_mass_move, shared_library_wheel_split*, hand_to_library_shuffle_local_continuation_requires_exact_scoped_pair, all_player_library_wheel (unedited)}: `scratch/p15/clean.log` (retained under scratch only).
- Revert legs (in-place mutants, restored and byte-compared afterwards):
  - M2 gate ignores the shared axis: W2 red (`[P0]` vs `[P0, P1]` shufflers), `shared_library_wheel_split_leaves_separate_libraries_unchanged` red.
  - M6 draw also re-tagged `SequentialSibling`: W1, W1b, W4, W5, W6 red (draw order sequential: the dealer refused the seat) + `retags_only_the_shuffle` red.
  - M4 `Fixed` conjunct dropped: hostile row "EventContextAmount draw" accepted. M5 draw-own-scope conjunct dropped: row "unscoped draw" accepted (both in one run, labels listed by the test).
  - Mcz `scope_player_for_member` reverted to `owner == player`: Dandan Cane row red.
- `cargo clippy --workspace --all-targets -- -D warnings`: rc 0. Full `cargo nextest run -p phase-engine`: 33039 passed, 0 failed (120 slow, 12 skipped). `scripts/check-interaction-bindings.sh --check`: rc 0. `cargo fmt` was NOT run tree-wide: the checked-in tree is not rustfmt-clean under this toolchain (about 30 unrelated files reformat), so my hunks were checked with `rustfmt --check` restricted to my line ranges (clean) and the new files were formatted whole. An accidental whole-module rustfmt reformatted ~95 files; the unrelated files were restored from `git show HEAD:` and five unrelated hunks in `mod.rs` reverse-applied (all my own accidental edits, tree clean at start).

## Parser preparatory gate
No file under `parser/` changed: N/A.

## Production-path coverage map
| Claim | Seam | Entry | Test | Fails when reverted | Siblings |
|---|---|---|---|---|---|
| One shuffle after every move, then dealt draw, Day's Undoing | `shared_library_wheel_split` + driver call | `GameRunner::cast` of the real card | `w1_days_undoing_moves_every_seat_then_shuffles_once_then_deals_fourteen` | shuffle count 1 and alternating order (base: 2 shuffles) | W1b non-canonical active seat, W4 Timetwister (no tail), W5 empty zones (701.24d), W6 deck-out 12/13 |
| Gate on shared axis | same | Standard twin | `w2_separate_libraries_shuffle_and_draw_per_player` | M2: one shuffle | `w3_standard_deals_each_player_in_turn` |
| Tail survives the split | driver detach | cast | `w1e_end_the_turn_follows_the_last_dealt_card` | none by the split itself (green at base; guards tail loss only) | |
| Native split reaches the dealer | none changed | cast | `w3_wheel_of_fortune_discards_both_hands_then_deals` | green at base by design (composition guard) | |
| Shape class | `shared_library_wheel_split` | unit | `..._retags_only_the_shuffle`, `..._refuses_every_other_shape` (10 mutations, paired with the accepted base), `..._leaves_separate_libraries_unchanged` | M4/M5/M6/M2 | |
| Whole shared pile moves for a scoped player | `scope_player_for_member` in `resolve_all` | activate real Feldon's Cane as P0 and P1 | `the_whole_shared_graveyard_moves_for_either_seat` | Mcz; base | `separate_graveyards_move_only_the_activators_cards` (Standard, per-owner) |

Every mapped test drives the real pipeline (`cast`/`activate` + `resolve`) except the three unit rows, which are the seam's own shape tests; none is parser-shape-only.

## Maintainer-simulation matrix
| Seam | Entry / first branch | Authority bound | Binding | Storage | Consumers | Invalidation | Hostile fixtures |
|---|---|---|---|---|---|---|---|
| wheel split | `driver_scope` block; `shared_zones().library == Shared` arm | the single shuffle's actor = `ability.controller` of the detached shuffle (`ScopedPlayer` unbound outside an iteration), at tail resolution, live | live | none (re-tag of an in-flight `ResolvedAbility`; serializes as ordinary abilities) | `shuffle.rs` -> `library_of(controller)` = the pile | a pause in the move passes parks remaining seats and the whole `after_scope` through the existing continuation path (unchanged) | W1b: controller P1 on non-canonical seat; hand/graveyard cards of both owners |
| dealt draw | Phase 14 `plan_simultaneous_draw` via the re-entered scoped `Draw` clause | seats from `matching_players` APNAP | clause start | `DrawDealer` frame (Phase 14) | `deal_sequence` | W6 deck-out | W1, W5, W6 |
| scoped pile move | `change_zone_all` scan, non-battlefield arm | scoped player collapsed to the owner when both resolve to the same container | scan time, live | none | zone move + chained `Shuffle` | none | Cane as P0 and P1; Standard twin |
No row is incomplete; no DEFERRED row.

## CR annotations
CR 108.3, 400.1, 608.2c, 701.24a (production); 104.4a, 121.2c, 121.4, 701.24d, 724.1 (tests/comments). Diff gate output: all nine `ok` against `docs/MagicCompRules.txt`; 724.1 (end the turn) and 701.24d, 121.4, 104.4a text read and match the claim. Plan's "722" correction honored (CR 724).

## Judgement calls
- Plan U1/W1(d) etc. were adapted to the real tree: draw order/holder checks use `CardDrawn.object_id`; W6 reach-guard uses the dealt order (a drawn game clears hands, so the plan's "hands 6 and 6" is unobservable).
- Plan P3 (seat 1's graveyard pass finds its owned cards) is superseded by item 2: seat 0's pass now takes the whole pile.
- The Standard W2 event-order is not asserted against the printed card (plan R1: not determined by the card or CR); only shuffle count and CR 121.2c draw order, as in the plan.

## Stop-and-return items
None. (An initial one-line edit to `engine_resolution_choices.rs` was backed out for scope; see Design notes.)

## Risks
- `legacy_mass_library_order_prompt_is_current` keeps owner-keyed membership; a pre-identity archive can never carry Dandan, but a future change that routes a new-format prompt through the Legacy validator would reject cross-owner batches.
- W1e and W3 are green at base by design; they guard composition, not the split.
- Plan's stale numerals (protocol 98/80) not used; protocol check rc 0 with no pin edits.
