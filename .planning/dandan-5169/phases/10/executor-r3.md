# Phase 10 executor r3 (phase mode, implementation/fix)

MODEL: claude-sonnet-5-5
Mode: implementation/fix. BASE_SHA/START_SHA: 6f59a8e4495f6911784dceac564f8eb01e456373. IMPLEMENTATION_WORKTREE: /home/lgray/vibe-coding/dandan-run/wt-dandan (start: clean, HEAD == START_SHA, nothing staged; bounce.rs had no pending edit). Nothing staged or committed; preparatory evidence only.

## Premise check
`owner_scoped_nonbattlefield_mass_filter` moves `controller` into `Owned` and adds no zone prop, so `claimed_shared_zone` returns `None`. Probe (Release to Memory over a pile of one P0-owned and one P1-owned Bears, cast by P0, Dandan): at 6f59a8e449 the P1 card exiles, the P0 card stays (`v17_exile_all_opponents_graveyards_takes_the_whole_shared_pile` red on the second assertion, reach assertion green).

## Change (one authority, three seams)
- `game/filter.rs`: new `claim_scan_zones(filter, zones)` names the scan zone (`InZone`, or `InAnyZone` for several) on every typed leaf that names none (recursing And/Or/Not/TrackedSetFiltered); no-op for Battlefield or empty zones. The reviewer's "push InZone inside the Owned rewrite" is generalised: a filter that already carries `Owned`/`controller` without the controller rewrite (resolution path, quantities) needs the same claim.
- `effects/change_zone.rs`: `ChangeZoneAll` resolution wraps the owner-scope rewrite in `claim_scan_zones(.., &origin_zones)`; `resolution_zone_candidates` (resolution-time `ChangeZone` with `origin`) claims `scan_zones`.
- `quantity.rs`: `ZoneChangeCountThisTurn` and `ZoneChangeAggregateThisTurn` claim `from` (record InZone is `from_zone`, so the claim equals the existing `from` predicate).

## Class sweep (state of search)
Searched: `client/public/card-data.json` effects/quantities/triggers whose zone field (origin/from/to/destination/zone) is Graveyard/Library and whose Typed leaf has a controller or Owned axis and no InZone/InAnyZone (scripts in the scratchpad, positive control = the 10 reviewer cards re-found), plus every production `FilterProp::Owned {` / `controller.take()` / `InZone {` constructor under `game/` and `game/effects/`.
- Fixed: `ChangeZoneAll` origin (10 cards: Author of Shadows, Disciple of Perdition, Elspeth's Nightmare, Hedonist's Trove, Kaya's Guile, Mnemonic Betrayal, Phyrexian Scriptures, Release to Memory, Szat's Will, Trash Panda); resolution-time `ChangeZone` origin (Cogwork Progenitor); `ZoneChangeCountThisTurn.from=Graveyard` with `Owned You` (Bonecache Overseer, Essence Anchor, Gau Feral Youth, Living History, Primary Research, Relic Retriever, Wilt in the Heat; Welcome the Dead is `from Hand`/`to Graveyard`, see below).
- `ChangeZone` origin Hand (8 cards): Hand is per-seat in Dandan, the licence is inert; not changed.
- Other `Owned {` hits are readers/rewriters of a filter that already names its zone (`filter.rs` bind_declaring_owner_authority, `ability_utils`, `bounce.rs` chosen-player, `meld`, `dungeon`): single occurrence, nothing to claim.
- NOT fixed, measured remainder (nameable ground: needs its own design, the record door keys `InZone` on `from_zone` and has no destination-keyed licence): (a) trigger valid_card filters whose zone is the trigger's `origin`/`destination` field: 37 `ChangesZoneAll` origin Graveyard + 9 `ChangesZone` Battlefield-to-Graveyard with an `Owned` axis (the `controller` axis, ~440 cards, is the plan-5a departure-controller decision); (b) `ZoneChangeCountThisTurn` with `to: Graveyard` and an `Owned` axis (Welcome the Dead, `from Hand`); (c) `ZoneChangeObjectMatchesFilter.origin` (Archfiend's Vessel, Grist, Prized Amalgam). Same cause: the licence reads InZone on the filter only, and these records carry the zone in `to_zone`/trigger fields.

## Tests (crates/engine/tests/integration/dandan_filter_owner_axis.rs; fixture regenerated: +arcane signet, cogwork progenitor, release to memory, relic retriever; `gen-test-fixture.py --check` ok)
Red at 6f59a8e449 (git-archive scratch copy plus the new test file and fixture; scratch and its target deleted), green after:
| claim | seam | test | assertion that flips |
|---|---|---|---|
| exile all opponents' graveyards takes the pile | ChangeZoneAll mass | v17_exile_all_opponents_graveyards_takes_the_whole_shared_pile | `mine` zone == Exile |
| "an artifact card in your graveyard" offers the pile | resolution_zone_candidates | v17_an_artifact_card_in_your_graveyard_offers_the_whole_shared_pile | offered contains the P1 artifact |
| a card leaving the pile left your graveyard | ZoneChangeCountThisTurn.from | v18_a_card_leaving_the_shared_graveyard_left_your_graveyard | treasures == 1 for a P1-owned card |
Standard twins (green at base and after, per-seat unchanged): v17_exile_all_opponents_graveyards_in_standard_leaves_the_casters_own, v17_an_artifact_card_in_your_graveyard_in_standard_offers_only_your_own, v18_in_standard_only_your_own_graveyard_counts. Reach guards: P1 card exiled / P0 artifact offered / card in Hand (Regrowth) / Standard positive control (own card -> Treasure) and exile reach for the negative leg. Cogwork stages two P0 artifacts so a one-candidate auto-pick cannot hide the prompt. Real cards, verbatim Oracle from the fixture.

## Preparatory verification (not completion evidence)
- `cargo fmt --all -- <the four edited .rs paths>`; fixture `--check` ok.
- `cargo clippy --workspace --all-targets -- -D warnings`: rc=0 (log p10r3-clippy.log).
- Targeted nextest (bounce, change_zone, filter, quantity, dandan_filter_owner_axis, dandan_scoped_counts, dandan_axis_collapse_tests, dandan_scoped_zone_tests, dandan_read_sweep): 2489 run, 2489 passed (p10r3-nextest.log).
- Full `-p phase-engine` nextest: 32597 run, 32597 passed (p10r3-full.log).
- Parser gate: no file under parser/ touched. CR gate: no CR number added or changed (0 found in the diff).

## Maintainer-simulation (changed seams)
| seam | entry / first branch | authority and binding | storage | consumer | invalidation | hostile rows |
|---|---|---|---|---|---|---|
| claim_scan_zones at ChangeZoneAll | resolve ChangeZoneAll, origin_zones non-Battlefield, Typed leaf with no zone prop | zone = effect origin, bound at resolution, live predicate | transient filter clone | `filter_inner` typed arm `claimed_shared_zone` | per-seat zones and non-shared formats: `zone_storage_seat` identity, extra-seat set empty | Standard twin; Battlefield origin returns early |
| claim_scan_zones at resolution_zone_candidates | ChangeZone with origin Some and resolution-time choice | scan_zones, bound per call | transient | same | same | Standard twin, two-own-artifacts staging |
| claim_scan_zones at ZoneChangeCountThisTurn/Aggregate | `from` Some | `from`, equal to the record-side InZone meaning | transient | `zone_change_filter_inner` Owned arm | `from` None leaves filter unchanged | Standard negative via Release (P1 card), positive via Regrowth |
Serde/protocol/card-data: none (no enum, state or serialized shape change; fixture only).

## Judgement calls, deviations, risks
- Claimed on all typed leaves lacking a zone, not only where `controller` moved into `Owned`: the licence reads the filter's own zone for `Owned` and controller arms alike, and nested Not/Or leaves are over objects already in the scanned zone, so matching is unchanged.
- Risk: remainder (a)-(c) above are the same shape in the record door and need the destination-keyed design; carry them to the Phase 10 summary or a named later phase.
- The 10-04 addendum for change_zone.rs already names this file; no new scope file (quantity.rs, filter.rs already scoped).
