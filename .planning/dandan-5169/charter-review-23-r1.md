# Charter review r1 — Phases 23–25 (review-engine-plan, charter mode)

MODEL: claude-opus-5-5. Base `91c34cdc96` (feat/dandan-custom-piles).

**Verdict: 1 decision finding (behavior), 0 machinery; corrections C1–C7 for the orchestrator.**

## Probes (re-run; scratch tree `dandan-run/scratch/c23r-tree` = `git archive 91c34cdc96` + the planner's probe sources, `FORGE_TEST_FULL_DB=1`, target `target-scratch-c23r`; deleted after)

- F1, F2 (`c23_probe::*`, 11 tests): 10 FAIL / 1 PASS (the Standard own-card control). Every failure fails on the owner/offer assertion with its reach-guard satisfied: Future Sight / Magus / Citadel `stack-owner=P0`, permanent `owner=P0`, Unsubstantiate → `hand=Some(P0)`; spell on stack bounced → P0; Svella during-resolution cast `zone=Stack owner=P0`; land played from pile top `owner=P0`. Think Twice / Deep Analysis / Faithless Looting `shared own=true cross=false | standard own=true cross=false`; real flashback cast `InvalidAction("Card is not in a castable zone")`; Lurrus own=true cross=false; Emry own=true cross=false (activation resolved, so targeting admitted the cross-owner card; the cast offer refused it).
- F3 (`c23_f3_custom_piles_leave_the_mulligan`): 80 Island, 79+1 Opt, 78+2 Opt stuck 2/2 seeds on both routes (`max_free_reveals` 60/60/59); varied 30/30/20 leaves (0 free reveals). Charter's reproduction claims hold.
- Extra probe `c23r_depletion_after_first_keep` (deterministic route, 400 steps, 78 Island + 2 Opt): seed0 `step34 P0 Keep hand_opts=2 lib_opts_after=0`, seed1 `step6 …`, seed2 `step64 …` (same shape); all three end `left=false pending=[P1]` — P1 redraws for the remaining steps from a library with no nonland. (Run stopped after 3 seeds for time; 77+3 rows not run.)
- Cards verified verbatim from `client/public/card-data.json` (Future Sight, Magus of the Future, Bolas's Citadel, Unsubstantiate, Svella, Think Twice, Deep Analysis, Faithless Looting, Lurrus, Emry, Crucible of Worlds, Sol Ring). CR 103.5, 108.3, 108.4a, 109.5, 305.1, 400.3, 404.1, 601.2a, 602.2, 702.34a, 702.66a, 702.81a, 702.138a grepped; each subject matches its claim.

## Decision finding

### D1 [behavior] Phase 25 decision 1 computes "unimprovable" over the wrong population; the deterministic residual is larger than decision 4 states

Decision 1 answers improvability from `deck_pool_of(seat).registered_main` ("any hand dealt from this seat's pile"). A free reveal reshuffles the seat's hand into the **shared library**, which during the mulligan excludes every card the other seat holds. Once the other seat keeps, the redraw population is `library_of(seat) ∪ hand(seat)` = registered list minus the kept hand, and the registered-list answer is no longer a proof about the action taken.

Measured (above): on 78 Island + 2 Opt, P0's AI keeps exactly when it holds both Opts (the only non-qualifying hand), leaving P1 a pool with zero nonlands. Decision 1 still answers "improvable" for P1 (registered 2 ≥ `min_nonlands`), so P1 takes the free reveal at every step — a **deterministic** driver-cap fault, not the "≈0.7% per redraw" residual decision 4 describes. The same holds for any pile whose minority category has k cards, k ≥ 2: the first seat's AI keeps only once it holds ≥ 2 of them, leaving ≤ k−2 for the other; whenever that is < the threshold, the second seat is stuck forever under decision 1. This is the case the maintainer named ("including after the other player keeps") and it is reachable from the first AI keep, so it is in finding 3's class, not the near-degenerate fork.

Required revision (a decision, for the planner): state the population the engine fact quantifies over.
- **Recommended:** the redraw population (`library_of(seat)` ∪ the seat's hand — exactly what the free reveal shuffles and draws from), with the same count rule (≥ `min_lands` lands, ≥ `min_nonlands` nonlands, sum ≤ starting hand size). Provably correct for the action taken, covers 80 / ≤1-nonland piles and the post-keep depletion, still stateless. State in the decision that the engine reads hidden-zone composition and hands the AI one boolean; it reveals only that the redraw is futile.
- Alternative: keep the public registered list and restate decision 4's residual to include the deterministic post-keep case (then the PR ships a known deterministic fault on admitted piles; not recommended).
- Verification consequence (plan level): a row where the other seat has kept the minority cards and the futile seat leaves the mulligan; red under the registered-list rule.

## Corrections (orchestrator applies; no round)

- **C1 (chartered.md "No code-state assertions" / "Architecture only").** Delete each phase's "Claims to establish (measured)" and "Verification plan" sections and the "Reproduction at the base" paragraph's result sentences from the charter; the phase plans carry and measure them. Measured above: every reproduction statement holds, so no decision moves.
- **C2 Phase 23 decision 2.** Old: "`graveyard_keyword_routes_open` and `mayhem_castable_from_graveyard` (their consumers, method enumeration and preparation, read the predicate and need no edit)". New: "`graveyard_keyword_routes_open` and `mayhem_castable_from_graveyard` (their consumers, method enumeration and preparation, read only the predicate; `graveyard_keyword_routes_open(obj, player)` takes no `GameState` today, so its two call sites gain the argument)".
- **C3 Phase 23 T1∧T2.** Old: "Scope 9 paths (7 source, new test, `main.rs`), fixture excluded." New: "Scope 10 paths (8 source: `game_state.rs`, `casting.rs`, `statics.rs`, `game_object.rs`, `casting_costs.rs`, `interaction.rs`, `cast_from_zone.rs`, `zone_pipeline.rs`; new test; `main.rs`), 11 if the delve test is compiler-forced; fixture excluded."
- **C4 Phase 24 seam notes.** Old: "`zone_pipeline.rs` was edited by Phases 11 and 23 (receiver function here, `hand_entry_receiver` delegation there)". New: "`zone_pipeline.rs` was edited by Phases 11 and 23; Phase 24 generalizes the same `hand_entry_receiver` whose shared-container test Phase 23 delegated".
- **C5 Phase 24 decision 1 rationale contradicts its own land arm.** Old: "(every other Battlefield producer leaves `performed_by` empty, so a permanent that nobody cast keeps its owner — the format text names the caster, and silence about uncast entries is not extended)". New: "(every other Battlefield producer leaves `performed_by` empty, so a permanent nobody cast or played keeps its owner; the land arm follows issue #5169's owner contract, which binds ownership at cast finalization and at `handle_play_land`)".
- **C6 "Why three phases".** Old: "the tree is green between them (no phase's tests depend on another's code)". New: "the tree is green between them; Phase 24's shared-graveyard land-play row (Crucible) also relies on Phase 23's membership authority, which the order provides".
- **C7 Phase 25 decision 4** follows D1's revision: drop "≈0.7% per redraw" as the whole residual; the residual is only piles whose redraw population still holds the thresholds at a low hypergeometric rate.

## Checklist results

- **Seams / order / green tree:** 23 → 24 → 25 is dependency-respecting (24's Crucible row needs 23; 25 is disjoint). No phase changes a serialized shape: `HandEntryOwnership` derives no serde (`types/format.rs`), `GraveyardPermissionPool` changes only a method signature (client mentions are comments), `performed_by`/`rebound_from` already serialize. Independent of Phase 22 (paths disjoint except `tests/integration/main.rs` and the fixture).
- **T1∧T2:** none fires (23: 1 unit, 10–11 paths; 24: 1 unit, 11 paths — recount matches; 25: 1 unit, 5 paths).
- **Revised decisions repaired by later phases, not in place:** Phases 2/11, 8, 13, 17, 21 listed with repairing phase; Phase 11 quote verified verbatim in `phase-charter`. No other accepted decision found broken (`grep` over `phase-charter`, `phase-charter-18.md`, `summaries.md`, addenda).
- **Cause-level design:** F2 = one storage-layer membership method (`zone_storage_seat`-based), every graveyard cast gate delegating — matches review 2's "authority = game_state.rs shared-zone storage resolution". F1 = the existing `hand_entry_receiver` generalized, carried by the existing `EntryMods.performed_by` (sole Stack constructor `casting_to_stack`, two production callers: the commit and the evoke preview — `git grep 'casting_to_stack('`), installed by `install_rebound_owner`. No call-site patching.
- **Review gates:** F2 — `statics.rs` OwnGraveyard pool, the all-OwnGraveyard scan exit, keyword-route predicate and both consumers, Mayhem, Disturb, storage authority, personal-graveyard negative controls: all in scope. F1 — `effects/bounce.rs` (verify-only, reasoned), normal + during-resolution casts (one commit site), serialized replay controls: in scope. Unjournaled Stack leg is a stated, reasoned limit: production replay (`engine-wasm` `replay_seek_js`) re-applies actions, and `apply_resolved_zone_change` has no production caller outside `resolve_and_apply_zone_change`. F3 — local, both routes, free reveal stays legal, no persisted budget: in scope, subject to D1.
- **Kept-as-ownership drops (decision 3):** reasoned; CR 602.2 names controller-or-owner, and the three AI candidate sites the census prints are activation loops. Census predicate re-run plus a wider one (reversed comparisons, ±8 lines): no additional cast-permission gate; `targets_commit_crime` (CR 700.13) is crime classification, not a cast gate.

## USER forks — recommendations

1. **Near-degenerate piles (78 Islands + 2) vs "provably unimprovable":** does not need the USER. The maintainer's words decide it — "provably unimprovable pile compositions" and "a general persisted redraw budget would need structural review rather than a local patch"; the only rule that guarantees termination for an improvable pool is that budget. Ship the provable rule (on the population D1 fixes), and disclose the residual (pools that can still clear at a low rate) with its measured rate in the PR body for the maintainer.
2. **Owner of a played land:** does not need the USER. Issue #5169's owner contract already decided it (§2.3 and the Identity/Provenance contract bind ownership at `finalize_cast` **and** `handle_play_land`; "forgetting the land path leaves 20/80 deck cards with the game-start canonical owner"); the canonical-seat owner is an engine artifact, not a rules answer. Keep Phase 24's land arm, citing that contract (C5).

## Pre-existing

None established (no hunt beyond the change's claims).
