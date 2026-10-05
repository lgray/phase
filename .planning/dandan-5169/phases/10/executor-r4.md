# Phase 10 executor r4 (phase mode, implementation/fix)

MODEL: claude-sonnet-5-5
Mode: implementation/fix. BASE_SHA/START_SHA: 0825f1f505f0685a2c67ee2df0385fc65d81d750. IMPLEMENTATION_WORKTREE: /home/lgray/vibe-coding/dandan-run/wt-dandan (start: clean, HEAD == START_SHA, nothing staged; end: HEAD unchanged, nothing staged, unstaged delta = the three paths below, all in scope). Preparatory evidence only; nothing committed.

## Premise re-verified
Probe on the candidate (new `v19_*` row, real Chalk Outline "Whenever one or more creature cards leave your graveyard", P0 Regrowth on a pile Bears), run before the fix: P0-owned card -> 1 Detective (reach), P1-owned card -> 0 (`v19_a_creature_card_leaving_the_shared_graveyard_left_your_graveyard` red on the second assertion). Reviewer's premise holds; the cause is a trigger-field zone (`origin`) the licence cannot see.

## Change
- `game/trigger_matchers.rs::zone_change_clause_matches`: on the record branch, claims the trigger's origin zones (`Equals(z)` other than Battlefield, `OneOf(zs)`) on `valid_card` via `claim_scan_zones`. The live-object branch (`to == Battlefield`) is untouched: an `InZone` claim would fail a battlefield object.
- `game/quantity.rs`: test only (aggregate row).
- `tests/integration/dandan_filter_owner_axis.rs`: `retriever_run` split into `pile_run` (watcher parameter) + `battlefield_named`; v19 rows added. No fixture change (Chalk Outline, Regrowth, Release to Memory already in the fixture; `gen-test-fixture.py` rewrote identical content, 0 cards differ).

## Enumeration (one pass)
Population: carriers = struct/enum variants in `types/ability.rs` holding both a `Zone`-typed field and a `TargetFilter` field (`python3 /tmp/.../tyscan.py`, regex over the type definitions, 28 hits); card population = every supported card (`coverage-data.json` supported x `card-data.json`) walked for any dict whose zone-key (origin/from/to/destination/source_zones/zone/...) is Graveyard/Library and whose own filter fields (And/Or/Not/TrackedSetFiltered-reachable) hold a Typed leaf with `controller` or `Owned` and no `InZone`/`InAnyZone` (538 hit cards; positive control Aang re-found; the walked names resolve 23/23). Source-zone fields `trigger_zones`/`active_zones`/`activation_zone` excluded (location of the ability source, not of a scanned object); `StaticDefinition.affected_zone` has 0 populated values in card data and no production reader.

| member | zone carried by | class | supported hit cards (bare leaf) | disposition |
|---|---|---|---|---|
| `ChangeZoneAll` origin | effect | ORIGIN | 9 | fixed r3 |
| `ChangeZone` origin, resolution-time choice | effect | ORIGIN | 0 supported (Unforgiving One leaves are in nested `ObjectCount`) | fixed r3 |
| `ZoneChangeCountThisTurn.from` / `ZoneChangeAggregateThisTurn.from` | quantity | ORIGIN | 7 + 1 Library (Welcome the Dead); aggregate: 0 bare, only Genesis of the Daleks (`from Battlefield`) | fixed r3; aggregate row added here |
| trigger `origin`/`origin_zones`/`zone_change_clauses[].origin` (`ChangesZone`/`ChangesZoneAll`, record branch) | trigger | ORIGIN | ChangesZoneAll Graveyard 34, ChangesZoneAll Library 5 (all destination Graveyard), ChangesZone Library 4 incl. `origin_zones` (Pedantic Learning, Undead Alchemist, Desert Warfare, Oglor), clause origin Graveyard 6 (Murktide Regent, Kishla Skimmer, ...) | fixed here |
| `AbilityCost::Exile`/`ExileWithAggregate`/`ReturnToHand` `zone` | cost | ORIGIN | 0 (every non-self filter in all printed Graveyard/Library cost rows carries `InZone`) | none needed |
| `SearchLibrary.source_zones`, `Dig`, `RevealUntil`, `Seek`, `ChooseFromZone`, `FreeCastFromZones`, `ChooseAugment` zones | effect | ORIGIN | 0 on the object filter (`SearchLibrary` hits are `target_player`, a player filter) | none needed |
| trigger `spell_cast_origin` + `valid_card` (`SpellCast`/`PlayCard`) | trigger | ORIGIN (object on the stack) | 0 (no `valid_card` bare leaf in any of the printed rows) | none needed |
| `ZoneChangeObjectMatchesFilter.origin` (Archfiend's Vessel, Grist, Prized Amalgam: `Card + Owned You`) | condition | ORIGIN by meaning, but evaluated on the live battlefield entrant / its LKI | 3 | RECORDED DROP (see verdict) |
| batched-trigger subject count (`count_trigger_subjects_in_batch`, live moved object) | trigger | live moved object, not a scan | 0 (no supported origin-Graveyard/Library batched trigger reads "that many") | none needed |
| `ChangesZone`/`ChangesZoneAll` destination Graveyard/Library, `ZoneChangeCountThisTurn.to`, `Mill`/`ChangeZone`/`ChangeZoneAll` destination, replacement `destination_zone` | trigger/effect/quantity | DESTINATION | 474 hit cards | RECORDED DROP (see verdict) |

Verdicts (for the Phase 10 summary; not code):
- Destination-keyed members: dropped. The object is not yet in the shared pile when the filter reads its pre-move state, so the record door has no origin zone to license; owner-based destination triggers are a separate design. One-line walk: `dest ∩ names = []`, `origin-keyed ∩ names = []`, `all 538 hit cards ∩ the 23 Dandan names = []` (control: the same sets are non-empty against the walked population, 23/23 names resolve in card-data).
- `ZoneChangeObjectMatchesFilter.origin`: dropped. Its origin is the leaving zone, but the filter reads the entrant's live/LKI battlefield state (an `InZone` claim would fail it), so the licence needs a different channel than `claim_scan_zones`; no Dandan-list card, 3 supported cards.

## Tests (crates/engine/tests/integration/dandan_filter_owner_axis.rs; quantity.rs inline `dandan_scoped_zone_tests`)
Red-by-revert on a git-archive scratch copy of 0825f1f505 (my test files overlaid; `trigger_matchers.rs` at START, aggregate claim line removed; scratch deleted): the Dandan trigger row and the aggregate row failed; v17/v18 rows and the v19 Standard twin stayed green (10 run, 8 pass, 2 fail), green with the change.
| claim | seam | production entry | test | assertion that flips | siblings |
|---|---|---|---|---|---|
| a creature card leaving the shared graveyard fires "leave your graveyard" for a P1-owned card | `zone_change_clause_matches` record branch | Regrowth resolves through `apply`, real Chalk Outline trigger | `v19_a_creature_card_leaving_the_shared_graveyard_left_your_graveyard` | Detectives == 1 for P1 pile owner | own card (reach, helper asserts the card left); Standard twin `v19_in_standard_only_your_own_graveyard_triggers` (own -> 1, Release to Memory on P1 card -> 0, with exile reach) |
| aggregate `from Graveyard` claims the pile | `ZoneChangeAggregateThisTurn` arm | `resolve_quantity` over a constructed record (no supported card has a non-Battlefield `from`) | `zone_change_aggregate_from_the_shared_graveyard_claims_the_pile` | P1-owned record sum == 3 in Dandan | P0-owned record (reach), Standard 0 for P1 and 3 for P0 |

Fixture is non-degenerate: pile Bears is a creature card (reaches the typed leaf), Chalk Outline's trigger is `batched` with origin `Graveyard` and `Owned You`, destination None.

## Maintainer-simulation (changed seam)
| field | value |
|---|---|
| seam / first branch | `zone_change_clause_matches`: `valid_card` present, not the `to == Battlefield` live branch |
| selected authority | the trigger's `origin` constraint |
| bound value, when | zone list, derived per evaluation from the trigger definition; record `from_zone` is already required to match by `matches_from` one statement earlier |
| binding mode | live predicate on the definition (nothing latched) |
| storage | none; transient filter clone only when origin names a non-Battlefield zone |
| consumer | `zone_change_filter_inner` typed arm `claimed_shared_zone(properties, record.from_zone)` |
| invalidation | `Any`/`NotEquals` claim nothing; Battlefield origin skips the clone; per-seat zones and non-Dandan formats keep identity `zone_storage_seat`, so the licence is inert (Standard twin) |
| hostile rows | Standard twin; own-seat reach; `OneOf` including Hand stays per-seat for Hand records (claim matches `from_zone`) |
| serde/protocol/card-data | none |

New-field sweep: no field added. CR gate: only `CR 400.1` added, verified (`grep -n "^400.1" docs/MagicCompRules.txt`), same citation `filter.rs` uses for the shared-zone axis. Parser gate: no file under `parser/` touched.

## Preparatory verification (not completion evidence)
- `cargo fmt --all -- <the three edited .rs paths>`.
- `cargo clippy --workspace --all-targets -- -D warnings`: rc=0 (p10r4-clippy.log).
- Targeted nextest (trigger_matchers, change_zone, filter, quantity, dandan_filter_owner_axis, dandan_scoped_counts, dandan_axis_collapse, dandan_scoped_zone, dandan_read_sweep, bounce): 2703 run, 2703 passed (p10r4-nextest.log).
- Red probes: p10r4-probe.log (pre-fix premise), p10r4-red.log (revert).

## Judgement calls, deviations, risks
- `OneOf` origins are claimed as well as `Equals` (Desert Warfare/Oglor shape); the brief named `Equals` only. Same shape, three lines, no new branch.
- Library-origin triggers that land in the graveyard ("put into your graveyard from your library") read the Library claim; correct because Dandan shares both zones, and no other format shares either.
- Brief said ~8 lines; the seam is 11 added lines (Battlefield guard avoids a filter clone on every "dies" trigger).
- Shared-target scratch build left artifacts in `target-dandan` for the deleted scratch path; sources removed.
- No addendum needed: the 10-04 addendum already admits `trigger_matchers.rs`.
