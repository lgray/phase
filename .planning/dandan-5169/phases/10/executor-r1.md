# Phase 10 executor report r1 (implementation/fix, phase mode)

Mode implementation/fix. BASE_SHA = START_SHA = 1acc74bca68775098b9a9c8700700c2a3815446f. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Worktree record: clean at start, HEAD == START_SHA; at end HEAD == START_SHA, nothing staged, delta only in the six scope paths (scope.nul sha256 9f45bebf...0327 re-read). All results below are PREPARATORY, not completion evidence.

## Verdict
Both units implemented; all new tests green; 8 mutation probes red as intended; clippy workspace green; full phase-engine nextest 32589/32590 green. ONE STOP-AND-RETURN: Phase 8's inline test `game::effects::bounce::dandan_read_sweep_tests::chosen_non_canonical_player_returns_a_card_from_the_shared_pile_graveyard` (bounce.rs, not in scope) fails because it encodes the pre-collapse reading.

## Step 0 re-measurement (at START_SHA)
- `targeting.rs` has no `matches_target_filter_in_owner_zone` hit; it calls `matches_target_filter_for_zone` (re-validation arm and `zone_object_ids` scan). Plan 0 premise holds.
- `zone_storage_seat`, `graveyard_of`, `library_of`, `canonical_seat` exist (`types/game_state.rs`); `candidate_player_scalar_with_state` already reads `graveyard_of` (Phase 8 swap landed), so V9 is red at base (both seats read the pile, count 2).
- `shared_resource_dedup_key`, `aggregate_over_teams` exist. HEAD `quantity.rs` raw `.graveyard/.library` reads: exactly the 5 scoped-player lines.
- Plan premise replaced: P-closure's "`return false` rewritten to `false`" is unnecessary; `return false` inside the closure returns from the closure, arms are untouched (only wrapped).
- Oracle text of all 16 cards re-read from `client/public/card-data.json`: verbatim match with plan section 0.

## Diff summary
- `game/filter.rs`: `claimed_shared_zone` (licence = the filter's own `InZone`/`InAnyZone` naming the object/record-origin zone), `zone_axis_admits` (holder, then every seat with equal `zone_storage_seat`), `owned_axis_admits` (the two 16-arm `Owned` bodies collapse into one; comment-only differences between them), typed-arm controller check wrapped in a closure under `zone_axis_admits`, top-level `Owned` handled in the typed loops (object and record door). Inline tests `dandan_axis_collapse_tests` (V4).
- `game/quantity.rs`: `zone_dedup_key`, `distinct_zone_holders`, `scoped_zone_holders`, `zone_container`, `resolve_per_zone_scalar` (`resolve_per_player_scalar` is now a `None` wrapper), `player_attribute_container_zone`; the `CardTypeSetSource`, `ZoneCardCount`, `GraveyardSize` arms read through the accessor and the keyed population; `resolve_player_count` PlayerAttribute arm dedups on container. Inline tests `dandan_scoped_zone_tests` (V12 + relation guard).
- `tests/integration/dandan_filter_owner_axis.rs` (new: V1, V2, V3, V14, V16), `dandan_scoped_counts.rs` (new: V5-V11), `main.rs` (+2 mod lines), fixture `integration_cards.json.gz` regenerated (+8 cards, none removed/changed; `gen-test-fixture.py --check` ok).
- No enum/struct/WaitingFor/serde shape change: new-field threading sweep n/a; protocol/bindings untouched (`check-protocol-version.mjs` ok; `check-engine-authorities.sh` PASS).

## Preparatory verification
- `cargo fmt` on the four exact .rs paths (never `main.rs`, whose mod list makes rustfmt recurse).
- `cargo clippy --workspace --all-targets -- -D warnings`: rc 0 (before the final test-only additions); `cargo clippy -p phase-engine --all-targets -- -D warnings` after: rc 0.
- `cargo nextest run -p phase-engine --no-fail-fast`: 32590 run, 32589 pass, 1 fail (the bounce.rs test above). New tests: 32 (31 + the chosen-player row), all pass in default fixture mode.
- Parser gate: no file under `parser/` touched; n/a.

## Discriminating-test gate / maintainer matrix (revert evidence)
Mutations applied to a pristine copy, tree restored and sha-checked after each (restored sha256 filter.rs 8241dccf..., quantity.rs d37541da... at the time):
| mutation | red tests |
|---|---|
| M1 controller arm single comparison + M2 typed-loop `Owned` plain + M3 record `Owned` plain | v2 (M1); v1, v3 (M2); unit: naming-shared-zone, owner-zone-entry (M1), record-origin (M3) |
| M1' licence := object zone (always `Some`) + M8 record typed controller collapsed on destination | v14 (counter on Bloodbriar), v16 (Zulaport drain); unit: unlicensed-keeps-one-comparison, record-origin |
| M4 `zone_container` and GraveyardSize extractor back to raw reads | v5, v6-opponents, v7, v10 (x2) |
| M5 `scoped_zone_holders` -> `scoped_players` + M6 `counted_containers.insert` removed + M7 `distinct_zone_holders` removed from `resolve_per_zone_scalar` | v11 (M5), v9 + unit relation row (M6), unit Sum/Min row (M7) |
Reach-guards are paired in each test (own-seat/sacrifice/sixth-card legs, Fengraf in graveyard after cost, sanctuary untapped, pile staged). Standard-format twin for every shared row. Real faces only.
- V4 deviation: the plan's "`InZone Hand` claim the object does not meet" leg is dropped: such a filter fails the `InZone` property for every seat, so it cannot discriminate. The unlicensed (no property) and nested-`Not` legs carry M1'.
- V15 not built: `match_destroyed` needs a `TriggerSourceContext` with no cheap constructor, no supported card carries a `Destroyed` trigger, and V14 drives the same live typed arm through the real `Sacrificed` matcher. Verdict: superseded.
- V8/V11 (Visions of Beyond, Cognivore) are positive controls for the pre-existing raw sum, as the plan labels them; V11 is red only under M5.
- Added row beyond plan: `a_chosen_players_shared_graveyard_admits_every_owner` (ChosenPlayer axis on both doors' arm; both owners' pile cards admitted, a non-seat chosen player admits none).
- `DEFERRED(phase 11)`: which hand receives a returned/drawn card (V1 asserts zone == Hand only; V8 pile side only).

Maintainer-simulation rows (selected authority = the player the existing arm resolves: source controller, scoped/target/chosen/specific player; bound at evaluation, live; no storage; consumers = every `matches_target_filter*` caller; invalidation: none, seat set never shrinks): object typed controller arm; live `Owned`; record `Owned`; record typed controller (deliberately NOT collapsed, V16/M8); count arms (key evaluated at resolution; storage = canonical seat container); PlayerCount (key inserted only for candidates that passed relation+comparison). Hostile rows: owner != controller (V1 override, unit owner-zone row), V14/V16 unlicensed observers on pile objects, per-seat zones and Standard format (unit).

## CR annotations
Added: CR 400.1, 404.1, 109.5 (grepped in docs/MagicCompRules.txt: 400.1 "each player has their own library, hand, and graveyard", 404.1 graveyard definition, 109.5 "you" for an owner-held object). Gate output: zero `UNVERIFIED`. The other CR numbers in the diff are moved/pre-existing lines. CR 400.3/108.4a from the plan are not newly written (existing annotations on the owner-zone function remain).

## STOP-AND-RETURN: Phase 8 reference reading now superseded (out-of-scope file)
- Derived reading: "the chosen player's graveyard" in a shared-graveyard format is the pile for that player; per the plan's section 0 reading both `Own` (P0-owned) and `Theirs` (P1-owned) pile cards satisfy `Typed{controller ChosenPlayer, Owned{ChosenPlayer}, InZone Graveyard}`.
- Measured: the new unit row shows both are admitted; the Phase 8 test (bounce.rs, `Bounce` AtResolution over exactly that filter) asserts `players[1].hand.contains(theirs)` with `own` left in the pile, true only while the owner axis excluded `Own`; it now fails at that assertion (full-suite run, panic at the hand assertion).
- Rows affected: that single test.
- Smallest fix site: the test body in `crates/engine/src/game/effects/bounce.rs` (add that path to scope): both cards become candidates, so it must answer the resolution choice (select `theirs`) or assert the offered set is `{own, theirs}`. I did not edit it.

## Judgement calls / risks
- Licence per plan 3.1 (filter's own claim), not `obj.zone`; closure keeps all arms verbatim.
- `resolve_per_zone_scalar` takes 8 args: `#[allow(clippy::too_many_arguments)]` on it.
- Dedup is applied after the `exclude` filter in `AllPlayers`, so excluding the canonical seat in a shared format still counts the pile once via the other seat (no supported card uses `exclude` on `GraveyardSize`).
- Risk for review: every `Typed{controller, InZone Graveyard/Library}` and top-level `Owned` + that `InZone` filter now widens in Dandan; the Phase 6/8 integration suites stayed green apart from the bounce test above.
- Logs: p10-*.log under `W/.planning/dandan-5169/`.
