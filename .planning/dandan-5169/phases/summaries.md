# Accepted phase summaries (for later planners; no debates)

## Phase 1 — card-bot /lfg format autocomplete
- `scripts/card-bot/lfgInteractions.ts` exports `LFG_FORMAT_OPTION` (autocomplete, no static choices) and `suggestFormats(query, registry = FORMATS)` (prefix before contains, lowercased label+key, capped at 25). `register.ts` uses `LFG_FORMAT_OPTION`. `formats.test.ts` asserts no static choice list; mirror-equality test unchanged (Phase 2 adds the Dandan entry to `scripts/card-bot/formats.ts`).

## Phase 2 — S1 format registration
- `GameFormat::Dandan` (after FreeformCommander, before Custom); `FormatConfig::dandan()` (20 life, 2 players, exactly 80, supplies_fixed_deck, no command zone); registry entry after Momir.
- Six exhaustive axis methods on `GameFormat` in `crates/engine/src/types/format.rs`: shared zones, draw deal order, free-reveal mulligan, hand-entry ownership, opening-hand equivalence, best-of-three ceiling. Census test `format_axis_census.rs` covers them.
- Deck validation: Dandan joins the Freeform constructed arm group (no card pool; axes read). `custom_format.rs` refuses Dandan as a Custom source. manabrew-compat refuses shared-zone snapshots (`upstream.shared-zone-ownership-missing`).
- Protocol: full-game 93, lobby 15 (`MIN_LOBBY_PROTOCOL_FOR_DANDAN = 15` frozen in ws-adapter.ts), wire 75. card-bot `formats.ts` has the Dandan entry.
- Known interim: `supplies_fixed_deck` true but deck loading keys on Momir until Phase 6.

## Phase 3 — AI force-keep re-gate
- `FixedDeckKeepMulligan` (phase-ai `policies/mulligan/fixed_deck_keepables.rs`) force-keeps only when `format.opening_hand_equivalence()` is Equivalent (Momir); Dandan abstains. Trace fact `opening_hand_equivalent`. ai-gate owed at run level after Phase 17.

## Phase 4 — PREREQ-0 swallow check
- `parser/swallow_check.rs::effect_is_replacement_carrier` accepts `Effect::Counter { countered_spell_zone: Some(_), .. }`. Memory Lapse, Lapse of Certainty, Remand, Spell Crumple now supported; Hinder stays flagged.

## Phase 4b — CR 613.8b loop-only dependency ordering (fix-first)
- `layers.rs::order_with_dependencies` now orders EFFECTS (entries grouped by `ContinuousEffectGroupKey`, contiguous): SCCs (`tarjan_scc`) have intra-loop edges replaced by a timestamp-rank chain; lowest-rank Kahn over the rest. Loop members keep timestamp order, each still waits for its own outside dependencies; independent effects keep timestamp order. Tests: unit H/G rows, `tests/integration/loop_only_dependency_fallback.rs` (Curse of Conformity, March of the Machines, Prismatic Omen, Cloak and Dagger, Ultima).

## Phase 5 — CR 612 text-changing primitive
- `ContinuousModification::SubstituteTextWord { TextSubstitutionSpec }` (Fixed | Chosen over `TextSubstitution` Color | BasicLandType) at `Layer::Text`; single rewrite authority `game/text_substitution.rs` (WORD_CARRIERS, rewrite, active_text_substitutions, Layer-3 pre-pass, `restamp_resolving_spell_text` called from `stack.rs::resolve_top`); `layers.rs` pre-pass, bucket filter, incremental-flush escalation, recipient-scoped `depends_on` arm (consumes Phase 4b ordering); latch `latch_chosen_text_words` (take). Parser `parser/oracle_effect/text_change.rs` (nom). Supported: Magical Hack, Crystal Spray, Sleight of Mind, Alter Reality, Glamerdye, Mind Bend, Spectral Shift, Trait Doctoring, Whim of Volrath. Protocol 94 / wire 76. Fixture re-sliced (+87/-2).

## Phase 6 — S2a canonical-seat storage + pool resolver (8eecaec29b, 1b112fdec9)
- `GameState::canonical_seat`, `zone_storage_seat`, `library_of/_mut`, `graveyard_of/_mut`, `deck_pool_of`, `seats_with_empty_library` in `types/game_state.rs`; library-knowledge epoch keys/stamp resolve to the storage seat. Zone appliers, library shuffle, scry/dig reorder writers, `EffectZoneChoice` placement, conjure, draw selection, `mulligan.rs` shuffle_hand_into_library/draw_n and the wasm boot guard route through the accessors (non-shared formats = identity). `deck_loading.rs`: `dandan_fixed_deck_names`, Dandan branch loads one 80-card pile on the canonical seat; non-holder pools/library loads dropped. visibility.rs unedited. Tests: `dandan_shared_pile_storage.rs` (20) + 5 inline. Fixture regenerated (names the five snow basics as literals). No serialized shape change, no protocol bump.
- Read sweep (raw `.library`/`.graveyard` reads) is Phases 8–9; hand-entry ownership Phase 11.

## Phase 7 — S5 best-of-three ceiling (be100055ae, e0d9cb1208; plus merge-fix c1cf96cf63 before it)
- `engine.rs::match_type_within` clamps the requested match type to `format.best_of_three_ceiling()` in `start_game_with_starting_player` (Dandan Bo3 plays as Bo1); `engine-wasm` exports the ceiling (+ regenerated `engine_wasm.d.ts`), `engineRuntime.ts` wrapper, `HostSetup.tsx` exports `useBestOfThreeCeiling`/`cappedMatchType` and caps the selector, `GameSetupPage.tsx` caps it too. Tests: `best_of_three_ceiling.rs` (R1–R6), HostSetup H1–H5, GameSetupPage G1/G2 (Momir counterpart). No protocol bump.

## Phase 8 — S2b-1 read sweep game/ (616148f99e, e35fc63606)
- All 45 `game/` production sites that read a seat's library/graveyard container now resolve through the Phase 6 accessors (classes S, S-guard, L, Q, W, R); shared helper `non_owner_graveyard_ids` (casting.rs) for the L-class scan; `candidate_player_scalar` GraveyardSize arm moved to `candidate_player_scalar_with_state`; clash reads the top via `.front()`; `morph.rs` owner clause; `quantity.rs` `TargetZoneCardCount`. Residual raw reads = the plan's F and D sets plus Phase 6's kept-raw lines (24 lines at the candidate). Tests: `dandan_read_sweep.rs` (46 rows) + inline `dandan_read_sweep_tests` modules. Quantity arms Krosan Avenger / Kindly Stranger remain Phase 10; AI/analysis sites are Phase 9/17.
