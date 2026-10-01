# Phase 5 executor r2 (fix round)

MODEL: claude-sonnet-5-5
Mode: implementation/fix (phase mode). BASE_SHA/PHASE_BASE 91ad0dff88426ed6d983e8e060d67cd0a030c926. START_SHA eccc407e7ae8d35a83c71f5f2d0f15bc5691f106. IMPLEMENTATION_WORKTREE /home/user/phase (clean at start, HEAD == START_SHA, nothing staged; HEAD unchanged at end, only unstaged edits).
Evidence below is PREPARATORY, not completion evidence.

## Changes (all unstaged, all inside scope.txt)
- F1: every comment added in 91ad0dff..eccc407e trimmed to one sentence, plan labels (P1..P6, S1..S5, C1, E1, L1, ST1, PR2, SHAPE, CEN1/2, F7, X/Y) dropped, no code changed. Files: game/layers.rs, game/stack.rs, game/effects/effect.rs, game/text_substitution.rs, parser/oracle_effect/text_change.rs, types/ability.rs, tests/integration/text_substitution_cr612.rs, lobby-broker/src/protocol.rs, client/src/adapter/ws-adapter.ts, client/src/network/protocol.ts. Number-only edits to the existing p2p wire-gate test comment, the server-core protocol test doc and the mjs +N lines were left (one sentence each already).
- F2: CR 305.7 citations removed from text_substitution.rs, the layers.rs/test docs and the test assertion message; now `// CR 305.6: the replaced type's intrinsic mana ability goes with its word, and the new type's ability is derived after the Type layer.` (305.6 grepped: basic land type has the intrinsic mana ability).
- F3: new test `indefinite_text_change_ends_when_the_permanent_leaves_and_returns` in tests/integration/text_substitution_cr612.rs (Magical Hack on Bog Wraith: Plains while it stays [reach-guard], Swamp after Hand and back to Battlefield, CR 400.7).
- F4: report-only correction: executor-r1 delta is 87 added / 2 removed (net +85), not "85 added".

## Checks
- cargo fmt --all: run, clean.
- Discrimination: new test green (r2-green.log); with the body of `zones::prune_object_bound_effects_on_exit` disabled it FAILS at the `["Swamp"]` assertion (r2-red.log); mutation reverted (`git status` shows zones.rs unmodified).
- clippy --workspace --all-targets -D warnings: exit 0 (r2-clippy.log).
- check-parser-combinators.sh vs HEAD: Gate A/G PASS. Against 91ad0dff it flags three `.contains(...)` lines in the `text_change.rs` test module that predate this round (not in my delta); no parser non-test code changed.
- CR gate over `git diff`: zero UNVERIFIED.
- Final full `cargo nextest run -p phase-engine --no-fail-fast`, started after the last edit with no file touched since: 31355 run, 31355 passed, 12 skipped (r2-full.log).

## Matrix / coverage
Only comments plus one test added: no behavioral seam changed. Coverage map for F3: claim = text change ends on battlefield exit (CR 400.7); seam = zones::prune_object_bound_effects_on_exit; entry = real cast of Magical Hack + zone moves + evaluate_layers; assertion that flips = final `["Swamp"]`; positive paired = `["Plains"]` while on battlefield.

## Judgement calls
Zone moves use `zones::move_to_zone` (existing helper used by other integration tests) rather than a bounce card.
