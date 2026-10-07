# Charter review r3 (delta) — re-charter Phases 18–22, charter mode

Reviewer model: claude-sonnet-5-5. Inputs: `charter-18-r3.diff` against the pre-r3 copy, `phase-charter-18.md` at W HEAD `303d76fa69`, r1/r2 findings, `phase-claims-18-22.md`.

Verdict: CLEAN. 0 decision findings, 0 corrections, 0 machinery. The delta had zero design findings, so the one whole-artifact pass ran; it also found none.

## Delta (two hunks: Phase 21 scope rule, Phase 22 architecture decisions)

- **F2 closed.** Phase 21 scope rule now reads "...only if its plan's measurement of the create-handler path needs a driven row". `grep -n 'Claims to establish\|Verification plan\|claim below\|blocks below\|Claims below'` over the charter: 0 hits (control: `grep -c 'Scope rule'` returns 5, so the instrument fires). No reference to a deleted block remains.
- **F1 closed; the repair is a deletion of the second-predicate claim, not an addition.** The new sentence keeps `formatSuppliesDeck` (registry-mirrored `supplies_fixed_deck`) with its meaning and routes only the pile choice and pile-seat deck construction through `deck_supply`. Coherent with Phase 21 decision 2 (`supplies_fixed_deck()` re-derived as `deck_supply() != PlayerBuilt`, so true for Dandan and Momir, value unchanged; `GameFormat::supplies_fixed_deck` at `types/format.rs` confirmed as the current exhaustive match, Momir|Dandan true).
  Site check, `grep -rn formatSuppliesDeck client/src` (non-test): definition `data/formatRegistry.ts`; readers `AiOpponentConfig.tsx` (1), `GameSetupPage.tsx` (2), `GameProvider.tsx` (5, of which the `buildLocalAiDeckList` branch is the pile-seat construction and is async, so it can await the `engineRuntime.ts` wrapper). The other readers are the "no deck required" gates, which stay on the coarse flag. All three reader files and `engineRuntime.ts` are in Phase 22's scope rule; `formatRegistry.ts` needs no edit (a doc-comment-only change is admitted by the scope rule).
- **Decision 1/4 of Phase 21 with a seat-less validator.** Decision 1 matches `dandan_fixed_deck_payload` (takes `pile_seat`, returns `PlayerDeckPayload::default()` for every other seat); `canonical_seat()` exists in `types/game_state.rs`. Decision 3 now says nothing about seats; decision 4's `validate_deck_list_seats` loop calls the one authority per seat. The "Momir empty submission accepted instead of refused" seam note is true at base (`quick_momir_check` requires exactly 60, so empty is refused).
- **Neighbors.** T-items from r1/r2 all still applied (re-verified by diff against the pre-r3 copy: only the two hunks changed). Phase 22's `deck_supply` routing: no leftover "every reader" sentence in the charter; `phase-claims-18-22.md` carries no sentence contradicting the new one (its stale "measured at charter" strings are in the claims file, not the charter).

## Whole-artifact pass (premises, scope, seams)

- All 25 literal scope paths exist at W HEAD; new paths (`dandan_custom_pile.rs`, `PileSourceChoice.tsx`) are absent as intended; `GameProvider.*.test.tsx`/`MultiplayerPage.*.test.tsx` families exist for the "beside the existing" claims; locales: `en` plus seven mirrors.
- Code-state premises a decision rests on, re-checked: `shared_zone_holder` (`pub(crate)`, same crate as `candidates.rs`); `parse_word_mana_color_spent_condition` doc says it is shadowed by the `QuantityCheck` bridge and the symbolic form is live; `bestOfThreeCeilingForFormat` wasm export and `engineRuntime.ts` wrapper exist; CR 613.8a/b/c, 612.2, 407.3 grep-verified and subjects match their claims.
- LOC sum 110+450+220+280+390 = 1,450 matches the stated estimate. No perf-gate unit (`grep -i perf`: 0 hits); "no ban list / no copy limit" appear only as non-goals. No file inventory; scope entries are rules.
- Seams: 18→19→20→21→22 order and green-tree notes unchanged; Phase 21's `DEFERRED(phase 22)` row has a named landing phase.

## Constraints for the dispatch (exit-round material, no round)

- Phase 22 plan: sites outside the pile choice and pile-seat construction keep `formatSuppliesDeck`, including the host's own "no active deck required" gates in `GameSetupPage` (start handler and `deckBlockedForSelectedFormat`) and the `GameProvider` local/AI/native sites; the sentence's category list names only guest, AI-seat and lobby sites. At base `MultiplayerPage`/`HostSetup` do not read `formatSuppliesDeck` at all (claims file), so Phase 22 adds that read there.
- Phase 22 plan: the three existing `GameProvider.*.test.tsx` files mock `formatSuppliesDeck: () => false`; if the plan's helper is reached on that path, their `engineRuntime` mocks need the new export (not compiler-forced; measure at step 0).
- Carried from r2: Phase 18 text "registered pool names"/"registered-pool order" should read "pool order" (source is `current_main`); Phase 22's plan must carry Phase 21's `DEFERRED(phase 22)` end-to-end row.

## Pre-existing

None found.

## Probe provenance

`/usr/bin/grep` and `sed` over W HEAD (`crates/engine/src`, `client/src`, charter and claims files); no cargo run (delta is text plus premise-only).
