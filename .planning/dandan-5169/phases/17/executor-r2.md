# Phase 17 executor report r2

Mode: implementation/fix (phase mode). BASE_SHA(phase) = 1370f485471706db195f5a33b8cdf6dcbf670216. START_SHA = 4deaccc74884f5d29f6c01d49795926d5573d493. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Start: clean, HEAD == START_SHA. End: HEAD unchanged, nothing staged, delta = casting.rs, casting_costs.rs, dandan_look_top_of_library.rs (all in scope.nul). All checks PREPARATORY.

## Card reading (jq verbatim, client/public/card-data.json)
Future Sight: "You may play lands and cast spells from the top of your library." Realmwalker: "You may cast creature spells of the chosen type from the top of your library." Bolas's Citadel: "You may play lands and cast spells from the top of your library. If you cast a spell this way, pay life equal to its mana value rather than pay its mana cost." Under Dandan "your library" is the one shared pile (CR 400.1 shared zone, `library_of`), so the top card is castable by the holder whoever owns it; Citadel's life rider applies equally.

## Fix and class sweep
`git grep -n -E 'Zone::Library.*owner|owner.*Zone::Library' casting*.rs` + whole-file read of every top-of-library reader. Four sites skipped a non-holder-owned pile top; all now compare containers:
1. `casting.rs::castable_from_current_zone` (the admission gate behind `casting_candidates`/announcement): the top-of-library disjunct sat inside `obj.owner == player && (...)`; moved out as a sibling disjunct (its own `library_of(player).front()` test already carries the container).
2. `casting.rs::top_of_library_alt_ability_cost_for_object`.
3. `casting.rs::prepare_spell_cast..` `top_of_library_permission_src` guard.
4. `casting_costs.rs` alt-cost branch (calls 2).
Sites 2-4 use one new `pub(crate) fn object_in_players_library` (`zone == Library && zone_storage_seat(Library, owner) == zone_storage_seat(Library, player)`); no existing accessor expresses it (inline form in cast_from_zone.rs/engine_resolution_choices.rs is the same expression).
Swept and unchanged (no owner clause on the library path): `top_of_library_permission_source`, `top_of_library_selected_permission`, `top_of_library_plot_source`, `top_of_library_land_playable_by_permission`, `spell_objects_available_to_cast` top block, `runtime_granted_top_of_library_plot_abilities`. Other `obj.owner == player` hits (hand/command/graveyard/exile) are not library. `cast_free_origin_admits_object` has no Library origin. Single-owner-clause library reads elsewhere in engine/src: only `cast_from_zone.rs` (r1, fixed).

## Verification (PREPARATORY)
- `cargo fmt --all -- --check` rc 0; `node scripts/check-protocol-version.mjs` rc 0 (no types change).
- Focused nextest (`dandan_look_top_of_library`, `casting::`, `casting_costs::`, `top_of_library`, `census`, integration `bolas_citadel|mystic_forge|future_sight|top_of_library|realmwalker|dandan`): 1533 passed (e17r2-final.log). New rows alone: 7/7 (e17r2-green1.log).

## Discriminating-test gate
Rows (`dandan_look_top_of_library.rs::top_cast`, real cards Future Sight / Bolas's Citadel / Mental Note, driven through `spell_objects_available_to_cast`, `can_cast_object_now`, `legal_actions`, `runner.cast().resolve()`):
| Claim / seam | Test | Reach guard | Red when reverted |
|---|---|---|---|
| gate disjunct (castable_from_current_zone) | `dandan_future_sight_casts_the_pile_top_whoever_owns_it` (holder P0 and P1) | top is `library_of(holder).front()`, owner != holder | mutB (owner clause re-added at the disjunct): Future Sight + Citadel rows fail (e17r2-mutB.log) |
| permission_src guard | `dandan_citadel_pays_life_for_a_pile_top_the_opponent_owns` | no mana in pool; effective cost zero | mutD (that site only): Citadel fails (e17r2-mutD.log) |
| alt-cost-for-object | same | same | mutE (that site only): Citadel fails (e17r2-mutE.log) |
| casting_costs alt branch | same | same | mutC (that site only): Citadel fails (e17r2-mutC.log) |
| helper (sites 2-4 together) | same | same | mutA: Citadel fails (e17r2-mutA.log) |
Standard twin `standard_casts_only_the_holders_own_library_top`: own top castable (positive), P1's own-library top not castable by P0 (negative, reach: it is `library_of(P1).front()`); green at base and under every mutation, as intended (pins that Standard is unchanged). Future Sight row alone reaches only site 1 (no alt cost), hence the Citadel row. Realmwalker/Mystic Forge/Vizier share `top_of_library_permission_source` and these four gates; not given separate rows (same permission, filter-only difference).

## Maintainer-simulation matrix
| Seam | Entry / first branch | Authority | Bound, when | Mode | Storage | Consumers | Invalidation | Hostile rows |
|---|---|---|---|---|---|---|---|---|
| top-of-library cast admission | `castable_from_current_zone`, library disjunct | `TopOfLibraryCastPermission` source selected by `top_of_library_permission_source` | per call | live | none | announcement gate, candidates | top changes each call | Future Sight Dandan P0/P1, Standard negative |
| life rider | `top_of_library_alt_ability_cost_for_object` -> casting_costs branch | same selected source | announcement | live | pending cast cost | `check_additional_cost_or_pay` | n/a | Citadel Dandan P0/P1 |
Serde/protocol/fixture impact: none.

## CR gate
Added/kept numbers 400.1, 401.1, 401.5, 118.9, 601.2a: all present (`grep -anE "^(400\.1|401\.1|401\.5|118\.9|601\.2a)[ .]" docs/MagicCompRules.txt`); zero UNVERIFIED. 400.1 (zones, shared-zone formats) and 401.1 (each player's library) are cited for the library `player` reads.

## Judgement calls / risks
- New helper chosen over three inline copies of the container comparison (three casting-path sites).
- Per-site mutations (A-E) each flipped the Citadel row; mutB also flips Future Sight. No stop-and-return items.
