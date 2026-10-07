# Charter review r1 — re-charter Phases 18–22 (charter mode)

Reviewer model: claude-sonnet-5-5. Inputs: `phase-charter-18.md` against W HEAD `303d76fa69`. Skills read from the 1ecbc840cf checkout (`review-engine-plan` charter mode, `chartered.md`).

Verdict: 1 decision finding (`behavior`), 8 corrections (`text`), 0 `machinery`. The decision finding changes what the Phase 21 implementer writes; the round is a design round until it is repaired. Everything else is applied by the orchestrator with no further round.

## What held (measured, not read)

- Phase 18: `card_name_choice_candidates` has exactly one library read (`grep -n 'library_of\|\.library\b'` over `ai_support/candidates.rs` + `ai_support/mod.rs` returns one hit, `candidates.rs` in that function; control: the same grep with `\.hand\b` returns 11 hits). `determinize_opponents` resamples `library_of(opponent)` (the shared pile) together with opponent hand slots (`unknown_slots`), so the library read drifts across samples. The `finalize_mean` `debug_assert_eq!(observed, k)` and per-sample `score_candidates_core(&sampled, ..)` are as the maintainer described. Derived by reading, not driven.
- Phase 19: hand-traced the three-way example through the base `depends_on` arm (`b_to == a_from || b_from == a_from`): A→B→C→A, one SCC, timestamp order A,B,C, ends Forest. Under decision 1's relation, pending [A,B,C] on Island: only C depends (on A), A applies, then C, then B, ends Island. The existing rows (`chain_of_text_changes...`, `loop_of_text_changes...`, `chain_plus_loop_on_one_object...`) give the same results under the new relation, so "replaced" rows are limited to those that call `active_text_substitutions`. The select-next kernel in `apply_ability_effects_with_referenced_grants` is a closure over `edges` and is separable. Not driven (no scratch build).
- Phase 20: census over `client/public/card-data.json`: 39 cards carry `ManaColorSpent`, 0 of them with word-form text; every card whose text matches `{X}… was spent to cast` carries `ManaColorSpent` (39/39, none carries `ManaSpentToCast`); 15 of 18 Adamant cards carry `ManaSpentToCast`+`OfColor`, none `ManaColorSpent`. Firespout/Batwing Brume Oracle text verbatim as the charter says. A carrier-wide search (WORD-classed color carriers on cards whose text lacks the color word) shows `ManaColorSpent` as the only symbol-provenance instance.
- Oracle texts of Magical Hack, Crystal Spray, Sleight of Mind, Firespout, Batwing Brume, Predict match the charter's use.
- CR 613.8a/b/c, 612.2, 407.3 verified by grep of `docs/MagicCompRules.txt` and subjects match (except finding T1).
- Decision 5 (Custom exposure rejected): `from_lobby_config_rejects_dandan_source` exists; `GameFormat::Custom(_)` returns `SharedZones::NONE`; `StructuralRules` mirrors `FormatConfig`; `customFormatHostUnavailable` blocks hosting a saved Custom format (`HostSetup.tsx`). The fork's measured reason stands.
- Phase 21 premises: `validate_deck_for_format` is the one gate behind the server create handler, `ai_seat_setups`, wasm `validate_deck_list_seats`, and the P2P guest gate (`evaluateDeckFormatGate` -> `evaluate_deck_format_gate` -> `validate_deck_for_format`); the wasm loop skips `supplies_fixed_deck` formats; an empty list fails "exactly 80". `session.rs::start_game` takes `decks[0]` as the player seat; `ai_seat_setups` skips seat 0; P2P host passes `hostDeck.player` as seat 0. `reconstruct_initial_state` re-runs `load_and_hydrate_decks` on the header deck list. No perf-gate unit appears; no ban list or copy limit is introduced.
- Seams: 18→19→20→21→22 each leave the tree green (21 keeps every client submitting empty seats until 22); revised decisions are repaired by later phases, no accepted phase is reworked in place; T1∧T2 verdicts hold (Phase 22 counts 13 with tests but one unit).

## Decision finding

**B1 `behavior` — Phase 21 decision 3, non-pile-seat refusal cannot be written at the authority and contradicts decision 1.**
Quote: "A non-empty submission from a seat that is not the pile seat is refused by the same rule, so clients submit nothing for those seats."
Evidence: `DeckCompatibilityRequest` has no seat field (`grep -n 'pub ' ` over the struct: main_deck, sideboard, commander, companion, planar_deck, scheme_deck, signature_spell, selected_format, selected_match_type, player_count, summary_only, draft_set_codes). Every caller validates one deck at a time (server create, `ai_seat_setups`, wasm per-seat loop, `evaluate_deck_format_gate` for the P2P guest, which is a wasm-exposed wire request). Writing the refusal needs a seat parameter on a shared signature and a wire type (outside the 9-path scope), or a per-boundary seat check at four call sites, which is the "every boundary re-decides" shape decision 3 itself rejects. Decision 1 already says the loader ignores every non-pile seat's submission, so the refusal adds no safety the loader does not give.
Repair: delete the sentence. Non-pile seats' lists are ignored by `dandan_fixed_deck_payload`; clients submit nothing there (Phase 22). Closing test: in E1 a deck on seat 1 is ignored (already planned). Nothing else changes.

## Corrections (`text`; apply, no review round)

T1. Phase 20 Goal, CR anchor subject. Old: "A color-word text change replaces words, not printed mana symbols (CR 612.1, CR 612.2)." New: "A color-word text change replaces words, not printed mana symbols (CR 612.2)." CR 612.1 says text-changing effects "can apply to any words or symbols printed on that object", the opposite subject; 612.2 ("a Magic color word being used as a color word") carries the claim.

T2. Phase 19 seam note. Old: "`text_substitution.rs` is Phase 5's file and edited by no other phase." New: "`text_substitution.rs` is Phase 5's file and is edited again by Phase 20."

T3. Phase 18 pool authority. Old: "(`deck_pool_of(controller)`, `registered_main`, public decklist knowledge, constant for the game)". New: "(`deck_pool_of(controller)` names from `current_main`, the pool `deck_knowledge::unknown_hidden_pool` samples from; public decklist knowledge, written only at deck load and between games)". The claim bullet "`PlayerDeckPool::registered_main` does not change during a game while `current_main` does" is false: `current_main` is written only at load (`deck_loading.rs`) and by `match_flow.rs` between games; every sibling AI consumer reads `current_main`. Both are constant in a game, so this is a consistency fix, not a behavior change for Dandan (Bo1).

T4. Phase 20 scope. Old: "`crates/engine/src/parser/oracle_trigger.rs` (the Adamant word emitter and the symbolic-form extractor);" New: append "`crates/engine/src/parser/oracle_trigger_tests.rs` (rows that pin an emitter the phase deletes)". `extract_adamant_three_red` there calls the word emitter directly; deleting the emitter on the SYMBOL branch breaks it, and the path is neither compiler-forced nor registration.

T5. Each of Phases 18–22 omits its addenda file. Add to each entry: "**Addenda file.** `addenda/phase-18`" (resp. 19, 20, 21, 22).

T6. Acceptance rows and claims belong to phase plans, not the charter. Delete from Phases 18–22 every "**Claims to establish.**" block and every "**Verification plan.**" block (sites: `grep -n '^\*\*Claims to establish\|^\*\*Verification plan' phase-charter-18.md`, ten lines), plus the "measured at charter" figures and "red at base" outcomes they carry, and the Phase 18 deferral sentence "Sampled-hand consumers other than card-name choice have no hit in the measured generators" (code-state assertion; replace the list with "None."). The orchestrator carries the V/E/C rows into the phase dispatch briefs as constraints, with these adjustments for the planner:
- Phase 20 V3 rationale "it proves the fix is not a blanket SYMBOL tag" is false on the SYMBOL branch: Adamant carries `OfColor`, not `ManaColorSpent`. The row pins the word-form `OfColor` carrier (the maintainer's required paired word-form control). A blanket-tag exclusion needs a row on a reachable word-form `ManaColorSpent` emitter, which exists only on the discriminator branch. The census above says the SYMBOL branch applies; the plan must re-run it.
- Phase 22 closing "browser run against the built wasm" must use the existing `run` skill launch; no new driver, session manager or seeding service (machinery needs an accepted expansion case).

T7. Phase 22 decision text leaves the five `formatSuppliesDeck` readers (`GameSetupPage`, `GameProvider` x5 sites, `AiOpponentConfig`) ambiguous. Append to the architecture decisions: "Every existing `formatSuppliesDeck` reader decides from the engine's `deck_supply` answer, so no client site keeps a second supply predicate; `EngineFixed` and `PlayerBuilt` behavior is unchanged." (`formatSuppliesDeck` is true for Dandan today, so an unrouted site would skip the pile choice for `HostPile`.)

T8. Opening paragraph, garbled list. Old: "(`layers.rs`/`text_substitution.rs`; `ai_support/candidates.rs`; the condition parsers and carrier table; deck loading/validation/lobby)". New: "(H: `layers.rs`, `text_substitution.rs`; M: `ai_support/candidates.rs`; S: the condition parsers and the carrier table in `text_substitution.rs`; P: deck loading, validation, lobby)".

## Pre-existing (untagged, no action)

None confirmed. One unconfirmed observation from the carrier search: WORD-classed `Token.colors[]`, `AddKeyword.Color`, `SetColor.colors[]` appear on cards whose text has no color word (keyword shorthand such as "For Mirrodin!", "all colors", "color of your choice"); a different class from printed-symbol provenance, not asserted as a defect.

## Probe provenance

`/home/lgray/vibe-coding/dandan-run/scratch/probe_carriers.py` (read-only census over `card-data.json`; positive control: it reports the known `ManaColorSpent` hits). No cargo run was made; the Phase 18 and 19 failure claims are derived by reading and hand-trace, and each phase's own failing-first row is their measurement.
