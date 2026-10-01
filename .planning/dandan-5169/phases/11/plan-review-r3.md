# Phase 11 plan review, round 3 (phase-plan mode)

Reviewer model: claude-sonnet-5-5. Plan reviewed: `phases/11/plan.md` (rev 3) against the charter Phase 11 entry and the code at HEAD `695933e5`. No cargo run (instructed); every claim rests on a read, grep, or a python walk over `client/public/card-data.json`. Sizing: present and consistent (3+3+1+2+1+3+7+2 = 22; 19 without the three test paths; 1 unit, T1 fails, no decomposition).

## Verdict: REVISE. 1 blocking finding (F1 behavior), 1 non-blocking text note. Design (carrier, seam, replayable install, protocol bump) stands.

## r2 closure check

- r2 F1 (reveal_until / scoped_library_search supply): closed. 3.4 table now lists only draw, `draw_n`, `dig.rs` put-all, the `DigChoice` kept map, `explore.rs`, `seek.rs`; both files are named in the dropped-class paragraph; section 7 shows 2 census siblings; V7 is Jadelight Ranger plus Adherent's Heirloom only; section 0 has no Abundant Harvest.
- r2 F2 (bounce.rs reason): closed; the row now says a Graveyard origin is reachable, owner's-hand wording, no recipient, stated limit.
- r2 F3 (V2 enters tapped): closed; V2 untaps before activating.

## Finding

### F1 (blocking, tag `behavior`): an unconditional `.performed_by(player)` on the Dig kept map (and `dig.rs` put-all) newly records an exiling player for every Dig-to-Exile delivery

3.4 says of the `DigChoice` kept map: "gains `.performed_by(player)` (unconditionally; the seam gates on the final `to`)". The seam gate (`hand_entry_receiver`, `to == Hand`) governs only the rebind. `EntryMods.performed_by` has a second, live consumer, which 3.1 itself names ("Its one consumer today is the `to == Exile` arm"): `deliver_replaced_zone_change` (`zone_pipeline.rs` ~3763) calls `record_exiling_player(state, object_id, player)` whenever `to == Exile` and `performed_by` is `Some`, which writes `obj.exiled_by` (`exile_links.rs:425-430`).

The kept map in `engine_resolution_choices.rs` (~4585-4600) is generic over `kept_zone = kept_destination`, so the new `.performed_by(player)` is also set on Exile deliveries. Measured: a walk over `card-data.json` finds 22 `Dig` effects with `destination: Exile` (e.g. Cemetery Tampering, Clive's Hideaway, Collector's Cage). Today those kept cards end with `exiled_by == None` (`zones.rs:393` clears it on the way out and nothing sets it); after the edit they end with `exiled_by == Some(player)`. `exiled_by` is a read field: `casting.rs:5001` (`own_exiles_of` admits only `obj.exiled_by == Some(player)`) and `game_state.rs:29437` (state equality). So the edit silently changes grant eligibility and state equality for 22 non-Dandan corpus cards, in a plan whose preservation claims (V1/V3/V5 "non-shared formats unchanged", the "no sibling-route result copied" note) are scoped to Hand and assert no row for it. The same hazard applies to `dig.rs::move_mass_put_all_selected` (also generic over destination) and to `seek.rs` (`destination` is a parameter; corpus Seek destinations are Hand only, so seek is measured-safe but is the same unconditional form).

Required revision (pick one, state it in 3.4, step 5 and the Verification Matrix): supply the taker only for a Hand destination at each generic producer, e.g. `(kept_zone == Zone::Hand).then_some(player)` applied through one small helper or a conditional builder step, so no Exile (or other non-Hand) delivery gains a performer; the unconditional form is acceptable only at sites whose destination is Hand by construction (draw, `draw_n`, explore's land-to-hand). Add a preservation row beside V4: a Dig-to-Exile delivery (any of the 22, or a unit row in `engine_resolution_choices.rs`) leaves `exiled_by` unchanged (`None`), paired with V4's Hand case that does rebind, revert-failing by deleting the Hand condition. Update the 3.1 sentence "one concept, not two" to say the producers set the carrier only for the destination whose consumer they intend.

## Non-blocking text note

V6 mana arithmetic: "P1 controls four Islands: it draws Dandân from the pile (V1 flow), casts it ({U}{U}...), and keeps two Islands untapped for Unsubstantiate's {1}{U}". If "V1 flow" is Brainstorm ({U}) on the same turn, the total is 1 + 2 + 2 = 5 mana from four Islands. Replacement text: "draws Dandân from the pile (the V1 Brainstorm flow on an earlier P1 turn, or the draw step), then on a P1 turn casts it ({U}{U} from two of four Islands) and keeps two untapped for Unsubstantiate's {1}{U}". The executor can resolve this itself as a fixture detail; not blocking.

## Verified (no finding)

Carrier plumbing (`performed_by` copied in draw, placement, exempt and batch arms; `into_pending`/`from_pending`); `ZoneMoveRequest::draw(` single caller; `move_to_zone_with_entry_flags` single production call in the plain `_` arm; `resolve_and_apply_zone_change` / `apply_resolved_zone_change` shape matches the plan's description (`OwnerMismatch` at both, container ops keyed by `command.owner`, `zone_change_command_is_invalid` has `command.owner != record.owner` which the installed-owner choice keeps true); the Attraction redirect precedes the resolved-zone-change branch; `record_zone_change_library_knowledge_stamp` reads `record.owner`, which Phase 6's plan resolves to the pile seat (Phase 6 plan line 66); DigChoice arm's `player` is the choosing player, distinct from `library_owner`.
