# Phase 9 implementation review r2 (phase mode), delta 761f5ff216..1acc74bca6

MODEL: claude-sonnet-5-5. W HEAD == 1acc74bca6, tree clean.

## Findings

**[MED] [text]** F3 (deferral labels) is not closed. `DEFERRED(phase 11)` / `DEFERRED(phase 17)` remain in `phases/9/plan.md` and `executor-r1.md`. Neither charter Phase 11 nor Phase 17 lists determinize/candidates or an other-owned-pile activation, and `addenda/phase-9` and `addenda/phase-17` carry no extension or DROPPED verdict. Measured: `grep -rn "DEFERRED(phase 1[17])" crates` is 0, so no shipped code carries the label; the executor left the plan/report text unedited. No line the implementer writes changes, so this is an exit-round constraint, not a design finding. Replacement (from r1) goes in the addenda file: `DROPPED: no Dandan-list card has a graveyard-zone activated ability, so the obj.owner == player gate is never contested`, and `DROPPED: determinization_samples = 0 in every shipped tier, config-reachable only`.

## Judged clean

- F1 closed: V8 now stages a pile graveyard card (Memory Lapse). Revert probe (see Probes): reverting only the graveyard statement turns V8 red.
- F2 is cause-level: `classify_win_kind` Decking arm compares `GameState::zone_storage_seat(Zone::Library, _)` of the victim and the controller. There is no Dandan or bool guard; per-seat formats resolve each seat to itself, and `None` is identity. Rules: CR 104.3c ("required to draw more cards than are left in their library ... lose") and CR 121.4 / 704.5b (draw from an empty library) attach the loss to the player who draws, not to the player who emptied the pile. Under one pile the controller and the opponent both draw from it and the analysis cannot name a loser, so a certified opponent-loss (Decking) would be false; `Advantage` is the non-win classification. Function role: Decking feeds `certified_bounded_cycle_offer` (Advantage -> `AdvantageOnlyCycle` refusal), the interactive loop bridge and `build_cert`; an unproved win must not certify. The live firewall `live_mandatory_loop_winner` already returns None on any `library_delta < 0` and is unaffected.
- engine.rs (15 lines, 3 call sites) and ability_graph.rs (2 sites): the diff adds only the third argument (`Some(state)` / `None`) plus rustfmt re-wrapping, no other behavior change.
- Stateless `None` callers cannot reach a shared pile: the only non-test callers are `candidate_cycles_from_nodes` (static analysis of card faces, synthetic CONTROLLER/OPPONENT seats, no `GameState`), and `git grep candidate_cycles` shows no consumer outside `analysis/` and its tests.
- New unit test has a positive reach-guard (Standard state and `None` both Decking) and a `detect_loop` end-to-end row.

## Probes

Scratch `git archive` of 1acc74bca6 (not W), own target (reflink copy) and CARGO_HOME. Two mutations in one build: Decking arm back to `*pid != controller`; `candidates.rs` graveyard statement back to `controller.graveyard.iter()`. Result, finished rc=100: `shared_pile_drain_is_advantage_per_seat_drain_is_decking` FAIL ("shared pile, controller PlayerId(0): draining the pile is not decking an opponent"); `v8_card_name_candidates_come_from_the_pile` FAIL ("the pile's graveyard then library names, not the fallback"). Green control at 1acc74bca6 is the executor's `r2-unit.log` / `r2-int.log` (not re-run by me). Scratch copies removed.

## Pre-existing (non-blocking)

None new.
