# Phase 6 plan review, round 2 (phase-plan mode; phase-fit context declared; Sizing consistency check blocking)

Reviewer model: claude-sonnet-5-5. Plan: `phases/6/plan.md` rev 2 against HEAD `67cd221e`. No cargo run (instruction); every claim rests on grep, sed, git grep, jq over the tree.

Verdict: NOT CLEAN. 1 blocking finding (F9, tag `text`). F1 to F8 are closed. Sizing passes.

## F1 to F8 closure (each re-measured)

- F1 closed. V8 now pins `command.destination_position == library_of(P0).len()` (and the graveyard leg) measured before the move, plus `command.owner == P1`, and drops the `DestinationPositionMismatch` claim. Sound: `zone_container_len` (zones.rs 849) is private and feeds both capture and replay, so only the recorded value discriminates. `resolve_and_apply_zone_change` and `apply_resolved_zone_change` are `pub` under `cfg(any(test, feature = "test-support"))` (game/mod.rs 203-207); P-replay covers reach.
- F2 closed. The census script path is gone: 11 scope paths, 3.8 and V11 state "unchanged", blind spot recorded as an orchestrator verdict. Sizing: charter T2 counts 12 (incl. visibility.rs); plan edits 11, visibility.rs authorized and not edited; one unit, T1 fails. Consistent.
- F3 closed. V14 and 3.7 pin the wasm call with `source_census::code_lines` (exists, `pub(crate)`, reachable through the `#[path]` module in `tests/integration/main.rs`). Slice markers verified in `engine-wasm/src/lib.rs`: `fn initialize_game_impl` (1729), `// CR 103.1: Start the game` (1921), message string "Empty library after deck load" (1913); the only other `library.is_empty()` is in the test module (6561), outside the slice.
- F4 closed. V5/V9 build the pile with `add_real_card` (appends at the bottom) and reposition with `library_of_mut`; `add_card_to_library_top` / `add_spell_to_library_top` are excluded for a non-canonical owner.
- F5 closed. 3.4 uses `zone_storage_seat(Zone::Library, owner)` throughout.
- F6 closed. V1 pins an independent literal table; V3 is "before minus the 7 drawn", no multiset equality.
- F7 closed. mulligan.rs uses the holder-seat idiom, same as engine_resolution_choices.rs; no annotation, no `mem::take`.
- F8 closed. The 3.6 table was checked against `git grep -n deck_pools -- crates/engine/src` file by file: every production hit is covered (commander.rs 177/251, companion.rs 299/364, mulligan.rs 140 tiny-leaders-gated, candidates.rs 5433 `sideboard_actions`, interaction.rs 2351, printed_cards.rs 58/1224, visibility.rs 2710, search_outside_game, boosters, card_subset). filter.rs, ability.rs, player.rs, payment_transaction.rs and game_state.rs hits are doc comments, tests, or struct/eq plumbing.

## Blocking finding

### F9 [text] Four rewritten library writers have no revert-failing row
The plan rewrites these writers to resolve through the storage seat, but no V-row fails when any of them is reverted:

1. `engine_resolution_choices.rs` `DigChoice` library-reorder arm (raw `player_state.library.retain/insert/push_back` at about 4455-4478, keyed on `library_owner`).
2. `engine_resolution_choices.rs` `ScryChoice` arm (about 2138-2148, keyed on `player`).
3. `zones.rs::move_to_library_at_index` insertion (2382-2385), reached through `zone_pipeline.rs` 860 and 3718 for any "put on top/Nth from top" placement.
4. `zones.rs::reorder_within_library` (2251, `pub(crate)`), reached only by `surveil_keep_on_top` (engine_resolution_choices.rs about 10254).

V9 uses real Halimar Depths (a `Dig` with `keep_count: null`, `destination/rest_destination: Library`, measured in `card-data.json`) but controlled by P0, the canonical seat. A P0-controlled Dig resolves the same under the raw and the routed lookup, so reverting the Dig arm passes V9. V8 covers only the `add_to_zone` / `zone_container_len` / shuffle path. Phase 8 owns card-level put-back legs, but this phase owns the edits and must test them: the failure mode is the phase's own defect class (ids inserted into P1's empty container while the pile is unchanged, or an id in two containers).

Fix (text): append this row after V14 in the section 6 table, and add "V15" to the Sizing line "Discriminating test: V1 through V10 and V14":

| V15 in-library writers | Real cards, P1 acting in the booted-or-scenario Dandan state (pile of real cards, positive reach-guard: pile length > 0 and `players[1].library` empty before). (a) DigChoice: P1 plays real Halimar Depths, its ETB reaches `WaitingFor::DigChoice`, P1 answers the reorder; (b) ScryChoice: P1 resolves a real scry card (for example Opt) and answers the keep-on-top choice; (c) `move_to_library_at_index`: a P1-owned real card moved to the library top through `zone_pipeline::move_object` with `.at_library_position(LibraryPosition::Top)` (a P1 Memory Lapse countering a P0 spell is the card-level form); (d) `reorder_within_library`: P1 resolves a real surveil card and keeps cards on top | After each: the pile order equals the chosen order (`library_of(P0)` front matches), `players[1].library` is empty, no `ObjectId` appears in two containers, `library_of(P1) == library_of(P0)`. Revert any one routed writer: its ids land in `players[1].library` (or the pile order is unchanged) and the assertion fails | Paired: the same flow in a Standard state reorders P1's own library. Reach-guard: the WaitingFor / placement actually occurs for P1 (assert the choice state before answering) |

If any of (a) to (d) cannot be reached from the integration crate, run that leg as an inline test in its own file, named in the plan as a probe (P-writers). Keep it in the same revision; the test file (path 10) absorbs (a) to (c), (d) may need an inline test in `zones.rs` or `engine_resolution_choices.rs`, both already scope paths.

## Non-blocking notes (no action required for acceptance)

- V8/V9 write `library::resolve_and_apply_library_shuffle(state, P1)`; the real signature also takes `&mut Vec<GameEvent>` (library.rs 16-19). The executor sees it at compile time.
- Measured as correct, no finding: pre-Phase-11 hand routing (`draw_n` and the draw pipeline deliver to the card owner's hand, so P1's opening draws land in P0's hand; V3's "before minus 7" and the paired guard's returned hand of the canonical seat, which will be 14 cards, should be read from the measured hand, not assumed 7; the plan already words it as "the returned hand"); draw.rs, ledger `CardsDrawn` and its validator carry no hand-holder check, so V5's flow does not panic on the owner-hand interim (P-draw still probes it); `conjure.rs` whole-container assignment derives `rest` from the container at `pidx`, so routing `pidx` is sufficient and `NthFromTop { n: 7 }` is index 6; wasm is the only `load_and_hydrate_decks` caller outside `match_flow` (between-games, Phase 7).
- CR numbers in section 8 were accepted in round 1 and are unchanged.
