# Impl review r1 - Phase 4 (PREREQ-0), phase mode
MODEL: claude-sonnet-5-5
Review Head: 67cd221ea26fd73a3fa26328ef14cf8f688243d4 (range 06554d0e..67cd221e; `git diff --stat`: only crates/engine/src/parser/swallow_check.rs, +94/-1)

Verdict: ACCEPT (no HIGH/MED findings). Completion Gate: PARTIAL at time of review (see Evidence); Maintainer-Simulation Gate: PASS.

## Checks
- Diff is exactly the plan: one or-pattern arm `Effect::Counter { countered_spell_zone: Some(_), .. }` in `effect_is_replacement_carrier`, terminator `=> true` / `_ => false` intact. Typed match, no string/nom concern, no new variant.
- CRs grepped in docs/MagicCompRules.txt: 614.1a, 701.6a, 608.2c (its example is the Memory Lapse text). All real and fit.
- Tree walk verified: `def_tree_has_replacement_carrier` covers sub/else/delayed/mode, so nested Counter is covered.
- Tests: positive test has a reach guard on the parsed zone before the negative assertion (not vacuous); Hinder test has reach guard (`countered_spell_zone: None`) + `only_swallow`, which guards against widening to any Counter. Executor red log shows the positive test failing without the arm.
- `Some(_)` also admits `SpellStackToGraveyardReplacement::Exile`; type doc says the counter parser never emits it (exile riders use the sub-ability), so no over-acceptance in practice.
- Deferral: charter list None; nothing foreclosed.

## Constraint closure (plan-review r1)
- F1 (test name `condition_if_accepts_countered_spell_zone_redirect`): CLOSED - executor runs the whole module (`nextest --lib swallow` 260/260 per executor-r1/green.log); the real test exists at swallow_check.rs:8448.
- F2 (Spell Crumple full-text end-to-end runtime test absent): ACKNOWLEDGED, out of scope, LOW; unchanged, not regressed.

## Findings
None HIGH/MED. 
- N1 [LOW, behavior, non-blocking] Lapse of Certainty has no dedicated runtime test (identical AST to Memory Lapse). No action.

## Evidence (completion.txt at 21:31 UTC)
- HEAD==candidate, dirty=0; `cargo fmt --all -- --check`: FMT-OK; clippy phase-engine --all-targets -D warnings: Finished (tail line, no error). Full nextest, parser-combinator gate and GEN= line were NOT present after >10 min (nextest still building/running); card-data regeneration NOT available, so the parser-output diff (expected: Memory Lapse, Lapse of Certainty, Remand, Spell Crumple flip; Hinder unchanged) was not measured by me. `client/public/*-data.json` mtimes (13:16/13:17) predate the base copies (21:04), i.e. not regenerated. Orchestrator must confirm full nextest + GEN diff before accepting.
- Executor-side (preparatory): swallow lib 260/260, integration counter_spell_zone_redirect 4/4, clippy exit 0, combinator Gate G/A PASS.
