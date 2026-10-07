# Charter review r2 (delta) — re-charter Phases 18–22, charter mode

Reviewer model: claude-sonnet-5-5. Inputs: `charter-18-r2.diff` against `phase-charter-18.md` at W HEAD `303d76fa69`; r1 findings; `phase-claims-18-22.md`.

Verdict: 1 decision finding (`behavior`, introduced by repair T7), 1 correction (`text`, left by repair T6), 0 `machinery`. The delta has a design finding, so no whole-artifact pass ran; this is a design round for the one finding.

## Closed, measured

- **B1 closed.** The refusal sentence is gone and nothing else in the charter leans on it (`grep -o -i 'refus\|ignored\|other seat\|non-pile'` over the charter: remaining hits are the ante refusal, Momir seam note, decision 1's "every other seat's submission is ignored", Phase 22's "submits nothing for any other seat"). Decision 1 matches the loader: `dandan_fixed_deck_payload` already takes `pile_seat` and returns `PlayerDeckPayload::default()` for every other seat; `canonical_seat()` exists in `types/game_state.rs`. Decisions 1, 3 and 4 are coherent with a validator that has no seat concept; the claims file keeps E1's "deck on seat 1 is ignored" row as the closing test.
- **T1** applied (`CR 612.2` only). **T2** applied. **T4** applied (`oracle_trigger_tests.rs::extract_adamant_three_red` pins the word-form trigger extractor). **T5** applied (five `Addenda file` lines, 18–22). **T8** applied.
- **T3 applied correctly and its new assertion measured.** `grep -rn current_main crates/engine/src` (control: same grep on `registered_main` returns 25 hits): production writers are `deck_loading.rs` (pool construction) and `match_flow.rs` (sideboard submission, between games); `visibility.rs` only redacts a filtered copy; `phase-ai/src/deck_knowledge.rs::unknown_hidden_pool` reads `deck_pool_of(player).current_main`. So "written only at deck load and between games" holds, and the Phase 18 decision (domain independent of resampling) rests on a true premise. The claims file's Phase 18 claim 3 matches.
- **T6 applied.** `grep -n 'Claims to establish\|Verification plan'` over the charter: 0 hits; no "red at base", "measured at charter" or "below"-row references to deleted blocks remain except F2. Claims file carries every deleted V/E/C row with the two required adjustments (Phase 20 V3 re-labelled to the word-form `OfColor` carrier with the discriminator-branch note; Phase 22 browser run through the existing `run` skill, no new driver).
- No file inventory: scope rules stay path rules; no new code-state assertion beyond T3's (measured true). Phase 22's `deck_supply` routing sites: the readers are `AiOpponentConfig.tsx`, `GameSetupPage.tsx` (2), `GameProvider.tsx` (5); all three files are in Phase 22 scope.

## Findings

**F1 `behavior` — T7 sentence makes `formatSuppliesDeck` a dead second predicate and its definition is outside Phase 22's scope.**
Quote: "Every existing `formatSuppliesDeck` reader decides from the engine's `deck_supply` answer, so no client site keeps a second supply predicate; `EngineFixed` and `PlayerBuilt` behavior is unchanged."
Evidence: `grep -rn formatSuppliesDeck client/src` (non-test): definition `data/formatRegistry.ts:700` (reads the mirrored `default_config.supplies_fixed_deck`; its doc says "single source of truth"), readers `AiOpponentConfig.tsx`, `GameSetupPage.tsx`, `GameProvider.tsx`; the three existing `GameProvider.*.test.tsx` files mock `formatSuppliesDeck: () => false`. Phase 21 decision 2 keeps `supplies_fixed_deck()` and the derived flag unchanged (true for Dandan and Momir). If every reader moves to `deck_supply`, `formatSuppliesDeck` has no caller: the implementer must delete or rewrite it in `formatRegistry.ts` (not in the Phase 22 scope rule; not compiler-forced) and fix three unlisted test mocks; if it stays, a second supply predicate remains, contradicting the sentence. Also, most readers (guest needs no deck, AI seat needs no catalog deck, lobby needs no active deck) only need the coarse "player builds no deck" answer that `formatSuppliesDeck` already gives for `HostPile`; only the pile picker and pile-seat construction need `deck_supply`. GameProvider's sites are inside async builders and effect callbacks, so "decides from the engine answer" there means an awaited `engineRuntime.ts` wrapper, not the hook.
Repair (decision, no scope change): replace the T7 sentence with "`formatSuppliesDeck` keeps its meaning (the player builds no deck, true for `EngineFixed` and `HostPile`) for the guest, AI-seat and lobby-deck-requirement sites; only the pile choice and the pile-seat deck construction read the engine's `deck_supply` answer, so `HostPile` is distinguished in exactly those places; `EngineFixed` and `PlayerBuilt` behavior is unchanged." Closing test: Phase 22 C1 with a `HostPile` format shows the picker while `PlayerBuilt` and `EngineFixed` siblings do not (already in the claims file). The alternative (delete the function and reroute all readers) is valid only if `client/src/data/formatRegistry.ts` and the three `GameProvider.*.test.tsx` mocks join the scope rule.

**F2 `text` — Phase 21 scope rule points at a deleted block.**
Old: "is verify-only and gains inline test rows only if the claim below needs a driven row." New: "is verify-only and gains inline test rows only if its plan's measurement of the create-handler path needs a driven row."

## Constraints for the dispatch (exit-round material, no round)

- Phase 18 decision says "registered pool names" / "registered-pool order" while its source is `current_main`; the plan should say "pool order".
- Phase 22's Goal does not restate that it lands Phase 21's `DEFERRED(phase 22)` end-to-end row; the row lives in the claims file, which the Phase 22 plan must carry.

## Pre-existing

None found in this delta.

## Probe provenance

`/usr/bin/grep -rn` over `crates/engine/src`, `crates/phase-ai/src`, `client/src` at W HEAD; no cargo run (delta is text and premise-only; the one new premise, T3, is a static writer census with a positive control).
