# Charter review r2 — Phases 23–24 (review-engine-plan, charter mode, delta round)

MODEL: claude-opus-5-5. Delta = `diff phase-charter-23.r1.md phase-charter-23.md`; engine code read at W HEAD `b892a663e9` (engine unchanged from `91c34cdc96`).

**Verdict: 1 decision finding (behavior), 0 text, 0 machinery. No corrections.**

## Decision finding

### D2 [behavior] Phase 24 decision 1: the redraw population omits the hands of seats that have already declared a redraw

Decision 1 lists the population as `library_of(seat)`, the seat's hand and "the hands of seats whose Declare entry is still pending". Its own rationale is that under a shared library "the round closes with every declarer's hand returning together", but the term names only seats still deciding. Seats already in `WaitingFor::MulliganDecision.declared` keep their hands until the close, and those hands are certain to go back into the library before this seat's redraw is dealt.

Evidence:
- `mulligan.rs::handle_mulligan_decision`, Simultaneous arms of `Mulligan`/`FreeReveal`: `pending.remove(idx); declared.push(..)`, with the comment "the hand stays until every player has declared".
- `close_declare_round` runs `return_hand_to_library` for every declaration, then shuffles once per holder, then deals.
- Existing tests, re-run here (`cargo nextest run -p phase-engine --lib`, target-dandan): `dandan_mulligan_is_held_until_every_player_has_declared` asserts `declared=[p1]`, `pending=[p0]` with p1's hand untouched. `dandan_close_returns_every_hand_before_the_deal_and_deals_active_player_first` has p1 declare first and asserts 14 Hand→Library moves before any deal. `dandan_round_waits_for_owed_bottoms_before_redrawing` asserts that a BottomCards seat's bottomed card is in the library before the other seat redraws. Result: see Probes.

Reachable counterexample: Dandan, `WhenHandLacks { min_lands: 2, min_nonlands: 2 }` (`format.rs`), pile 78 Island + 2 Opt. P1 holds 1 Opt, is offered the free reveal and declares it, so it moves to `declared`. P0 holds 7 Islands; the library holds 1 Opt. Under decision 1 as written, P0's population is the library plus P0's hand plus no pending Declare seat, so it holds 1 nonland and is declared futile. The real close population holds both Opts, so P0's redraw is improvable. This contradicts decision 1's own definition ("whether a free-reveal redraw by this seat **can** produce a hand…"). Decision 2's futile branch can then choose a regular Mulligan, which redraws from the same population but costs a bottom and forfeits further free reveals, or keeps a 7-land hand.

**Answer to the brief's question (2).**
- **The review's exact rule (`library_of(seat)` ∪ own hand) is a subset of the real population.** So its "improvable" is always true, and it cannot cause a non-residual loop. Its "futile", however, is not a proof: it misfires whenever another seat's hand will return at the close. It also reads more hidden information, because while the other seat is undecided the answer depends on that seat's hand.
- **The planner's extra term is needed and points the right way.** Futility must be computed over an upper bound of what can be in the library at the deal. It is still incomplete without the declared seats.
- **The extra term is not overbroad in a harmful sense.** If an undecided seat then keeps, at most one free reveal draws from a smaller population. On the next evaluation that seat has left `pending` and the answer is exact, so termination holds. Whole-hand inclusion of a BottomCards seat is a sound over-approximation: only `count` cards go back, but the over-count is bounded the same way.
- **Stateless:** yes. It reads live zones plus the live `pending`/`declared` of `WaitingFor`.
- **Leak:** with the corrected term, during the mulligan the population is every card except the hands of seats that have finished the round (and Serum Powder exile, which is public). So the boolean depends only on finished seats' kept hands. That is the minimum any provable rule needs after a keep. The registered-list rule leaks nothing but faults deterministically (r1 D1), and a persisted budget is excluded by the maintainer. The bit equals what repeated legal redraws converge to. This is consistent with the r1 acceptance, and the charter states it.

**Correct population:** `library_of(seat)`, the seat's own hand, and the hands of every other seat still in the round that draws from the same library: those in `declared` (their hands return at the close) and those in `pending` (a Declare entry may still redraw; a BottomCards entry bottoms from its hand before the close).

Required revision (decision 1 wording, for the planner):
- Old: "that is `library_of(seat)`, the seat's hand and, because under a shared library the round closes with every declarer's hand returning together (`mulligan_timing`, `close_declare_round`), the hands of seats whose Declare entry is still pending."
- New: "that is `library_of(seat)`, the seat's hand and, because under a shared library the round closes with every declarer's hand returning together (`mulligan_timing`, `close_declare_round`), the hands of every other seat still in the round that draws from the same library: each seat in `declared` (its hand returns at the close) and each seat in `pending` (a Declare entry may still redraw; a BottomCards entry bottoms from its hand before the close). This is an upper bound, so futile is a proof; an over-count lasts at most one round, because a seat that keeps leaves the round."

Decision 5 also gains this obligation: a case where the other seat has **declared** a free reveal holding a minority card that the library lacks, and this seat is not declared futile (red under the term as written).

## Checks

1. **Fold coverage.** Every gate, witness and control from r1's Phases 23 and 24 survives in decisions 2–3 and 10. Gates: keyword route and both consumers, OwnGraveyard pool, all-OwnGraveyard scan exit, timed permission, Mayhem, Disturb, delve, graveyard land play. Producers: Future Sight, Magus, Citadel, a stack bounce, a cast during resolution, a land from the pile top and from the shared graveyard. Controls: Control Magic, replay of the journaled Battlefield→Hand command and the land play's `rebound_from`, the hostile effect-put permanent, personal-zone and per-seat negatives, the explicit-ownership refusal, and the Phase 8 test rewrite. Against review-9696-1/-2: owner-hand return of spell and permanent, normal and during-resolution casts, per-seat and replay controls, offers, keyword/static/timed admission, method selection/preparation, Mayhem/Disturb, and personal Hand/Command/Exile kept are all present. Only plan-level items were dropped (the Lurrus MV3 sibling and per-gate mutation). Crucible is `GraveyardCastPermission { play_mode: Play }` (card-data.json), and its land-play legality lives in `casting.rs` (`CardPlayMode::Play` arms), so it is inside the scope rule. The coverage contract names covered classes, not measured outcomes, and the orchestrator's brief required these to be kept. It is acceptable as the phase's scope obligation.
2. **D1:** closed in substance (redraw population, same count rule, stateless, one boolean, leak stated; C7 residual rewritten). Its extra term is incomplete; see D2.
3. **C1–C7:** applied. "Claims to establish", "Verification plan" and "Reproduction at the base" are gone. C2 and C5 texts are verbatim. C3 is superseded by the fold's recount (16 paths: 13 source + census test + new test + `main.rs`, 17 with the delve test; T2 threshold ≥13 per `engine-implementer/SKILL.md`, so T2 fires and T1 fails). C4 is folded into Phase 23's seam note, and C6 is made moot by the fold (the "Why two phases" text names the Crucible dependency). No measured figures, no disposition ledger. The remaining code-state clauses are C2's supplied text and r1-reviewed content.
4. **Renumbering:** consistent. Title, Phase list, phase headers, "Why two phases", Revised decisions (11, 2/11, 8, 17 → 23; 13, 21 → 24), seam notes and addenda names (`addenda/phase-23`, `addenda/phase-24`, neither file exists yet). `grep -n 25` finds no stale reference. `mulligan_timing` and `close_declare_round` exist as named.
5. **USER forks as decisions:** Phase 23 decision 11 (played land owned by the caster, per the #5169 contract) and Phase 24 decision 4 (provable rule, residual disclosed in the PR body) are both stated and both still sound.

## Probes

The run of the three existing mulligan tests had not finished compiling when this review was handed back (log: `dandan-run/scratch/c23r2-probe.log`). D2's premise therefore rests on the code paths cited and on what those committed tests assert, read but not run. The orchestrator may read the log and confirm `done rc=0`.

## Pre-existing

None established.
