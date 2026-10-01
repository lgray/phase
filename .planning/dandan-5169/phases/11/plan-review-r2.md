# Phase 11 plan review, round 2 (phase-plan mode)

Reviewer model: claude-sonnet-5-5. Plan reviewed: `phases/11/plan.md` (rev 2) against the revised charter Phase 11 entry (r5 `engine_resolution_choices.rs` exception) and the code at HEAD `695933e5`. No cargo run (instructed); every claim rests on a read, grep, or python walk over `client/public/card-data.json`. Sizing context: phase-fit (Sizing consistency blocking); Sizing is present and arithmetically consistent (3+3+1+4+1+3+7+2 = 24; 21 without the three test paths; 1 unit, T1 fails), subject to F1's recount.

## Verdict: REVISE. 1 blocking finding (F1 behavior), 2 non-blocking text findings. Design (carrier, seam, replayable install, protocol bump) stands.

## r1 closure check

- F1 (engine_resolution_choices.rs over-extension): closed for the file. 3.4 now edits only the `DigChoice` kept map; the search-partition, Cultivate, Discover, cast-rejected and `route_kept_card_or_defer` rows are gone; `route_rest_partition_then` keeps its signature; the stated-limit paragraph is present. The `SelectCards` arm binds `player` from `WaitingFor::DigChoice` (read, engine_resolution_choices.rs ~4390), the choosing player, distinct from `library_owner`, so `.performed_by(player)` is the right taker. See new F1 below: the same limit is still violated in two `effects/` files.
- F2 (cr733_resolved_zone_change.rs): closed. `git grep -n 'ResolvedZoneChangeCommand {' -- crates` = `zones.rs:963`, `log.rs:3321`, `unmaterialized_lki_serialization.rs:188`; `cr733_resolved_zone_change.rs:17` is a return type. `resolve_and_apply_zone_change(` callers: `zones.rs` (1532, tests 4921, 4970) and `cast_from_zone.rs` 4796, 4860. Matches the plan.
- F3 (debug_assert reachable): closed. `redirect_attraction_to_command` rewrites `to` at zones.rs ~1377 before the raw branch; the plan's `receiver.filter(|_| to == Zone::Hand)` after the redirect plus V9(e) is correct.
- F4 (V6 Island/mana): closed. Timing is also feasible: P1 taps two Islands on its own turn for Dandan, keeps two untapped through P0's turn (P1 untaps only in its own step), P0 casts the sorcery-speed Aura, P1 answers with Unsubstantiate.
- F5 (corrections): closed. V4/dig.rs framing, CR 400.1 wording, and the 3.5 `phases/summaries.md` sentence are present.

Other r2 facts verified: `move_to_zone_with_entry_flags` callers are exactly the wrapper (zones.rs:1241), tests (4342, 4452) and the one production call (zone_pipeline.rs:3737); post-move `owner` use inside it is only `record_descend_on_graveyard_arrival` (Graveyard) so no stale owner survives; `ZoneChangeRecord` has `owner` and `controller`; `reset_for_battlefield_exit` sets `base_controller = Some(owner)` so the applier's `base_controller = Some(receiver)` is consistent; `apply_zone_exit_cleanup` writes `controller = base_controller.unwrap_or(owner)` (zones.rs:524); `performed_by` is copied in the draw, placement, exempt and batch arms; `ZoneMoveRequest::draw(` has one caller (draw.rs:924); CR 108.3, 108.4a, 109.4, 110.2, 121.1, 400.3, 616.1, 717.6 present with cited meaning.

## Findings

### F1 (blocking, tag `behavior`): the plan supplies reveal-until and library-search producers that the charter's stated limit forbids, and one of them forces edits outside scope

Charter Phase 11 Deferral list: "no Dandan card reaches a library-to-hand search partition, discover or reveal-until-to-hand, so those producers are not supplied ... this run builds no supply for them." The plan's own 3.4 stated-limit paragraph repeats this, yet the 3.4 table still supplies:
- `effects/reveal_until.rs` "kept `other` arm" (a reveal-until kept card to Hand, `ZoneMoveRequest::effect(*hit, other, ..)` at line ~478) and "the `dest`-parameterised rest delivery". The latter is `move_rest_then` (reveal_until.rs:778), whose signature has no player (`state, cards, rest_destination, rest_order, completion, events`), so "revealing_player / controller already in scope" is false there. Supplying a taker means a new parameter, which forces its three callers in `engine_resolution_choices.rs` (2819, 3139, 9885) and two in `reveal_until.rs` (283, 547): edits in the charter-limited file outside the one kept-map line.
- `effects/scoped_library_search.rs` `deliver_collected_cards` (`ZoneMoveRequest::effect(.., move_destination, ..)`, line ~805): a library-search delivery, the same class the charter and the plan's own stated limit drop.
Also V7's "Abundant Harvest (kept card to hand)" and "reveal-until kept" exercise a producer the limit says is not supplied, and V8/the mutation list inherit it.

Required revision: delete the two `reveal_until.rs` rows and the `scoped_library_search.rs` row from 3.4 and 3.2's producer list, from the section 7 table ("Producers, census siblings" 4 becomes 2: `explore.rs`, `seek.rs`), section 8 Edited list, section 2 sentence "the `engine_resolution_choices.rs` search, discover and reveal-until deliveries are the dropped class" (keep, and extend to name `effects/reveal_until.rs` and `effects/scoped_library_search.rs` as members of the dropped class), step 5, and V7 (keep Jadelight Ranger and Adherent's Heirloom; remove Abundant Harvest, its Oracle text in section 0, and its mutation). Recount Sizing with `git grep` at scope-freeze: 24 becomes 22 total, 21 becomes 19; T2 still fires, T1 still fails. `explore.rs` and `seek.rs` stay: they are Hand-capable library producers not named in the stated limit, admitted by the scope rule's enumeration clause, and `explore.rs` has the controller in scope (`controller` at the CardsRevealed event, line ~308).

### F2 (non-blocking, tag `text`): bounce.rs classification reason is false for its Graveyard arm

Old text (3.4 last table row): "`effects/bounce.rs`, ... | not a shared-zone origin (battlefield, stack, or hand-cost) or not Hand | none; keep the owner's hand (CR 400.3)".
`bounce.rs` ~412 delivers `destination` for `current_zone` in `Battlefield | Graveyard`, so a Graveyard origin is reachable (5 corpus `Bounce` effects carry `InZone Graveyard`: Forcemage, Nullmage, Pulsemage, Shieldmage, Spurnmage Advocate; each reads "return target card from an opponent's graveyard to their hand", an owner's-hand wording). Replacement text: "`effects/bounce.rs` | Graveyard origin is reachable (5 corpus cards, none in the Dandan list), but every such card says \"their/its owner's hand\" and `Effect::Bounce` carries no recipient | none; keep the owner's hand (CR 400.3); same stated limit as the `ChangeZone` owner's-hand boundary". Drop `bounce.rs` from the "not a shared-zone origin" clause.

### F3 (non-blocking, tag `text`): V2 cannot activate The Surgical Bay on the turn it is played

Old text (V2): "plays it with `GameAction::PlayLand` ... pays {1}{U}, taps, sacrifices". Oracle (section 0): "This land enters tapped." so `{T}` is unpayable that turn. Replacement: "plays it with `GameAction::PlayLand` (it enters tapped), then untaps it (fixture untap, or the next P1 untap step) before paying {1}{U}, tapping and sacrificing".

## Residual assumptions (named executor probes already in the plan)
Dandan `GameScenario` driver and pile helpers; `GameRunner` driving the `DigChoice` prompt; Phase 6 `zone_storage_seat(&self, zone, seat) -> PlayerId` signature (Phase 6 plan line 34 states it); Phase 5 pin names.
