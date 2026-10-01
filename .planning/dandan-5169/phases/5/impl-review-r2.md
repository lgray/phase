# Phase 5 implementation review r2

MODEL: claude-sonnet-5-5
Range reviewed: eccc407e7ae8d35a83c71f5f2d0f15bc5691f106..772c18d1d3b99bac61557f42d790b4eea63efda2 (029b93dc, 772c18d1). No cargo run (per brief).
Verdict: APPROVE. No HIGH/MED findings. One LOW note.

## Checks
- r1 F1 (comments): 53 added comment lines in 029b93dc; none has a second sentence (regex for ". X" in added comments is empty) and none carries a plan label (P#/S#/C1/E1/L1/ST1/PR2/SHAPE/CEN#/F7 grep empty). The kept "Object X:/Object Y:" lines are fragments inside a test body.
- r1 F2: "CR 305.7" is gone from code and from the assertion message ("CR 305.6: taps for {U}..."); only 305.6 remains.
- CR grep in docs/MagicCompRules.txt for every CR number in added comments (113.1c, 305.6, 400.7, 400.7a, 514.2, 601.2b, 608.2b, 608.2d, 611.2a, 611.2c, 612, 612.1-612.3, 613.1c, 613.7b, 613.8, 613.8a, 613.8b): all present.
- Code-line delta in 029b93dc: filtering out comment and blank lines leaves only (a) the new test `indefinite_text_change_ends_when_the_permanent_leaves_and_returns` and (b) one assertion message string (305.7 removed). layers.rs, stack.rs, types/ability.rs: comment-only. No production line changed.
- r1 F3 test discrimination: asserts ["Plains"] while on battlefield (reach guard), moves Hand then Battlefield via `zones::move_to_zone`, re-runs layers, asserts zone == Battlefield and ["Swamp"]. executor-r2.md records green, and red at the ["Swamp"] assertion with `prune_object_bound_effects_on_exit` disabled (mutation reverted, zones.rs unmodified). Test shape is consistent with that claim.
- r1 F4: executor-r1 count correction is recorded in executor-r2.md (report-only).
- 772c18d1 (test module of text_change.rs only; non-test parser code untouched): `has_unimplemented` now walks sub_ability / else_ability / mode_abilities and matches `Effect::Unimplemented` or `AbilityCost::contains_unimplemented`; `has_word_substitution` matches `GenericEffect.static_abilities[].modifications` containing `SubstituteTextWord`. Positive reach-guard stays `assert!(has_word_substitution(..))` (no vacuous pass); the fail-closed test still asserts has_unimplemented true AND no word substitution, so a too-narrow walker could only fail it, not pass it vacuously. Debug is retained only inside failure messages.
- I ran `scripts/check-parser-combinators.sh b9ba9360` on HEAD 772c18d1: Gate A and Gate G PASS, exit 0.
- Evidence consistency: executor-r3.md reports full phase-engine 31355/31355 after the last edit, gate exit 0, fmt OK; HEAD matches 772c18d1.

## Findings
### F1 LOW (no action required)
The typed `has_unimplemented` walker is narrower than the old Debug substring: it does not descend into ability definitions embedded inside effect payloads (e.g. delayed-trigger or nested-effect fields). For the positive "parses completely" test this is slightly weaker than before; for the fail-closed test it is only stricter. The cards in scope (Magical Hack class, keyworded/modal members) lower to flat sub_ability chains, so no gap is reachable today; widen the walker if a class member with a nested payload is added.
