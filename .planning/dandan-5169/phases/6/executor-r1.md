# Phase 6 executor report r1 (S2a canonical-seat storage + pool resolver)

Mode: implementation/fix, phase mode. BASE_SHA = START_SHA = 4e141877ca74ca8c8398a36d7563b59d3bdfe3a7. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.

VERDICT: implementation complete, all preparatory gates green, one admitted-class addition requested (test fixture regen, item 9). Not committed.

## 1. Diff summary (10 modified, 1 new; ~504+/164- plus 780-line test file)

- `types/game_state.rs`: new `impl GameState` block: `canonical_seat`, private `shared_zone_holder` (exhaustive `match zone`), `zone_storage_seat`, `library_of/_mut`, `graveyard_of/_mut`, `deck_pool_of`, `seats_with_empty_library`. Epoch keys (`advance_library_knowledge_epoch`, `library_knowledge_epoch`, `library_knowledge_boundary_generation`, `canonicalize_library_knowledge_epoch`) and `record_zone_change_library_knowledge_stamp` resolve to the storage seat; the `facts` comparisons compare resolved holders. Inline `shared_zone_storage_tests`.
- `game/zones.rs`: `zone_container_len`, `remove_from_zone`, `add_to_zone`, `reorder_within_library`, insertion in `move_to_library_at_index` go through the accessors.
- `game/library.rs`: shuffle precondition/compare/install through accessors; `UnknownPlayer` refusals kept as explicit `players.iter().any`.
- `game/engine_resolution_choices.rs`: ScryChoice and DigChoice-reorder raw writers use the holder seat; the two `EffectZoneChoice` placement reads use `library_of(holder)` and visit each distinct storage seat once (`library_holders_of`; replay filter compares storage seats). Inline V16 unit tests.
- `game/effects/conjure.rs`, `game/effects/draw.rs`, `game/mulligan.rs` (`shuffle_hand_into_library` holder-seat idiom, `draw_n` via `library_of`).
- `game/deck_loading.rs`: `DANDAN_DECKLIST`, `dandan_fixed_deck_names`, `fixed_seat_payload` (Momir now calls it), `dandan_fixed_deck_payload`, Dandan branch in `load_and_hydrate_decks`, and in `load_deck_into_state` a single `library_holders` set that drops non-holder pools (`retain`) and skips their `load_player_library` calls.
- `engine-wasm/src/lib.rs`: boot guard calls `seats_with_empty_library()`.
- `tests/integration/dandan_shared_pile_storage.rs` (new, 20 tests) + one `mod` line in `main.rs`.
- `visibility.rs` authorized, not edited (callers pass seats; it only compares stamps).

## 2. Worktree record

START check: `git rev-parse HEAD` = 4e141877...; `git status --porcelain` empty; no staged entries. Executor did no staging/commit. End: HEAD unchanged; `git status --short` shows exactly the 10 modified paths + the new test file (all in SCOPE_PATHS). PREPARATORY evidence below is not completion evidence.

## 3. PREPARATORY verification (all at the final tree; env per common.md, `FORGE_TEST_FULL_DB=1`)

- `cargo fmt --all -- <my paths>`: applied.
- `cargo clippy -p phase-engine -p engine-wasm --all-targets -- -D warnings`: rc=0 (log `phases/6/clippy2.log`).
- `cargo nextest` integration, filter `dandan_shared_pile_storage | momir|mulligan|shuffle|scry|surveil|conjure|deck_pool|format_axis|zone_authority|library_order|census`: 196 passed, 0 failed (20 of them mine).
- `cargo nextest --lib`, filter zones/library/mulligan/draw/deck_loading/visibility/game_state/conjure/engine_resolution_choices modules: 605 passed, 0 failed (incl. 3 V16 + 2 seam tests).
- `python3 scripts/zone_authority_census_tests.py` OK; `python3 scripts/zone_authority_census.py --check` PASS (51 hits / 32 rows). Neither script edited. The two new `// allow-raw-zone:` rows are the `library_of_mut`/`graveyard_of_mut` accessor borrows (the census BORROW regex matches them; unannotated they fail the gate). The gate's blind spot (`library_of_mut(p).push_back(..)` at a call site) is unchanged and no production call outside zones.rs does that.
- `node scripts/check-protocol-version.mjs` exit 0 (no serialized shape added: no field/variant/type).
- Full suite not run (per brief). `check-interaction-bindings.sh --check` see item 12.
- NOT green without the fixture addition (item 9): in CI-default mode (fixture DB) the 15 cards named there are missing, so the dandan integration tests that load real cards fail loudly (they do not skip).

## 4. Parser preparatory gate
No file under `crates/engine/src/parser/` touched: N/A.

## 5. Discriminating-test gate: production-path coverage map

Mutation evidence: every row below was reverted in situ (`phases/6/mutA.log`, `mutB.log`, `mutC.log`; patches applied then sha-verified restore + `touch`); results quoted are the failing tests in the batch. Batch A (7 seams at once) failed exactly 7 tests, one per seam; batch B (14 patches) failed 10 integration + 4 lib tests; each failing test maps to its patch (v3 additionally fails under the B `draw_n` patch because the opening deal changes). The seams of batch B overlapping the stale `draw.rs` mutant are re-run alone in batch C (item below).

| Claim / seam | Entry | Test | Revert (patch) -> failing assertion | Sibling / paired guard |
|---|---|---|---|---|
| Boot loads one 80-card pile in canonical seat, one pool | `load_and_hydrate_decks` | `v1_boot_loads_one_pile_into_the_canonical_seat`, `v1_pile_stays_in_the_lowest_seat_when_another_seat_starts` (seat_order rotated) | B4 (drop non-holder pool `retain`) -> `deck_pools.len()==1`; B5 (raw empty-library predicate) -> `seats_with_empty_library().is_empty()` | `v1_non_shared_formats_keep_per_seat_libraries` (Momir 60/60, 2 pools; Standard reports `[P0,P1]` then `[P1]`) |
| Fixed list | `dandan_fixed_deck_names` | `v2_fixed_list_resolves_against_the_real_database` (80 names, 23 distinct, `Exactly(80)`, each resolves incl. Dandân; Momir list 60) | fixture-missing cards fail it | Momir list |
| Pool resolver | `deck_pool_of` | `v10_pool_resolver_maps_both_seats_to_the_pile_holder` | A5 (seat identity) -> `deck_pool_of(P1)` is None | Momir: each seat its own pool |
| Boot guard call site | wasm `initialize_game_impl` | `v14_wasm_boot_guard_reads_the_library_accessor` (source pin, positive reach on `"Empty library after deck load"`) | B11 (raw filter) -> fails | N/A (JsValue-typed entry; no native path) |
| Opening deal | `start_game_with_starting_player` -> `start_mulligan` -> `draw_n` | `v4_opening_deal_draws_both_hands_from_the_pile` (pile 66, both `CardsDrawn{7}`) | B2 (raw `draw_n`) -> fails | seat P1 starts |
| Mulligan shuffle | `apply(MulliganDecision{Mulligan})` | `v3_mulligan_by_either_seat_shuffles_the_pile` (P1 and P0 legs; compares to the un-shuffled expectation) | A2 (shuffle own container) -> fails (P1 leg: order equals un-shuffled) | P0 leg is the paired canonical case |
| Entropy of empty-seat shuffle | `shuffle_vector` | `shuffling_the_empty_seat_consumes_no_entropy` (with positive control) | n/a (documents the `start_mulligan` all-seat loop premise) | real shuffle consumes |
| Draw selection + graveyard writes | `runner.activate(Lonely Sandbar cycling)` -> `select_cards_to_draw`, `add_to_zone` | `v5_v6_either_seat_cycles_from_and_into_the_shared_zones` (P1 and P0) | A3 (draw raw) -> pile unchanged/flag; B1 alone (batch C) -> graveyard lands in `players[1].graveyard` | `v5_v6_standard_format_cycles_in_the_owners_own_zones` |
| Conjure into library | cast real Mine Security, enter trigger | `v7_conjure_into_the_library_lands_in_the_pile_only` (one conjured object, slot < 8, no id in two containers) | B3 (assign owner's container) -> fails | Standard leg in the same test |
| Recorded commands + replay | `zones::resolve_and_apply_zone_change`, `apply_resolved_zone_change` | `v8_zone_change_commands_record_and_replay_pile_positions` (library + graveyard legs; `destination_position`, replayed container equality) | A1 (raw `zone_container_len`) -> recorded position wrong; B1 | Standard leg |
| Shuffle command | `library::resolve_and_apply_library_shuffle(P1)` / `apply_resolved_library_shuffle` | `v8_library_shuffle_acts_on_the_pile_and_replays` (command.player == P1, precondition == pile, event player, replay equality) | B8 (raw install) -> fails | n/a |
| Library knowledge | real Halimar Depths ETB `DigChoice`, then shuffle by P1 / by P0 | `v9_library_boundary_by_either_seat_forgets_the_looked_at_pile` (reach: owners of the 3 looked-at cards differ, `viewer_knows` true before) | A4 (no holder resolution in advance) -> fails (P1 leg) | `v9_standard_shuffle_keeps_the_other_players_library_knowledge`; seam test `library_stamp_names_the_storage_seat` (B10 -> fails) |
| In-library writers | constructed `WaitingFor` + `apply(SelectCards)` (dig/scry/surveil readers are raw until Phase 8, so no card flow reaches P1 here) and `zones::move_to_library_position` | `v15_dig_choice_...`, `v15_scry_choice_...`, `v15_surveil_...`, `v15_library_placement_by_the_zone_pipeline_targets_the_pile` | A6 (dig holder), B6 (scry holder), B7 (reorder), A7 (placement) -> each lands ids in `players[1].library`/wrong order | pile order == chosen order, `players[1].library` empty, no id in two containers |
| `EffectZoneChoice` placement reads | `effect_zone_non_library_delivery_order`, `reposition_library_origins_after_batch_delivery` | lib tests `nth_from_top_pair_reads_the_shared_pile_not_the_empty_seat`, `mixed_owner_top_batch_is_one_pass_over_the_shared_pile`, `library_origin_reposition_sees_one_pile_across_owners` | B9a-d (pre-amendment code) -> all 3 fail | each has a Standard leg (identity) |
| Storage resolver | `zone_storage_seat` | `storage_seat_resolves_only_the_shared_zones` (all 7 zones x Dandan/Standard) | exhaustive match | |

Unmapped seams: none. No test is shape-only. Fixtures: every library-bearing fixture is non-degenerate (mixed-owner pile, both owners; reach asserts that `players[1].library` is empty before and the looked-at set has both owners). Dandan payload branch (`dandan_fixed_deck_payload`) revert not mutated separately: reverting it yields empty libraries which v1/v2 fail by construction.

DEFERRED(phase 11): which hand holds a dealt/drawn card (no hand-holder asserted). DEFERRED(phase 13): FreeReveal. DEFERRED(phase 8/9): raw read sites (`dig.rs`, `scry`, etc.). DEFERRED(phase 7): Bo3 between-games. DEFERRED(phase 17): `phase-ai` pool readers.

## 6. Maintainer-simulation matrix

| Row | Entry / first branch | Authority | Bound value + when | Mode | Storage | Consumers | Invalidation | Hostile fixtures | Serde |
|---|---|---|---|---|---|---|---|---|---|
| Canonical seat | any accessor; `shared_zone_holder` Shared arm | seat set | `PlayerId`, derived on read | live from an immutable set (`players` never shrinks: no `state.players.remove/retain/pop/drain` in `crates/engine/src`, re-grepped) | none (no field) | `zone_storage_seat` | none | seat_order rotated (`v1_pile_stays...`) | none |
| Library/graveyard storage | `zone_storage_seat(zone, seat)` | format axis `shared_zones()` | seat, per call | live | `players[canonical].library/graveyard` | accessors, appliers, epoch keys | n/a | P1-owned card in pile keeps `owner==P1` (v5/v8/v15) | none |
| Zone appliers / replay | `apply_resolved_zone_change` -> `zone_container_len` | recorded `owner` | `destination_position` at resolve, recomputed at replay by the same accessor | latched value, same resolver both sides | journal command | `apply_resolved_zone_change` | `DestinationPositionMismatch` | v8 | none |
| Shuffle | `resolve_and_apply_library_shuffle(player)` | acting seat | `command.player` = acting seat | storage resolved, acting seat recorded | journal | `apply_resolved_library_shuffle` | `UnknownPlayer` refusal kept | v8 shuffle by P1 | none |
| Mulligan shuffle / deal | `shuffle_hand_into_library`, `draw_n` | seat | holder seat at call | live | pile | mulligan flow | n/a | v3/v4 both seats, seat rotated | none |
| Draw selection | `select_cards_to_draw` | drawing player | top of resolved library | live | pile | `can_draw_at_least_one`, `apply_draw_after_replacement` | unknown-player guard kept | v5 | none |
| Epoch keys / stamp | `advance_library_knowledge_epoch` etc. | storage seat | epoch/generation per resolved seat | live | `product_knowledge_state` vectors | `viewer_knows_card_identity`, visibility stamp compare | pile reorder drops facts whose own storage seat == resolved seat | v9 (mixed owners, both shuffling seats) | existing vectors, unchanged shape |
| Deck provisioning | `load_deck_into_state` `library_holders` | storage seat | set computed once per load | live at load | `deck_pools` (one), pile in canonical seat | `deck_pool_of`, boosters/subset/printed_cards loops | non-holder payloads ignored entirely | v1 | none |
| Pool resolver | `deck_pool_of` | storage seat | `PlayerDeckPool` ref | live | `deck_pools` | Phase 17 readers; engine readers classified (below) | None when holder has no pool | v10 | none |
| Placement reads | `library_holders_of(chosen)` | card owner -> storage seat | holder set per batch | live | pile | delivery-order + reposition | per-card holder filter | V16 mixed-owner | none |
| Boot guard | wasm `initialize_game_impl` | n/a | `seats_with_empty_library()` | live | n/a | wasm | n/a | v14 source pin | none |

Seat-keyed `deck_pools` readers (re-run `git grep -n deck_pools -- crates/engine/src`): production hits are the plan's classes (boosters, card_subset, commander, companion, search_outside_game, tiny-leaders, visibility, printed_cards, match_flow, interaction, candidates sideboard); remaining hits are tests/comments/definitions. No uncharted reader; none converted. `deck_pool_of` is the authority for Phase 17.

## 7. CR-annotation diff gate
Command per executor.md run over `git diff` + untracked: zero `UNVERIFIED`. Numbers: 103.1 (test marker, pre-existing text), 103.5, 121.1, 400.1, 400.3, 401.4, 608.2c (retained on a function whose doc I edited), 701.24a. Content checked: 400.1 ("Each player has their own library, hand, and graveyard", modified here by the axis), 400.3 (owner's corresponding zone), 401.4 (placement order of a simultaneous batch), 121.1 (draw takes top of library), 103.5 (mulligan shuffle), 701.24a (shuffle).

## 8. Judgement calls
1. V7 vehicle: Calim does NOT parse as the plan assumed. In the current `client/public/card-data.json` Calim's conjure trigger has no `library_position` ("seventh from the top" dropped) and Calim's Breath has `Discard` with `self_ref:false` and no `activation_zone`, while coverage reports supported. I used real Mine Security ("conjure ... into the top eight cards of your library at random") for the same building block (`place_conjured_in_library`). Pre-existing, outside this phase; routed in Risks.
2. V15 a/b/d (`DigChoice`, `ScryChoice`, `SurveilChoice`) use a constructed `WaitingFor` driven through `apply(SelectCards)`: dig/scry/surveil readers are raw until Phase 8, so no card flow reaches a P1 choice in this phase. (c) uses `zones::move_to_library_position` (`zone_pipeline::move_object` is `pub(crate)`).
3. V16 leg 3 differs from the plan: a single P1-owned library-origin card at `NthFromTop{3}` (pile index 2 vs 0 under the old owner read). The plan's two-origin `Top` expectation is not what `reposition_library_origins_after_batch_delivery` produces (see Risks, item 2); mutation B9 shows leg 3 fails under the old code.
4. Pool gating in `load_deck_into_state` is a post-hoc `deck_pools.retain` against one `library_holders` set instead of three `if` wraps (identical result, no re-indentation of the pool constructors).
5. Accessors annotated `// allow-raw-zone:` (census BORROW rule), not an edited census.
6. `library_of(seat)` panics `"seat exists"` on an unknown seat, as the replaced `expect` sites did.
7. V8 builds records with `GameObject::snapshot_for_zone_change` (`ZoneChangeRecord::test_minimal` is `cfg(test)` in the lib, unreachable from the integration crate).

## 9. Stop-and-return items (admitted-class additions)
- `crates/engine/tests/fixtures/integration_cards.json.gz` must be regenerated (`python3 scripts/gen-test-fixture.py`): `--check` reports 15 uncovered cards, all referenced by the new test file: accumulated knowledge, capture of jingzhou, chart a course, day's undoing, flametongue kavu, halimar depths, haunted fengraf, lonely sandbar, mental note, mine security, mystic sanctuary, predict, remote isle, svyelunite temple, the surgical bay. I ran every test with `FORGE_TEST_FULL_DB=1` (full export) instead. Generated artifact, out of SCOPE_PATHS.

## 10. CR annotations added/changed
See item 7; each number verified with `grep -n "^<n>" docs/MagicCompRules.txt`.

## 11. Deviations from the plan
Items 8.1-8.4 above; also V3 pairs with a P0-mulligan leg compared to its own un-shuffled expectation; V9's second leg is acting seat P0 with a mixed-owner pile as planned. The plan's Step-0 note "main.rs carries an uncommitted Phase 4b mod line" did not hold (clean tree); one line added.

## 12. Risks / reviewer attention
1. Calim, Djinn Emperor (a Dandan card) parses without its library position and with a self-less discard cost; reports `supported`. Needs its own owner (card-data/parser), likely before Phase 8's card-level rows.
2. Pre-existing, independent of this change: `reposition_library_origins_after_batch_delivery` reinserts library-origin cards in descending index order, so two library-origin cards chosen for `Top` end interleaved (measured on the shared pile `[p1c, x, p0c, ...]`; same code path for one owner). Unfixed, no test depends on it.
3. The census row count moved by two (accessor annotations); `check-engine-authorities.sh` full diff-based script was not run (census legs only).
4. `scripts/check-interaction-bindings.sh --check` is cargo-driven and was not run; no type/serialized shape changed (protocol script exit 0).
5. Mutation batches ran in the worktree itself (restored by sha-verified copy + touch; final tree diffed clean, tests re-run after restore). One run (first regression attempt) was voided because restored files kept old mtimes and cargo reused the mutated build; the reported numbers are from the rerun after touch.
6. The Dandan payload branch needs `Some(db)`; with `None` (desktop path) a submitted payload is used, as Momir.
