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
