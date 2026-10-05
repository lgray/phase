MODEL: claude-sonnet-5-5

Mode: implementation/fix (PHASE MODE) | BASE_SHA 4bc7c2a358d5672c5f3333671dcf621fcd7b2f84 | START_SHA 418fb9a95e281dcf997c559e55e021086169d3f3 | IMPLEMENTATION_WORKTREE /home/lgray/vibe-coding/dandan-run/wt-dandan

Worktree: clean at start, HEAD == START_SHA, nothing staged. End: HEAD unchanged, nothing staged, tracked delta = crates/engine/src/game/mulligan.rs only (in scope.nul). All evidence below is PREPARATORY, not completion evidence.

## Diff (mulligan.rs only)
- New private `seat_walk_from_active(state)`: living seats of `state.seat_order` from the active player (`turn_order_index`, `is_alive`). `deal_sequence` (both `DealOrder` arms, so Interleaved is defined for shared formats too) and `close_declare_round` redrawers use it instead of `apnap_order_from`. Base order restored for every non-Dandan format (base walked `seat_order`, rotated active-first at game start).
- Docs: `handle_mulligan_decision` Mulligan bullet + closing paragraph per replacement text; deal comment now "CR 121.2c order (active player first) applied as the pregame default; CR 103.5 sets no deal order."; implicit-Keep comment now "CR 103.5 final sentence + CR 103.5c".
- New test `shared_team_turn_opening_deal_walks_seats_from_the_starting_player` (2HG 4 seats, start P1 and P3 through `start_mulligan`; reach-guard `has_shared_team_turns()`).

## Class check
`git diff 4bc7c2a358 HEAD -- crates | grep -nE '^[+-].*(apnap|seat_order)'` (non-empty: 8 hits). Sites that moved a seat-order walk to `apnap_order_from`: `deal_sequence` and `close_declare_round` redrawers (mulligan.rs) only; both fixed. The other hits (`start_mulligan` deal list, `normal_mulligan_decision`, `close_declare_round` entries) still walk `seat_order`. Single-class, two members, both repaired.

## Verification (PREPARATORY)
- Revert probe (git-archive scratch copy, own target dir, touch after edit, deleted after): `seat_walk_from_active` body replaced by `apnap_order_from(state, None, active)`: new test FAILS (start P1: left [0,1,2,3] vs right [1,2,3,0]), 61 other mulligan lib tests pass. Control on the fixed worktree: 62/62 mulligan lib tests pass incl. the new one. (r2-lib.log)
- fmt on mulligan.rs; `cargo clippy --workspace --all-targets -- -D warnings` rc 0 (r2-clippy.log).
- Full `cargo nextest run -p phase-engine --no-fail-fast`: 32743 passed, 0 failed (r2-full.log); includes dandan_declare_round 6, dandan_shared_pile_storage 20, dandan_hand_entry_ownership 12, interaction_contract 107, waiting_for_actor_authority_census 2 (all pass), and `standard_opening_deal_event_order_is_seat_by_seat`.
- `node scripts/check-protocol-version.mjs` rc 0 (no protocol change).
- CR gate: 103.5, 103.5c, 121.2c all exist in docs/MagicCompRules.txt; no UNVERIFIED. 103.5 = the deal (subject matches); 103.5c = cap on mulligans in multiplayer (subject matches the implicit-Keep cap path).
- Parser gate: no parser files touched.

## Coverage map (F1)
Claim: opening deal order for shared-team-turn formats equals base seat order. Seam: `deal_sequence`/`deal_hands` via `start_mulligan` (production entry of the deal). Test: `game::mulligan::tests::shared_team_turn_opening_deal_walks_seats_from_the_starting_player`. Reverting flips the `assert_eq!(recipients, expected)` at start P1. Siblings: Standard 2-seat (`standard_opening_deal_event_order_is_seat_by_seat`), Dandan interleave (`deal_sequence_interleaves_from_the_active_player_when_the_format_says_so`), 2HG start P3 in the same test.

## Judgement calls
- The helper is local to mulligan.rs because players.rs is outside scope.nul; it repeats the seat-walk loop already inside `players::apnap_order_from`'s non-shared branch. Extracting that loop into a shared `players` helper is a one-line follow-up if the driver widens scope.
- Base order was not rules-mandated (CR 103.5 sets no deal order); preserved, as the brief required.
- Interleaved arm for a shared-team format: defined as the same active-first seat walk; no current format combines the two, so it has no dedicated test.

## Stop-and-return / deviations / risks
None / none. Risk: logs r2-*.log are untracked files under phases/12 (kept as evidence; delete before commit if unwanted).
