# Phase 4b executor report, round 1 (retry after environment kill)

MODEL: claude-sonnet-5-5
Mode: implementation/fix (phase mode). BASE_SHA(run)=b9ba9360; START_SHA=PHASE_BASE=67cd221ea26fd73a3fa26328ef14cf8f688243d4; IMPLEMENTATION_WORKTREE=/home/user/phase (clean, HEAD==START_SHA at start; HEAD unchanged at end, nothing staged).
All results below are PREPARATORY, not completion evidence.

## STATUS: STOP-AND-RETURN (one pre-existing test changed outcome)

`game::engine::tests::a_set_name_exception_survives_the_room_name_derivation` (crates/engine/src/game/engine_tests.rs, final assertion, message "CR 613.1a: the later ordinary copy resets the exception marker") now FAILS: left "Wrong Turn", right "Bright Hall". Output: phases/4b/full-r1.txt (search the test name). It passed `--lib layers::` (355/355) because it lives in `game::engine::tests`; the full run found it. Per plan F1 a changed outcome is a stop-and-return.

Derivation (read + the failing assertion): the test installs, on one object, transient effect 1 = [CopyValues, SetName "Wrong Turn"] then transient effect 2 = [CopyValues(plain)]. Both CopyValues are Copy-layer entries; `depends_on(_, CopyValues)` is true for any other generator, so CV1 and CV2 form an engine loop. SN1 shares CV1's transient_id (no edge to CV1) but depends on CV2 (different transient id), so the new order is CV1, CV2, SN1 and the exception name wins. The old whole-bucket fallback gave CV1, SN1, CV2 (timestamp), i.e. "Bright Hall", which is the rules-correct result: the name exception is part of copy effect 1 (CR 707.9b) and effect 2 is later. The plan's F3 measurement ("no printed card produces copy-A-rider-then-copy-B on one object") did not foresee this existing synthetic test, and the comment in it says it mirrors production installation (CopyValues then SetName as separate entries).
Smallest fix sites (scope decision needed, both in layers.rs): (a) in `depends_on`, no edge from a same-effect rider... but the rider is on effect 1 and the edge is to effect 2's CopyValues, so the actual fix is: a Copy-layer `SetName`/`Retain*` rider must not depend on another generator's `CopyValues` that is later in timestamp (ordering within the copy layer is pure 613.7 timestamp, since CR 613.8a dependency is not about copy exceptions), i.e. narrow `depends_on` for Copy-layer modifications; or (b) keep the old whole-bucket fallback whenever the stalled loop members are all `CopyValues`. Both widen the phase; (a) is the charter's "depends_on precision" seam the plan review marked out of scope. Orchestrator decides.

## Diff summary (left in the worktree, unstaged; adopted from partial-r1.patch after re-reading against plan and F1-F4)
- crates/engine/src/game/layers.rs: `order_with_dependencies` now delegates to new `dependency_application_order` (Kahn with rank tiebreak; on a stall it deletes the in-loop edges of the earliest source strongly connected component via new `earliest_source_loop`, petgraph `tarjan_scc`, then continues, so dependents of the loop keep waiting); six unit tests (`dependency_order_*`, cases H1-H6: whole loop, chain+loop, dependent on whole loop, recurring stalls and independent loops, loop member waiting for upstream, acyclic plain Kahn); `mutually_copying_permanents_reach_a_layer_one_fixed_point` doc sentence rewritten (F2; `grep "falls back to timestamp order"` prints nothing).
- crates/engine/tests/integration/loop_only_dependency_fallback.rs (new): real cards Curse of Conformity, March of the Machines, Prismatic Omen, Cloak and Dagger; four tests with reach-guards; doc states the March/Omen loop is an engine-model loop (F4).
- crates/engine/tests/integration/main.rs: `mod loop_only_dependency_fallback;`.
- No census instrument (F1). petgraph already a dependency of the engine crate (Cargo.toml `petgraph = "0.6"`).

## Red / green
- Red at base (production layers.rs original, new test file + mod line only): `phases/4b/red-r1b.txt`: 4 run, 3 pass, 1 FAIL `curse_older_than_the_loop_still_applies_after_it`: left ["Rogue","Equipment"], right ["Equipment"]. That is the discriminating assertion; the other three are reach/negative controls that hold both ways (loop_alone..., curse_newer..., curse_on_another_player...).
- Green with the fix: `--lib layers::` 355/355; new integration file 4/4 (`phases/4b/green-r1.txt`).

## PREPARATORY checks
- `cargo fmt --all -- <the three scope paths>`: run, clean.
- `cargo clippy -p phase-engine --all-targets -- -D warnings`: rc=0 (`clippy-r1.txt`).
- Full `cargo nextest run -p phase-engine --no-fail-fast`: 31312 run, 31311 passed, 1 FAILED (above). Changed pre-existing outcomes: exactly that one. (Not run at base; attribution rests on the derivation above, the test was not reported failing by the prior red run.)
- Parser gate: not applicable (no parser path touched). CR gate: CR 613.8, 613.8a, 613.8b all present in docs/MagicCompRules.txt (grep, zero UNVERIFIED).

## Coverage map (abridged)
Claim: a dependency loop reverts only its members to timestamp order. Seam: `dependency_application_order`/`earliest_source_loop`. Entry: `evaluate_layers` -> `order_with_dependencies`. Test: `loop_only_dependency_fallback::curse_older_than_the_loop_still_applies_after_it` (flips Rogue vs none) plus unit H1-H6. Negatives: curse newer, loop alone, curse on other player.
Maintainer matrix: no new stored/bound values, no serde or protocol surface, no new fields; rows N/A except the ordering seam above. Judgement: curse-newer test is green at base too (intentional control).

## Risks
The Copy-layer double-copy-with-name-exception ordering (above) is the open behavior issue; no other test changed.
