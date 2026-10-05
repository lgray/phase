# Phase 10 plan review r4 (delta plan.r4.md -> plan.md; phase-plan mode, phase-fit declared; Sizing blocking)

MODEL: claude-sonnet-5-5
Verdict: 0 behavior, 0 machinery, 3 text (each with replacement), no RE-CHARTER. B1 closed. Sizing consistent (two units). The delta has zero design findings, so the whole-artifact pass was run (sections A, 0, 2-5, 7-11 re-read; nothing further).

Probe method: scratch `git archive` of W HEAD `e0a2dfdaa2` + `dandan-run/scripts/p10_prototype.diff` (applies cleanly), own target dir, deleted after (W untouched, `git status` clean). Throwaway tests: `r4_caster_row` (the V20 caster row as specified: `pile_run` with the caster seat as a parameter, real Reanimate cast by P1, watcher P0, pile Bears owned by P0 / by P1, both formats, watchers Along the Crooked Way and Chalk Outline) and `probe_doors_pile_object` (departure cells evaluated on the pile creature). Mutations N1, N4, N6, N7 applied to the prototype and run on the existing rows plus the two probes.

## (1) B1 closed

- Baseline (prototype): caster row, Dandan, Along the Crooked Way: 1 Goblin Army for pile owner P0 and for P1 (Bears reach: on the battlefield). Standard: 0 and 0 (the pre-existing R2 reading), so "no Standard leg for the caster row" is right.
- N6 (`resident_zone` ignores `Departed`): caster row Along 0/0 (red for both owners); V19 reanimation test red (Chalk Outline P1-owned 0); V4 departure `Graveyard` cell (true,true) -> (true,false).
- N7 (live-entrant branch uses the plain door): same flips (Along 0/0, Chalk Outline P1-owned 0, `v19_a_card_reanimated_from_the_shared_graveyard_left_your_graveyard` red); the existing V20 rows stay green, so the row that discriminates is the new caster row, exactly as N6/N7 now say.
- N4 (record door `licensed = None`): red = V18, V19 Regrowth leg, three V20 rows (Regrowth, Library, `OneOf`); reanimation legs and caster row stay green. Matches the plan's N4 line.
- The row can fail in the direction it guards (controller form x live entrant x other-seat reanimator) and N6/N7/N1 name rows that exist and flip.

## (2) Sweep accuracy (re-grepped at W HEAD)

Accurate. `dandan_filter_owner_axis.rs` has v1-v3, v14, v16-v20 (`goblin_armies`/`pile_run` take no caster; P0 always casts); no V15 (the `v15_*` tests in `dandan_shared_pile_storage.rs` / `dandan_read_sweep.rs` are other phases' dig/scry/clash rows, a label collision only), no V21 (no Dandan test in `change_zone.rs`), no V22 (no Impact Tremors test), no V23 (no departure-gate test); V14/V16 have Dandan legs only (`sacrifice_run` P0/P1), so "Dandan legs only" is right; V4's old pins `an_unlicensed_filter_keeps_one_comparison`, `a_zone_change_record_collapses_only_for_a_named_shared_origin`, `caller_claimed_zones_license_exactly_the_claimed_shared_zone` exist and the first two fail on the prototype (by design). Chalk Outline's trigger is `ChangesZoneAll`, `valid_card` `Typed{Creature, controller null, [Owned You]}` (Owned form); Along the Crooked Way's clause is `zone_change_clauses[origin Equals Graveyard, Typed{Creature, controller You}]` (controller form). `GraveyardPermissionSource::admits_card` and its `pool.admits(obj.owner, player)` exist. Issue #9559 exists (open) and matches R2.

## (3) The three deviations

- T3 (command row instead of "19"): correct, and the count was already stale: `p10_trigger_census.py buckets` now reports 21 for `('ChangesZone', 'origin_none', 'Graveyard', 'C')`. The collapse rule it states (Library/Graveyard `from_zone` only) matches the record door.
- T5 (regenerate command): correct. `git diff --name-only 1acc74bca6 9287a640b9` = 10 files; `scope.nul` (6) and the seven scope paths are subsets, so the union with `addenda/phase-10` is 11, below 13. The reviewer's `<tip>` form would give 131 (the upstream merge), so the planner's pinned range is the right one.
- T6: section 3.1 item 8, section A and section 8 are correct (108.4a + 109.4; 109.4 text "Objects that are neither on the stack nor on the battlefield aren't controlled by any player. See rule 108.4"). 109.5 in 3.2 (the "your graveyard" count: the word "your" on the source object) and section 8 has the right subject. Section 0 does not (finding T-a).

## Text findings (each with replacement)

T-a. Section 0, "The \"your graveyard\" reading adopted": old "CR 108.4a + CR 109.5 (\"you\" for a card with no controller means its owner) are why the engine reads \"your graveyard\" as an ownership claim" -> "CR 108.4a + CR 109.4 (a card in a graveyard has no controller, so a controller comparison against it reads its owner) are why the engine reads a controller leaf on a pile card as an ownership claim; CR 109.5 (\"your\" on the source names its controller, whose graveyard it is) names the seat". Why: the quoted 109.5 clause concerns a controller-less source object, not the candidate card the claim is about.

T-b. V4 row, "the executor turns the probe into assertions" -> "the executor turns the probe into assertions, with every departure-door cell evaluated on the pile creature; `probe_doors` evaluates them on a battlefield entrant, whose Hand/Battlefield cells do not move under N1". Measured: with the pile creature, N1 turns departure `Hand` and `Battlefield` from (true,false) to (true,true) (red); with the battlefield entrant N1 leaves them (true,false) and only the `Graveyard` cell flips. The row's own text ("the cell object sits in the pile graveyard") is right; the pointer to the probe contradicts it, and the over-admission-refusal sentence for the departure door, `Owned` axis, rests on these cells.
Also (non-gating): N1's line omits that its first return replaces the `Departed` arm, so N1 also reddens the V19 P1-owned Reanimate leg and the V20 caster row (measured: Along 0/0, Chalk Outline P1-owned 0). Append "and, because it also replaces the `Departed` arm, the V19 P1-owned Reanimate leg and the V20 caster row" to the N1 line.

T-c. Section 7, last-but-one bullet (known T4 wording): old "V8, V14, V15, V16, V22 preserve the current reading of their card; each expected value is derived from the card text quoted in its row" -> "V8, V14, V16, V22 preserve the current reading of their card; each expected value is derived from the card text quoted in its row (...). V15 is synthetic and quotes no card: its expected values are derived from the 'you control' leaf of its hand-built trigger". Why: V15 is labelled synthetic (no supported `Destroyed` card), so no card text exists to derive from.

## Pre-existing (untagged, never blocking; carried from r3, not repeated as findings)

- Standard format: a P0-owned card reanimated by P1 does not fire P0's controller-form "leaves your graveyard" trigger (measured again: Along 0/0 for caster P1, both owners); filed as #9559, no Phase 10 row depends on it.
