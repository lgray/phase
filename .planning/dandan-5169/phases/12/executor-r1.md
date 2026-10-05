MODEL: claude-sonnet-5-5

Mode: implementation/fix, PHASE MODE. BASE_SHA = START_SHA = 4bc7c2a358d5672c5f3333671dcf621fcd7b2f84. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.

## Worktree record
Start: clean, HEAD == START_SHA, nothing staged. End: HEAD == START_SHA, 0 staged, 24 modified + 1 new file (`dandan_declare_round.rs`), all inside scope.nul (the fixture `integration_cards.json.gz` is in scope and unchanged: `gen-test-fixture.py` rewrote nothing, `--check` rc 0). PREPARATORY only; not completion evidence.

## Diff summary
- `game/mulligan.rs`: `deal_sequence` (pure order, reads `deal_order()`), `deal_hands` (the one pregame deal), `draw_one`; `draw_n` and `start_mulligan` go through them. `MulliganTiming`/`mulligan_timing` (library axis). `Mulligan` arm: `Immediate` = old body, `Simultaneous` = record in `declared`. `advance_after_decision` takes `declared`, closes the round when `pending` is empty (`close_declare_round`: return all hands, one shuffle per distinct library holder, deal interleaved from active player, cap -> implicit Keep). `shuffle_hand_into_library` split into `return_hand_to_library` + `shuffle_library_of`. `finish_mulligans_public` removed.
- `types/game_state.rs`: `MulliganDeclaration`; `WaitingFor::MulliganDecision.declared` (serde default + skip when empty); docs; fixture list + serde test.
- `game/elimination.rs`: `prune_mulligan_pending` filters `declared` and calls `advance_after_decision`.
- 10 compiler-forced literal files (`declared: Vec::new()`), `client/src/adapter/types.ts` optional mirror.
- Phase 6/11 tests: `dandan_shared_pile_storage.rs` v3, `dandan_hand_entry_ownership.rs` v3 now submit the other seat's Keep.
- New `tests/integration/dandan_declare_round.rs` (+ mod line); unit tests in `mulligan.rs` (V1, V2, V2c, V4a-d, V4f, V4g) and `game_state.rs` (V6).
- Protocol: lobby-broker `PROTOCOL_VERSION` 109 -> 110, `MIN_SUPPORTED_PROTOCOL` assert 108 -> 109, server-core pin test renamed `protocol_version_is_110_for_mulligan_declare_round`, ws-adapter 110, network/protocol.ts wire 91 -> 92, protocol.test.ts, p2p test (91/92 literals + title), script `+39` / `+38`.

## PREPARATORY verification (no Tilt; isolated direct cargo)
- `cargo fmt --all --check` rc 0 (fmt run only on touched `.rs` paths).
- `clippy --workspace --all-targets -D warnings`: rc 0 (log `p12-clippy.log`).
- Full nextest `-p phase-engine -p lobby-broker -p server-core -p phase-ai -p manabrew-compat --no-fail-fast`: 36493 run, 36493 passed (`p12-full.log`). engine-wasm compiled under clippy `--all-targets`; its tests not run.
- `node scripts/check-protocol-version.mjs`: red after bumping only the Rust constant ("Rust=110, client=109", rc 1), green after pins (rc 0). Asserted-numeral sweep: remaining 109/91 hits are historical comments.
- `scripts/check-interaction-bindings.sh --check` rc 0 (no diff).
- Frontend: `tsc -b --noEmit --force` rc 0; vitest `protocol.test.ts` 57 pass; p2p "wire-protocol version gate" 6 pass. (`pnpm lint` not run: only a type member and literals changed.)
- Parser gate: not applicable (no file under `parser/`).

## Premises re-measured at START_SHA (step 0)
Phases 6/11 landed as planned (`zone_storage_seat`, `library_of(_mut)`, `draw` taker `.performed_by`). Stale: protocol numerals (95/77 -> actual 109/91, bump to 110/92, offsets +39/+38); forced-literal population is 10 files (not 12; `mulligan.rs`/`game_state.rs` are the others). `finish_mulligans_public` single caller confirmed. Names `legal_actions`: used `candidate_actions` metadata actor.

## Discriminating-test coverage map (revert = mutation applied in place, restored by `cp` + utime, byte-identical to backup)
| Claim | Seam | Entry | Test | Red under |
|---|---|---|---|---|
| Deal alternates, starting player first | `deal_sequence`/`deal_hands`, `start_mulligan` | `start_game_with_starting_player` | `v3_opening_deal_alternates...` (starting P0 and P1; Standard seat-by-seat paired guard); unit `deal_sequence_interleaves...`, `deal_hands_alternates...` | M1 (per-seat loop: v3), M2 (PlayerByPlayer: 5 tests) |
| Mulligan held, not carried out | `Simultaneous` arm | `apply(P1, Mulligan)` | `v4a_...`, unit `dandan_mulligan_is_held...` | M3 (7 tests) |
| Close: return all, one shuffle, deal active-first | `close_declare_round` | `apply` last declarer | `v4c_...` (both starting seats; arrival opposite to deal order; shuffle equals clone replay), `v4b_...`, unit `dandan_close_returns...` | M4 arrival order (v4c), M5 skip shuffle (v4c + Phase 6 v3), M5b no dedup (v4c) |
| Round waits for owed bottoms | `advance_after_decision` case 1 | `apply(SelectCards)` | `v4d_...`, unit `dandan_round_waits...` | M6 (2 tests) |
| Powder stays immediate | UseSerumPowder arm untouched | `apply(UseSerumPowder)` | `v4e_...` | M7 (v4e + 9 Standard Powder unit tests) |
| Cap -> implicit Keep at close | `close_declare_round` | unit | `dandan_cap_applies...` (count 6 at cap, count 5 paired) | M8 |
| Elimination keeps held declaration | `prune_mulligan_pending` | `apply(Concede)` | unit `dandan_elimination_with_a_held_declaration...` (paired Keep leg) | M9 |
| serde: omitted when empty | `declared` attrs | serde | `mulligan_decision_declared_round_trips...` | M10 |
| Declared seat has no candidates / authority | `pending`-only consumers | `candidate_actions`, `acting_players` | `v4a_...` (reach guard: P0 has candidates) | M3 |
Every row mutation turned red; M1-M10 results in `p12mut/results.txt` (scratch outside tree). Non-shared preservation: unit `standard_opening_deal_event_order_is_seat_by_seat` (7 moves, CardsDrawn P0, 7 moves, CardsDrawn P1) green at base semantics, plus the whole mulligan unit suite. No shape-only tests.

## New-field threading sweep (`declared`)
Produce: `normal_mulligan_decision` defaults intentionally (empty at round start); `handle_mulligan_decision` threads (clone + push); `handle_mulligan_bottom` threads; `advance_after_decision` threads; `prune_mulligan_pending` threads (filtered). Consume: `acting_authority`, `candidates.rs`, `interaction.rs`, `ai_support/mod.rs`, `engine.rs`, `aiController.ts`, `GamePage.tsx`: read `pending` only, so defaults intentionally (a declared seat is not in `pending`). Test literals: `declared: Vec::new()` (compiler-enumerated, 10 files).

## Maintainer-simulation matrix (condensed; one row per seam)
- Declaration: entry `apply(Mulligan)` -> `mulligan_timing` Simultaneous arm; authority = submitting `PlayerId` (authorized via `acting_players`); bound at submission; snapshot of `mulligan_count`; stored `WaitingFor::MulliganDecision.declared`; consumed by `close_declare_round`; invalidated by close or elimination (filtered in `prune_mulligan_pending`); hostile: v4a second submit refused, v4c arrival opposite; serde omit-when-empty (V6), protocol 110/92.
- Deal: authority = `(PlayerId, count)` pairs bound at call; live; consumed by `draw_one` `.performed_by`; order from `apnap_order_from(None, active_player)`; hostile: starting P1 (non-canonical seat first).
- Powder: validated and executed at declare point, unchanged; a held declarer cannot Powder (v4e second assertion).

## Judgement calls / deviations
1. `advance_after_decision` made `pub(crate)` and called directly by elimination instead of adding a `settle_mulligan_pending` wrapper (identical behaviour; one fewer function).
2. `close_declare_round` implicit Keep uses `.expect` on `resolve_declare_point` (Keep arm has no fallible step); the alternative was threading `Result` through `advance_after_decision` and elimination.
3. V4d/V4f/V4g and the V2/V4 held-flow checks are unit tests over a synthetic Dandan pile (`handle_mulligan_decision` directly); V3/V4a/V4b/V4c/V4d/V4e additionally drive `apply` over the real 80-card pile. V8 edited exactly the two Phase 6/11 tests that submit a lone Dandan `Mulligan` (discovery grep: `MulliganChoice::Mulligan` in `crates/engine/tests` hits only those two plus unit tests).
4. Integration assertions "new hand disjoint from old hand" were removed: returned cards re-enter the shuffled pile and may be redealt; replaced by `ne` plus pile-or-hand membership.
5. `deal_hands` emits `CardsDrawn` only for players with a nonzero count (the old `draw_n(0)` emitted `count: 0`; no caller passes 0).

## Stop-and-return items: none. Risks
- Under shared-team-turn topologies (2HG) `apnap_order_from` delegates to `topology::apnap_order_from`, so the PlayerByPlayer opening-deal event order is team order rather than `seat_order`; per-player libraries make draws order-independent, and the full suite is green, but no 2HG test pins the event order.
- CR annotations added: 103.5, 121.1, 121.2c, 400.1, 701.24a (all `grep -nE "^N"` verified; gate output: zero UNVERIFIED).
