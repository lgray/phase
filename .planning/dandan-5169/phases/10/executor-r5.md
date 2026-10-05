# Phase 10 executor r5 (phase mode, implementation/fix)

MODEL: claude-sonnet-5-5
Mode: implementation/fix. START_SHA == BASE of this round: 7eaadea7bc6cb43af573bffdde849702c51d1db3. IMPLEMENTATION_WORKTREE: /home/lgray/vibe-coding/dandan-run/wt-dandan. Start: clean, HEAD == START_SHA. End: HEAD unchanged, nothing staged, unstaged delta = filter.rs, quantity.rs, trigger_matchers.rs, effects/change_zone.rs, casting.rs (compiler-forced: two `FilterContext` literals), integration_cards.json.gz (+4 cards), tests/integration/dandan_filter_owner_axis.rs. Preparatory only; nothing committed.

## Step 1: cause confirmed
`logical_zone_production_carriers_settle_...` failed at 7eaadea7bc; neutralising the `OneOf` origin claim in `zone_change_clause_matches` turned it green. Cause: `claim_scan_zones` injects a real `InAnyZone`, and the record door's property table fails `InAnyZone` closed (a `OneOf` origin such as Hand+Graveyard). The injected `InZone` is likewise a real `from_zone` constraint. Hypothesis held.

## Cut
- `FilterContext.claimed_zones: &[Zone]` + `with_claimed_zones`; default `&[]` in all constructors and the 6 literals; `filter_inner_for_object` takes it as a parameter (all 17 call sites thread `ctx.claimed_zones` / the local).
- `claimed_shared_zone(properties, claimed, zone)` is the one licence authority for both doors (filter's own `InZone`/`InAnyZone`, or `claimed` contains a non-Battlefield zone). `claim_scan_zones` and its 5 call sites deleted (0 references remain).
- Seams set the ctx field: `change_zone.rs` `resolution_zone_candidates` (scan_zones) and the `ChangeZoneAll` mass path (origin_zones); `quantity.rs` `ZoneChangeCountThisTurn`/`ZoneChangeAggregateThisTurn` (`from`); `trigger_matchers.rs::zone_change_clause_matches` (origin Equals(non-Battlefield)/OneOf, unchanged match).
- Record door `controller` arm: the existing match became the `admits` closure over the record's controller (arms unchanged, `_ => {}` still admits) and runs through `zone_axis_admits(state, licensed, record.controller, admits)`; `Owned` arm uses the same `licensed`.

## Walk (member list; command)
Command: `perl -0ne` over `crates/engine/src crates/phase-ai crates/server-core` for `matches_target_filter_on_zone_change_record` call sites outside filter.rs, plus the live-door seam list from executor-r4.
Record-door callers: quantity.rs `ZoneChangeCountThisTurn`, `ZoneChangeAggregateThisTurn` (seam, covered); `SacrificedThisTurn`, `TokensCreatedThisTurn` (no non-filter zone: battlefield departure / creation, per-seat); trigger_matchers `zone_change_clause_matches` (seam, covered), sacrificed-self look-back, `match_exploited`, ceased-token row (all Battlefield-origin records, per-seat); `match_land_played` (origin = event `from_zone`; walk of card-data `LandPlayed` `valid_card`: no controller/Owned leaf with a shared origin except The Magic Bandit `Owned Opponent`, hand/exile origins; event-keyed, no hit); triggers.rs suppressor source_filter (event record, filter-carried zone only); effects/mod.rs `use_lki` condition (event record, filter-carried); delayed_trigger.rs return-result count (keyed on `record.to_zone == spec.destination`, destination-keyed: recorded drop with r4's destination verdict). Live-door seams: unchanged enumeration from executor-r4 (cost/Dig/Search/Reveal members have 0 bare leaves).
Population caveat: the walk is over the call sites named above (grep-instrumented); live-door callers outside the effect/quantity/trigger seams were not re-walked this round.

## Tests (crates/engine/tests/integration/dandan_filter_owner_axis.rs, filter.rs `dandan_axis_collapse_tests`)
All real parsed cards; Oracle text read from card-data (Along the Crooked Way "Whenever a creature card leaves your graveyard, amass Goblins 1."; Colossal Grave-Reaver "Whenever one or more creature cards are put into your graveyard from your library, put one of them onto the battlefield."; Desert Warfare "...whenever a Desert card is put into your graveyard from your hand or library, put that card onto the battlefield under your control at the beginning of your next end step.").
| claim | test | flips when reverted | twin / reach |
|---|---|---|---|
| controller-form graveyard origin (r3 probe) | v20_a_controller_form_origin_trigger_reads_the_whole_shared_pile | P1-owned pile card gives 1 Goblin Army | own card 1; helper asserts card left graveyard; Standard twin v20_in_standard_..._controller_form (own 1, Release to Memory on P1 card 0) |
| Library-origin ChangesZoneAll controller form | v20_a_library_origin_controller_form_reads_the_whole_shared_library | P1-owned milled Bears put onto battlefield (1) | reach: graveyard+battlefield == 2 milled; Standard twin (P0 own 1, P1 library 0) |
| OneOf-origin (Hand, Library) real card | v20_a_one_of_origin_trigger_reads_the_whole_shared_library | 2 delayed triggers for P1-owned Deserts | reach: 2 Deserts in graveyard; Standard twin (own 2, opponent 0) |
| admitted-member refusal | `caller_claimed_zones_license_exactly_the_claimed_shared_zone` (constructed: literal controller-You and `Owned:You` filters with no zone) | n/a (refusal rows) | no claim, other-zone claim ([Hand]), [Battlefield], and a per-seat format all stay OWNER_ONLY on both doors; claimed pile gives BOTH on both doors |
Existing row `a_zone_change_record_collapses_only_for_a_named_shared_origin` changed: the r3-pinned OWNER_ONLY for a record controller axis with a named pile origin now asserts BOTH (that pin was the defect); a Battlefield-origin controller row asserts OWNER_ONLY.
Red-by-revert (git-archive scratch of the working tree, reflink target, deleted): all reverts (record controller arm direct, trigger/change_zone/quantity ctx claims removed): 10 red of 33 (v17 exile-all, v17 cogwork, v18, v19, v20 x4 Dandan rows, quantity aggregate row, both filter unit rows), Standard twins green. Controller-arm revert alone: 5 red (3 v20 Dandan rows incl. Desert Warfare, 2 filter unit rows). Scratch removed.

## Preparatory verification (not completion evidence)
- fmt on the six edited .rs paths (rustfmt --check clean).
- `cargo clippy --workspace --all-targets -- -D warnings`: rc=0 (r5-clippy2.log).
- `cargo nextest run --workspace --no-fail-fast`: 37299 run, 37299 passed, 51 skipped (r5-nextest.log). The 3 round-4 failures pass: logical_zone_production_carriers_settle_..., perpetual_gains::opponent_qualified_union_fires_on_opponent_events_only, perpetual_gains::oglor_repeated_grants_retain_multiplicity_across_zone_cycles. v17/v18/v19/dandan_scoped_counts rows green.
- CR gate: added citations CR 400.1, CR 109.4 both verified in docs/MagicCompRules.txt; 400.1 (zones/containers) and 109.4 (objects outside stack/battlefield have no controller) describe the annotated code. Parser gate: no parser file touched.

## New-field threading: `FilterContext.claimed_zones`
Constructors (9) and literals in casting.rs x2, aura-enchant ctx, shares-quality ctx: `defaults intentionally because` no seam scans a non-filter zone there. `filter_inner_for_object` recursion and `StackSpell`/`ChosenDamageSource` rebuilds: threads the field. Seams above: set it.

## Judgement calls, risks
- Mixed `[Battlefield, Graveyard]` scans now license the Graveyard member (old rewrite claimed nothing when Battlefield was present); inert outside shared-zone formats.
- The record door's `_ => {}` ControllerRef arms (DefendingPlayer, ActivePlayer, ...) still admit unconditionally, as before; not changed.
- casting.rs is outside scope.nul but forced by the struct field (two 1-line literals).
- Destination-keyed members and `ZoneChangeObjectMatchesFilter.origin` remain the recorded drops from r4; delayed_trigger.rs return-result count joins the destination-keyed class.
