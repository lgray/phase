MODEL: claude-sonnet-5-5

# Phase 11 executor r2 (implementation/fix, phase mode)
Mode implementation/fix. BASE_SHA = phase base (unchanged, not re-derived). START_SHA = IMPLEMENTATION_WORKTREE HEAD = d574d8fa4da747d40010a839c08839f8f106a023, /home/lgray/vibe-coding/dandan-run/wt-dandan. Clean at start, no staged entries; at end HEAD unchanged, nothing staged, unstaged delta = the 6 files below (5 in scope.nul + dandan_shared_pile_storage.rs per addenda/phase-11). All evidence below is PREPARATORY, not completion evidence.

## Diff
- effects/dig.rs: `move_mass_put_all_selected` request gains `.hand_taker(player)` (F1).
- zones.rs: one `pub fn resolve_and_apply_zone_change(.., owner, receiver: Option<PlayerId>, record)`; cfg wrapper and `_to_receiver` deleted; production call and 4 tests updated (F2).
- effects/cast_from_zone.rs tests (2 calls), tests/integration/dandan_shared_pile_storage.rs (1 call): `None` receiver (F2).
- zone_pipeline.rs: test renamed `a_format_without_shared_zones_does_not_rebind`; `hand_taker` doc replaced verbatim (F3).
- tests/integration/dandan_hand_entry_ownership.rs: new row `v4c_dig_put_all_into_hand_belongs_to_the_receiver`.
`git grep 'to_receiver\|the_hand_entry_axis_gates'` empty; no `cfg(any(test, feature` left on the function.

## F1 census (plan-vs-code note: the plan's census premise was wrong)
Command: `jq -r 'to_entries[] | .key as $n | [ (.value | .. | objects | select(.type? == "Dig" and .destination? == "Hand" and .keep_count? == 4294967295 and .up_to? == false)) ] | select(length>0) | $n' client/public/card-data.json | sort -u` -> accumulate wisdom, depala pilot exemplar, desperate research, marina vendrell, see the truth, tamiyo collector of tales, wood sage (7; matches the reviewer). The put-all sentinel is `keep_count == u32::MAX`, not null; r1 keyed on null and dropped a real Hand-capable producer. Marina Vendrell Oracle (jq, verbatim): "When Marina Vendrell enters, reveal the top seven cards of your library. Put all enchantment cards from among them into your hand and the rest on the bottom of your library in a random order." Cost WUBRG.

## Class check
Search: non-test `ZoneMoveRequest::`/`move_objects_simultaneously*`/`performed_by`/`hand_taker` in dig.rs, seek.rs, explore.rs, engine_resolution_choices.rs (awk strip at `#[cfg(test)]`, grep), then read each Hand-capable site; plus jq for Dig `rest_destination == "Hand"`.
Members: supplied = DigChoice kept map, seek, explore, ChangeZone; put-all = the one fixed here. Unsupplied Hand-capable sites left: route_kept_card_or_defer (445), route_rest_partition_then (1054), apply_search_partition primary (1555), Discover/cast-rejection to Hand (9663, 9694; origin Exile). All are plan 3.4 dropped sites already judged to hold in r1. The unkept-to-`zone` arm (4588) is reachable only for `kept_destination == Library`; the 4 corpus rest->Hand Dig cards (genesis ultimatum, harper recruiter, nine-fingers keene, nissa nature's artisan) have kept destination Battlefield/null, so none reaches it. Result: single occurrence of the class (kept-side arm supplied, sibling kept-side arm not), repaired; no other member.

## Verification (PREPARATORY, all at START_SHA + unstaged delta)
- fmt: `cargo fmt --all -- <the 6 paths>`.
- `cargo clippy --workspace --all-targets -- -D warnings` rc 0 (r2-clippy.log).
- targeted nextest (dandan|hand_entry|rebind|cast_from_zone|dig|zone_change): 700 passed (r2-targeted.log); includes v4c, v4, v4b, a_format_without_shared_zones_does_not_rebind.
- full `nextest -p phase-engine`: 32727 passed, 12 skipped, rc 0 (r2-full.log).
- `node scripts/check-protocol-version.mjs` rc 0 (no protocol/serde change in this round).
- `python3 scripts/gen-test-fixture.py --check` rc 0 ("fixture covers all 5001 referenced cards"); Marina Vendrell already in the fixture, no regeneration.
- Parser gate: no parser/ file touched. New-field sweep: no field added. CR gate: zero CR tokens on added lines.

## Discriminating-test gate (coverage map)
Claim: a Dig put-all to Hand delivers the card to the casting player's hand, owned by that player. Seam: `move_mass_put_all_selected`. Entry: real Marina Vendrell cast via `GameRunner::cast` + trigger resolution (`run_to_prompt`) -> Dig put-all branch -> `move_mass_put_all_selected`. Test: `v4c_dig_put_all_into_hand_belongs_to_the_receiver`, legs (Dandan, P1 over P0-owned pile), (Dandan, P0 over own pile), (Standard, P1 own library). Reach guards: the enchantment starts in Library; `assert_hand` requires exactly-one-hand membership, owner, controller and arrival-record owner. Revert evidence (git-archive scratch copy of HEAD + delta, own CARGO_TARGET_DIR, scratch deleted; r2-mutation.log): control 3/3 pass; mutant (`.hand_taker(player)` deleted) v4c FAIL, v4 and v4b pass; restore + touch -> 3/3 pass. Only the Dandan P1-over-P0 leg can differ under the mutant (receiver == owner in the other two). Standard twin: owner unchanged.
Maintainer-simulation row: authority = `PlayerId` in `EntryMods.performed_by`, bound at request construction (resolution of the ETB Dig), latched in the request, stored in `PendingBatchZoneMoveRequest`, consumed by `hand_entry_receiver` at delivery against the final `to`; invalidation: a Moved redirect to Exile keeps the performer (accepted residual, doc now says so); no serde/protocol change.

## Judgement calls
`resolve_and_apply_zone_change` stays `pub` (base had plain `pub`; the integration test crate calls it). Its doc keeps the `owner`/`receiver` paragraph from the deleted `_to_receiver` doc.
## Stop-and-return: none. Deviations: none beyond the above. Risks: Marina Vendrell fixture row relies on the in-tree gz fixture (shared_card_db early-returns when absent, repo idiom).
