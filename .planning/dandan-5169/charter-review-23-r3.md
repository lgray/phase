# Charter review r3 — Phases 23–24 (review-engine-plan, charter mode, delta round)

MODEL: claude-opus-5-5. Delta = `diff phase-charter-23.r2.md phase-charter-23.md`. Changed: Phase 24 decision 1, decision 5, Goal/decision 4/Revised-decisions wording ("population" becomes "pool"), and the scope rule. Engine code read at W HEAD `b892a663e9`.

**Verdict: CLEAN. 0 decision findings. 2 corrections for the orchestrator to apply. D2 is closed. No pool or population finding remains.**

## D2 closed: the shared pool function can be built as stated

- **There is a hand enumeration to factor.** `return_hand_to_library(state, player, events)` opens by collecting `players.find(player).hand` into `hand_ids`, then moves each id with `ZoneMoveRequest::pregame(.., Zone::Library).at_library_position(Bottom)`. Factoring that collect out is a small change. The new function can then serve both the close's return loop and the pool.
- **The holder's library is the right target.** `zone_storage_seat(Zone::Library, seat)` is `shared_zone_holder(zone).unwrap_or(seat)`. Under a shared library that is `canonical_seat()`. `library_of` and `library_of_mut` resolve through it, and so do `shuffle_library_of` and the close's holder dedup. `draw_one` deals from `library_of(player)`, so the deal draws from that same library.
- **Every seat state is covered.** The states come from `WaitingFor::MulliganDecision { pending, declared }` and `MulliganDecisionPhase { Declare, BottomCards { count, then } }`:
  - pending in Declare: listed; it may still redraw or keep.
  - pending in BottomCards with then: Keep: this is a seat that kept but still owes bottoms. It is listed, and the whole hand counts as a sound over-count, because only `count` cards go back.
  - pending in BottomCards with then: UseSerumPowder: listed. Serum Powder exiles cards and draws, so it only ever shrinks the pool.
  - declared: listed. `close_declare_round` returns its hand.
  - kept with nothing owed: `resolve_declare_point` removes it from `pending` and it never enters `declared`, so it is not listed.
- **Nothing can grow the pool.** The pool at decision time is an upper bound on what the close returns. Between the decision and the close, cards leave the pool through a keep or a Powder exile, and only bottoms come back from within the listed hands.
- **No seat-state enumeration.** The fact reads `pending ∪ declared`. Only two fields are read, so it is not a hand-written list of states.
- **The engine behavior the decision relies on was run, not just read.** Both logs end `done rc=0`: `dandan-run/scratch/c23r2-probe.log` from r2 and `c23r3-probe.log` from the r3 planner. In each, all 3 tests pass:
  - `dandan_mulligan_is_held_until_every_player_has_declared`
  - `dandan_close_returns_every_hand_before_the_deal_and_deals_active_player_first`
  - `dandan_round_waits_for_owed_bottoms_before_redrawing`
- **Stateless, and only one boolean leaks.** The fact reads live zones plus the live `WaitingFor`. Only the futility function needs to reach `ai_support`; the pool function stays `pub(crate)`.

## Decision 5 cases: each can be built, and each wrong rule fails at least one

Dandan thresholds are `WhenHandLacks { min_lands: 2, min_nonlands: 2 }` (`format.rs::free_reveal_mulligan`). The pile is 78 Island and 2 Opt. Each case can be built from a real Dandan deal. The table shows each wrong rule's verdict. "red" means the planned assertion fails under that rule.

| Case | True pool | Registered list | Library + own hand | Plus pending seats only |
|---|---|---|---|---|
| A: P1 declared, holds 1 Opt; library holds 1 Opt; P0 holds 7 Islands. Assert not futile. | 78 Island + 2 Opt, improvable | improvable (green) | 1 Opt, futile (**red**) | P1 not pending, futile (**red**) |
| B: P1 pending, holds the Opts; library has none. Assert not futile. | improvable | green | futile (**red**) | green |
| C: P1 kept both Opts; P0 holds 7 Islands. Assert futile, and P0 leaves the mulligan. | 73 Island, futile | improvable (**red**) | green | green |

Every wrong rule fails at least one case: the registered list fails C, library plus own hand fails A and B, and pending-only fails A. Taken together, the three cases tell every wrong rule apart. However, the charter claims "each red under every narrower pool rule", and 5 of the 9 cells contradict that. Also, the registered list is not narrower than the true pool: after a keep it is a superset. See C1.

## Corrections (orchestrator applies; no round)

**C1 [text] Phase 24 decision 5.** As written, this obligation cannot be met: case A cannot be red under the registered-list rule.
- Old: "each red under every narrower pool rule (the registered list; library plus own hand; plus pending seats only)."
- New: "each rejected pool rule red on at least one of them (the registered list on the kept case; library plus own hand on the declared and pending cases; library, own hand and pending seats only on the declared case)."

**C2 [text] Phase 24 decision 5, the tie.** The close runs returns, shuffle and deal inside one `apply`, so "the library contents after the close's returns and before the deal" can only be observed by adding a test hook to production code. The same equality can be checked from outside, because the deal only draws from `library_of`.
- Old: "the pool it names before the close equals the library contents after the close's returns and before the deal."
- New: "the pool it names before the close equals, as a set, the library after the close together with the redrawers' new hands."

## Residual assumptions (not findings)

- "The close calls it with its actual redrawers": the close needs only the hands, not library ∪ hands. The phase plan settles which part the close actually consumes. Either the close iterates the factored per-seat hand enumeration and the pool is the library plus that enumeration, or the close iterates the pool's non-library ids. The tie test in C2 checks the equivalence either way.

## Other checks

- **Architecture-only.** The delta adds no measured outcome and no disposition ledger. Decision 4's "measured rate" is an obligation on the PR body, not a figure stated here.
- **Renumbering intact.** `grep -n "population\|Phase 25\|phase-25"` matches nothing. Control: the same file has 84 lines, and "pool" occurs in decisions 1, 4 and 5 and in the Phase 13 revised decision. Addenda are still named `addenda/phase-23` and `addenda/phase-24`. The scope rule's added text names functions inside `mulligan.rs`, which is already in scope, so the path count stays at 5.

## Pre-existing

None.
