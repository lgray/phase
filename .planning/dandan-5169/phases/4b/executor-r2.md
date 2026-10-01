# Executor r2 — phase 4b (effect-level CR 613.8b loop fallback)

Mode: implementation/fix, phase mode. MODEL: claude-sonnet-5-5
BASE_SHA(run)=b9ba9360; START_SHA=67cd221ea26fd73a3fa26328ef14cf8f688243d4; IMPLEMENTATION_WORKTREE=/home/user/phase
Start: clean, HEAD==START_SHA. End: HEAD unchanged, nothing staged, delta = exactly the 3 scope paths (M layers.rs, M main.rs, ?? loop_only_dependency_fallback.rs). PREPARATORY evidence only.

## Diff summary
- crates/engine/src/game/layers.rs: `order_with_dependencies` now builds the graph over effects (`effect_nodes`, keyed by `continuous_effect_group_key`), deduplicated node edges (BTreeSet), `dependency_application_order` (Kahn plus loop-only stall handling via `earliest_source_loop`, petgraph tarjan_scc), expands nodes to entries contiguously; false "unreachable today"/"coarser" comments removed; `mutually_copying_*` doc sentence rewritten; unit tests H1-H7, G1 (`effect_nodes_group_entries_by_effect_identity`), G2 (`a_copy_exception_stays_with_its_own_copy_effect_in_the_ordering`).
- tests/integration/loop_only_dependency_fallback.rs (new): R1-R4. main.rs: one mod line.
- Started from candidate-r1.patch (git apply on clean tree), then added effect grouping.

## Red/green (outputs in this dir)
- R1 red at base: red-r1b.txt (round 1, same base) and red-r2.txt (this round: ordering short-circuited to whole-bucket timestamp order, base-equivalent for loops): R1 `curse_older_than_the_loop_still_applies_after_it` FAIL, R2-R4 PASS. Restored.
- N1 mutation (`effect_nodes` returns singletons), mutation-r2.txt: G1, G2 and E1 (`a_set_name_exception_survives_the_room_name_derivation`) all FAIL. Restored (verified no `if true` left).
- Green: green-r2.txt `layers::` + new integration file: 371 passed. 
- Full `cargo nextest run -p phase-engine --no-fail-fast` (full-r2.txt): 31315 run, 31315 passed, 12 skipped. Zero changed outcomes; named tests (room-name, double_urborg, two_ragosts, mutually_copying, same_static_modifications) green.
- fmt applied to scope .rs paths; `cargo clippy -p phase-engine --all-targets -- -D warnings`: clean (EXIT=0, clippy-r2.txt).
- Claims check: grep for "unreachable today|This implementation is coarser|falls back to timestamp order" prints nothing; `CR 613.8b` appears 6 times in the diff. CR-annotation gate: zero UNVERIFIED.

## Coverage map
R1 (stall branch via evaluate_layers; revert flips R1), R2/R3/R4 negatives/guards with reach-guards in `assert_reached`; G1 pins identity key; G2 through `order_with_dependencies` pins copy exception adjacency; E1 existing test via evaluate_layers. Parser gate, new-field sweep, serde: not applicable (no parser file, no field/variant/serialized surface).

## Judgement calls
- H7 has the same graph as H2 (the plan's node-level shape); kept as its own named test.
- Red-at-base for this round used a whole-bucket short-circuit mutation (cannot checkout base); round-1 red-r1b.txt is the true base run.

## Stop-and-return items
- None for the code. Note: `python3 scripts/gen-test-fixture.py --check` reports the fixture stale (76 cards not covered, e.g. abandon hope, adaptive automaton). Pre-existing and unrelated to this phase's five cards (they load, tests pass); fixture not touched (not in SCOPE_PATHS).

## Risks
Acyclic ordering can change only for multi-entry effects (plan §5); full suite showed no change.
