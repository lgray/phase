# Phase 8 implementation review r1 (phase mode), range e0d9cb1208..616148f99e

MODEL: claude-sonnet-5-5. W HEAD == 616148f99e at review time, tree clean; scratch probes in a `git archive` of 616148f99e (not W).

Review Head: 616148f99e92d7650fd8b777d2b5594515e784ed

## Findings

**[MED]** [text] Stale doc on `effects/mod.rs::candidate_player_scalar`: it still claims to read the candidate's graveyard directly off `Player`, but the `GraveyardSize` arm moved to `candidate_player_scalar_with_state` (the stateless fn now returns `None` for it, and the moved unit row asserts that). Evidence: the doc block above `pub(crate) fn candidate_player_scalar(p: &Player, attr: &QuantityRef)`. Why it matters: a false statement about the code, and it names a CR (404.1) the function no longer implements. Suggested fix: old string `/// CR 402.1 / 119.1 / 119.3 / 122.1f / 404.1: Read scalar \`attr\` for one` -> `/// CR 402.1 / 119.1 / 119.3 / 122.1f: Read scalar \`attr\` for one`; old string `/// reads each player's own hand size / life total / life lost or gained /\n/// graveyard / player-counter rather than the controller's.` -> `/// reads each player's own hand size / life lost or gained /\n/// player-counter rather than the controller's.`; and on the `candidate_player_scalar_with_state` doc replace `/// CR 402.1 / 119.1 / 403.3 / 608.2h: Per-candidate scalar lookup that needs game-state\n/// backing (battlefield entry ledger).` with `/// CR 402.1 / 119.1 / 404.1 / 403.3 / 608.2h: Per-candidate scalar lookup that needs game-state\n/// backing (battlefield entry ledger, shared-zone graveyard).` (404.1 verified present in W/docs/MagicCompRules.txt).

No behavior or machinery findings. Counts: behavior 0, text 1 (MED), machinery 0.

## Pre-existing / non-blocking observations (untagged, not repeated later)

- `clash.rs::top_card_of_library` read `.last()` (the bottom) at base, in every format; the candidate fixes it to `.front()` per CR 701.30a (grepped). Rows `v15_clash_reveals_the_top_card_of_a_standard_library` and `v15_clash_reveals_the_pile_top_for_either_seat` go red when it is reverted (probe below). A format-wide behavior change for the 27 Clash cards, stated in plan 4.5.
- `morph.rs::top_library_object` owner-clause drop (plan 4.3, decided there): P1 manifesting/cloaking a P0-owned pile top now succeeds where it errored at base, and the card enters under the library owner's control (`manifest.rs`: `controller: None` keeps the library owner's control, CR 110.2a would give the putting player control). No Dandan decklist card reaches it (plan measured); `v8_soul_summons_...` asserts zone/face-down only, so the controller leg is unasserted by design.
- The `obj.owner == player` gates on graveyard/library cast and play (`casting.rs` 2379/6613/8851-area, `casting_costs.rs`, `cast_from_zone.rs`) remain; plan 4.3 labels them `DEFERRED(phase 11)`, but charter Phase 11 does not touch them (it states "no Dandan card casts or plays from the shared library/graveyard directly"). The charter's rationale holds the decision; the plan's `DEFERRED(phase 11)` label is a mislabel only.
- Tests locate abilities with `format!("{:?}", ability.cost).contains("...")` (Surgical Bay, Kylox's Voltstrider, Altar, Baron): stringly, but each row also asserts an outcome (and the Altar/Baron rows have a negative leg), so they are not vacuous.

## Probes

Scratch tree = `git archive 616148f99e`, six production reverts applied at once, then `nextest` of `dandan_read_sweep_tests` (lib) and `dandan_read_sweep` (integration); the same build's unreverted rows are the positive control (lib 14 of 18 pass, integration 42 of 46 pass).

| Reverted site | Rows red |
|---|---|
| `casting.rs::non_owner_graveyard_ids` dedup (iterate every seat) | lib `non_owner_graveyard_ids_dedup_the_shared_pile`, `non_owner_grants_list_each_pile_card_once` |
| `effects/clash.rs` `.front()` -> last | integration `v15_clash_reveals_the_top_card_of_a_standard_library`, `v15_clash_reveals_the_pile_top_for_either_seat` |
| `morph.rs` owner clause restored | integration `v8_soul_summons_manifests_the_pile_top_whoever_owns_it` |
| `effects/reveal_top.rs` single-target read raw | integration `v4_sifter_wurm_reveals_the_pile_top_for_life` |
| `engine_payment_choices.rs::pay_top_library_exile_cost` length raw | lib `top_library_exile_cost_pays_from_the_shared_pile` |
| `effects/mod.rs` `GraveyardSize` read `candidate.graveyard` raw | lib `candidate_graveyard_size_reads_the_storage_authority` |

Exactly the mapped rows went red (4 lib, 4 integration), none other. Not run: a W-tree full suite (orchestrator's completion checks). Executor's base-red table is not re-run.

## Gates checked

- Scope: `git diff --name-only` equals `phases/8/scope.nul` (48 paths, diffed). Frontend untouched. Fixture: `python3 scripts/gen-test-fixture.py --check` ok (4964 cards). `scripts/check-engine-authorities.sh` PASS in W.
- Census (plan V13) at the candidate, `REPO=. python3 brief/scripts/count_reads_lines.py 616148f99e '\.(library|graveyard)\b(?!\s*\()'`, `engine/src/game/` residual 24 lines: F set (derived `reveal_top_all` 1, costs unrestricted 1, engine `has_libraries` 1, targeting 4, triggers 1, visibility 2 = 10) + D set (quantity 5, the three scoped-player arms) + Phase 6 kept-raw (deck_loading 1, engine_resolution_choices 6, mulligan 2 = 9). Equals the plan's sets; no other residual. Each F site read: all-seat flat loops or existentials that see the pile once under canonical storage.
- Class-for-the-card: no further single-seat container read in the 45 paths; owner-keyed gates are the plan-4.3 set above.
- L-class `non_owner_graveyard_ids` (casting.rs): partitions the pile by owner. The own pass (`graveyard_spell_objects_available_to_cast`) skips `obj.owner != player`, the L pass keeps exactly `owner != player`, so no card is listed by both and none twice; non-shared formats are identity because a card sits in its owner's graveyard (CR 400.3, 404.1). Matches CR 400.1/404/601.3 (all grepped). Unit rows cover dedup in Dandan from both callers and Standard.
- `candidate_player_scalar`: only production caller is `candidate_player_scalar_with_state`, which falls through to the stateful arm; no regression from the stateless `None`.
- Substitutes (Oracle text from W `client/public/card-data.json`, matching Scryfall): Corpseberry Cultivator ("At the beginning of combat on your turn, you may forage. ... Whenever you forage, put a +1/+1 counter on this creature."), Kylox's Voltstrider ("Collect evidence 6: This Vehicle becomes an artifact creature until end of turn. ..."), Parker Luck, Oaken Brawler: each exercises the handler the plan named and asserts an outcome the swapped read decides (offered set equals pile, exiled ids, per-seat `CardsRevealed`, life/rad). Inline-unit substitutes for Skullwinder (bounce), Krosan Avenger/Kindly Stranger (restrictions), Swallowed by Leviathan (stack_reach) are each paired with a Standard/per-seat control and the stack_reach and bounce rows are red at base per executor; the revert probe covered 6 sites, not these three.
- CR annotations in added lines (118.12, 118.3, 305.1, 400.1, 400.3, 404.1, 406.3, 601.2a, 608.2d, 701.17b, 701.25a, 701.30a, 701.30d, 701.40a, 701.58a, 728.1) all resolve in W/docs/MagicCompRules.txt and describe their lines (118.3 and 400.3 are the loosest fits, adequate).
- Nom/idiom: no parser code; no `Effect::Unimplemented` literals; `allow-raw-zone` annotations removed only with their raw statements; building blocks reused (`library_of`/`graveyard_of`/`_mut`, `zone_storage_seat`, `candidate_player_scalar_with_state`).
