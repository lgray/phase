# Phase 4b implementation review, round 2 (fresh whole-phase)

MODEL: claude-sonnet-5-5
Range: 67cd221e..695933e5 (layers.rs, loop_only_dependency_fallback.rs, integration/main.rs). Read-only; no cargo run. Replica: scratchpad `sim2.py` (python port of the SCC edge drop + lowest-rank Kahn pass).
Verdict: REVISE (1 MED behavior).

## Algorithm re-derivation (CR 613.7 / 613.8a-c, grepped at docs/MagicCompRules.txt:3013-3048)
- Loop + independent newer: [0,1,2] correct. Loop + independent older: [0,1,2] correct (unit rows cover both).
- Dependent of a loop member (older or newer): waits until just after the member(s) it depends on, applies at its rank; [1,0,2] / [1,2,0] correct under literal 613.8b/613.8c; documented.
- Two independent loops: timestamp order, correct.
- Same-effect entries contiguous: `effect_nodes` groups by `ContinuousEffectGroupKey`, flat_map keeps written order (613.7a); in-degree cannot underflow (edges deduped in a BTreeSet, same-node pairs skipped, intra-SCC edges decremented exactly once). Correct; mutation-r3 shows G1/G2/E1 bite.
- Loop member depending on an outside effect: see F3.

## Findings

### F3 MED behavior: a loop member that depends on a NEWER outside effect applies after its loop sibling, so the loop is not applied in timestamp order
Intra-SCC edges are dropped outright, so the loop members carry no ordering constraint among themselves. A member that still waits on an outside effect is therefore overtaken by its younger loop sibling.
Replica results (nodes by timestamp rank; (d,o) = d depends on o):
- loop {0,1}, 0 depends on newer outside 2: `[1, 2, 0]`. Loop member 1 applies before 0. CR 613.8b: "the effects in the dependency loop are applied in timestamp order" (0 before 1). Expected `[2, 0, 1]` (0 waits for 2, per plan §5 "A loop effect that also depends on an effect outside the loop waits for that effect"; its sibling stays behind it).
- loops {0,1} and {2,3}, 0 depends on 2: `[1, 2, 0, 3]`; loop {0,1} is applied 1 then 0. Expected `[2, 0, 1, 3]`... at minimum 0 before 1.
- Mirror (1 depends on 2): `[0, 2, 1]` is fine, so the defect is asymmetric and invisible to the existing rows: `dependency_order_loop_member_waits_for_upstream_node` uses an OLDER outside node, and `dependency_order_stalls_recur_across_loops` has the cross-loop dependent as the newer member (`(2,0)`), so neither exercises an older loop member waiting on a newer outside effect.
Base returned whole-bucket timestamp order here, and plan r2 design (stall + edge deletion, Kahn by rank) gave `[2, 0, 1]`; the r3 SCC drop regressed this shape while fixing F1.
Reachability: any loop (two type-adders with typed filters, two CopyValues) where one member also reads an effect from a newer-timestamp source; the class is the same one the phase targets.
Suggested fix (machinery, no new type): instead of discarding intra-component edges, replace them by a chain in timestamp rank (each member of a component gains one edge from the next-lower-ranked member, in_degree adjusted), so loop members keep timestamp order among themselves while outside dependencies and dependents behave as now. Add unit rows `order_of(3, [(0,1),(1,0),(0,2)]) == [2,0,1]` and the two-loop variant.

## F1 / F2 closure (from impl-review r1)
- F1 (independent effect newer than a loop applied first): CLOSED. SCC drop makes loop members rank like independent effects; unit rows and integration row `newer_independent_effect_applies_after_the_loop_members` (red-r3 shows red at the old bytes, green-r3 green) cover it. F3 is a distinct residual in the same function, not a re-raise.
- F2 (LOW, doc wording): CLOSED; the doc comments now state the SCC rule in one sentence and the CR 613.8b tracking comment is consistent.

Verification evidence reviewed: full-r3 31317/31317 on committed bytes; fmt OK; completion-clippy.txt was still "Checking" when read (not judged).
