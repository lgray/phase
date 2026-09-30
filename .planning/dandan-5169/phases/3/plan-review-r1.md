# Phase 3 plan review, round 1 (phase-plan mode)

MODEL: claude-sonnet-5-5
Verdict: ACCEPT (no blocking findings). Phase-fit: Sizing consistency check passes.

## Sizing check (blocking gate)
- Units: 1 (force-keep gate swapped to `opening_hand_equivalence`). Paths: 1 (`crates/phase-ai/src/policies/mulligan/fixed_deck_keepables.rs`).
- Measured: `git grep supplies_fixed_deck crates --include=*.rs` shows the only phase-ai reader is this file (lines 6, 44-53, 80). `mulligan/mod.rs` registers `FixedDeckKeepMulligan` (line 273) unchanged. No enum variant, serialized surface, or WaitingFor/GameAction change. Small-change lane CONFIRMED (1 unit, 1 path).

## Unestablished claims measured
1. Other phase-ai fixtures setting supplies_fixed_deck without format==Momir: REFUTED. No phase-ai file other than this one mentions `supplies_fixed_deck`. The mod.rs all-land test (`momir_all_land_hand_force_keeps_without_dead_hand_override`, line 921) uses `FormatConfig::momir()`; `momir_curve.rs` uses `FormatConfig::momir()` too. `FormatConfig::momir()` sets format Momir, which answers Equivalent. The stop-and-return contingency in Verification command 4 will not fire.
2. `for_format` over `GameFormat::iter()`: CONFIRMED safe. `Custom(CustomFormatId)` is `#[strum(disabled)]` (format.rs:161), so `iter()` never yields it. `for_format` has an `Ok` arm for every other variant, including `Dandan => Self::dandan()` (format.rs:3490-3524). The Err/Custom hedge in the test is dead code, harmless but unnecessary.
3. CR 103.5: CONFIRMED. `grep -n "^103.5" docs/MagicCompRules.txt` line 296 is the mulligan rule ("A player who is dissatisfied with their initial hand may take a mulligan ... until their opening hand would be zero cards"). Keep the annotation.
4. Base-red prediction: by read, `FormatConfig::dandan()` has `supplies_fixed_deck: true` (format.rs ~3347) and base `evaluate` branches on it alone, so the Dandan test is red at base. Not run: `pgrep -f cargo-nextest` was non-empty, so no cargo was run.
5. Duel suite reach: `grep -n -i 'momir\|GameFormat::\|FormatConfig::' crates/phase-ai/src/duel_suite/*.rs` returns no match, confirmed. The ai_duel/ai_commander bins use `FormatConfig::commander()` (Distinguishable, old gate false too). Zero-flip is structurally sound.

## Findings
- F1 [text, non-blocking, LOW]: the plan's Verification Matrix row 2 and the Unestablished list say `iter()` may yield `Custom` and describe skipping `Err`. Measured: it cannot. Replacement text: "GameFormat::iter() excludes Custom (#[strum(disabled)]); for_format returns Ok for every yielded value, so the test unwraps it (`.expect(...)`) instead of skipping Err." Keeping `Ok` skipping would let a future Err silently shrink coverage; unwrap is stricter. No seam/layer/type change.
- F2 [text, non-blocking, LOW]: `mulligan/mod.rs` line ~384 message ("so Momir-family all-land hands are kept") and the module-head sentence "whenever the format supplies a fixed deck" in the policy file stay accurate only after the doc edit in step 1c; the plan covers the policy file. mod.rs text remains true (Momir still kept), so no edit, no scope growth.

Both findings are optional polish. No finding changes a seam, layer, or type.
