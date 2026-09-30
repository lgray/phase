# Phase 3 plan — AI force-keep re-gate on the opening-hands-equivalent axis

Mode: phase-plan (engine-planner). `PHASE_BASE_SHA` = `41b2e35369ada8c138b506d280ff7f41a166114b`. Charter entry: Phase 3 (r3). Plan loop round 0.
Skills applicable: `add-ai-feature-policy` (the file is a `MulliganPolicy`; no new `DeckFeatures` axis, no new policy, no `PolicyId` change, so only its review traps and its "attach an ai-gate report" rule apply). Not applicable: `add-engine-variant` (no enum variant is added; Phase 2 already added `OpeningHandEquivalence`), `add-engine-effect`, `oracle-parser` (no parser file), `add-interactive-effect` (no `WaitingFor`/`GameAction`), `card-test` (no cast-pipeline test; see Verification Matrix).

## Step 0 — premise
No card's Oracle text is the subject of this phase. The premise is a code claim, bought by reading at `PHASE_BASE_SHA`:
- `crates/phase-ai/src/policies/mulligan/fixed_deck_keepables.rs::FixedDeckKeepMulligan::evaluate` branches on `state.format_config.supplies_fixed_deck` alone and emits `MulliganScore::ForceKeep` (fact `supplies_fixed_deck`=1) when true, else a neutral `Score { delta: 0.0 }`. No format identity is consulted (read; not yet run).
- `GameFormat::supplies_fixed_deck` is true for `Momir` and `Dandan` (Phase 2); `FormatConfig::dandan()` and `FormatConfig::momir()` both set `supplies_fixed_deck: true` (read).
- `GameFormat::opening_hand_equivalence() -> OpeningHandEquivalence` exists: `Momir => Equivalent`, `Dandan` and every other built-in and `Custom(_) => Distinguishable` (read in `crates/engine/src/types/format.rs`).
- Consequence for Dandan today: the policy ForceKeeps every Dandan hand, and `ForceKeep` outranks every `ForceMulligan` in the registry's three-way precedence (module docs in `mulligan/mod.rs`), so the AI would never mulligan a Dandan hand. Dandan's 80-card pile is varied, so its hands are distinguishable (CR 103.5: a player mulligans to find a workable hand, which presumes hands differ).
- The closing measurement for the claim is the Dandan test failing at `PHASE_BASE_SHA` (executor shows it red before the edit).

## Sizing
- Units: 1 — "the fixed-deck mulligan force-keep is gated on the format's opening-hand-equivalence axis instead of the fixed-deck predicate". One registration surface (the policy's `evaluate`); no registry change (`FixedDeckKeepMulligan` is already in `MulliganRegistry::default()`, no edit). Discriminating test: Dandan config is not ForceKeep, paired with Momir still ForceKeep.
- Inter-unit dependency edges: none. Upstream: consumes Phase 2's `opening_hand_equivalence` axis. Downstream: Phase 13 (FreeReveal emission by the AI chooser) does not touch this file.
- Scope-path count (phase-fit counting rule): 1 — `crates/phase-ai/src/policies/mulligan/fixed_deck_keepables.rs` (production and its inline `mod tests`). No generated artifact, mirror, fixture, serialized shape or wire surface.
- T1 fails (1 unit). T2 fails (1 path). Small-change lane: eligible (1 unit, 1 counted path, no enum variant, no serialized surface, no `WaitingFor`/`GameAction` change, no parser, no cross-crate consumer). Estimated ~60 LOC changed including tests.

## Pattern Coverage
Class: every format whose opening hands are not all interchangeable but whose deck the engine supplies. Charter attribution: Dandan is the first such format (the brief's Dandan decklist is the class instance; any later fixed-deck format with a varied pile inherits the right answer by answering `Distinguishable` on the axis, with zero policy edit). Momir stays ForceKeep through the same predicate, so one predicate answers for both classes and there is no per-format arm. Cards covered by this phase alone: none by construction (AI infrastructure); the charter attributes the benefit to the Dandan decklist, about 80 distinct cards' worth of hands.

## Building Blocks
- `GameFormat::opening_hand_equivalence()` and `OpeningHandEquivalence` (`engine::types::format`, Phase 2): the single authority for "does a mulligan change anything". The policy composes it; it adds no format list of its own.
- `state.format_config.format` (the `FormatConfig.format: GameFormat` field): the route from the policy's `state` to the axis (read: `FormatConfig` has a `format` field; `FormatConfig::momir()`/`dandan()` set it).
- `MulliganScore::ForceKeep`/`Score`, `PolicyReason::with_fact`: existing verdict and trace types, unchanged.
- Test fixtures: `GameState::new_two_player(0)`, `FormatConfig::momir()`, `FormatConfig::dandan()`, `FormatConfig::for_format(GameFormat)` (returns `Result`; used by the class-level test), `strum::IntoEnumIterator` over `GameFormat` (`GameFormat` derives `strum::EnumIter`). No new helper.
- `supplies_fixed_deck` is deliberately no longer read by this policy: the policy's question is hand equivalence, not deck provenance. `supplies_fixed_deck` keeps its other consumers (the client setup flow and `load_and_hydrate_decks`), untouched.

## Logic Placement
The equivalence decision is an engine rule-axis answer (`GameFormat`, Phase 2). The AI policy is only its consumer, so the policy holds no format identity and no hand-analysis logic; this matches "the engine owns all logic". `FormatConfig` is not edited (no field, no serde default).

## Rust Idioms
- Compare the typed enum, not a bool: `state.format_config.format.opening_hand_equivalence()` matched exhaustively with `match` over `OpeningHandEquivalence::{Equivalent, Distinguishable}`, no wildcard arm, so a future third variant fails to compile here.
- The trace fact changes from the raw `supplies_fixed_deck` flag to `opening_hand_equivalent` (1/0), naming what the gate now reads. Measured by `grep -rn 'fixed_deck_force_keep\|fixed_deck_not_applicable\|"supplies_fixed_deck"'` over `crates` and `client/src`: no consumer outside this file keys on the reason strings or the fact key (only unrelated `supplies_fixed_deck` hits in `custom_format_schema.rs` and a fixture JSON about the `FormatConfig` field). Reason strings `fixed_deck_force_keep` / `fixed_deck_not_applicable` are kept (they are the policy's identity in traces; no rename needed).
- Module and item doc comments are updated to say the gate is hand equivalence (Momir's all-land deck) rather than "fixed deck"; the `PolicyId::FixedDeckKeepMulligan` name and type name are kept (renaming a `PolicyId` is a wider, unrelated change; no rename in this phase).

## Nom Compliance
N/A — no file under `crates/engine/src/parser/` changes.

## Extension vs Creation
Extends the existing policy in place: one predicate swap. Creates nothing (no type, no variant, no registry entry).

## Analogous Trace
Traced the same-shape re-gate already in the tree: Phase 2's axis `GameFormat::opening_hand_equivalence` (`crates/engine/src/types/format.rs`, enum `OpeningHandEquivalence` and method, plus its census test in the same file) → consumer `FixedDeckKeepMulligan::evaluate` (`crates/phase-ai/src/policies/mulligan/fixed_deck_keepables.rs`) → registration `MulliganRegistry::default()` (`crates/phase-ai/src/policies/mulligan/mod.rs`, `Box::new(FixedDeckKeepMulligan)`) → registry precedence (`ForceKeep` outranks `ForceMulligan`) → `PolicyId::FixedDeckKeepMulligan` (`crates/phase-ai/src/policies/registry.rs`). Only the first consumer hop is edited.

## Variant Discoverability
No enum variant added. `OpeningHandEquivalence` already exists from Phase 2 (read in `format.rs`); `cargo engine-inventory` is not needed and is not run (it is a cargo command, and the change adds nothing to the inventory).

## Implementation steps
1. `crates/phase-ai/src/policies/mulligan/fixed_deck_keepables.rs`, production:
   a. Import `engine::types::format::OpeningHandEquivalence`.
   b. In `evaluate`, replace `if state.format_config.supplies_fixed_deck { ForceKeep } else { Score }` with an exhaustive `match state.format_config.format.opening_hand_equivalence()`: `Equivalent` returns `ForceKeep { reason: PolicyReason::new("fixed_deck_force_keep").with_fact("opening_hand_equivalent", 1) }`; `Distinguishable` returns the neutral `Score { delta: 0.0, reason: PolicyReason::new("fixed_deck_not_applicable").with_fact("opening_hand_equivalent", 0) }`.
   c. Doc comment (module head and `evaluate`): state the gate as CR 103.5 "a mulligan only matters when hands differ; the format declares when every hand is equivalent (Momir's all-basic-land deck); Dandan's supplied pile is varied, so its hands are judged by the archetype policies". Grep-verify the CR text before writing the annotation: `grep -n "^103.5" docs/MagicCompRules.txt` (the module already cites CR 103.5; the executor re-greps and keeps the existing citation if it reads as the mulligan rule; "103.5" reading is not yet verified by this planner — see Unestablished claims).
   d. `_hand`, `_plan`, `_features`, `_turn_order`, `_mulligans_taken` stay unused with their existing `input-unused` comments; the `_hand` parameter is unchanged.
2. Same file, inline `mod tests` (see the Verification Matrix): add the Dandan test, the class-level axis test, and keep the two existing tests unchanged.
3. `cargo fmt --all` (direct). Then targeted verification per the Verification Matrix (all cargo commands prefixed `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0`, one at a time, only after `pgrep -f cargo-nextest` is empty).

## Verification Matrix
| Claim | Seam | Production entry | Test (inline, `fixed_deck_keepables.rs`) | Revert-failing assertion | Positive reach-guard / siblings |
|---|---|---|---|---|---|
| A Dandan hand is not force-kept | `FixedDeckKeepMulligan::evaluate` gate | `MulliganRegistry::default()` includes the policy; `evaluate_hand` calls `evaluate` per policy | `dandan_format_abstains`: `state.format_config = FormatConfig::dandan()`, call `evaluate(&[], ...)`, assert `MulliganScore::Score { delta == 0.0 }` and its reason fact `opening_hand_equivalent == 0` | Red at `PHASE_BASE_SHA` (returns `ForceKeep` because `supplies_fixed_deck` is true for `FormatConfig::dandan()`); executor runs it red before the edit, green after. Reverting the gate to `supplies_fixed_deck` turns it red again. | Existing `fixed_deck_format_force_keeps` (Momir, `FormatConfig::momir()`) must stay `ForceKeep` — it is the paired instrument-fires guard, and the test asserts the Dandan config really has `supplies_fixed_deck == true` first, so the negative cannot pass because the flag was false. |
| The gate follows the axis for every built-in format, not a Momir/Dandan list | same | same | `force_keep_iff_format_declares_equivalent_hands`: iterate `GameFormat::iter()`; for each `format` where `FormatConfig::for_format(format)` is `Ok(cfg)`, set `state.format_config = cfg`, and assert `matches!(score, ForceKeep)` == (`format.opening_hand_equivalence() == Equivalent`); count the `ForceKeep` formats and the abstaining formats and assert both counts are nonzero (reach-guard: the loop observes both answers, so the policy is read per format) | Red at the base for `Dandan` (ForceKeep but `Distinguishable`). | `Custom(_)` and any format whose `for_format` returns `Err` are skipped by the `Ok` pattern; how `Custom` is constructed for `GameFormat::iter()` is an unestablished point (see below) — if `iter()` yields a `Custom` value that `for_format` accepts, it is covered by the same assertion; if it errs, it is skipped and the nonzero-count guards still bind. |
| Non-fixed-deck format abstains (unchanged) | same | same | existing `non_fixed_deck_format_abstains` (default `GameState::new_two_player(0)` config), unchanged | n/a (regression guard) | Covered also by the loop above. |

Hostile fixtures: (a) the policy's own negative (Dandan) is paired with the positive (Momir) in the same file; (b) the flag-true guard inside the Dandan test closes the "vacuous negative" hole; (c) the axis loop is the multi-format sibling. A `Custom` format carrying a `supplies_fixed_deck` flag is unreachable to test as ForceKeep: `Custom(_)` answers `Distinguishable` on the axis, and the gate now follows the axis, so it abstains by design (the class is decided by the format's axis answer, not by a field a lobby could set). Coverage status impact: none (no parser or card data change). Parser changes: none, so no Oracle text is accepted with deferred semantics.

Card-level runtime test: intentionally none. The phase's discriminating instrument is the policy verdict on a format config (charter verification plan); a cast-pipeline test would exercise no code in this phase. The deal-and-mulligan flow for Dandan is `DEFERRED(phase 12)` (declare round and pregame dealer) and `DEFERRED(phase 13)` (FreeReveal emission by the AI mulligan chooser); neither can exist until those phases land.

## Reference Readings
One reference row: the Momir test preserves today's behavior (Momir stays ForceKeep). Derived independently of the old code: Momir's Madness's deck is 60 snow basic lands (`FormatConfig::momir()` docs; format.rs), so every legal opening hand is seven basic lands and equivalent; the axis's Momir arm says `Equivalent`. The reference reading (the existing Momir test's `ForceKeep` at base) agrees with the derivation, so there is no pre-existing defect on that row. Whether the Momir test passes at base is measured by the executor running it before the edit (not run by the planner: a nextest run was in progress when this plan was written).

## Identity / Provenance Contract
N/A — no "this way"/"that source"/chosen/selected-authority text, no duration-bound effect, no owner/controller-relative rule change in this phase. The one identity the policy reads is the game's format, `state.format_config.format`, latched at game setup (the same field `format_config` already carries), consumed live in `evaluate`.

## AI gate (`cargo ai-gate`) — required statement
This change alters AI mulligan behavior (a Dandan hand stops being force-kept), so CLAUDE.md's "AI behavior changes must run `cargo ai-gate`" rule applies and the gate is triggered. Expected result: zero flips, byte-identical to the baseline, for a structural reason the executor also confirms:
- The pinned suite exercises decks in non-fixed-deck formats only: `grep -n -i 'momir\|GameFormat::\|FormatConfig::' crates/phase-ai/src/duel_suite/*.rs` printed no match at planning time, and `DEFAULT_QUICK_FILTER` is `red-mirror,affinity-mirror,enchantress-mirror`. For every format the suite can reach, the old gate (`supplies_fixed_deck == false`) and the new gate (`Distinguishable`) both return the same neutral `Score { delta: 0.0 }`, so the verdict is unchanged. The reason's fact key changes (`supplies_fixed_deck` → `opening_hand_equivalent`), and no scoring path reads fact keys (grep above).
- How: after the green targeted tests, run once, with nothing else on cargo: `cd /home/user/phase && CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo ai-gate` (alias: `run --profile server-release --bin ai-gate --`, default quick filter, default seed) and attach the printed paired-seed report to the executor report. A zero-flip report is acceptable (the change is narrow and the suite cannot reach it). Do NOT refresh baselines (`--refresh-baseline`) for this change; a flip would be a bug and is a stop-and-return. The `server-release` profile build is heavy on this box (4 cores, 15 GB): check `df -h /home/user/phase` first (stop and report below 4 GB free), and if the build cannot run on this box, return the missing evidence rather than skipping the statement.
- `cargo ai-perf-gate` is not triggered: the change adds no board-wide or affordability engine call (one enum method call on a `Copy` value per evaluation); `add-ai-feature-policy` perf rules do not apply.

## Verification commands (executor)
1. Red: add only the two new tests first, `cd /home/user/phase && CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo nextest run -p phase-ai fixed_deck_keepables` — `dandan_format_abstains` and `force_keep_iff_format_declares_equivalent_hands` fail; `fixed_deck_format_force_keeps` and `non_fixed_deck_format_abstains` pass (red → green evidence).
2. Edit production; rerun the same filter: all four green.
3. `cargo fmt --all`; `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo clippy -p phase-ai --all-targets -- -D warnings` (clippy is the CI gate; scoped to the one crate).
4. `mulligan` module tests: `cargo nextest run -p phase-ai mulligan` to confirm the registry tests (`default_registry_contains_fixed_deck_keepables`, the all-land Momir hand test in `mod.rs`) stay green. Note the `mod.rs` all-land test's Momir fixture must still pass through the policy; if that test builds its config from a struct literal with `supplies_fixed_deck: true` but `format` left at a default non-Momir format, it turns red under the new gate — that would be a needed out-of-scope edit in `mulligan/mod.rs` and is a stop-and-return with evidence (unestablished: the planner did not read that fixture, see below).
5. `cargo ai-gate` as above.

## Unestablished claims
1. The Momir fixtures elsewhere in `crates/phase-ai` (the all-land hand test near `mulligan/mod.rs` line ~929 that builds "Momir Land N" cards) may set `supplies_fixed_deck` without `FormatConfig::format == Momir`; not read. If so, the re-gate turns that test red and `mulligan/mod.rs` would need an edit outside the 1-path scope (stop-and-return; it would raise the count to 2 paths, still below T2).
2. That `FormatConfig::for_format` returns `Ok` for every `GameFormat::iter()` value, and how `iter()` constructs `Custom(_)`: not measured; the class-level test skips `Err` and guards nonzero counts instead.
3. That CR 103.5's text reads as the mulligan rule: the module already cites it; not re-grepped by this planner (executor greps `^103.5` before keeping or amending the annotation).
4. The Dandan test's red at `PHASE_BASE_SHA` and the zero-flip `ai-gate` result are predictions from reading; no cargo command was run (a nextest run was in progress).
5. That the `duel_suite` never reaches a fixed-deck format: rests on one grep over `crates/phase-ai/src/duel_suite/*.rs` (no match); suite decks' formats are not otherwise enumerated.
