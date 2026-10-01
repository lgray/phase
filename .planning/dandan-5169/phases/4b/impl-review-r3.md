# Phase 4b implementation review, round 3 (fresh whole-phase)

MODEL: claude-sonnet-5-5
Range: 67cd221e..91ad0dff (layers.rs, loop_only_dependency_fallback.rs, integration/main.rs). Read-only; no cargo run.
Verdict: ACCEPT (0 findings).

## Method
CR 613.7 / 613.8a-c grepped (docs/MagicCompRules.txt:3013-3048). Python replica of `dependency_application_order` (tarjan-equivalent SCC, intra-SCC edge drop, rank-ordered chain per component, lowest-rank Kahn) at scratchpad f.py, plus 200k-case random fuzz (n in 2..7, up to 10 edges) checking: all ordering terminates with every node, every cross-component dependency respected, every component applied in timestamp rank order. Result: 0 violations.

## Re-derivation
- Loop + independent newer / older: [0,1,2] (unit rows cover both).
- Loop member depending on outside newer: {0,1}, (0->2): [2,0,1]; member 1 depending on 2: [0,2,1]. F3 CLOSED (matches unit rows).
- Loop member depending on outside older: [0,1,2] correct.
- Dependent of a loop member (older/newer): applies just after the member(s) it depends on; [0,1,2] / [1,2,0] correct under literal 613.8b/c.
- Two loops with cross dependency (0 on 2, or 1 on 2): [2,0,1,3] / [0,2,1,3]; each loop stays in timestamp order.
- Three-member loop {0,1,2} with member 1 depending on outside 3: [0,3,1,2]; loop order preserved.
- Same-effect contiguity: `effect_nodes` + flat_map keeps written order (613.7a); copy exception stays with its own copy (unit test). Termination: chain edges are intra-component (a total order) and the condensation is a DAG, so the combined graph is acyclic; Kahn always completes. in_degree decrements are exact (edges deduped in a BTreeSet before the call, intra edges removed before chain edges added).
- Dependency: petgraph 0.6 already in crates/engine/Cargo.toml.

## Closure
F1 (r1 HIGH) and F3 (r2 MED) closed; F2 closed earlier. Verification reviewed: full-r4 31320/31320 on committed bytes (rc=0).

## Findings
None.
