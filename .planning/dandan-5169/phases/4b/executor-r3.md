# Phase 4b executor r3 (fix round for impl-review r1 F1/F2)

MODEL: claude-sonnet-5-5
Mode: implementation/fix, phase mode. BASE_SHA(run)=b9ba9360; PHASE_BASE=67cd221e; START_SHA=c7dfc79e0549ed4da33a777afa4e96c1a34f28f5; IMPLEMENTATION_WORKTREE=/home/user/phase.
Worktree record: clean at start with HEAD == START_SHA; at end HEAD == START_SHA, nothing staged, unstaged delta = layers.rs and loop_only_dependency_fallback.rs only. All evidence is PREPARATORY.

## Diff summary
- layers.rs: `dependency_application_order` now builds a petgraph graph, runs `tarjan_scc`, drops every edge inside a component (decrementing in-degree), then runs the plain lowest-rank Kahn pass. Removed `earliest_source_loop`, the stall branch, the `debug_assert`/`expect`. One-sentence doc (F2). Unit rows added: `dependency_order_independent_effects_keep_timestamp_order_around_a_loop` ([0,1,2] for loop {0,1}+later node; [0,1,2,3] for earlier node + loop {1,2} + later node). Existing row `dependency_order_stalls_recur_across_loops` second case changed from [0,3,1,2] to [0,1,2,3] (independent loops {0,3},{1,2} now apply in timestamp order, CR 613.7a/613.8b).
- loop_only_dependency_fallback.rs: new row `newer_independent_effect_applies_after_the_loop_members` (real cards: Plains, March of the Machines, Prismatic Omen, Ultima Origin of Oblivion attacking; Ultima's attack-trigger effect "loses all land types" is a newer ParentTarget effect that depends on neither loop member). Header wording "stall path" -> "ordering".
- main.rs unchanged (mod line already present).

## Red / green
- red-r3.txt: candidate c7dfc79e layers.rs plus only the new unit row: unit row fails (left [2,0,1], right [0,1,2]); integration row fails at the last assertion (5 land types, expected 0), after reach-guards (5 types before attack, blight counter present) pass.
- green-r3.txt: `cargo nextest run -p phase-engine layers loop_only_dependency_fallback`: 673 passed.
- clippy-r3.txt: `cargo clippy -p phase-engine --all-targets -- -D warnings` clean.
- mutation-r3.txt: effect_nodes returning singletons: `a_copy_exception_stays_with_its_own_copy_effect_in_the_ordering` (G2), `effect_nodes_group_entries_by_effect_identity` (G1), `a_set_name_exception_survives_the_room_name_derivation` (E1) fail. Mutation reverted (file byte-identical to pre-mutation copy).
- full-r3.txt (run last, no file touched after start): 31317 passed, 12 skipped, 0 failed.

## Gates
- Maintainer-simulation / coverage map: seam = `dependency_application_order` via `evaluate_layers`; entry = the integration row (Type bucket: loop {March, Omen} + independent transient effect, first branch = SCC edge drop); revert-failing assertion = Plains basic land type count 0; siblings R1-R4, H1-H7, G1-G2, E1 unchanged and green. Binding = effect identity (ContinuousEffectGroupKey), timestamps; no new field/serde surface.
- CR annotation grep of added lines: zero UNVERIFIED (CR 613.7a/613.8a/613.8b/613.8c, 707.9 cited previously, all grepped).
- Parser gate: not applicable (no parser path).

## Judgement calls
- SCC condensation is placed inside `dependency_application_order` (pure, unit-testable) rather than in `order_with_dependencies`.
- Independent-loop unit expectation [0,3,1,2] changed to [0,1,2,3] as a direct consequence of F1's rule (timestamp order among loop members and independent effects alike).
- Red against the candidate was produced by temporarily installing the candidate layers.rs (plus the new unit row) and restoring my version afterwards (both copies in the scratchpad; comparison done with cmp).

## Stop-and-return items: none. Deviations: plan H4 independent-loops expectation changed (above). Risks: Phase 5 hand-off statement "source-component choice and edge removal" now reads "SCC condensation before the Kahn pass".
