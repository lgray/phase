# MSH-E Fix-Round Report — Hawkeye, Master Marksman + The Ruinous Wrecking Crew

Worktree: `/home/lgray/vibe-coding/wt-msh-f` · branch `feat/msh-e-marksman-ruinous` · base `aa4e88ec4`.
Source review: `.planning/msh-rollout/MSH-E-IMPL-REVIEW.md` (1 HIGH, 1 MED, 2 LOW).
Cargo run DIRECTLY in this isolated worktree. `data/*.json` symlinks NOT regenerated. No commit / no `git add`.

**Verdict: all 4 review fixes applied surgically. All verification green. No stop-and-return items.**

---

## Verification counts (cargo-direct, this worktree)

| Gate | Result |
|---|---|
| `cargo fmt` | clean |
| `cargo build -p engine` | OK (exit 0) |
| `cargo clippy -p engine --lib --tests -- -D warnings` | clean (exit 0, after one `doc_lazy_continuation` fix) |
| `cargo test -p engine --lib` (full) | **13830 passed · 0 failed · 7 ignored** (was 13827 + 3 new fix-round tests) |
| serde/roundtrip/partial_eq subset (154 tests) | **154 passed · 0 failed** (eq-include did NOT break roundtrip equality) |
| `./scripts/check-parser-combinators.sh` | PASS (exit 0) |
| Parser diff-gate inline grep | only the 2 pre-existing annotated swallow-detector marker scans (allowed); fix-round swallow_check.rs change was doc-comment only |
| `cargo ai-gate` (B mandate) | **0 FAIL · 1 WARN · 2 PASS · 0 NEW · 0 REMOVED** (exit 0) — WARN = pre-existing affinity-mirror avg-turn drift, 0 W→L/L→W flips, scenario does not touch the Hawkeye path |
| CR-annotation diff gate | **10/10 verified · 0 UNVERIFIED** |

New fix-round tests: **3** (Fix 1 serde-roundtrip, Fix 2 predicate, Fix 3 Explosive targeted mode). Two load-bearing discriminators MEASURED by temporary revert (left/right recorded below).

---

## Fix 1 — HIGH: K must survive cross-`apply()` serde boundary

**File:** `crates/engine/src/types/game_state.rs`

- `:6838` — `optional_cost_payments_this_resolution` attribute changed
  `#[serde(skip, default)]` → **`#[serde(default, skip_serializing_if = "is_zero_u32")]`** (now serialized when nonzero; faithful roundtrip).
- `:8175-8176` — **eq-INCLUDED** in `impl PartialEq for GameState`, next to `exiled_from_hand_this_resolution`; the old "INTENTIONALLY excluded (mirrors `static_gate_truth`)" comment block removed and replaced with a CR 603.12a rationale tying K's serde/eq treatment to the same per-iteration `OptionalEffectChoice` pause that the paired `pending_repeated_optional_payment` continuation survives.
- Field doc (`~:6811-6837`) rewritten: states K is nonzero AT the per-iteration pause (a serde boundary across separate `apply()` calls — `to_persisted`/`from_persisted`, save/load, host-resume); mirrors `exiled_from_hand_this_resolution` (resolution-local counter observable at a pause), NOT `static_gate_truth` (derived/recomputable). The false "nonzero only mid-`apply()`" and "reconstructed by re-resolving" claims are deleted. CR 603.12a / 700.2d kept.

**Discriminating test:** `marksman_tests.rs::k_counter_survives_serde_roundtrip_mid_payment_loop` (`:361`) — pays twice through the production harness (paused at the 3rd `OptionalEffectChoice` with K=2 + `pending_repeated_optional_payment = Some`), `serde_json` roundtrips the paused state, asserts K==2 survives + the continuation survives, asserts K is eq-included (clone+perturb K ⇒ `assert_ne`), then resumes from the RESTORED state and declines → asserts the reflexive modal cap == `Some((0, 2))` = `min(K=2, mode_count=3)` (CR 700.2d).

**MEASURED non-vacuity (temporary revert to `#[serde(skip, default)]`):**
- `restored.optional_cost_payments_this_resolution` — **left `0`** vs expected **right `2`** → `assert_eq!` "K must survive the mid-payment-loop serde boundary" FAILS.
- Consequence on resume (under the same revert): K=0 → `finish_repeated_optional_payment` skips the reflexive → resumed cap = `None` vs expected `Some((0, 2))`.
Fix restored; test green.

---

## Fix 2 — MED: predicate must admit only synchronous (pure-mana) costs

**File:** `crates/engine/src/game/effects/mod.rs`

- `:3976` `is_repeated_optional_payment` — the cost arm changed from `matches!(ability.effect, Effect::PayCost { .. })` to **`is_synchronous_mana_pay_cost(&ability.effect)`** (new helper, `:4000`). Doc updated to explain that a pausing cost cannot be driven.
- `:4000` new `is_synchronous_mana_pay_cost(effect)` matches ONLY
  `Effect::PayCost { cost: AbilityCost::Mana { cost }, scale: None, .. }` **with `!casting_costs::cost_has_x(cost)`**.

**Predicate-narrowing approach (preferred path per the review):** the only resolution-time `PayCost` class guaranteed synchronous (auto-tap + pool deduction, never `PayAmountChoice`/`Paused`) is a pure, fully-static mana cost. The narrowing excludes (a) unannounced-`{X}` mana — pauses via `WaitingFor::PayAmountChoice` (pay.rs `cost_has_x` arm); (b) a per-object `scale` (resolution-only multiplier, not the repeated-optional shape); (c) every non-mana `AbilityCost` (Discard/Sacrifice/PayLife/PayEnergy/Composite/…) — interactive/`Paused`. Any repeated-optional ability whose cost pauses now falls through to the generic `repeat_for` path and stays honestly unimplemented (no false green) instead of mis-counting K and clobbering its own continuation. The one residual pause vector (a payment-replacement effect on a pure mana cost) is documented in the helper doc as outside the current corpus and invisible at AST time; a future pause-resume driver would lift it.

**Negative test:** `effects/mod.rs::tests::repeated_optional_payment_admits_only_synchronous_mana_cost` (`:8437`) — a positive control (`{1}` mana ⇒ satisfies) and three negatives that previously satisfied the bare predicate: unannounced-`{X}` mana, interactive `Discard`, and scaled mana ⇒ all `!is_repeated_optional_payment`.

**MEASURED non-vacuity (temporary widen back to `matches!(effect, Effect::PayCost { .. })`):** the X-mana fixture satisfies the widened predicate ⇒ `assert!(!is_repeated_optional_payment(&x_mana))` panics (effects/mod.rs:8489). Fix restored; test green.

---

## Fix 3 — LOW: end-to-end coverage for a TARGETED reflexive mode

**File:** `crates/engine/src/game/marksman_tests.rs`

- `:294` new `explosive_targeted_mode_resolves_once_through_apply` — pays {1}×3 (K=3, cap `(0,3)`), then drives the **Explosive** mode (index 1, "Hawkeye deals 2 damage to target player") through `apply`: `SelectModes{indices:[1]}` ⇒ surfaces `WaitingFor::TriggerTargetSelection` ⇒ `ChooseTarget{Player(P1)}` ⇒ `advance_until_stack_empty()` resolves the reflexive stack entry. Asserts P1 life **20 → 18** (exactly 2, not 2×K) and no second `AbilityModeChoice`. CR 700.2b + CR 120.3.
- Closes the report's "each mode resolves correctly" overclaim (only Boomerang, a no-target mode, was previously exercised). The reflexive→`SelectModes`→target-selection hand-off for a targeted mode under the new driver is now verified end-to-end.

**Non-vacuity (MEASURED during development):** before adding `advance_until_stack_empty()`, the test failed with P1 life **left `20` vs right `18`** — proving the damage assertion is real (the reflexive ability is pushed to the stack and only deals damage on resolution; a vacuous test would already read 18). Stack-resolution drive added; test green.

**Overclaim wording corrected** in `MSH-E-IMPL-REPORT.md` (coverage-map reflexive row + a fix-round correction note): Boomerang (net 0) + Explosive (targeted −2 life) now tested end-to-end; Net's CantBlock remains covered only by shared mode-resolution infrastructure (no dedicated test).

---

## Fix 4 — LOW: detector false-RED (conservative-RED limitation)

**File:** `crates/engine/src/parser/swallow_check.rs`

- Doc-comment of `detect_modal_dynamic_max_dropped` extended with an explicit **CONSERVATIVE-RED LIMITATION** paragraph: the three gates are independent whole-text / whole-AST scans, so a single card carrying BOTH an unrelated fixed modal node AND a separate non-modal "choose up to X `<nouns>`" clause would fire. This errs toward RED (understates coverage, never false-green), is empty on the current corpus, and a per-node association would duplicate the parser's `oracle_modal` negative-lookahead in audit code and risk regressing the measured Frillback/Hawkeye discrimination — so it is intentionally NOT done while the false-RED set is empty.

**Decision:** documented rather than structurally tightened, per the review's "otherwise document … Do not over-engineer" and the conservative-RED (never false-green) safety direction. No code-path change; no test needed (the detector's existing behavior is unchanged).

---

## Discriminating-test gate — production-path coverage map (fix-round additions)

| Behavioral claim | Changed seam/fn | Production entry | Test reaching it | Revert-failing assertion | Sibling/negative |
|---|---|---|---|---|---|
| K survives the mid-payment-loop serde boundary | K serde attr + eq-include (game_state.rs:6838 / 8175) | `apply` `DecideOptionalEffect`×2 → pause → `serde_json` roundtrip → resume `apply` | `k_counter_survives_serde_roundtrip_mid_payment_loop` | `restored.K == 2` (revert ⇒ 0, MEASURED); resumed cap `Some((0,2))` (revert ⇒ None) | eq-include perturbation (`assert_ne` on K-only delta) |
| Pausing-cost repeated-optional ability is NOT driven | `is_synchronous_mana_pay_cost` (effects/mod.rs:4000) | predicate dispatch (driver gate at `:5560`/`:5611`) | `repeated_optional_payment_admits_only_synchronous_mana_cost` | `!is_repeated_optional_payment(x_mana/discard/scaled)` (revert ⇒ x-mana satisfies, MEASURED at :8489) | positive control: `{1}` mana satisfies |
| Targeted reflexive mode resolves once with correct targeting | reflexive resolution + `SelectModes`/`ChooseTarget` hand-off | `apply` `SelectModes` → `TriggerTargetSelection` → `ChooseTarget` → stack resolve | `explosive_targeted_mode_resolves_once_through_apply` | P1 life `18` (no stack-resolve ⇒ 20, MEASURED) | no 2nd modal; sibling Boomerang (no-target) |

No fix-round seam is left covered only by a degenerate fixture. The serde test uses a real paused mid-loop state (K nonzero, continuation Some); the predicate test uses three genuinely-pausing/branching cost classes; the Explosive test drives a real player target through `apply` and resolves the stack.

---

## Maintainer-simulation matrix (fix-round rows)

| Seam / first prod branch | Selected authority | Bound value / when | Binding mode | Storage | Consumer | Invalidation | Hostile rows | serde/protocol |
|---|---|---|---|---|---|---|---|---|
| K counter at the per-iteration `OptionalEffectChoice` pause | repeated-payment count K | incremented per successful synchronous payment during resolution | live counter; latched into the modal cap by `modal_choice_for_player` | `GameState.optional_cost_payments_this_resolution` (`serde` when nonzero, **eq-included**) | `resolve_quantity(TimesCostPaidThisResolution)` → reflexive modal cap | cleared at depth==0 prelude (next top-level resolution) | **save/restore mid-loop** (server crash, save/load, host-resume) ⇒ K serialized-when-nonzero + eq-included survives (Fix 1) | **CHANGED: K now on the wire when nonzero; eq now includes K.** No card-data migration risk (defaults to 0/omitted; existing serde roundtrip suite stays green — 154/154) |
| `is_synchronous_mana_pay_cost` (driver eligibility) | the cost shape | classified at predicate time from `Effect::PayCost.cost` | live predicate over the AST | — (pure predicate) | `is_repeated_optional_payment` → driver dispatch / up-front-gate suppression | n/a (pure predicate) | unannounced-`{X}` mana, interactive Discard/Sacrifice, pay-any-amount life/energy, scaled mana, Composite ⇒ NOT driven (UNREACHABLE for driver), fall to generic `repeat_for` (honestly unimplemented) | none |
| Targeted reflexive mode hand-off | the chosen mode's target (player) | `ChooseTarget` at resolution, after `SelectModes` | live selection through `TriggerTargetSelection` → stack entry | reflexive trigger stack entry (`pending_trigger` → stack) | the mode effect (`DealDamage`) on stack resolution | single resolution; entry leaves stack on resolve | K==0 ⇒ no modal (no targets prompted); choosing fewer modes than K resolves only chosen | `AbilityModeChoice`/`TriggerTargetSelection` existing shapes, unchanged |

No global-rescan concern introduced. The only serialized-surface change is K (Fix 1), justified by the cross-`apply()` pause; the predicate narrowing (Fix 2) removes surface (fewer abilities driven), adds none.

---

## CR-annotation diff gate

`git diff | grep CR …` → **10/10 verified present in `docs/MagicCompRules.txt`, 0 UNVERIFIED:**
CR 107.3m · CR 117.1 · CR 118.1 · CR 120.3 · CR 601.2 · CR 603.12a · CR 608.2c · CR 700.2 · CR 700.2b · CR 700.2d.

Fix-round CR annotations added/changed (each grep-verified to *describe* the code):
- **CR 118.1** (`is_synchronous_mana_pay_cost` doc + the predicate-test doc) — "to pay a cost, a player carries out the instructions" — accurately covers paying a mana cost during resolution.
- **CR 700.2b** (Explosive test doc) — "controller of a modal triggered ability chooses the mode(s) … illegal mode can't be chosen" — the reflexive triggered modal's chosen mode.
- **CR 120.3 / 120.3a** (Explosive test doc) — "damage dealt to a player … causes that player to lose that much life" — Explosive's −2 life.
- CR 603.12a / 700.2d (Fix 1 field doc + eq comment) — reflexive-once + dynamic-cap clamp.

The remaining `CR 117.1` in the gate output is from **pre-existing, reviewer-accepted** impl code (the original `pay_three_times` test comment and the `resolve_repeated_optional_payment_choice` payment comment), untouched by this fix-round.

---

## Judgement calls

1. **CR 117.1 → CR 118.1 for the NEW mana-payment annotations.** The codebase has an established (reviewer-accepted) convention pairing `CR 117.1: Mana payment uses auto-tap + pool deduction` with `CR 118.1` (pay.rs:26-27). But CR 117.1 is literally the *priority* rule and does NOT describe payment synchrony. Per the CLAUDE.md mandate ("confirm the cited rule actually describes the annotated code"), my two NEW doc-comments cite **CR 118.1** instead. I did NOT churn the original impl's pre-existing CR 117.1 annotations (out of scope, load-bearing, reviewer-accepted).
2. **Fix 1 test eq-include assertion via clone+perturb, not full-state `assert_eq!` of the roundtrip.** A full `assert_eq!(restored, runner.state())` on the mid-resolution state failed on an unrelated transient field (a serde-skipped/eq-excluded mid-resolution carrier), coupling the test to roundtrip fidelity of the entire state. Replaced with a focused `assert_ne!` on a K-only perturbation — this directly proves K is eq-included (the fix) without depending on every other field's roundtrip behavior. K-survival is still proven by the direct `restored.K == 2` serde assertion; the existing 154-test serde/partial_eq suite covers full-state roundtrip equality.
3. **Fix 3 drives the stack to resolution (`advance_until_stack_empty`).** The reflexive modal is resolved by pushing a `pending_trigger` stack entry (the "push first, choose second" modal-trigger contract in `begin_pending_trigger_target_selection`), not purely inline. A targeted mode therefore needs the stack entry to resolve for damage to land — so the test passes priority to empty the stack. (This also revealed that the original Boomerang test's "net 0" is weak — net-0 holds whether or not the entry resolves; the Explosive −2-life assertion is strictly stronger.)
4. **Fix 4 documented, not tightened.** Per the review's stated fallback and "do not over-engineer"; the limitation is conservative-RED (never false-green) and empty on the current corpus.

## Stop-and-return items

**None.** No measured plan-vs-code contradiction. All four fixes implemented surgically; no working code reverted or re-architected.

## Risks / what `/review-impl` should watch

1. **K is now a serialized + eq-included field.** The 154-test serde/roundtrip/partial_eq subset stays green and the full 13830-test suite passes, so the eq-include did not break dedup/roundtrip equality. The only states where K≠0 are mid-repeated-payment-loop; everywhere else it is 0 (omitted on the wire, default on read). No card-data migration.
2. **Fix 2 residual pause vector.** A payment-replacement effect intercepting a pure mana cost is the one pause class the AST-time predicate cannot see; it is outside the current corpus and documented. A future pause-resume driver (or a post-payment `Paused`/`PayAmountChoice` bail) would close it.
3. **ai-gate WARN unchanged** (affinity-mirror avg-turn drift, 0 flips, Hawkeye-independent scenario) — sample/seed noise, re-confirm on a paired-seed run if desired.
4. **End-to-end coverage flips (A-ii/A-iv/B-v) remain NOT verified in-worktree** (symlinked `data/` must not be regenerated here). Unchanged from the original report — orchestrator `card-data` Tilt job confirms Ruinous→true, Hawkeye→true, Frillback→false, no card turns red.
