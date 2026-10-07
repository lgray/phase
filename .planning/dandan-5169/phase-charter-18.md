# Phase charter — re-charter over the remaining work: Phases 18–22 (run dandan-5169)

Input: charter `phase-charter` (r5, Phases 1–17 and 4b, all accepted), `phases/summaries.md`, addenda files `addenda/phase-4` … `phase-17`, issue #5169 body and comments, two maintainer reviews of PR #9669 at `10b758a281` (`run-level/review-9669.json`, `run-level/review-9669-2.md`), and the USER scope addition (custom Dandan piles). Premises were measured at `10b758a281` and the layers code re-read at the merged base `303d76fa69` (the select-next kernel, the `SubstituteTextWord` arm of `depends_on` and `active_text_substitutions` are unchanged in shape; upstream #9654 added `granted_static_dependency` beside them); every phase plan takes `303d76fa69` as its base and re-measures its own premises (candidate claims and rows: `phase-claims-18-22.md`); "derived" marks a premise bought by reading, not by a driven run.

**Why five phases.** Four independent units arrived: (H) the CR 613.8 text-change dependency repair (maintainer HIGH), (M) the card-name candidate domain (maintainer MED), (S) word-versus-symbol provenance of the spend-color condition under text change (second maintainer review, MED), (P) host-supplied Dandan piles (USER). Their code paths are disjoint except that H and S both edit `text_substitution.rs` (H: `layers.rs`, `text_substitution.rs`; M: `ai_support/candidates.rs`; S: the condition parsers and the carrier table in `text_substitution.rs`; P: deck loading, validation, lobby), so S follows H, each is red-testable alone, and the tree stays green between them. Pile work is two units (an engine mechanic and its client display, different skill checklists, joined by one dependency seam, the engine's wasm export), so the combined bundle trips T1∧T2 and splits at that seam. Order: the small repairs first (M, H, then S, which shares `text_substitution.rs` with H), then the engine half of the pile, then the client half.

**Deliverable change.** The run's PR now carries Phases 1–22. Planning estimate for the new phases ~1,450 LOC primary (sum of the entries below); the PR total is the charter's phase-list sum plus this, still far below the ~30,000-line ceiling.

**Standing rules** of `phase-charter` apply unchanged (verbatim Oracle text fetched at plan time, protocol script, bindings regeneration, locale parity).

## Phase list

18. Card-name candidate domain constant across determinized worlds (maintainer MED) — ~110 LOC
19. CR 613.8 text-change dependencies from the recipient's current text, one effect at a time (maintainer HIGH) — ~450 LOC
20. Word versus symbol provenance of `ManaColorSpent` under text change (maintainer MED 2) — ~220 LOC
21. S9a — Host-supplied Dandan pile: deck-supply axis, loader, validator, wasm export — ~280 LOC
22. S9b — Host-supplied Dandan pile: setup, lobby, P2P and native client flows — ~390 LOC

---

### Phase 18 — Card-name candidate domain

**Goal.** A card-name choice (Predict: "Choose a card name, then target player mills a card.") issues the same candidate names in every `determinize_opponents` sample and in the actual-state contract, so `finalize_mean`'s constant-support invariant and the actual-state contract filter hold under `determinization_samples` K > 1 in a shared-library format.

**Architecture decisions.** The fix is at the candidate generator, not the resampler: the determinizer's pin invariant (candidate support is constant across samples) is the contract, and any generator reading a zone the determinizer writes violates it. `card_name_choice_candidates` (`crates/engine/src/ai_support/candidates.rs`) reads the controller's `library_of` identities; under a shared library that container is the resampled pile. The domain becomes a pure function of state the determinizer never writes: the source display name, battlefield/exile/graveyard (public), the controller's own hand, and, where the format shares the library, the pile's registered pool names (`deck_pool_of(controller)` names from `current_main`, the pool `deck_knowledge::unknown_hidden_pool` samples from; public decklist knowledge, written only at deck load and between games) in place of the live library identities. Where each seat owns its library the controller's own library is never resampled and the existing read stays, so non-shared candidate lists stay byte-identical. The shared-library test reads the existing engine authority (`GameState::shared_zone_holder(Zone::Library)`), never a format literal. Candidate order is registered-pool order, deduplicated case-insensitively, capped as today. Pinning all domain-bearing identities and a fixed root domain are rejected: neither exists on this path and either contradicts the finding.

**Scope rule.** `crates/engine/src/ai_support/candidates.rs` (`card_name_choice_candidates` and its inline `mod tests`); `crates/phase-ai/src/search.rs` (inline determinization test module only: the K > 1 regression).

**Deferral list.** None.

**Seam notes.** `ai_support/candidates.rs` was edited by Phase 9 (the `library_of` swap this phase replaces in this one function) and Phase 13; `phase-ai/src/search.rs` is edited by Phases 12, 13, 17 at other functions.

**Addenda file.** `addenda/phase-18`

**T1∧T2 check.** 1 unit (one candidate-domain rule). Scope 2 paths. T1 fails and T2 fails.

---

### Phase 19 — CR 613.8 text-change dependencies

**Goal.** Text-changing effects on one recipient apply in the order CR 613.8 gives from that recipient's current text: dependency is decided by whether applying another effect changes what an effect does to the text it has now (CR 613.8a), one effect is applied at a time, and the remaining relation is recomputed after each (CR 613.8c). Example the review fixes: on an Island, A = Island→Swamp, B = Forest→Island (Magical Hack), C = Swamp→Forest (Crystal Spray), all live in one turn: the correct order A, C, B ends Island (blue); the word-pair graph forms a three-way loop, falls to timestamp A, B, C and ends Forest (green).

**Architecture decisions.**
1. The relation is computed from the recipient's text, never from word pairs: effect X depends on effect Y iff the occurrences X would rewrite in the recipient's text now differ from those it would rewrite in the text after Y — measured through the one rewrite authority in `text_substitution.rs` (`TextSubstitution::rewrite`), previewed on a scratch copy of the recipient (permanent object, or the resolving spell's `ResolvedAbility`).
2. Selection extends the existing state-aware authority, not a parallel graph: the select-next kernel that `apply_ability_effects_with_referenced_grants` (`layers.rs`) holds inline (edges over pending effects, the CR 613.8b loop rule through `dependency_path_exists`, oldest-loop-member tiebreak) is extracted into one function both layers call; the text layer supplies its edges from decision 1, applies the selected effect to the recipient, and rebuilds the edges. The referenced-grant behavior is unchanged by the extraction.
3. `active_text_substitutions` stops returning a pre-ordered, frozen list. Both consumers (`apply_battlefield_text_substitutions` at the Layer 3 pre-pass and `restamp_resolving_spell_text` at resolution) go through the one selection-and-apply routine over recipient text.
4. The `SubstituteTextWord` arm of `depends_on` (word-pair inference) is deleted; the fixed graph of `order_with_dependencies` no longer sees substitutions. Phase 4b's loop-only fallback stays for the layers that still use the fixed graph.
5. No new variant, no serialized shape, no protocol change.

**Scope rule.** `crates/engine/src/game/layers.rs` (the extracted selection kernel, its call from the ability-layer selector, the `depends_on` text arm, the `order_with_dependencies` comment, inline tests that pin the deleted arm); `crates/engine/src/game/text_substitution.rs`; `crates/engine/src/game/stack.rs` (the `restamp_resolving_spell_text` call site only if its signature changes); `crates/engine/tests/integration/text_substitution_cr612.rs` (rows that call `active_text_substitutions`, plus the new rows).

**Deferral list.** None.

**Seam notes.** `layers.rs` was edited by Phases 4b and 5 and by upstream #9654 (`granted_static_dependency`, left as is); this phase touches the ordering functions, the ability-layer selector and Phase 5's `depends_on` arm. `text_substitution.rs` is Phase 5's file and is edited again by Phase 20.

**Addenda file.** `addenda/phase-19`

**T1∧T2 check.** 1 unit (one dependency-evaluation rule applied to two layers by one kernel). Scope 5 paths. T1 fails and T2 fails.

---

### Phase 20 — Word versus symbol provenance of `ManaColorSpent`

**Goal.** A color-word text change replaces words, not printed mana symbols (CR 612.2). Firespout's rider is printed "{R} was spent to cast this spell" and Batwing Brume's "{W}"/"{B}"; today the spend condition is classified as a replaceable word, so Sleight of Mind or Crystal Spray (Red→Blue) cast in response turns Firespout's red rider into a blue one and the non-flying damage branch is skipped for a spell paid with red.

**Architecture decisions.** Provenance is carried by the typed condition and decided where the parser emits it, and `text_substitution.rs` reads it; the carrier table (keyed by serialized `type` tag and field) gains no blanket reclassification of the tag `ManaColorSpent` while any word-form emitter of that tag is reachable (Adamant's "at least three white mana" is a color word and must still change). Rule for the plan: the classification must be exact over what the parsers emit. If every reachable emitter of the tag is symbolic (the word-form emitters being shadowed by the generic `QuantityCheck { ManaSpentToCast … }` bridge, as the doc on `parse_word_mana_color_spent_condition` says for the ability form), the carrier is SYMBOL and the unreachable word emitters are deleted so the table cannot lie; if a word-form emitter is reachable, the condition carries a typed word-or-symbol discriminator set by each parser (an existing or minimal typed enum, never a `bool`), and the carrier reads it. A representation change to a serialized condition is a wire-shape change and takes the protocol bump per the standing rule; the plan states which branch applies from its census. CR 612 behavior elsewhere (word carriers, Phase 5's rewrite authority) is untouched.

**Scope rule.** `crates/engine/src/game/text_substitution.rs` (the `ManaColorSpent` carrier row and its census); `crates/engine/src/parser/oracle_effect/conditions.rs` (the symbolic and word spent-condition parsers); `crates/engine/src/parser/oracle_trigger.rs` (the Adamant word emitter and the symbolic-form extractor); `crates/engine/src/parser/oracle_trigger_tests.rs` (rows that pin an emitter the phase deletes); `crates/engine/src/types/ability.rs` only if the discriminator branch applies; `crates/engine/tests/integration/text_substitution_cr612.rs` (new rows). Compiler-forced consumers of the condition and, on the discriminator branch, the protocol pins and bindings, are standing-class inclusions.

**Deferral list.** None. The PR body and coverage signatures: a representation change shows in the parse-diff; the plan states it.

**Seam notes.** `text_substitution.rs` is also edited by Phase 19 (the selection routine), so this phase runs after it; the carrier table is Phase 5's.

**Addenda file.** `addenda/phase-20`

**T1∧T2 check.** 1 unit (one provenance rule for one condition family). Scope 5 paths plus compiler-forced consumers on the discriminator branch (admitted class). T1 fails, so the conjunction cannot fire.

---

### Phase 21 — S9a: host-supplied pile (engine)

**Goal.** One Dandan game plays one 80-card pile chosen by the host: the Secret Lair list (`DANDAN_DECKLIST`) is the default, and an 80-card list submitted for the pile seat replaces it. The engine decides what the pile is, validates it, and exposes whether a format accepts a host pile; every transport reaches the one `load_and_hydrate_decks` authority, so AI, P2P, native and server games and replay take the same path.

**Architecture decisions.**
1. **The pile rides as the pile seat's submitted deck; no new wire or config field.** The pile seat is `GameState::canonical_seat()` (seat 0, host or creator in every start path). `dandan_fixed_deck_payload` (`deck_loading.rs`) loads that seat's submitted `main_deck` when it is non-empty and the default list otherwise; every other seat's submission is ignored and gets the empty payload as today. Replay reads the stored deck list through the same function, so a custom-pile game replays without a new header field (a pile held in format config would not be in the deck list `reconstruct_initial_state` resolves).
2. **A typed deck-supply axis.** `GameFormat::deck_supply() -> DeckSupply { PlayerBuilt, EngineFixed, HostPile }` (exhaustive, Momir `EngineFixed`, Dandan `HostPile`, `Custom(_)` `PlayerBuilt`), added to the axis census; `supplies_fixed_deck()` is redefined as `deck_supply() != PlayerBuilt` so the derived `FormatConfig.supplies_fixed_deck` and the registry stay one source of truth. No serialized shape changes.
3. **One validation authority.** `validate_deck_for_format` (the gate every game-creation boundary and the P2P guest gate run) treats a submission with every slot empty as "supplies no deck" for a format whose `deck_supply()` is not `PlayerBuilt`, and checks any other submission with the existing constructed validator that Dandan already uses (exactly 80, no sideboard, no commander slot, unlimited copies, every name resolvable, plus the format-independent ante refusal, CR 407.3). No ban list and no copy limit. Standard-style formats keep refusing an empty deck.
4. **The wasm boundary** drops its `supplies_fixed_deck` skip in `validate_deck_list_seats` and calls the authority for every seat; a new export `deck_supply_for_format` (beside `bestOfThreeCeilingForFormat`) returns the axis for the client.
5. **Custom-format exposure is rejected.** Measured reasons: the axes are `GameFormat` methods that answer the stock value for `Custom(_)`; `StructuralRules` mirrors `FormatConfig` 1:1 and `from_lobby_config` refuses Dandan as unrepresentable, so a carry needs a schema change to the shipped Custom type (a migration) while Dandan's choice needs none; the lobby blocks hosting any saved Custom format (`customFormatHostUnavailable`), so no start path exists to expose; and the validator that checks the pile is the shared constructed validator Custom would reuse, so exposure adds nothing to validation.

**Scope rule.** `crates/engine/src/types/format.rs` (`DeckSupply`, `deck_supply`, `supplies_fixed_deck` re-derivation and its doc); `crates/engine/src/game/deck_loading.rs` (`dandan_fixed_deck_payload`); `crates/engine/src/game/deck_validation.rs` (`validate_deck_for_format`, empty-submission rule); `crates/engine-wasm/src/lib.rs` (`validate_deck_list_seats`, the new export) with its committed binding `client/src/wasm/engine_wasm.d.ts` (groups with it for T2); `crates/engine/tests/integration/format_axis_census.rs`; a new `crates/engine/tests/integration/dandan_custom_pile.rs` with its `mod` line in `crates/engine/tests/integration/main.rs`; `crates/engine/tests/integration/dandan_shared_pile_storage.rs` (rows naming the default list). `crates/phase-server/src/main.rs` (`ai_seat_setups`, the create-game handler) is verify-only and gains inline test rows only if its plan's measurement of the create-handler path needs a driven row.

**Deferral list.** Client setup, lobby, P2P and native flows that let the host choose the pile and stop gating guests and AI seats → Phase 22 (until it lands every client keeps submitting empty seats, so the default pile plays and the tree is green). The test that a UI-chosen pile reaches the engine end to end is `DEFERRED(phase 22)`. Rules-correctness of cards beyond the 23 and the recorded hand-rebind limits (Bounce, reveal-until, scoped library search) stay as accepted: dropped by the USER, who handles reports as they come. No pile editor/exporter: dropped, no consumer asked.

**Seam notes.** `crates/engine-wasm/src/lib.rs` is edited by Phases 2, 6, 7 at other functions. `deck_loading.rs` is Phase 6's file. `validate_deck_for_format` is shared by every constructed format; the empty rule is gated on the axis so other formats are untouched, and Momir's empty-submission behavior changes (accepted instead of refused) because the axis is the single rule, recorded for the PR body.

**Addenda file.** `addenda/phase-21`

**T1∧T2 check.** 1 unit (one supply mechanic: axis, loader, validator, boundary export). Scope 9 paths (format.rs, deck_loading.rs, deck_validation.rs, lib.rs with its binding, census test, new test, `main.rs` mod line, shared-pile storage test, phase-server verify-only). T1 fails and T2 fails.

---

### Phase 22 — S9b: host-supplied pile (client flows)

**Goal.** Every client flow that starts or joins a Dandan game lets the host pick the pile (the Secret Lair default or one of their saved decks) and submits nothing for any other seat; the client displays the engine's verdict and dispatches. Joining guests and AI seats need no deck.

**Architecture decisions.** The one engine answer is `deck_supply_for_format`, wrapped in `client/src/services/engineRuntime.ts` and read through one hook (the pattern of `useBestOfThreeCeiling`); no client code lists formats or counts cards. For `HostPile` the host's pile selection is "default" or a named saved deck; "default" submits an empty pile-seat deck, a named deck submits its expanded list and its legality is the engine's existing deck-compatibility verdict for the format, which blocks start when `selected_format_compatible === false`. The pile selection must not silently adopt the persisted active deck (that deck may be for another format). `EngineFixed` (Momir) keeps today's flow: no picker. Every empty-seat construction (`buildLocalAiDeckList`, `buildPlayerOnlyDeckList`, the P2P guest and the lobby join) takes the pile from one helper, so no path re-decides it. The lobby's host and join paths stop requiring and format-validating an active deck for a format whose supply is not `PlayerBuilt` (the pile deck is validated, the guest submits empty), and the lobby's AI-seat default-deck requirement is bypassed for such formats. `formatSuppliesDeck` keeps its meaning (the player builds no deck, true for `EngineFixed` and `HostPile`) for the guest, AI-seat and lobby-deck-requirement sites; only the pile choice and the pile-seat deck construction read the engine's `deck_supply` answer, so `HostPile` is distinguished in exactly those places; `EngineFixed` and `PlayerBuilt` behavior is unchanged.

**Scope rule.** `client/src/services/engineRuntime.ts`; `client/src/pages/GameSetupPage.tsx`; `client/src/pages/MultiplayerPage.tsx`; `client/src/components/lobby/HostSetup.tsx`; `client/src/providers/GameProvider.tsx`; `client/src/components/menu/AiOpponentConfig.tsx`; a new `client/src/components/menu/PileSourceChoice.tsx`; `client/src/i18n/locales/en/menu.json` and `client/src/i18n/locales/en/multiplayer.json` with their seven mirrors each (parity-enforced; one T2 path per namespace); the vitest files beside each edited component (`client/src/pages/__tests__/GameSetupPage.test.tsx`, `client/src/components/lobby/__tests__/HostSetup.test.tsx`, a new `client/src/pages/__tests__/MultiplayerPage.dandanPile.test.tsx` (beside the existing `MultiplayerPage.*.test.tsx` rows), a new `client/src/providers/__tests__/GameProvider.dandanPile.test.tsx`). `client/src/adapter/p2p-adapter.ts` is verify-only (the host's guest gate and rebuilt deck payload).

**Deferral list.** None. The pile-source choice is not persisted across sessions: dropped, a default is the safe state.

**Seam notes.** `GameSetupPage.tsx` and `HostSetup.tsx` are edited by Phase 7 (match-type control) at other functions; `engineRuntime.ts` by Phase 7 (ceiling wrapper); `GameProvider.tsx`, `MultiplayerPage.tsx` and `AiOpponentConfig.tsx` by no accepted phase.

**Addenda file.** `addenda/phase-22`

**T1∧T2 check.** 1 unit (one client display of the engine's pile answer across the start flows). Scope 9 non-test paths (`engineRuntime.ts`, `GameSetupPage.tsx`, `MultiplayerPage.tsx`, `HostSetup.tsx`, `GameProvider.tsx`, `AiOpponentConfig.tsx`, `PileSourceChoice.tsx`, the two locale groups); the four test files add 13 only if counted. T1 fails, so the conjunction cannot fire either way.

---

## Revised decisions

- **Phase 2** (`supplies_fixed_deck: true` for Dandan, "Dandan joins the constructed arm group"): `supplies_fixed_deck` is re-derived from the new `deck_supply` axis (value unchanged, doc corrected); the constructed arm is not revised, it becomes the pile validator. Repaired by Phase 21.
- **Phase 6** (a Dandan deck load ignores the submitted deck and loads the fixed pile): the pile seat's non-empty submission wins, the default list is the empty-submission fallback. Repaired by Phase 21.
- **Phase 2's client claim** ("the setup flow for a fixed-deck format is gated on `formatSuppliesDeck`") and the Phase 6/7 lobby assumption that a fixed-deck format needs no deck: for `HostPile` the host's deck is optional and validated, guests and AI seats submit nothing. Repaired by Phase 22.
- **Phase 5** (word-pair `depends_on` arm, `active_text_substitutions` as a pre-ordered list): replaced by recipient-text selection. Repaired by Phase 19.
- **Phase 5** (carrier table classifies every `ManaColorSpent.color` as a word): classification becomes exact over the parsers' emissions, symbolic spend conditions stay unchanged under a color-word change. Repaired by Phase 20.
- **Phase 4b** (loop-only fallback): stands; its CR 613.8b loop rule is reused as the shared select-next kernel and the Text layer no longer routes through the fixed graph.
- **Phase 9** (the mechanical `library_of` swap in `card_name_choice_candidates`) and **Phase 17** (pool resolution through `deck_pool_of`): the swap is replaced by the pile's registered pool for a shared library; Phase 17's resolver is consumed unchanged. Repaired by Phase 18.
- **Phase 3** (force-keep re-gate): no revision; it reads the format axis, not the list.
- **Phase 7** (best-of-three ceiling): no revision; a custom pile is still Bo1.

No decision breaks beyond a later-phase repair, so no phase is reworked in place and no fork is needed.
