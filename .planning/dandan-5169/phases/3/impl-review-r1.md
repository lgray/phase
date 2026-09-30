# Phase 3 implementation review r1 (phase mode)
MODEL: claude-sonnet-5-5
Range: 41b2e35369ada8c138b506d280ff7f41a166114b..06554d0e8e610d9d935f251197de7530607aa92b (1 file: fixed_deck_keepables.rs, +74/-21). Worktree clean.

Verdict: ACCEPT. No HIGH/MED findings.

## Measured
- Completion (phases/3/completion.txt): fmt --check OK; clippy -p phase-ai --all-targets -D warnings finished clean; nextest -p phase-ai 2837 passed, 21 skipped.
- Diff read in full: gate is an exhaustive `match format_config.format.opening_hand_equivalence()` (no wildcard); Equivalent => ForceKeep, Distinguishable => neutral Score; fact key renamed to `opening_hand_equivalent`. grep of crates and client/src shows no consumer of the old fact key. `supplies_fixed_deck` has no remaining reader in phase-ai except the Dandan test's reach-guard.
- Tests: `dandan_format_abstains` (reach-guard flag true, asserts Score/0.0/fact) and `force_keep_iff_format_declares_equivalent_hands` (every GameFormat, `.expect` on for_format, both counts nonzero); Momir and default-config tests kept. Red at base shown by executor (2 failed, panic "Dandan: left true right false"); consistent with the diff (old gate keyed on the flag Dandan sets).
- CR 103.5 annotation: only citation, verified by plan review (line 296, mulligan rule).

## Constraints from plan-review-r1
- F1 (unwrap instead of skipping Err): CLOSED (`.expect("every enumerated GameFormat has a built-in config")`, no Err skip).
- F2 (docs consistent; no mod.rs edit): CLOSED (module head and evaluate docs reworded to equivalence; mod.rs untouched, scope stays 1 path).

## Findings
- L1 [LOW, text, non-blocking]: executor's rewording of the two `input-unused` comments is justified (old text "every fixed-deck hand is equivalent" / "a fixed all-land deck is always kept" became false for Dandan) and the lint test (marker presence) passes in the full run. Judgement call ACCEPTED. Optional nit: the `_turn_order` comment ("an equivalent-hands format has one hand's worth of information") is wordier than the others; no change required.

## ai-gate judgement
Acceptable to defer to one run-level execution after the last AI phase (17); does not block acceptance. Reasons (measured): grep of duel_suite and bin sources for momir/dandan/GameFormat::/supplies_fixed_deck returned nothing, so the pinned suite cannot reach a fixed-deck format; for every format it can reach, old gate (flag false) and new gate (Distinguishable) both return the same neutral Score delta 0.0, and no scoring path reads fact keys. Condition: the run-level gate must be run without --refresh-baseline and any flip is a bug attributable to this phase or later AI phases; the PR description should state the deferral honestly.
