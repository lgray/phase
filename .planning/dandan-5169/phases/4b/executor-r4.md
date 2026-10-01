MODEL: claude-sonnet-5-5
Mode implementation/fix, phase 4b r4. START_SHA 695933e5 (HEAD unchanged, no staging), IMPLEMENTATION_WORKTREE /home/user/phase, only crates/engine/src/game/layers.rs modified.

## Diff
`dependency_application_order`: intra-SCC edges are still removed (in_degree decremented), then each multi-member component gets a chain over its members in rank order (one edge each from the next-lower member, in_degree+1); inter-component edges untouched; lowest-rank Kahn pass unchanged. Three new unit tests (6 new rows): older loop member waiting on newer outside node ([2,0,1]; mirror [0,2,1]), cross-loop ({0,1},{2,3}: 0 dep 2 -> [2,0,1,3]; 1 dep 2 -> [0,2,1,3]), dependent of loop member (loop {0,1}, 2 dep 1 -> [0,1,2]; loop {1,2}, 0 dep 2 -> [1,2,0]). All prior rows unchanged. The "dependent 3 on member 1" row was asserted with 3 nodes (node 2 as dependent) plus the older-dependent variant.

## Red/green
- red-r4 (chain disabled via .take(0), r4-red.txt): the newer-outside row fails left [1,2,0] right [2,0,1]; the cross-loop test fails left [1,2,0,3] right [2,0,1,3]. 11 other rows pass.
- green-r4 (r4-green.txt): 13/13 dependency_order tests; green-r4.txt layers + loop_only_dependency_fallback 676/676.
- Mutation (effect_nodes singletons, reverted byte-identical): G1 effect_nodes_group_entries_by_effect_identity, G2 a_copy_exception_stays_with_its_own_copy_effect_in_the_ordering (mutation-r4.txt), E1 a_set_name_exception_survives_the_room_name_derivation (mutation-r4-e1.txt) all fail.
- fmt: `cargo fmt --all -- --check` rc 0. clippy -p phase-engine --all-targets -D warnings: rc 0 (clippy-r4.txt).
- full-r4.txt (run last, no file touched after start): 31320 passed, 12 skipped, 0 failed.

## CR
Added comment cites none new beyond existing CR 613.8b in the function doc (previously grepped, docs/MagicCompRules.txt).

## Judgement/Risks
Chain cannot create a new cycle (edges only intra-component, condensation is a DAG). Evidence is PREPARATORY.
