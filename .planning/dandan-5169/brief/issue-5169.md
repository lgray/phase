Anyone who want to works on this please feel free to take a stab! If you are working on it, make a comment in the PR that you have the token so no one else works in parallel. If you make a PR, reference this issue and denote what portions of the plan, if not all, that you have completed. Note when you have finished contributing work on this issue to release the token so that others may review and determine how to contribute best.

:robot: _AI text below_ :robot:

> **Refreshed 2026-09-30** against upstream `d95cc66019`, with the issue author's rulings in "Settled decisions". The July 2026 plan is in this issue's edit history. The July "Part 2" comment is superseded.

# Dandan format — refreshed design brief (issue phase-rs/phase#5169)

Pinned code: upstream `d95cc66019`. The July plan was written against `17052fc878`. Every anchor below is `path` + symbol. Line numbers have moved everywhere, so none are given; find each anchor with `git grep -n 'fn <symbol>'`.

## Code shape relevant to Dandan

The repo describes its own shape in `.claude/skills/project-reference/SKILL.md` §Architecture and in the CLAUDE.md "engine owns all logic / frontend is display" rules. The parts are:

- **Format registry.** `crates/engine/src/types/format.rs` owns every per-format decision. That includes `GameFormat`, the exhaustive axis methods, `FormatConfig` with its validated deserialization, and `registry()`. `types/custom_format.rs` owns saving a lobby as a Custom format. `game/deck_validation.rs` owns deck legality.
- **Zones.** `game/zones.rs` and `game/zone_pipeline.rs` are the zone-mutation authority (`scripts/zone_authority_census.py` `AUTHORITY_FILES`); the census gates raw writes elsewhere and lists the annotated `allow-raw-zone` exemptions. `game/zone_pipeline.rs` owns replacement semantics for zone moves (the "would" layer). Its module doc states the split: zones.rs keeps the unconditional guards, zone_pipeline owns the "would" layer. Zone moves and library shuffles are journaled as `types/resolved_commands.rs::ResolvedRulesCommand`, and `game/library.rs` owns library shuffles.
- **Casting and ownership.** `GameObject.owner` is stamped at deck load. Hand, library and graveyard containers are chosen by `owner`.
- **Mulligan and draw.** `game/mulligan.rs` runs the pregame. Every in-game draw is a `DrawSequenceFrame` in a `DrawSequenceStack` (both `types/game_state.rs`), held as `draw_sequences` on a resolution frame (e.g. `types/resolution.rs::MultiDrawFrame`) and driven by `game/effects/draw.rs`; the wire version is `types/resolution.rs::RESOLUTION_STATE_WIRE_VERSION`. The multi-player fan-out happens in `effects/mod.rs::resolve_chain_body`.
- **Loss.** `game/sba.rs` handles loss, and the draw-from-empty flag is written only in `game/ledger.rs::apply_resolved_ledger_edit`, which `resolve_and_apply_cards_drawn` reaches through `resolve_and_apply_ledger_edit`.
- **Frontend.** `client/` renders engine state. `client/src/data/formatRegistry.ts` is a hand mirror of the registry, checked only by the Rust `include_str!` tests in format.rs and `tests/integration/format_axis_census.rs`.
- **AI.** `crates/phase-ai` is format-agnostic, and its search runs with perfect information (`determinization_samples = 0` on every preset). The engine's `ai_support/` generates legal candidates.

I confirmed the zone_pipeline and zones.rs split in the zone_pipeline.rs header at the pin.

## Authoritative rules

Source: https://magic.wizards.com/en/news/announcements/from-the-chaos-vault-secret-lair-dandan-decklist. The quotes below are verbatim.

> players still start with 20 life and seven cards in their opening hands. There's still a maximum hand size of seven and attempting to draw from an empty library is a way to lose the game. There are a few differences, however:
> Players have a shared library and graveyard. Anything referring to "your" graveyard or library is referencing the shared zone.
> The "owner" of a card on the stack or permanent on the battlefield is considered to be the person who cast it.
> Whenever multiple players are drawing cards simultaneously, they are dealt one at a time, starting with the active player (i.e., whoever is going first or whoever's turn it is).
> When taking mulligans, if a player draws a hand with less than two lands or spells, that player may reveal the hand and take a free mulligan. This no longer applies once that player has taken a regular mulligan.

These rules modify the following CR rules. Cite them by number only, since line numbers differ between rules-file checkouts:

- CR 400.1: each player has their own library and graveyard.
- CR 108.3 + CR 110.2: owner of a card, and of a permanent.
- CR 121.2c: the active player performs all of their draws, then the next player.
- CR 103.5: opening hand and mulligans.

These still apply unchanged:

- CR 121.1: a drawn card goes into the drawer's hand.
- CR 400.3: a card goes to its owner's hand, graveyard or library.
- CR 704.5b and CR 104.4a: deck-out loss, and the draw when everyone left loses.

"No sideboard" is a design decision, not a rule: the article never mentions sideboards.

**Decklist (80 cards; from the article's `<deck-list set="DAN">` element).**

| Count | Card |
|---|---|
| 10 | Dandân |
| 20 | Island |
| 8 | Memory Lapse |
| 4 | Accumulated Knowledge |
| 2 each | Magical Hack, Mystic Sanctuary, Brainstorm, Capture of Jingzhou, Chart a Course, Control Magic, Crystal Spray, Day's Undoing, Mental Note, Metamorphose, Predict, Telling Time, Unsubstantiate, Halimar Depths, Haunted Fengraf, Lonely Sandbar, Remote Isle, The Surgical Bay, Svyelunite Temple |

That is 23 distinct cards. Dandân is keyed `dandân` in the card data, and `card_db.rs::fold_card_name_key` resolves the name "Dandan" to it.

## Per-subsystem design

### S1: Format registration (`types/format.rs` and neighbours)

**Variant and constructor**

- Add `GameFormat::Dandan` after `FreeformCommander` and before `Custom`. Tests compare tables in declaration order via `GameFormat::iter()`.
- Add `FromStr` and `Display` arms. Serde goes through these (`Deserialize` calls `parse::<GameFormat>()`), so `custom_format_schema::game_format_from_str_display_roundtrip_builtins` and that file's per-registry `default_config` deserialization tests catch a missing `FromStr` arm.
- Add `FormatConfig::dandan()`, copied from `FormatConfig::momir()` with these values: 20 life, 2 players, `DeckSizeRule::Exactly(80)`, `supplies_fixed_deck: true`, `command_zone: false`.

**Compiler-forced exhaustive arms (18 sites)**

- `types/format.rs` (14). Use Momir's answers unless a different value is noted:
  - `Display::fmt`
  - `card_pool` → `NoEngineAuthority`
  - `commander_pairing` → `NoCommander`
  - `sideboard_policy`
  - `default_deck_copy_limit` → `Unlimited`
  - `deck_size_authority` → `RulesFixed`
  - `deck_size_subject` → `MainDeck`
  - `grants_free_first_mulligan` → `false`
  - `uses_commander`
  - `GameFormat::command_zone_holds_decklist_commander` → `Ok(false)`
  - `supplies_fixed_deck`
  - `has_unrepresentable_auxiliary_deck_component` → **`true`**, and extend its doc and error text. This is the only guard that stops a Dandan lobby being saved as a Custom format that would silently play a different game (`CustomFormatDef::from_lobby_config`).
  - `label`
  - `for_format`
- `game/deck_validation.rs` (2): `evaluate_selected_format_summary` and `evaluate_selected_format`. Add `quick_dandan_check` and `evaluate_dandan`, mirroring the Momir pair. They must read axis methods, not literals.
- `types/custom_format.rs` (1): `CommanderEligibilityRule::from_source_format` → `Ok(None)`.
- `engine-wasm/src/lib.rs` (1): `is_card_commander_eligible_for_format` → `false`.

The other six `match`es fall through and need no change: `card_subset`, `topology`, the binding arm of `FormatConfig::command_zone_holds_decklist_commander`, `max_deck_copies`, and two `unreachable!`s.

**Sites forced by tests or tsc**

- `registry()` entry with `group: Multiplayer`, placed after Momir. `Constructed` would fail `registry_constructed_formats_declare_a_deck_construction_pool`.
- The two ordered tables in `tests/integration/custom_format_schema.rs`.
- Client: the `types.ts` `BuiltInGameFormat` union, the `formatRegistry.ts` entry in the same position, the `ws-adapter.ts` `lobbyProtocolRequiredForFormat` switch, and `deckUrlImport.test.ts` `FORMAT_SHAPES`.
- No test compares the client `default_config` fields with `FormatConfig::dandan()`, so check them by hand.

**Format predicates are axis methods on `GameFormat`, not stored `FormatConfig` fields.**

- The new predicates are: shared zones, deal order, free-reveal mulligan, hand-entry ownership rebind, and whether opening hands are equivalent.
- Each is an exhaustive match with `Custom(_)` returning the stock answer. This follows the `card_pool` / `commander_pairing` precedent (`format_axis_census`).
- Use typed return values, never `bool` pairs. For example, a shared-zone descriptor enum per zone.
- Reason: a stored field would need a verdict row in `built_in_axes_no_looser_than_rules`, a `for_custom_rules` value, the client `format-config-shape.ts` guard plus rehydrate, 25 `formatRegistry.ts` edits, and an extra protocol bump. `allow_experimental_dungeons` (#9352) shows that cost. Dandan cannot be a Custom source anyway.
- The frontend never needs these predicates before a game exists. It reads pile identity from engine state (see S7).

**Protocol**

- Bump the lobby-broker `PROTOCOL_VERSION` and its ws-adapter mirror, `LOBBY_PROTOCOL_VERSION` (both sides), and client `network/protocol.ts` `WIRE_PROTOCOL_VERSION`.
- Add a frozen `MIN_LOBBY_PROTOCOL_FOR_DANDAN` floor returned by `lobbyProtocolRequiredForFormat`.
- Update the `scripts/check-protocol-version.mjs` `EXPECTED_*` values.
- Precedent: #9226 (Freeform). Re-derive the numbers at rebase: `git grep -n PROTOCOL_VERSION`.

**AI force-keep must be re-gated in S1.**

- `phase-ai/src/policies/mulligan/fixed_deck_keepables.rs::FixedDeckKeepMulligan` returns `ForceKeep` whenever `supplies_fixed_deck` is true, so the Dandan AI would never mulligan.
- Re-gate it on the new "opening hands are equivalent" axis, true only for Momir.
- Keep `supplies_fixed_deck: true` for Dandan. Its other readers, the engine-wasm `validate_deck_list_seats` skip and client `formatSuppliesDeck`, are correct for Dandan.

### S2: Shared library and graveyard

This keeps the July design. The shared pile is stored on the existing `Player.library` / `Player.graveyard` of one **canonical seat**, fixed at setup (lowest `PlayerId`, never recomputed). No new `GameState` field is added. `GameState::library_of` / `graveyard_of` (and `_mut`) resolve any seat to that storage when the format declares the zone shared, and act as identity otherwise.

The additions below replace July assumptions that no longer hold.

**Write-side canonicalization**

It goes inside the appliers so that journal replay is deterministic:

- zones.rs: `remove_from_zone`, `add_to_zone`, `move_to_library_at_index`, `reorder_within_library`, and `zone_container_len`. The last also validates the replayed `destination_position`.
- `game/library.rs`: `resolve_and_apply_library_shuffle` / `apply_resolved_library_shuffle`. Storage resolves to the canonical seat, while `ResolvedLibraryShuffleCommand.player` stays the acting player, because it drives `ShuffledLibrary` and "whenever you shuffle".
- The raw writers in `engine_resolution_choices.rs` and `conjure.rs`.
- The write population is `python3 scripts/zone_authority_census.py --list` plus the whole-container assignments the census has no family for (`\.(library|graveyard)\s*=(?!=)` in production code; see Regenerating commands).

**Read sweep**

- About 150 direct `.library`/`.graveyard` occurrences in 58 production engine files, plus 12 in phase-ai, 1 in engine-wasm and 3 in phase-llm (`render/game.rs`, which would report an empty library and graveyard for the non-canonical seat). manabrew-compat is out of scope: Dandan is declared unsupported there (S4).
- Classify each site:
  - Single-player reads switch to the accessor.
  - **All-players flat_maps stay raw.** Examples: `targeting.rs::zone_object_ids`, costs.rs, triggers.rs, visibility.rs, and `quantity.rs::object_count_matching_candidate_ids`. Under canonical storage these already return the pile once. Routing them per seat would duplicate it N times.
- Fold these private (player, zone) resolvers into the accessor: `choose_from_zone.rs::object_ids_in_player_zone`, `restrictions.rs::player_zone_ids`, and the inline `match zone` in quantity.rs.

**Count dedup**

- Aggregate folds over `scoped_players` must count the shared pile once. The affected paths are the quantity.rs `CardTypeSetSource` arm, `QuantityRef::ZoneCardCount`, and `QuantityRef::GraveyardSize` via `resolve_per_player_scalar` / `aggregate_over_players`.
- Model this on the 2HG precedent: `topology.rs::shared_resource_dedup_key` + `players.rs::aggregate_over_teams`, i.e. a shared-zone dedup key and a keyed fold. Do not hand-write collapse logic at each site.
- The regression test must drive `ZoneCardCount` or `GraveyardSize` under an all-players scope. Accumulated Knowledge goes through the id-deduped `ObjectCount` path and cannot fail.

**Filter owner-axis collapse** (a second building block, in `game/filter.rs`)

- "Your library/graveyard" is the shared zone (announcement text). For an object in a shared zone, `FilterProp::Owned` (both the live-object and record/LKI arms), `matches_target_filter_in_owner_zone`, and the typed-filter `controller` axis (`ControllerRef` resolved through `filter.rs::effective_controller`) treat the axis as satisfied for every seat.
- Haunted Fengraf (`Owned{You}` + InZone Graveyard) depends on the owner collapse. Mystic Sanctuary (`controller: You` + InZone Graveyard) depends on the controller collapse; `targeting.rs` never calls `matches_target_filter_in_owner_zone`, so that collapse alone does not reach it. Fixture: the non-canonical seat's Mystic Sanctuary targets an instant in the shared graveyard owned by the canonical seat.
- Opponent-scoped text collapses the same way (the July convention). No card in the fixed list exercises that.

**Library-knowledge identity**

- `advance_library_knowledge_epoch`, `library_knowledge_epoch`, `library_knowledge_boundary_generation`, `LibraryKnowledgeStamp.library_owner` and `viewer_knows_card_identity` key on owner. The `fact.owner == owner` comparisons in `advance_library_knowledge_epoch` (the `facts.retain`) and `canonicalize_library_knowledge_epoch` key on the card's owner, not its storage seat.
- Resolve every one of these keys to the canonical seat. Otherwise one shared library holding cards with mixed owners keeps stale knowledge.

**Other S2 items**

- Deck provisioning: add `dandan_fixed_deck_names` beside `game/deck_loading.rs::momir_fixed_deck_names`. Add a Dandan branch in `load_and_hydrate_decks` that loads **one** 80-card pile into the canonical seat and skips the other seat's deck load. `momir_fixed_deck_payload` builds one deck per seat, so it cannot be reused unchanged. `state.deck_pools` gets one pool, for the canonical seat; the other seat has none, and seat-keyed `deck_pools` readers resolve through the pile holder (S8).
- Boot guard: `engine-wasm/src/lib.rs` refuses to start when any seat has `library.is_empty()` ("Empty library after deck load"). Route this through the accessor, or Dandan never boots.
- Draw selection: `effects/draw.rs::select_cards_to_draw` is the single in-game draw-selection authority, so one edit covers every draw and the empty-library flag.
- Mulligan: `mulligan.rs::draw_n` reads `player.library[0]` directly and is routed separately.
- Raw shuffles: `mulligan.rs::shuffle_hand_into_library` moves the hand through the pipeline, then raw-shuffles the mulliganing seat's own `library`. For the non-canonical seat (mulligan or FreeReveal) that shuffles the empty seat and leaves the returned hand unshuffled in the shared pile (CR 103.5). Route it through the library accessor or `library.rs::resolve_and_apply_library_shuffle`; test that the shared pile's order changes when the non-canonical seat mulligans, and separately FreeReveals. The all-seat loops in `start_mulligan` and `deck_loading.rs` are harmless no-ops on the empty seat.
- Hidden information: open issue #9377 (object ids track cards into hand and library) is amplified by a library both players draw from. Note it in the S7/S2 review scope.

### S3: Ownership binds at hand entry (replaces July's cast/land-play stamp)

- Hands are stored per owner (`add_to_zone(Zone::Hand, owner)`), and `ZoneMoveRequest` has no recipient field. So a card drawn by the non-canonical seat from a pile owned by the canonical seat would land in the canonical seat's hand, and `draw.rs` drops the miracle offer whenever `obj.owner != player`.
- Design: under the format's rebind axis, a move from a shared zone into a hand carries the receiving player, and `obj.owner := receiver` before containers are chosen.
  - The receiver is the drawer for draws and the opening deal.
  - It is the effect's "your hand" player for Dig→Hand (Telling Time) and ChangeZone Graveyard→Hand (Haunted Fengraf).
- Put this at the Hand-destination seam in `zone_pipeline.rs`, not in each resolver.
- With the rebind in place, every card in hand is owned by its holder. Every cast (`casting_costs::finalize_cast_with_phyrexian_choices_inner` → `ZoneMoveRequest::casting_to_stack`) and every land play (`engine.rs::handle_play_land`) is therefore by the owner, and the announcement's owner=caster rule holds by construction.
- No cast- or land-play-time stamp is added: no card in the fixed list casts or plays from the shared library or graveyard.
- Control Magic changes only `controller` (layers.rs ChangeController). Unsubstantiate goes through `effects/bounce.rs` to the owner's hand, which is the caster's hand.
- The rebind must replay. `apply_resolved_zone_change` refuses with `OwnerMismatch` when `object.owner != command.owner`. So the rebind is either carried by the zone-change command or journaled as its own typed command, like `ResolvedControllerOverrideCommand`. The planner probes whether journal replay is live and picks one; a raw field write is not enough.
- Fixture:
  - The **non-canonical** seat draws (and separately Telling-Times) a creature from the shared library and gets it with owner = itself.
  - It casts the creature, the other seat Control-Magics it, and Unsubstantiate returns it to the caster's hand.
  - Revert leg: remove the rebind, and the card lands in the canonical seat's hand at draw.
- Drop `PutOnTopOrBottom` and the CR 401.4 citation from the consumer list (no Dandan card uses it).

### S4: Mulligan, free reveal and pregame dealing (`game/mulligan.rs`)

- **The engine has no CR 103.5 round.**
  - Today the `Mulligan` arm of `handle_mulligan_decision` redraws immediately on submission, and `pending` persists across decisions.
  - For formats with a shared library, add a declare round: Mulligan and FreeReveal declarations are recorded without redrawing. Once every `Declare` entry has declared, all redrawers are dealt interleaved, active player first.
  - The round is observable only on a shared pile, so gate it on the shared-zone axis.
  - This touches `MulliganDecisionEntry` / `MulliganDecisionPhase`, `advance_after_decision`, `ai_support/candidates.rs` and `game/interaction.rs`.
- **FreeReveal** (`types/actions.rs` `MulliganChoice::FreeReveal`):
  - Model it on the current `UseSerumPowder` shape: validated at declare time and gated on the `Declare` phase.
  - Legal only when `mulligan_count == 0` and the hand predicate holds (settled decision 1).
  - It reveals the hand, returns it through `shuffle_hand_into_library`, and redraws through the dealer. It does not increment the count and bottoms nothing.
  - Keep `grants_free_first_mulligan()` false. With 2 seats, `free_first_mulligan()` is then false, so `mulligan_count == 0` means "no regular mulligan yet". This relies on Dandan being 2-player only.
  - Carry availability as a typed value on the entry or folded into `MulliganDecisionPhase`, never a `bool`.
  - It is a round participant, because the text calls it "a free mulligan", so CR 103.5 simultaneity applies.
- **Dealer.** One pregame interleave primitive, used by the opening deal (`start_mulligan`) and by the round's redraws. Use `players.rs::apnap_order_from` for order.
- **Consumers to extend:**
  - `game/interaction.rs`: the `GameAction::MulliganDecision` surface match.
  - `ai_support/candidates.rs`: emit FreeReveal.
  - phase-ai `search.rs`: the two `MulliganChoice` emission points, `deterministic_choice` (Declare: Keep / UseSerumPowder / Mulligan) and `fallback_action` (Declare: UseSerumPowder / Keep), must emit FreeReveal. `policies/mulligan/card_floor.rs` needs no change: its ForceKeep floor fires only after several mulligans, never at `mulligan_count == 0`.
  - client `types.ts` `MulliganChoice` and the GamePage mulligan buttons, following the Powder candidate-button pattern. Add the FreeReveal label and description keys beside `gamePage.mulligan.usePowder` / `powderDescription` in all 8 `client/src/i18n/locales/*/game.json` files (`localeParity.test.ts`).
  - `manabrew-compat` maps `MulliganUseSerumPowder`. Do not add a protocol output there. Declare Dandan unsupported on that adapter.

### S4b: In-game simultaneous-draw dealer (`effects/draw.rs`, `types/game_state.rs::DrawSequenceStack`)

- Every draw is a CR 121.2 instruction frame with a CR 121.2a instruction-level replacement consult and CR 121.6b per-unit consults. A pause can happen at any unit.
- Design:
  1. A multi-player dealer frame settles each player's instruction count first.
  2. It then deals single draws round-robin, active player first. Use `apnap_order_from` with the ability's `starting_with`.
  3. It must survive unit pauses and keep each player's delivered count intact (`install_previous_effect_counts_by_player`).
- Intercept at the two fan-out layers in `effects/mod.rs::resolve_chain_body`: the `player_scope` driver and `resolves_for_each_target_player`.
- The precedent for a shape-gated simultaneous protocol inside that driver is `scoped_library_search::supports_simultaneous_delivery`.
- Bump `types/resolution.rs::RESOLUTION_STATE_WIRE_VERSION`.
- Day's Undoing:
  - Since #8757 the all-player wheel resolves one complete per-player wheel in sequence: move, shuffle and draw 7, then the next player. This is pinned by `tests/integration/all_player_library_wheel.rs` and, inferred from the source, kept together by `is_player_scope_local_continuation`.
  - On a shared pile, P1's hand is shuffled in after P0 has drawn. PREREQ-3 therefore also needs the scoped wheel split into an all-players move/shuffle phase followed by a dealt draw phase.
  - Semantics per settled decision 2.

### S6: Deck-out (verification only)

- The loss logic is unchanged: `sba.rs::collect_draw_from_empty_losers` → `elimination::eliminate_players_simultaneously`.
- The flag is set from `select_cards_to_draw` via `draw.rs::apply_draw_after_replacement` → `ledger.rs::resolve_and_apply_cards_drawn` → `apply_resolved_ledger_edit`. Routing `select_cards_to_draw` in S2 is sufficient.
- The test covers both branches:
  - Pile empty: CR 104.4a draw.
  - Pile short by one under the dealer: the non-active player alone loses.
- Pause hazard: `engine.rs::reconcile_terminal_result` runs SBAs when a player-loss SBA is pending during a resolution prompt. A pause between two empty draws would turn a draw into a single loser. The Dandan deck has no draw replacements, so the test pins the pause-free path. The dealer must not park once any player has flagged empty.

### S7: Frontend (display only)

- Engine: expose which seat holds the shared pile. Put it in a `derived` view (or a shared-pile descriptor) so the client never infers it.
- Anchors:
  - `pages/GamePage.tsx`: the `opponentPiles` / `playerPiles` DraggableWidgets. Render the shared library and graveyard once, in `playerPiles`, and keep ExilePile per seat. `OpponentSeatPane` only renders when `isSplitBoardActive` (seatCount > 2), so it is unreachable here.
  - `AnimationOverlay.tsx`: the `data-library-pile` / `data-graveyard-pile` fly-to anchors, which are keyed by owner id.
  - `hooks/useCastableZoneObjects.ts`
  - `viewmodel/gameStateView.ts::getPlayerZoneIds`
  - `LibraryPile.tsx`: `isMyLibrary`
- `DebugLibraryViewer` reads engine `derived.debug_library_cards`, so the engine-side derivation must use `library_of`.
- The `formatRegistry.ts` entry lands in S1.

### S8: AI

- Drop the July determinize-correctness row. Shipping presets never determinize: `determinization_samples = 0`, and `score_candidates_with_session` short-circuits at `k == 0`.
- Route these raw readers through the engine accessor:
  - `policies/payoff.rs::mill_scale`: with the opponent reading 0, urgency is pinned HIGH.
  - `policies/mill_targeting.rs::verdict`: the empty-library penalty is always on.
  - `threat_profile.rs::build_threat_profile_multiplayer`: returns None for the seat that holds no pile.
  - `zone_eval.rs::graveyard_value`: read through `graveyard_of`, so both seats score the shared graveyard identically and it cancels in `player_zone_score`'s me-minus-opponent difference (today it is credited wholly to the canonical seat, biasing `zone_bonus` on every graveyard change).
  - `search.rs::large_board_main_phase_has_no_development_sources`: its `player.graveyard.is_empty()` check reads through `graveyard_of` ("your graveyard" is the shared one).
- `deck_knowledge.rs::known_remaining_deck_counts` / `remaining_deck_view` read `deck_pools`, not the library: look the pool up for the pile holder, and for shared-zone cards drop the `object.owner != player` filter so every non-token card outside the shared library is subtracted, whoever owns it.
- `policies/draw_payoff.rs` is already correct through `can_draw_at_least_one`.
- Liveness: an in-crate test drives `run_ai_actions_bounded` on a Dandan state. `ai_duel` has no general format flag (only the Commander modes), so this is cheaper than a new mode.

## Card-coverage gate (coverage-data built at ≈ `5a522e6db6`)

| Prereq | Card(s) | Status at pin | Work |
|---|---|---|---|
| PREREQ-0 (new) | Memory Lapse ×8 | `supported:false`, gap `Swallow:Replacement_Instead`. This is a false positive: the AST `Counter{countered_spell_zone: Library Top}` is correct and the runtime honours it. Caused by #5684 removing the "instead" exemption. The same false positive hits Remand, Hinder, Spell Crumple and Lapse of Certainty. | In `parser/swallow_check.rs::effect_is_replacement_carrier`, accept `Effect::Counter` with `countered_spell_zone: Some(_)`, or reuse the slot-shaped precedent in the same file (`detect_condition_if`'s `evidence.has_slot("countered_spell_zone")` gate); the planner picks one. Add one test. About 5 lines. |
| PREREQ-1 | Magical Hack | unsupported, `unrecognized_clause_head` "change" | A CR 612 word-substitution `ContinuousModification` in `Layer::Text`, beside `SetTextName` / `SetChosenName`, over a typed word domain (CR 612.2: color word, basic land type; creature type is the class's next member, Artificial Evolution). Hack uses domain {BasicLandType}. Class: Sleight of Mind, Mind Bend, Alter Reality, Glamerdye (all unsupported). Trace Cleave (`database/synthesis.rs`) and Overload (`effects/overload.rs`) first. No substitution primitive exists. Estimate: 600–1200 LOC (not measured). |
| PREREQ-2 | Crystal Spray | same | Same primitive with domain {ColorWord, BasicLandType}, the player choosing which, and an until-end-of-turn duration. |
| PREREQ-3 | Day's Undoing | parses as supported | Needs S4b and the wheel reorder, with the semantics of settled decision 2. |

The other 19 cards parse as supported.

- Dandân's CantAttack gate now has a runtime test (`tests/integration/issue_8183_static_gate_fail_open.rs`).
- These still need runtime verification on shared zones: Mystic Sanctuary, Halimar Depths, Haunted Fengraf, Accumulated Knowledge, Brainstorm, Mental Note, Metamorphose, Predict, Telling Time, and The Surgical Bay (its sacrifice ability draws from the shared library).
- Control Magic and Unsubstantiate are covered by the S3 fixture.

Regenerate the coverage status from the repo root, after `./scripts/gen-card-data.sh` has written `client/public/coverage-data.json` (`scripts/names.txt`, beside this brief, holds the 23 names):

```
jq -r --rawfile n <brief-dir>/scripts/names.txt '($n|split("\n")|map(select(length>0))) as $ns | .cards[]|select(.card_name as $c|$ns|index($c))|[.card_name,.supported,.gap_count]|@tsv' client/public/coverage-data.json | sort -u
```

## Slice DAG

```
S0 gate (add-engine-variant, cargo engine-inventory) ─┐
PREREQ-0 (independent, tiny) ─────────────────────────┤
S1 format + axes + protocol + AI force-keep regate ───┴→ S2a storage/accessor/writes/deck-load/boot-guard/epochs
S2a → S2b read sweep (~2 commits) → S2c filter owner-collapse + count dedup
S2a → S3 hand-entry rebind → S4 declare round + FreeReveal + pregame dealer → S6 win tests
S4 → S4b in-game dealer frame (+ Day's Undoing wheel split per settled decision 2) → PREREQ-3
PREREQ-1/2 (parallel with S2–S4)
S2c + S4 → S7 frontend → S8 AI
```

**Sizing, measured at the pin**

- S1 is under one commit: 18 exhaustive arms, about 6 test or table rows, the constructor, the registry and client entries, the quick and full checks, and the protocol bump.
- S2 is about 3 commits in the 1–2k LOC band: about 150 engine read occurrences across 58 files, 12 phase-ai and 3 phase-llm reads, and 97 owner-keyed library-knowledge occurrences.
- S3 is about 100–300 LOC.
- S4 touches 32 `MulliganDecisionEntry` literals in 12 files and 14 files naming `UseSerumPowder`.
- Frontend: 9 pile render sites plus 3 anchor queries.

**Regenerating commands** (run from the repo root at `d95cc66019`)

`scripts/matches.py`, `scripts/count_reads.py`, `scripts/count_reads_lines.py` and `scripts/names.txt` are not in the repo; they are attached in the **Census scripts** section at the end of this issue. Save them to a directory `<brief-dir>/scripts/`. Usage: `python3 <brief-dir>/scripts/<name>.py [REV [PATTERN]] [-v]` (REV defaults to HEAD; `REPO=<path>` overrides the cwd; `matches.py` takes only REV).

- GameFormat match sites: `python3 <brief-dir>/scripts/matches.py d95cc66019`. It walks every `.rs` file from `git grep -l -P 'GameFormat::' d95cc66019 -- '*.rs'` (31 files).
- Engine read sites: `python3 <brief-dir>/scripts/count_reads.py d95cc66019 '\.(library|graveyard)\b(?!\s*\()' -v` (per line: `count_reads_lines.py`, same arguments).
- MulliganDecisionEntry literals: `git grep -c -P 'MulliganDecisionEntry \{' d95cc66019 -- crates`.
- Pile render sites: `git grep -n -P '<(LibraryPile|GraveyardPile|ExilePile)\b' d95cc66019 -- client/src ':!*__tests__*'`.
- Pile anchor queries: `git grep -n -P 'data-(library|graveyard)-pile' d95cc66019 -- client/src`.
- Zone write population: `python3 scripts/zone_authority_census.py --list` (in the repo), plus the whole-container assignments in production code: `python3 <brief-dir>/scripts/count_reads_lines.py d95cc66019 '\.(library|graveyard)\s*=(?!=)'`.

**Process**

- The engine-implementer phase-fit gate trips on both T1 and T2, so this is a **chartered** run (`chartered.md`, `executor.md`, `pr-handoff.md`).
- `.agents/pr-review-policy.toml` `[architecture_scope]` covers format.rs, deck_loading.rs, deck_validation.rs and mulligan.rs, so the change needs a maintainer implementation review.
- None of the hard-stop paths is needed.

## Changed since the July plan

- `GameFormat` has 25 built-ins plus `Custom`, uses `EnumIter`, and has hand-written serde. It needs `FromStr` and `Display` arms and must be placed before `Custom`.
- The predicates are no longer `matches!` blocks. There are exactly 18 compiler-forced exhaustive sites, and `legality_format` is now the `card_pool` axis.
- `has_unrepresentable_auxiliary_deck_component` must be `true` for Dandan (new method).
- The `FormatGroup` arm step is gone; the group is set in the registry entry as Multiplayer.
- The engine-wasm commander-eligibility match is now exhaustive, and `from_source_format` is a new site.
- The `SharedZones` stored field with `#[serde(default)]` is replaced by typed `GameFormat` axis methods. `FormatConfig` ingress is now validated per field, which would make a stored field costly.
- Adding a variant needs a PROTOCOL/LOBBY/WIRE bump plus a per-format lobby floor.
- `supplies_fixed_deck: true` makes the AI force-keep every Dandan hand (`FixedDeckKeepMulligan`), so it is re-gated in S1.
- The client registry integration test runs in no lane. The only live mirror checks are the Rust `include_str!` tests.
- Read sites: about 150 across 58 files, not 40–60. All-players flat_maps must stay raw; `zone_object_ids` already covers library and graveyard.
- There are about 7 write sites, not 3 (`zone_container_len`, `reorder_within_library`, library.rs, resolution-choice writers, conjure), and they must canonicalize inside the journaled appliers.
- The resolved-rules journal (#6522/#6551) and the library-knowledge epochs (#7094) both key on owner/player, so both must be resolved to the canonical seat.
- The owner-scoped filters (`FilterProp::Owned`, `matches_target_filter_in_owner_zone`) and the typed-filter `controller` axis need a shared-zone collapse; the accessor alone does not fix Haunted Fengraf or Mystic Sanctuary.
- The count-dedup test driver changes to `ZoneCardCount`/`GraveyardSize`. Accumulated Knowledge cannot fail the old test.
- The engine-wasm empty-library boot guard would block Dandan.
- Owner must bind at hand entry, not at cast or land play. The July cast stamp was a no-op and both of its revert legs were vacuous.
- `finalize_cast` no longer exists. The planechase restamps are `restamp_planar_*_to_controller` (CR 901.15b). PutOnTopOrBottom and CR 401.4 are removed.
- The engine has no mulligan round, so redraw interleaving needs a new declare round. `record_final_count` is gone; there are now declare-point bottoms (`resolve_declare_point`).
- Every in-game draw is a draw-sequence frame held by a resolution frame, so S4b is a new dealer frame (with a wire-version bump), not a reroute.
- Day's Undoing now resolves as a sequential per-player wheel (#8757), so PREREQ-3 also needs the wheel split.
- The drew-from-empty flag is now written only by the ledger. S6 stays verification-only.
- Memory Lapse is now a coverage false positive (PREREQ-0). The count is 20/23 supported, not 21.
- "No sideboard" is a design decision, not quoted rule text.
- Frontend: OpponentSeatPane is unreachable at 2 seats; there is a new `format-config-shape.ts`; the engine must expose the pile holder; AnimationOverlay and castable-zone hooks need changes.
- AI: determinize is dead at K=0 (drop that row); the free-mulligan emission points are `search.rs::deterministic_choice` and `fallback_action`; `ai_duel` has no general format flag.
- CR citations are given by rule number only, because the rules file is now the Aug 7 2026 release.

## Settled decisions (issue author, 2026-09-30)

1. **Free-mulligan predicate:** a hand qualifies when it has fewer than two lands OR fewer than two nonland cards, before that player's first regular mulligan.
2. **Day's Undoing on shared zones:** every hand and the shared graveyard are shuffled into the shared library once, then 14 cards are dealt one at a time, active player first, through the S4b dealer.
3. **v1 decklist:** the full authentic 80-card list. That puts PREREQ-0 (Memory Lapse), PREREQ-1/2 (the CR 612 text-changing word-substitution primitive, for Magical Hack and Crystal Spray) and PREREQ-3 (Day's Undoing via S4b) in scope. A PR is one complete thought, so the run's charter decides whether CR 612 ships as its own PR ahead of the format PR.
4. **Architecture-scope authorization:** #5169 carries the `accepted` label (applied 2026-09-30).
5. **Worker model:** spawned roles run on Sonnet, `claude-sonnet-5` or newer. Each PR declares the models that wrote the code.

---

## Census scripts

<details><summary><code>scripts/matches.py</code></summary>

```python
# Usage (from a repo root): python3 matches.py [REV]   REV defaults to HEAD; REPO=<path> overrides the cwd.
import os, re, subprocess, sys
REF=sys.argv[1] if len(sys.argv)>1 else 'HEAD'; R=os.environ.get('REPO','.')
files=subprocess.run(['git','-C',R,'grep','-l','-P','GameFormat::',REF,'--','*.rs'],capture_output=True,text=True).stdout.split()
files=[f.split(':',1)[1] for f in files]
def strip(src):
    # blank out comments and string/char literals, preserving length/newlines
    out=list(src); i=0; n=len(src)
    while i<n:
        c=src[i]
        if src.startswith('//',i):
            j=src.find('\n',i); j=n if j<0 else j
            for k in range(i,j): out[k]=' '
            i=j
        elif src.startswith('/*',i):
            j=src.find('*/',i)+2
            for k in range(i,j):
                if src[k]!='\n': out[k]=' '
            i=j
        elif c=='"' :
            j=i+1
            while j<n and src[j]!='"':
                j+= 2 if src[j]=='\\' else 1
            for k in range(i+1,j):
                if src[k]!='\n': out[k]=' '
            i=j+1
        elif c=="'" and i+2<n and (src[i+2]=="'" or (src[i+1]=='\\')):
            j=src.find("'",i+2 if src[i+1]=='\\' else i+1)
            for k in range(i+1,j): out[k]=' '
            i=j+1
        else: i+=1
    return ''.join(out)
res=[]
for f in files:
    src=subprocess.run(['git','-C',R,'show',f'{REF}:{f}'],capture_output=True,text=True).stdout
    s=strip(src)
    testspans=[]
    for tm in re.finditer(r'#\[cfg\(test\)\]\s*(pub(\([a-z]+\))?\s+)?mod\s+\w+\s*\{',s):
        j=tm.end(); d=1
        while d:
            d+={'{':1,'}':-1}.get(s[j],0); j+=1
        testspans.append((tm.start(),j))
    for m in re.finditer(r'\bmatch\b',s):
        # find opening brace of match body at depth 0 relative
        i=m.end(); depth=0
        while i<len(s):
            if s[i] in '([': depth+=1
            elif s[i] in ')]': depth-=1
            elif s[i]=='{' and depth==0: break
            elif s[i]==';': i=-1; break
            i+=1
        if i<0 or i>=len(s): continue
        scrut=s[m.end():i].strip()
        # body
        j=i+1; d=1
        while d: 
            if s[j]=='{': d+=1
            elif s[j]=='}': d-=1
            j+=1
        body=s[i+1:j-1]
        # top-level arm patterns: text at depth0 between arm separators ending with =>
        pats=[]; d=0; start=0; k=0
        while k<len(body):
            ch=body[k]
            if ch in '([{': d+=1
            elif ch in ')]}': 
                d-=1
                if d==0 and ch=='}':
                    # block-bodied arm end; next arm starts after optional comma
                    start=k+1
            elif ch==',' and d==0: start=k+1
            elif body.startswith('=>',k) and d==0:
                pats.append(body[start:k].strip()); start=k+2; k+=1
            k+=1
        if not any(re.match(r'^\|?\s*GameFormat::',p) for p in pats): continue
        catch=[p for p in pats if re.fullmatch(r'_( if .*)?',p,re.S) or re.fullmatch(r'[a-z_][a-z0-9_]*( if .*)?',p,re.S)]
        line=s.count('\n',0,m.start())+1
        fn=None
        for fm in re.finditer(r'\bfn\s+(\w+)',s[:m.start()]): fn=fm.group(1)
        res.append((f,line,fn,scrut,'WILDCARD:'+'|'.join(c.split()[0] for c in catch) if catch else 'EXHAUSTIVE','test' if any(a<=m.start()<b for a,b in testspans) or '/tests/' in f else 'prod'))
for r in res: print('\t'.join(map(str,r)))
```

</details>

<details><summary><code>scripts/count_reads.py</code></summary>

```python
# Usage (from a repo root): python3 count_reads.py [REV [PATTERN]] [-v]   REV defaults to HEAD; REPO=<path> overrides the cwd.
import subprocess,re,os,collections,sys
ARGS=[a for a in sys.argv[1:] if a!='-v']
REV=ARGS[0] if ARGS else 'HEAD'; R=os.environ.get('REPO','.')
PATSTR=ARGS[1] if len(ARGS)>1 else r'\.(library|graveyard)\b(?!\s*\()'
def git(*a): return subprocess.run(['git','-C',R,*a],capture_output=True,text=True).stdout
roots=['crates/engine/src','crates/phase-ai/src','crates/engine-wasm/src','crates/phase-llm/src']
files=[f for r in roots for f in git('ls-tree','-r','--name-only',REV,'--',r).split() if f.endswith('.rs')]
src={f:git('show',f'{REV}:{f}') for f in files}
TESTATTR=re.compile(r'#\[cfg\((test|any\(test\b[^\]]*)\)\]')
testmods=set(); prodmods=set()
for f,s in src.items():
    L=s.split('\n'); d=os.path.dirname(f); b=os.path.basename(f)
    base=d if b in('mod.rs','lib.rs','main.rs') else os.path.join(d,b[:-3])
    for i,l in enumerate(L):
        if not TESTATTR.search(l):
            m=re.match(r'\s*(pub(\([a-z]+\))?\s+)?mod (\w+);',l)
            if m and not (i>0 and re.search(r'#\[cfg\(', L[i-1])):
                prodmods.add(os.path.join(base,m.group(3)+'.rs'))
            elif m and i>0 and 'cfg(not(' in L[i-1]: prodmods.add(os.path.join(base,m.group(3)+'.rs')); prodmods.add(os.path.join(base,m.group(3))+'/')
            continue
        path=None
        for j in range(i+1,min(i+6,len(L))):
            m=re.match(r'\s*#\[path\s*=\s*"([^"]+)"\]',L[j])
            if m: path=m.group(1); continue
            if L[j].strip().startswith('#['): continue
            m=re.match(r'\s*(pub(\([a-z]+\))?\s+)?mod (\w+);',L[j])
            if m:
                if path: testmods.add(os.path.normpath(os.path.join(d,path)))
                else: testmods.add(os.path.join(base,m.group(3)+'.rs')); testmods.add(os.path.join(base,m.group(3))+'/')
            break
def is_test_file(f):
    if re.search(r"(_tests?\.rs$|/tests?/|/tests?\.rs$|/test_[a-z_]+\.rs$)", f): return True
    if f in prodmods: return False
    return f in testmods or any(t.endswith('/') and f.startswith(t) and not any(f.startswith(p) for p in prodmods if p.endswith('/')) for t in testmods) or '/bin/' in f
PAT=re.compile(PATSTR)
counts=collections.Counter(); perfile=collections.Counter(); excluded=0
for f,s in src.items():
    crate=f.split('/')[1]
    if is_test_file(f): excluded+=len(PAT.findall(s)); continue
    depth=0; skip=False; pending=False; skipdepth=0
    for l in s.split('\n'):
        code=re.sub(r'//.*','',re.sub(r'"(\\.|[^"\\])*"','""',l)).replace("'{'","").replace("'}'","")
        if not skip and TESTATTR.search(l): pending=True; continue
        if pending and not skip:
            if code.strip().startswith('#['): continue
            if '{' in code: skip=True; skipdepth=depth
            elif code.strip().endswith(';'): pending=False; continue
        o=code.count('{'); c=code.count('}')
        depth+=o-c
        if skip:
            if depth<=skipdepth: skip=False; pending=False
            continue
        n=len(PAT.findall(code))
        if n: counts[crate]+=n; perfile[f]+=n
print('occurrences by crate:',dict(counts)); print('files by crate:',dict(collections.Counter(f.split('/')[1] for f in perfile)))
print('excluded (test-only files):',excluded)
if '-v' in sys.argv:
    for f,n in perfile.most_common(): print(n,f)
```

</details>

<details><summary><code>scripts/count_reads_lines.py</code></summary>

```python
# Usage (from a repo root): python3 count_reads_lines.py [REV [PATTERN]] [-v]   REV defaults to HEAD; REPO=<path> overrides the cwd.
import subprocess,re,os,collections,sys
ARGS=[a for a in sys.argv[1:] if a!='-v']
REV=ARGS[0] if ARGS else 'HEAD'; R=os.environ.get('REPO','.')
PATSTR=ARGS[1] if len(ARGS)>1 else r'\.(library|graveyard)\b(?!\s*\()'
def git(*a): return subprocess.run(['git','-C',R,*a],capture_output=True,text=True).stdout
roots=['crates/engine/src','crates/phase-ai/src','crates/engine-wasm/src','crates/phase-llm/src']
files=[f for r in roots for f in git('ls-tree','-r','--name-only',REV,'--',r).split() if f.endswith('.rs')]
src={f:git('show',f'{REV}:{f}') for f in files}
TESTATTR=re.compile(r'#\[cfg\((test|any\(test\b[^\]]*)\)\]')
testmods=set(); prodmods=set()
for f,s in src.items():
    L=s.split('\n'); d=os.path.dirname(f); b=os.path.basename(f)
    base=d if b in('mod.rs','lib.rs','main.rs') else os.path.join(d,b[:-3])
    for i,l in enumerate(L):
        if not TESTATTR.search(l):
            m=re.match(r'\s*(pub(\([a-z]+\))?\s+)?mod (\w+);',l)
            if m and not (i>0 and re.search(r'#\[cfg\(', L[i-1])):
                prodmods.add(os.path.join(base,m.group(3)+'.rs'))
            elif m and i>0 and 'cfg(not(' in L[i-1]: prodmods.add(os.path.join(base,m.group(3)+'.rs')); prodmods.add(os.path.join(base,m.group(3))+'/')
            continue
        path=None
        for j in range(i+1,min(i+6,len(L))):
            m=re.match(r'\s*#\[path\s*=\s*"([^"]+)"\]',L[j])
            if m: path=m.group(1); continue
            if L[j].strip().startswith('#['): continue
            m=re.match(r'\s*(pub(\([a-z]+\))?\s+)?mod (\w+);',L[j])
            if m:
                if path: testmods.add(os.path.normpath(os.path.join(d,path)))
                else: testmods.add(os.path.join(base,m.group(3)+'.rs')); testmods.add(os.path.join(base,m.group(3))+'/')
            break
def is_test_file(f):
    if re.search(r"(_tests?\.rs$|/tests?/|/tests?\.rs$|/test_[a-z_]+\.rs$)", f): return True
    if f in prodmods: return False
    return f in testmods or any(t.endswith('/') and f.startswith(t) and not any(f.startswith(p) for p in prodmods if p.endswith('/')) for t in testmods) or '/bin/' in f
PAT=re.compile(PATSTR)
LINES=[]; counts=collections.Counter(); perfile=collections.Counter(); excluded=0
for f,s in src.items():
    crate=f.split('/')[1]
    if is_test_file(f): excluded+=len(PAT.findall(s)); continue
    depth=0; skip=False; pending=False; skipdepth=0
    for l in s.split('\n'):
        code=re.sub(r'//.*','',re.sub(r'"(\\.|[^"\\])*"','""',l)).replace("'{'","").replace("'}'","")
        if not skip and TESTATTR.search(l): pending=True; continue
        if pending and not skip:
            if code.strip().startswith('#['): continue
            if '{' in code: skip=True; skipdepth=depth
            elif code.strip().endswith(';'): pending=False; continue
        o=code.count('{'); c=code.count('}')
        depth+=o-c
        if skip:
            if depth<=skipdepth: skip=False; pending=False
            continue
        n=len(PAT.findall(code))
        if n: counts[crate]+=n; perfile[f]+=n; LINES.append((f,l.strip()))
print('occurrences by crate:',dict(counts)); print('files by crate:',dict(collections.Counter(f.split('/')[1] for f in perfile)))
print('excluded (test-only files):',excluded)
if '-v' in sys.argv:
    for f,n in perfile.most_common(): print(n,f)
for f,l in LINES: print(f.replace("crates/",""),"|",l[:110])
```

</details>

<details><summary><code>scripts/names.txt</code></summary>

```text
Dandân
Island
Memory Lapse
Accumulated Knowledge
Magical Hack
Mystic Sanctuary
Brainstorm
Capture of Jingzhou
Chart a Course
Control Magic
Crystal Spray
Day's Undoing
Mental Note
Metamorphose
Predict
Telling Time
Unsubstantiate
Halimar Depths
Haunted Fengraf
Lonely Sandbar
Remote Isle
The Surgical Bay
Svyelunite Temple
```

</details>


