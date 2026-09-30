# Phase 3 executor report r1
MODEL: claude-sonnet-5-5
Mode: implementation/fix (phase mode, k=3). BASE_SHA(run)=b9ba9360. START_SHA=41b2e35369ada8c138b506d280ff7f41a166114b. IMPLEMENTATION_WORKTREE=/home/user/phase.
Start check: HEAD == START_SHA, clean, nothing staged. End check: HEAD unchanged, nothing staged, only delta = `crates/phase-ai/src/policies/mulligan/fixed_deck_keepables.rs`. All evidence below is PREPARATORY, not completion evidence.

## Diff summary
`fixed_deck_keepables.rs` only: gate swapped from `supplies_fixed_deck` to an exhaustive `match state.format_config.format.opening_hand_equivalence()` (Equivalent => ForceKeep, fact `opening_hand_equivalent`=1; Distinguishable => neutral Score, fact 0); module doc rewritten around hand equivalence (CR 103.5); two tests added (`dandan_format_abstains`, `force_keep_iff_format_declares_equivalent_hands`) plus helper `evaluate_for`; existing two tests kept (only the Momir assert message reworded). F1 applied (`for_format(..).expect(..)`, no Err skipping). F2: no mod.rs edit.

## Red / green
Red (tests added, production unchanged): `cargo nextest run -p phase-ai fixed_deck_keepables` -> 3 passed, 2 FAILED (`dandan_format_abstains`, `force_keep_iff_format_declares_equivalent_hands`; panic "Dandan: left true right false"). Green after edit: 5/5 pass (incl. Momir force-keep, non-fixed-deck abstain, registry registration). Revert-failing assertions: Dandan `Score` match arm + `assert_eq!(kept, equivalent)` for Dandan.

## Preparatory checks
- fmt (scoped to the path): ok, no out-of-scope delta.
- `cargo clippy -p phase-ai --all-targets -- -D warnings`: clean.
- `cargo nextest run -p phase-ai mulligan`: 87/87 pass (incl. `mulligan_input_lint::unused_mulligan_inputs_carry_marker`).
- Parser gate: N/A (no parser file).
- CR gate: only `CR 103.5`; `grep -n "^103.5"` line 296 is the mulligan rule. Verified.
- `cargo ai-gate`: NOT RUN. `df -h /home/user/phase` = 2.5 GB free (13 GB before clippy; clippy's 6m55s build consumed ~10 GB), below the plan's 4 GB stop threshold; the server-release build would need more. Missing evidence: paired-seed report. Structural zero-flip argument from plan (suite reaches no fixed-deck format) is unmeasured by me. No baseline refreshed.

## Coverage map
Claim: Dandan hand not force-kept. Seam: `FixedDeckKeepMulligan::evaluate`. Entry: MulliganRegistry (registered, test `default_registry_contains_fixed_deck_keepables` green) -> `evaluate`; test calls `evaluate` directly (registry precedence unchanged). Tests: `dandan_format_abstains` (with reach-guard `supplies_fixed_deck == true`), `force_keep_iff_format_declares_equivalent_hands` (asserts both ForceKeep and abstain counts nonzero). Siblings: Momir ForceKeep, default config abstain. No card-level runtime test (per plan; Dandan deal/mulligan flow DEFERRED(phase 12)/(phase 13)).

## Maintainer-simulation matrix
Single row: gate reads `state.format_config.format` live in `evaluate` (no bound/latched value, no storage, no invalidation; consumers: registry precedence); hostile fixture: Dandan with flag true (reached, Distinguishable arm). `Custom(_)` unreachable in iter (`#[strum(disabled)]`, per plan review). Serde/protocol impact: none (fact key is a trace label only; grep per plan showed no consumer). New-field sweep: no field added.

## Judgement calls
Two `input-unused` comments reworded ("every fixed-deck hand is equivalent" -> "an equivalent-hands format has one hand's worth of information"; "a fixed all-land deck is always kept" -> "equivalent hands are always kept") because old text became inaccurate under the new gate; lint test still passes. Revert if reviewer prefers.

## Stop-and-return items
1. ai-gate not run: disk 2.5 GB free < 4 GB. Needs disk headroom (orchestrator decision; I deleted nothing) then `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo ai-gate` with no `--refresh-baseline`.

## Deviations / risks
None from plan besides the comment rewording. Risk: ai-gate evidence absent.
