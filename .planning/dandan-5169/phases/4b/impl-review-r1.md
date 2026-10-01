# Phase 4b implementation review, round 1

MODEL: claude-sonnet-5-5
Range: 67cd221e..c7dfc79e (layers.rs, loop_only_dependency_fallback.rs, integration/main.rs). Read-only; no cargo run.
Verdict: REVISE (1 HIGH).

## Findings

### F1 HIGH behavior: an independent effect that is later than a loop applies BEFORE the loop members
`dependency_application_order` (layers.rs, the `while` loop) only breaks a loop when no node has in-degree 0 ("stall"). Until the stall, the Kahn pass keeps taking the lowest-ranked in-degree-0 node, so every effect that does not depend on anything is applied ahead of loop members that are older than it.
Replica of the algorithm (python, scratchpad `sim.py`) for nodes 0 and 1 in a loop and an independent node 2: result `[2, 0, 1]`. Timestamp order, which CR 613.8b prescribes for loop members, with independent effects in timestamp order (CR 613.7a), is `[0, 1, 2]`.
Base returned `sorted` (timestamp order) in this shape, so this is a regression for any layer bucket holding a loop plus a later independent effect (e.g. a later-timestamp layer 7b or layer 4 setter alongside the March/Omen loop). None of the new tests (H1-H7, R1-R4) has an independent node ranked above a loop; the full suite cannot see it.
Fix direction (machinery, simplifies the code): condense the dependency graph once with `tarjan_scc` before the Kahn pass and drop the edges inside every multi-node component. This is equivalent for this state-blind graph (a stalled source component is exactly a static SCC with edges to it only from outside) and removes `earliest_source_loop`, the stall branch, the `debug_assert` and the `expect`. Add unit rows: `order_of(3, &[(0,1),(1,0)])` -> `[0,1,2]`; loop {1,2} with an independent node 0 and node 3 -> `[0,1,2,3]`; plus an integration row (loop plus a later independent effect in the same layer) red at base-of-fix.

### F2 LOW text: comment length / history framing
`dependency_application_order` has a three-paragraph doc and the stall branch has a two-sentence comment; the brief asks for one sentence each. Moot if F1's fix removes the stall branch; otherwise trim.

### F3 LOW behavior (note only, accepted): grouping now also makes same-timestamp, same-source transient effects contiguous
Sort key `(cda, ts, source_id, def_index, mod_index)` can interleave two transient effects of one source and one timestamp (`E1m0, E2m0, E1m1, ...`); `effect_nodes` now emits them contiguously, which matches CR 613.7a/613.6. Full-r2 run (31315 passed) shows no suite impact. No action.

## Checked and clean
- CR 613.8a (effect-level dependency; 613.8a (a)-(c) unchanged in `depends_on`), 613.8b (waits until just after; loop ignored, timestamp order), 613.8c tracking note kept, 613.7a (same-timestamp, contiguous), 707.9b (copy exception stays with its own copy effect): grepped in docs/MagicCompRules.txt, text matches annotations.
- Grouping key: reuses `continuous_effect_group_key` (Static/Transient/GrantedStatic), `None` keys become singleton nodes; G1 pins all four identities.
- Termination of the stall path: edges are deduped (BTreeSet), no self edges, a source component has in-edges only from inside so all its members reach 0 after the inside edges are removed; no `-=` underflow found by reading. Still moot per F1.
- Determinism: `HashMap` in `effect_nodes` is lookup-only; edges via `BTreeSet`; component selection via `min_by_key` on min rank; no iteration-order leak.
- Test discrimination: R1 (`curse_older_than_the_loop_still_applies_after_it`) fails at base (red-r2: `["Rogue","Equipment"]` vs `["Equipment"]`); G1, G2, E1 all fail under the singleton-nodes mutant (mutation-r2.txt). R2-R4 pass at base by design (guards).
- Scope: only the three SCOPE_PATHS changed; CLAUDE.md nom/parser rules not engaged.
- Completion (`completion.txt`) was still running at review time (fmt OK, HEAD matches); full-r2.txt shows 31315 passed.
