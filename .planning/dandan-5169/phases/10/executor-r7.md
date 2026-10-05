# Phase 10 executor r7 report

MODEL: claude-sonnet-5-5
Mode: implementation/fix (phase mode). BASE_SHA: not supplied (phase base per plan). START_SHA: 32b58e3df2d957534837d7fa834581d97f74f21a. IMPLEMENTATION_WORKTREE: /home/lgray/vibe-coding/dandan-run/wt-dandan.
Worktree record: clean start at HEAD == START_SHA, nothing staged; end: HEAD == START_SHA, nothing staged, unstaged delta = exactly 6 paths (casting.rs, change_zone.rs, filter.rs, quantity.rs, trigger_matchers.rs, tests/integration/dandan_filter_owner_axis.rs). All below is PREPARATORY, not completion evidence.

## Verdict
Unit A redesign implemented per plan; claim layer deleted; every mutation row flips (N2 count differs, see Deviations). No stop-and-return item.

## 1. Diff summary
- `game/filter.rs`: deleted `claimed_zones`, `with_claimed_zones`, `claimed_shared_zone`, every `claimed_zones` parameter/literal. Added `ControllerLookup::{OwnerZone, Departed(Zone)}`, `resident_zone` (single licence authority), `matches_target_filter_on_departure`, `filter_inner_with_lookup`; owner-zone door passes `OwnerZone`; record door `licensed = record.from_zone`. Tests: old claim pins deleted/rewritten; V4 `each_door_licenses_only_the_zone_it_states` (Graveyard + Library, Dandan + Standard, controller/Owned x You/Opponent, plain/owner-zone/departure/record doors; departure cells on the pile card), `a_battlefield_resident_is_judged_by_its_controller`, V15 `a_destroy_observer_ignores_...`.
- `game/trigger_matchers.rs`: `destination_constraint_names`; `zone_change_clause_matches` judges a live entrant by the departure door only when origin is `Equals`/`OneOf` and the trigger does not name the entered zone, else plain door (CR 603.6a); non-entrants use the record door. V23 test `a_named_battlefield_destination_judges_the_permanent_not_the_pile`.
- `game/effects/change_zone.rs`: both seams call `matches_target_filter_for_zone(obj.zone)`; no list claim. V21 `a_mixed_tracked_set_is_judged_per_member_zone` (incl. {P0 pile, P1 battlefield} row, both formats).
- `game/quantity.rs`: two `with_claimed_zones` lines deleted (rustfmt re-wrapped the `ZoneChangeCountThisTurn` arm). `game/casting.rs`: two literals deleted.
- `tests/integration/dandan_filter_owner_axis.rs`: `pile_run_by` (caster param; `pile_run` delegates with P0), V20 caster row `v20_a_reanimation_by_either_seat_...`, V22 `v22_a_creature_you_control_entering_...` (Impact Tremors, life (18,19)/(20,18) in Dandan and Standard).
- The planner prototype diff was applied (production hunks, patch -p1 after path rewrite, clean) and then reviewed; its probe tests were replaced by the assertion tests above. Unit B / V5-V12 untouched.
- `grep -rn "claimed_zones\|with_claimed_zones\|claimed_shared_zone\|claim_scan_zones" crates` : no hit.

## 3. PREPARATORY verification (direct cargo, no Tilt; own CARGO_TARGET_DIR)
- `cargo fmt --all -- <the 6 paths>`: done.
- `cargo clippy --workspace --all-targets -- -D warnings`: rc 0 (`.planning/dandan-5169/p10r7-clippy.log`).
- Targeted nextest (filter, trigger_matchers, change_zone, quantity, casting, dandan_*, perpetual_gains, triggers, bounce): 4709 passed, 0 failed (`p10r7-targeted.log`). The 3 Standard tests (`logical_zone_production_carriers_*`, `opponent_qualified_union_*`, `oglor_repeated_grants_*`) pass.
- Full `cargo nextest run -p phase-engine`: 32695 passed, 12 skipped, 0 failed (`p10r7-full.log`). No old pin failing: the three old-design pins were rewritten in the edit, not left failing.
- `node scripts/check-protocol-version.mjs` OK; `scripts/check-engine-authorities.sh` PASS; `scripts/check-interaction-bindings.sh --check` rc 0; `python3 scripts/gen-test-fixture.py --check` rc 0 (no new card name needed regeneration).
- Runtime rows are non-vacuous: real card db loaded (rows take 6-13 s each; `shared_card_db()` would return early otherwise) and every negative has its reach leg in the same test.

## 4. Parser gate: not applicable (no file under crates/engine/src/parser/ changed).

## 5/6. Production-path coverage map and matrix (one row per seam)
| Seam / door | Entry | Test | Assertion that fails on revert | Sibling/negative |
|---|---|---|---|---|
| owner-zone door (`OwnerZone`) | `targeting`/`change_zone` -> `matches_target_filter_for_zone` | V1-V3 integration, V4 | N0, N3 | Standard legs, Hand cell |
| plain door refusal | `match_sacrificed`, `match_destroyed` observers | V14 (real Bloodbriar), V15 (synthetic: no supported `Destroyed` trigger) | N1 | paired own-seat leg |
| record door (`record.from_zone`) | `zone_change_filter_inner` | V16, V18, V19 Regrowth, V20, V4 | N4, N5 | Battlefield/Hand/None cells |
| departure door | `zone_change_clause_matches` live entrant | V19 Reanimate, V20 caster, V4 | N1, N6, N7 | V22 (origin-less), V23 (gate) |
| mass/resolution seams | `change_zone::resolve_all`, `resolution_zone_candidates` | V17, V21 | N8, N3 | Standard legs |
Binding: all doors live except record/departure doors, which read the event snapshot `record.from_zone` (CR 603.10a); storage none; invalidation none. Hostile fixtures: owner != controller (V1 override), P1-owned card in P0's container (V19), battlefield member (V21, V4 battlefield test), caster seat != watcher (V20, V22). Serde/protocol/card-data: none (FilterContext is not serialized; ControllerLookup private).
New-field sweep: no field added; two variants on private enum `ControllerLookup` (consumers: `resident_zone` exhaustive match, `effective_controller` `matches!(LiveOrLki)` so both new variants skip LKI like `LiveOnly`). `claimed_zones` removed from all 11 struct literals + casting.rs.

## Mutation table (scratch copy of the working tree, own target dir; control = unmutated copy, 172 tests, all pass, run before N0 and after N10)
Filter: `test(dandan) or oglor or opponent_qualified or logical_zone_production` plus the new inline tests. "Rows" = tests that turned red.
| Mut | Red rows | Plan prediction |
|---|---|---|
| N0 | V1, V2, V3, V4 (each_door, battlefield, record, owner-zone entry, filter-named), V17 (both), V18, V19 (both), V20 (all 4 + caster), V21, V23, plus bounce Phase 8 test and `zone_change_aggregate_from_the_shared_graveyard_claims_the_pile` | match (superset) |
| N1 | V14, V15, V4 (each_door, `an_unlicensed...`, battlefield), V19 Reanimate, V20 caster, V23; V1-V3 stay green | match |
| N2 | `bounce::...chosen_non_canonical_player...` and `a_filter_naming_the_shared_zone_admits_every_seat` (V4 named cells); V1-V3 green | 2 failures, plan said 4 (see Deviations) |
| N3 | V17 (both), V4 each_door, V21 | match |
| N4 | V4 each_door + record pin, V18, V19 Regrowth, V20 (controller, Library, OneOf), quantity aggregate unit | match |
| N5 | V16, V4 each_door + record pin, V18, V19 Regrowth, V20 controller form, quantity aggregate unit | match (superset) |
| N6 | V4 each_door, battlefield test, V19 Reanimate, V20 caster, V23 | match |
| N7 | V19 Reanimate, V20 caster, V23 | match |
| N8 | V17 (both), V21 | match |
| N9 | V22 (Dandan leg), V23 | match |
| N10 | V23 | match |
Logs: `/home/lgray/vibe-coding/dandan-run/mut/{restore,N0..N10}.log` (the `restore.log` is the post-N10 control). Scratch tree, scratch target dir and scratch CARGO_HOME deleted.

## 7. CR gate
Added numbers: 109.5, 603.10a, 603.6a, all present in docs/MagicCompRules.txt (`grep -nE "^603.10a|^603.6a|^109.5"` shows each rule says what its annotation claims). UNVERIFIED: none.

## 8. Judgement calls
- Applied the prototype's production hunks mechanically (identical to plan sections 3.1, B); tests are written fresh from the plan's cells.
- V4 omits a pile-card-with-`InZone Hand` cell (that filter rejects a graveyard card outright); the named-per-seat-zone leg is the pre-existing Hand-zone legs in `a_per_seat_zone_never_collapses`.
- V20 caster row asserts both pile owners in one test (loop), so N6/N7 report it as one red row.

## 9. Stop-and-return: none.

## 11. Deviations
- N2: 2 failures observed against the plan's count of 4. Every row the plan names by property flips (Phase 8 bounce test, V4 named-leg cells); V1-V3 stay green. The prototype run's other two failing legs were not identified (unmeasured here; the old claim pins they may have been are deleted/rewritten).
- V9 "red at base" recording is not applicable: Unit B and V5-V12 were not touched.
- The rustfmt re-wrap of the `ZoneChangeCountThisTurn` arm in quantity.rs (about 20 lines moved) is formatter output.

## 12. Risks
- `departed` is computed in `zone_change_clause_matches` for non-entrants too (unused there); harmless.
- Standard-format legs of Reanimate by the other seat remain unasserted (R2, filed upstream #9559).
