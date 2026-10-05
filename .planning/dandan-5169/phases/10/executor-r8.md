# Phase 10 executor r8 report

MODEL: claude-sonnet-5-5
Mode: implementation/fix (phase mode). START_SHA d7c1237a6f783952740fd48254c5f8cf7a816a3f (clean, nothing staged at start; HEAD unchanged and nothing staged at end). IMPLEMENTATION_WORKTREE /home/lgray/vibe-coding/dandan-run/wt-dandan. All evidence PREPARATORY, not completion evidence.

## Verdict
F1 and F2 closed; no stop-and-return item. Unstaged delta = 4 paths: `game/filter.rs`, `game/trigger_matchers.rs`, `tests/integration/dandan_filter_owner_axis.rs`, `tests/fixtures/integration_cards.json.gz` (Archfiend's Vessel newly named; `gen-test-fixture.py --check` rc 0).

## Diff
- `filter.rs`: `zone_change_filter_inner` takes `licensed: Option<Zone>` (threaded through Not/Or/And; no local `record.from_zone`). `matches_target_filter_on_zone_change_record` keeps `record.from_zone` (callers with no origin notion unchanged); new `pub(crate) matches_target_filter_on_zone_change_record_licensed(.., licensed, ..)`. `matches_zone_change_event_object_filter`: live entrant with `origin: Some(from)` calls `matches_target_filter_on_departure` (F2, same channel as the trigger entrant arm); its LKI arm passes `origin` as `departed_from`; its two record-door calls use `origin` as licence (an origin-less condition states no zone).
- `trigger_matchers.rs`: `named_origin_zone(origin, record.from_zone)` is the single "trigger names the zone the card left" predicate; the entrant arm adds only the CR 603.6a named-battlefield-destination gate on top; the record arm calls the licensed door with it (F1). A named non-battlefield destination (e.g. exile from graveyard) does not suppress the licence.
- Comments: CR 603.10a cited only for a graveyard origin, CR 400.1 "as the format modifies it" for a library origin (the over-claiming trigger_matchers line was deleted; the same qualifier applied to the two filter.rs docs I touched).
- Tests (`dandan_filter_owner_axis.rs`): `play_to_end_step` extracted from `pile_run_by`; `energy`, `marvel_energy_after_mill`, `marvel_energy_after_own_destroy`, `reanimate_run`. Rows: Marvel Dandan/Standard; Vessel, Prized Amalgam, Grist Voracious Larva each Dandan (own + other-seat owner) and Standard twin. Unit: `each_door_licenses_only_the_zone_it_states` gains stated-licence cells (record door, Some(zone) vs None, both pile zones, both formats); new `an_event_object_condition_licenses_only_a_named_origin`; V15 `a_destroy_observer_...` and V23 `a_named_battlefield_destination_...` now run Standard too (`reanimation_matches` takes the format).

## Class check (one search)
`git grep -n "matches_zone_change_event_object_filter\|matches_target_filter_on_departure\|matches_target_filter_on_zone_change_record"` + `git grep origin_constraint game/replacement.rs`, plus a card-data walk (`client/public/card-data.json`, `ZoneChange*ThisTurn` and `ZoneChangeObjectMatchesFilter` nodes). Members: trigger arm (F1); `matches_zone_change_event_object_filter` live arm (F2) and its non-battlefield record arm (same shape as F1, 0 cards: no origin-less non-battlefield node with an axis leaf, fixed anyway, unit row); both callers (`effects/mod.rs`, `triggers.rs`) go through that one function. Left as is: `ZoneChangeCountThisTurn`/aggregate with `from: None` (6 cards, every leaf is `Owned`, i.e. a "put into your graveyard" destination claim where the shared-pile licence is the S2c reading, not a battlefield-control claim); sacrificed/created-token/exploit/ceased-row/suppressor/land-play/delayed-trigger callers (origin Battlefield, none, or no origin notion; identity or owner-guarded). `replacement.rs` EnteredFromZone never evaluates a filter against the named origin.

## PREPARATORY verification (direct cargo, own target dir; no Tilt)
- `cargo fmt --all -- <3 .rs paths>` and `--check` rc 0.
- `cargo clippy --workspace --all-targets -- -D warnings` rc 0 (`.planning/dandan-5169/p10r8-clippy.log`).
- Targeted nextest (filter, trigger_matchers, change_zone, quantity, casting, dandan_filter_owner_axis, dandan_scoped_counts, dandan_read_sweep, perpetual_gains, triggers, bounce): 4718 passed, 0 failed (`p10r8-targeted.log`).
- Full `cargo nextest run -p phase-engine`: 32704 passed, 12 skipped, 0 failed (`p10r8-full.log`).
- `scripts/check-engine-authorities.sh` PASS; `gen-test-fixture.py --check` rc 0; no protocol/binding type changed.
- Parser gate: not applicable (nothing under `parser/`).

## Mutation evidence (git-archive scratch of HEAD + the 4 edited files, own CARGO_TARGET_DIR/CARGO_HOME, files touched after each restore; scratch deleted; logs `/home/lgray/vibe-coding/dandan-run/mut-r8/`)
Selected set: `dandan_filter_owner_axis`, `a_named_battlefield`, `each_door_licenses`, `an_event_object_condition`, `a_destroy_observer` (38 tests).
| Mut | Meaning | Red |
|---|---|---|
| control (before) | unmutated | 38/38 pass |
| M1 | record arm licensed by `record.from_zone` (the d7c1237a6f behaviour) | exactly `a_card_milled_from_a_library_is_not_a_permanent_you_control` (real Aetherworks Marvel, Dandan, energy 2 instead of 0) |
| M2 | live entrant read through the plain door (the d7c1237a6f behaviour) | exactly 3: `an_entrant_from_the_shared_pile_entered_from_your_graveyard` (Archfiend's Vessel), `a_creature_from_the_shared_pile_entered_from_your_graveyard_for_a_graveyard_watcher` (Prized Amalgam), `a_creature_you_control_from_the_shared_pile_transforms_a_graveyard_grist` (Grist, Voracious Larva) |
| M3 | event-object non-battlefield record arm licensed by `record.from_zone` | exactly `an_event_object_condition_licenses_only_a_named_origin` |
| control (after) | restored | 38/38 pass |
I did not run the d7c1237a6f tree itself; M1/M2 reinstate its exact behaviour at the two seams.

## Production-path coverage map
| Claim | Seam | Entry / test | Fails on revert | Negative / twin |
|---|---|---|---|---|
| origin-less trigger over a Library-origin record states no zone | `zone_change_clause_matches` record arm | real Thought Scour milling P1's Bears with real Aetherworks Marvel: `a_card_milled_from_a_library_is_not_a_permanent_you_control` | M1 | reach: 2 Bears milled; paired control: P0's own destroyed Bears (Doom Blade) gives energy 1; Standard twin `in_standard_a_milled_card_is_not_a_permanent_you_control` (control 1, P1 mills own library 0) |
| named origin still licenses (departure/record doors) | same | existing V18-V20 rows, door matrix stated-licence cells | M1 variants of the matrix | origin-less cell `None` -> per-seat |
| "entered from your graveyard" reads the entrant as a pile resident | `matches_zone_change_event_object_filter` live arm | `an_entrant_from_the_shared_pile_entered_from_your_graveyard` (Vessel: Demon token), `a_creature_from_the_shared_pile_entered_from_your_graveyard_for_a_graveyard_watcher` (Amalgam returns), `a_creature_you_control_from_the_shared_pile_transforms_a_graveyard_grist` (`accept_optional`, transformed) | M2 | each: P0-owned control, Standard twin (own card 1/true, other seat's own card 0/false) |
| origin-less event-object condition | non-battlefield record arm | unit `an_event_object_condition_licenses_only_a_named_origin` | M3 | reach: named Library origin true |

## Maintainer-simulation matrix (rows touched)
- Record door, trigger arm: bound value = zone licence `Option<Zone>`, bound at trigger match from the trigger's own `origin` + event record; live predicate on the event snapshot; no storage; consumers `zone_axis_admits` (controller + `Owned` leaves); no invalidation (per-event). Hostile rows: origin-less + Library record (Marvel), Hand/Battlefield origins stay per-seat (door matrix), named non-battlefield destination keeps licence.
- Event-object condition, live arm: licence = the condition's `origin`; binding live read of the entrant with a stated departure zone; consumers as above; the entrant leaving again routes to the LKI arm, which now passes `origin` too. Hostile rows: other-seat owner from the pile (3 cards), Standard other-seat owner.
- Serde/protocol/card-data: none (private enum, `pub(crate)` fn).

## CR gate
Added: 400.1, 603.10a, 603.6a, all resolve in `docs/MagicCompRules.txt`; UNVERIFIED none. 400.1 ("each player has their own library, hand, and graveyard") supports the library-origin qualifier; 603.10a lists the leaves-a-graveyard look-back and is cited for a graveyard only.

## Judgement calls / deviations / risks
- The brief's F1 control reads "the watcher's own card is milled and the trigger still fires per the card's text"; per the card's text a milled card is not a permanent you control (CR 109.4), so no milled card fires Marvel. The control is the watcher's own permanent destroyed (fires, 1) in both formats. The Standard own-library mill still gives 2 (pre-existing parse gap, origin None): not asserted.
- Consequence to know: the 28 origin-less "put into your/an opponent's graveyard" destination-claim triggers (impl-review-r6 population) no longer collapse for a Library-origin record in Dandan either (a milled P1 card reads per-seat, as at the phase base). Fixing that needs a parser origin or a destination licence; not done.
- Prince of Thralls has the same shape and goes through the same arm; no separate row (one card class row per brief: Marvel).
- V23/V15 Standard twins added; both discriminate nothing about the fix beyond pinning Standard unchanged (V23 has one Standard-vs-Dandan cell that differs).
- `addenda/phase-10` lines for casting.rs and change_zone.rs (impl-review-r6 non-blocking text constraint) are not in this brief and were left untouched.
- Prized Amalgam / Grist rows reach their triggers from the graveyard / via `accept_optional`; they were red under M2 so the reach is real.
