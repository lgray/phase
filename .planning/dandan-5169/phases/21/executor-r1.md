# Phase 21 executor report r1 (implementation/fix, phase mode)

Mode: implementation/fix (phase mode, charter RE-CHARTER r6 Phase 21, addenda/phase-21 empty)
BASE_SHA = PHASE_BASE_SHA = START_SHA = a10c2316ddf3e9b6090b6c81d734d6b5375d8c76
IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan
Model: claude-opus-5-5

## 1. Diff summary
- `crates/engine/src/types/format.rs`: `DeckSupply { PlayerBuilt, EngineFixed, HostPile }` and `GameFormat::deck_supply` (exhaustive match, no wildcard). `supplies_fixed_deck()` is now `deck_supply() != PlayerBuilt`. Docs updated on the Dandan variant, `FormatConfig::dandan` and the `FormatConfig.supplies_fixed_deck` field. Two "matching `supplies_fixed_deck`'s style" comments now point at `deck_supply`, because the former is no longer a match.
- `crates/engine/src/game/deck_loading.rs`: `DANDAN_DECKLIST`/`dandan_fixed_deck_names` renamed to `DANDAN_DEFAULT_PILE`/`dandan_default_pile_names`. `fixed_seat_payload(db, names)` becomes `main_deck_only_payload(Vec<DeckEntry>)`, and Momir calls it with `resolve_names(..)`. `dandan_fixed_deck_payload` becomes `dandan_pile_payload`: the pile seat's submitted `main_deck` if non-empty, else the default list, loaded as main deck only. Every other seat gets the default empty payload.
- `crates/engine/src/game/deck_validation.rs`: adds `DeckCompatibilityRequest::is_empty_submission` (all seven card slots). In `validate_deck_for_format`, after the Custom guard, an empty submission is `Ok` when `selected.tag().deck_supply() != PlayerBuilt`.
- `crates/engine-wasm/src/lib.rs`: in `validate_deck_list_seats`, the two seat loops become one chained loop with the same labels and the same first-refusal short-circuit, and the `supplies_fixed_deck` skip is gone. Adds the `deckSupplyForFormat` export and `deck_supply_or_player_built`. The old skip test is replaced by the W1, W2 and W3 tests. Dropped the false sentence "Pure extraction: no behaviour change."
- `client/src/wasm/engine_wasm.d.ts`: regenerated. It adds the `deckSupplyForFormat` declaration with its doc comment and the `InitOutput` member, and nothing else (there was no drift hunk).
- Tests:
  - `tests/integration/dandan_custom_pile.rs` (new) and its `mod` line in `main.rs`.
  - `format_axis_census.rs`: E5a, E5b, E5c.
  - `dandan_shared_pile_storage.rs`: the rename, plus `expected_decklist` made `pub(crate)` for E1b.
  - `phase-server/src/main.rs`: the S1 and S2 inline rows.

Delta == the 10 frozen scope paths exactly. Checked with `comm -3` of `scope.nul` against `git diff --name-only` plus untracked files; the output was empty, and the count of both sets was 10.

## 2. Worktree record
- Start: HEAD == START_SHA, porcelain clean, no staged entries.
- End: HEAD == a10c2316dd, `git diff --cached` empty, unstaged delta == scope. Mutated files were restored byte-identical (see section 5) and then `touch`ed, so only their mtime changed.
- Everything in this report is PREPARATORY evidence, not completion evidence.

## 3. PREPARATORY verification (no Tilt in this checkout)
- Formatting: `rustfmt --edition 2021 --config skip_children=true` on the scope `.rs` paths, rc 0.
- Focused nextest:
  - Packages: `-p phase-engine -p engine-wasm -p phase-server --features phase-engine/test-support`.
  - Filter: `test(dandan_custom_pile) | test(format_axis_census) | test(dandan_shared_pile_storage) | test(/census/) | test(/deck_loading/) | test(/deck_validation/) | test(/types::format/) | test(deck_list_seat_validation_tests) | test(external_format_config_tests) | test(ai_seat_setup_tests) | test(issue_4548_full_create_tests) | test(custom_format_schema) | test(/momir/) | test(/dandan/)`.
  - Result: 765 passed, 0 failed, 0 compiler warnings. Log: `logs/focused2.log`; it was run after every mutation was restored.
- Protocol: `node scripts/check-protocol-version.mjs` rc 0, with no pin edit.
  - Control, on a scratch copy of every file the script reads: lobby-broker `PROTOCOL_VERSION` changed 117 to 118 → "Protocol version mismatch: Rust=118, client=117", rc 1.
  - No wire, config or serialized field changed. `DeckSupply` is only the return value of the wasm export; it is not a field of any serialized type.
- Bindings: `./scripts/check-interaction-bindings.sh --check` rc 0.
  - Control: one line appended to `client/src/adapter/generated/interaction/index.ts` → "stale interaction bindings", rc 1.
  - The file was restored and its sha256 compared identical; a re-check gave rc 0.
- d.ts: built with the engine half of `scripts/build-wasm.sh wasm-dev`, the same two commands: `cargo build --package engine-wasm --target wasm32-unknown-unknown --profile wasm-dev` then `wasm-bindgen --target web --out-dir client/src/wasm --out-name engine_wasm …`. This keeps the out-of-scope `draft_wasm.d.ts` from being rewritten.
- Asserted empty-deck refusals: `git grep -n "(found 0)\|(found 79)" -- crates client/src` matches only the new `(found 79)` line, which is the control. No existing test asserts a Momir or Dandan empty refusal.

## 4. Parser preparatory gate
Not applicable: no file under `crates/engine/src/parser/` changed.

## 5. Discriminating-test gate

Mutation map. Logs are under `logs/M*.log`; sha256 before and after is recorded in `logs/mutations.txt`, and every restore is identical.
- M1 (loader always uses the default list) is the base loader. Red: E1a/E1c, E1d, E4a, E4b.
- M2 (empty rule deleted) is the base validator. Red: E2 validator, E2 gate, E2s, S1, S2, W1, W2.
- M3 (wasm skip restored) is the base wasm. Red: W1, W2. Green: W3.
- M4 (`Dandan => EngineFixed`). Red: E5a, W3.
- M5 (pile seat's whole payload cloned). Red: E1d, and also E1b, because the mutation also bypasses the default.

| Claim | Seam | Entry point | Test | Assertion that fails on revert | Siblings / guard |
|---|---|---|---|---|---|
| E1a/E1c: submitted pile is the one shared library | `dandan_pile_payload` | `load_and_hydrate_decks` | `dandan_custom_pile::e1a_e1c_the_pile_seats_submission_is_the_one_shared_library` | library multiset == Jund; `current_main` == Jund (M1 red) | library len 80; one pool; `deck_pool_of(P1).player == P0`; seat 1 submitted 80 Islands; Forest split across two entries |
| E1b: empty submission plays the default list | same | same | `e1b_an_empty_submission_plays_the_default_list` | == `expected_decklist()`. PRESERVATION: green at base, red only under M5 | the independent literal list |
| E1d: only the main deck loads | `main_deck_only_payload` | same | `e1d_only_the_pile_seats_main_deck_loads` | command zone, pool sideboard and pool commander empty; object multiset == Jund (M5 red) | reach: payload carries sideboard 1 and commander 1 |
| E4a: a started game plays only the pile | loader then `start_game` | `start_game_with_starting_player` | `e4a_a_started_game_plays_only_the_pile` | every object's name is in the Jund multiset (M1 red) | 66 library + 14 in hand |
| E4b: replay reloads the pile | `reconstruct_initial_state` → loader | replay | `e4b_a_replay_reloads_the_pile_from_the_recorded_deck_list` | replayed object multiset == Jund (red at base and under M1). Replayed library sequence == live sequence is PRESERVATION (green at base) | library non-empty |
| E2: validator admits any resolvable 80 and an empty submission | `validate_deck_for_format` | `validate_name_deck_for_format_full` | `e2_the_validator_admits_any_resolvable_80_and_an_empty_submission` | empty → Ok (M2 red) | Jund, 80 Bolt and 4 Lotus → Ok; 79/81, unknown name, sideboard, commander and Amulet of Quoz → Err (CR 407.3) |
| E2: P2P guest gate | same | `evaluate_deck_format_gate` | `e2_the_p2p_guest_gate_admits_an_empty_dandan_submission` | empty Dandan → compatible (M2 red) | Jund compatible; 79 not compatible |
| E2s: the empty rule follows the axis | same | same | `e2s_the_empty_rule_follows_the_deck_supply_axis` | Momir empty → Ok (M2 red) | Standard empty → Err "found 0"; Momir with Jund → Err |
| E3/W1: wasm validates every seat | `validate_deck_list_seats` | wasm init | `dandan_validates_every_seat_and_admits_empty_seats` | 79 on Player / AI opponent / AI player 2 → exact labelled reason (M3 red) | pile 80 plus empty seats → None; all empty → None; Standard empty → Some |
| W2: Momir at the wasm boundary | same | same | `momir_admits_empty_seats_and_validates_a_submitted_deck` | 60 Plains → "Player deck: Momir's Madness…" (M3 red); empty → None (M2 red) | |
| W3: export helper | `deck_supply_or_player_built` | `deckSupplyForFormat` | `deck_supply_reads_the_format_axis_and_fails_to_player_built` | Dandan → HostPile (M4 red) | Standard, Custom and None → PlayerBuilt; Momir → EngineFixed |
| E5a: axis census | `deck_supply` | census | `engine_supplied_deck_declarations_match_the_committed_list`, `format_axis_methods_carry_no_wildcard_arm`, `no_format_axis_key_reaches_the_client_mirror` | committed list [(Momir, EngineFixed), (Dandan, HostPile)] (M4 red) | existing reach guards; `deck_supply` appears 0 times in both client files, `sideboard_policy` 26 and 3 times as the control |
| E5b: derived field agrees with the axis | `supplies_fixed_deck` | registry | `supplies_fixed_deck_is_derived_from_the_deck_supply_axis` | method and registry field == axis for each format. Structural: the value is unchanged from base | count == 2 |
| E5c: every supplying format loads a library | axis to loader | loader | `every_engine_supplied_format_loads_a_library_from_an_empty_submission` | no empty-library seat. PRESERVATION | checked == 2 |
| S1: create handler | create handler, then validator | full socket `CreateGameWithSettings` | `full_mode_create_accepts_an_empty_host_deck_for_dandan` | reply is `GameCreated` (M2 red) | existing Standard-empty row is still `DeckRejected` |
| S2: AI seat setup | `ai_seat_setups` | create with `ai_seats` | `an_empty_ai_deck_list_is_accepted_only_where_the_engine_supplies_the_deck` | Dandan and Momir empty → Ok with 1 setup (M2 red) | Standard empty → Err |
| U1: a UI-chosen pile reaches the engine | client | — | DEFERRED(phase 22) | — | — |

There is no shape-only test, and there is no unmapped seam. The degenerate-fixture trace:
- The loader fixtures reach the non-empty branch through E1a, E1d and E4a, and the empty branch through E1b. E1c covers a non-pile seat that submits a deck.
- The validator rule is reached by both an empty and a non-empty request, under both a supplying format and Standard.

## 6. Maintainer-simulation matrix

| Seam | Entry point / first branch | Authority | Bound value / when | Mode | Storage | Consumers | Invalidation | Hostile fixtures | Serde/protocol |
|---|---|---|---|---|---|---|---|---|---|
| Pile choice | `load_and_hydrate_decks` → `format == Dandan` → `submitted_pile.is_empty()` | pile seat = `canonical_seat()` (lowest PlayerId) | `Vec<DeckEntry>`, bound at load | latched at load (CR 103.3: the deck becomes the library) | holder seat's library plus the one `deck_pools` entry | `deck_pool_of`, `library_of`, `match_flow` reload | none: fixed for the game; replay re-derives it from `ReplayHeader.deck_data` | second seat's submission (E1c), sideboard/commander (E1d), empty (E1b), split entries (E1a) | none: `deck_data` already stores the DeckList |
| Empty rule | `validate_deck_for_format` → `is_empty_submission() && tag().deck_supply() != PlayerBuilt` | selected format tag | verdict at admission | live predicate on the request | none | wasm init, create handler, `ai_seat_setups`, P2P guest gate | n/a | Standard/Momir siblings, every non-empty Dandan case (E2) | none |
| wasm seat loop | `validate_deck_list_seats`, every seat | n/a | reasons at init | live | JS error envelope | `initialize_game_impl` | n/a | each seat position, extra AI seat (W1) | none |
| Export | `deckSupplyForFormat` → `from_value::<GameFormat>` | n/a | `DeckSupply` | live | none | Phase 22 (`engineRuntime.ts`) | undecodable → PlayerBuilt | None/Custom (W3) | the d.ts adds 1 declaration and 1 `InitOutput` member; no protocol change |

No row is incomplete. `HostPile` is reachable only for Dandan; any other format is UNREACHABLE, and E5a pins that.

## 7. CR-annotation diff gate
The grep ran over `git diff` plus the new file. Results: `OK: CR 103.3`, `OK: CR 103.4`, `OK: CR 400.1`, `OK: CR 407.3`, and zero `UNVERIFIED`.
- 103.4 and 400.1 sit on rewrapped existing doc lines.
- The new CR 103.3 ("The players' decks become their libraries") is on the loader comment.
- CR 407.3 appears only in the E2 test comment.

## 8. Judgement calls
- E1b sits in the new file and reuses `expected_decklist()`, which is now `pub(crate)`. Without this the independent reference would have to be duplicated.
- E4b's pile assertion uses the multiset of all objects, because after the deal the library alone holds 66 cards. The replay == live check compares library sequences and is labelled preservation (T3).
- M5 was written as "clone the pile seat's whole payload". It is broader than "clones `payload.player`", so it also turns E1b red. That is acceptable: the target row E1d does go red.
- I fixed comments that became false in code I edited: two "matching `supplies_fixed_deck`'s" style references and the "Pure extraction" sentence.

## 9. Stop-and-return items
None.

## 10. CR annotations added
`// CR 103.3: the pile seat's submitted main deck, else the default list, becomes the shared library.` in `dandan_pile_payload`. Verified with `grep -nE "^103\.3" docs/MagicCompRules.txt`, which returned line 280.

## 11. Deviations from the plan
- The d.ts diff has no drift hunk, so T1's allowance for drift went unused.
- The S1 test is a new sibling of the Standard row; the existing test is left untouched.

## 12. Risks
- The first M3 run gave a false result from stale artifacts. The cause: `shutil.copy2` restores kept the old mtimes, so cargo treated artifacts built from the M4 mutation as fresh.
  - The fix was to `touch` the restored files and rerun M3.
  - M1, M2, M4 and M5 are valid: each one mutated a phase-engine source, which forces a whole-crate rebuild from current contents.
  - `focused2` ran after all restores and touches.
- PR body / driver wording (T2): "wasm now validates non-empty Momir and Dandan seats. A host route that submits an unvalidated active deck is refused at init until Phase 22 routes every pile-seat construction through its helper; the routes are the GameProvider fresh `p2p-host` branch, and the lobby host once Phase 22 stops requiring a legal active deck." Momir's empty submission is now accepted at the create handler, `ai_seat_setups` and the guest gate.
- Phase-22 addendum lines from the review (helper submits an empty pile-seat deck for EngineFixed as well; `deck_supply` must stay out of `types.ts` and `formatRegistry.ts`) are for the orchestrator. I did not write them.
- Not run: clippy, full suites, card-data, ai-gate (orchestrator-owned).
