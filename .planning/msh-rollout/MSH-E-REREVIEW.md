# MSH-E Re-Review (post-fix-round) — Hawkeye, Master Marksman + The Ruinous Wrecking Crew

Reviewer: `mshe-rereviewer` (independent; did NOT write this code, did NOT write the first review or the fix round). Adversarial re-review per `/review-impl`.
Worktree: `/home/lgray/vibe-coding/wt-msh-f` · branch `feat/msh-e-marksman-ruinous` · base `aa4e88ec4`.
Diff: `git -C /home/lgray/vibe-coding/wt-msh-f diff aa4e88ec4` (11 files, +1191/−25).
Cargo run DIRECTLY in this isolated worktree. No commit.

**VERDICT: CLEAN** — all 4 prior findings genuinely resolved (each discriminator re-measured by temporary revert), zero new blocker/high, all 7 obligations confirmed. Two new LOW notes below (neither blocks).

---

## Verification gates (cargo-direct, this worktree)

| Gate | Result |
|---|---|
| `cargo fmt --check` | clean (exit 0) |
| `cargo test -p engine --lib` (full) | **13830 passed · 0 failed · 7 ignored · 0 filtered** (matches report) |
| `cargo clippy -p engine --lib --tests -- -D warnings` | **clean (exit 0)** |
| marksman_tests (8) baseline | all 8 green |
| Worktree restored after reverts | diffstat back to 11 files / +1191 / −25 (verified) |

---

## Prior-finding resolution (each non-vacuity MEASURED by temporary revert + restore)

### Fix 1 (HIGH) — K serde/eq correctness — RESOLVED ✓
- **(a) mirrors `exiled_from_hand_this_resolution`:** CONFIRMED. K (`game_state.rs:6837-6838`) carries `#[serde(default, skip_serializing_if = "is_zero_u32")]` — byte-identical to `exiled_from_hand_this_resolution` (`:6808-6809`) — and is eq-INCLUDED in `PartialEq` (`:8172-8176`), placed next to the `exiled_from_hand` eq line. `is_zero_u32` helper exists (`:48`). The paired continuation `pending_repeated_optional_payment` is `#[serde(default, skip_serializing_if = "Option::is_none")]` (`:6481`) and eq-included (`:8127`).
- **(b) test non-vacuous:** MEASURED. Reverted K's attr to `#[serde(skip)]`, re-ran `k_counter_survives_serde_roundtrip_mid_payment_loop` → **FAILED** `left: 0, right: 2` ("K must survive the mid-payment-loop serde boundary"). Restored → green.
- **(c) eq-include did not break at-rest roundtrip equality:** CONFIRMED. Full 13830-test suite (which includes the serde/partial_eq/roundtrip subset) is green with K eq-included. At rest K=0 / continuation=None, so they are omitted on the wire and never perturb equality except mid-loop (where the difference is intended).
- **(d) doc comment factually correct:** CONFIRMED. The doc (`:6811-6838`) now states K is nonzero AT the per-iteration pause across separate `apply()` calls (serde boundary: `to_persisted`/`from_persisted`, save/load, host-resume), mirrors `exiled_from_hand_this_resolution` (resolution-local, observable at a pause), NOT `static_gate_truth` (derived/recomputable). The prior false claims ("nonzero only mid-`apply()`", "reconstructed by re-resolving") are gone.

### Fix 2 (MED) — predicate narrowing — RESOLVED ✓
- **(a) still drives Hawkeye:** CONFIRMED. The runtime sweep tests (`pay_three_times…`, `pay_twice_then_decline…`, `k_counts_only_successful…`) reach the driver through the real parsed Hawkeye AST → the narrowed `is_synchronous_mana_pay_cost` admits Hawkeye's pure `{1}`. Positive control in the unit test also passes.
- **(b) rejects pausing/X/scaled costs:** CONFIRMED by reading the body (`effects/mod.rs:4000-4009`): matches ONLY `Effect::PayCost { cost: AbilityCost::Mana { cost }, scale: None, .. } if !cost_has_x(cost)`. X-mana pauses via `WaitingFor::PayAmountChoice` (`pay.rs:77→82`, guarded by `cost_has_x`); interactive Discard/replacement returns `PaymentOutcome::Paused` (`pay.rs:245`); non-mana costs are not the `Mana` variant; scaled costs have `scale: Some`. All four classes are excluded, so no pausing cost can reach the synchronous-only driver. `cost_has_x` (`casting_costs.rs:8169`) checks `ManaCostShard::X`.
- **(c) negative test non-vacuous:** MEASURED. Widened the body back to `matches!(effect, Effect::PayCost { .. })`, re-ran `repeated_optional_payment_admits_only_synchronous_mana_cost` → **FAILED** at "X mana prompts PayAmountChoice — must NOT be driven". Restored → green.

### Fix 3 (LOW) — targeted-mode coverage — RESOLVED ✓
`explosive_targeted_mode_resolves_once_through_apply` drives a TARGETED reflexive mode through the real `apply` route: `SelectModes{[1]}` → `WaitingFor::TriggerTargetSelection` → `ChooseTarget{Player(P1)}` → `advance_until_stack_empty()`, asserting P1 `20→18` exactly. Because the assertion is `assert_eq!(p1_life, 18)`, it discriminates BOTH the unresolved case (20) AND a per-payment 2×K bug (14). **MEASURED:** removing `advance_until_stack_empty()` → **FAILED** `left: 20, right: 18`. Restored → green.

### Fix 4 (LOW) — detector honesty — RESOLVED ✓
`detect_modal_dynamic_max_dropped` doc (`swallow_check.rs`) now carries the explicit **CONSERVATIVE-RED LIMITATION** paragraph. The detector fires only when all three gates hold (dynamic header marker + `"modal":{` node + NO `"dynamic_max_choices":{`), so it errs toward RED (understates coverage, never false-green). Negative tests confirm silence when the dynamic cap is present (Ruinous) and when no modal node exists (Heroic Feast / Temporal Firestorm). Registered at `check_swallowed_clauses:118`, after `detect_apnap`.

---

## Mandatory obligations (1–7)

1. **K serde/eq** — CONFIRMED (a/b/c/d above; non-vacuity measured).
2. **Predicate narrowing** — CONFIRMED (Hawkeye drives; X/scaled/discard rejected; non-vacuity measured). Caveat → NEW-2 below (coverage layer, not the driver).
3. **Targeted-mode coverage** — CONFIRMED (non-vacuity measured; assertion also catches 2×K).
4. **Detector honesty** — CONFIRMED (errs RED; limitation documented).
5. **No regression** — CONFIRMED: 13830/0/7, clippy clean, fmt clean.
6. **CR annotations** — CONFIRMED with one LOW: CR 118.1 (`docs/MagicCompRules.txt:968` "To pay a cost, a player carries out the instructions…") genuinely describes paying a cost and is the correct, defensible choice for the new mana-payment annotations. All added CR numbers resolve (603.12a, 608.2c, 700.2, 700.2b, 700.2d, 107.3m, 118.1, 120.3, 601.2). One residual mis-citation → NEW-1.
7. **No working code reverted / no new debt** — CONFIRMED. Fixes are surgical: Fix 1 = serde attr + eq line + doc; Fix 2 = one new helper gating the predicate; Fix 3 = one new test; Fix 4 = doc only. The K-sweep production path (`drive_/resolve_/finish_repeated_optional_payment`), the B4 parser negative-lookahead keeping Tranquil Frillback RED (measured: em-dash form excluded), and the Ruinous line-counter fold are all intact. `engine_payment_choices.rs` change correctly checks `pending_repeated_optional_payment.is_some()` before the generic `pending_optional_effect.take()`.

---

## Findings (both LOW — neither blocks CLEAN)

**[LOW]** CR 117.1 (priority) is mis-cited on cost-payment/pool-draining code. Evidence: `effects/mod.rs:4082` (`// CR 117.1 + CR 118.1: pay the cost…`) and `marksman_tests.rs:183` (`// CR 117.1: the three {1} payments drained the pool.`) — both added in this diff. CR 117.1 (`docs/MagicCompRules.txt:926`) governs *priority*; during resolution the ability instructs the payment, so priority does not apply. The first review accepted 117.1 as "describing the code"; it does not. Why it matters: a wrong CR number is worse than none (false confidence the code was verified against the right rule); CLAUDE.md requires the cited rule to describe the code. Impact is low — one is a test comment, the production one is paired with the correct 118.1, and neither affects behavior. Suggested fix: drop `CR 117.1`; use `CR 118.1` ("pay the cost") and `CR 118.3a` ("Paying mana is done by removing the indicated mana from a player's mana pool") for the pool-drain comment.

**[LOW]** The B4 parser arm emits the `TimesCostPaidThisResolution` dynamic cap for ANY "choose up to that many." (period/bullet-terminated) modal header regardless of cost class, while the runtime driver (`is_synchronous_mana_pay_cost`) only handles pure static mana. Evidence: `oracle_modal.rs:1655-1666` `scan_modal_count_override` is cost-agnostic; `effects/mod.rs:4000` is mana-only. Today this is **empty** (measured: the 4 corpus cards with "choose up to that many" are Hawkeye=period+synchronous-mana=driven, Heroic Feast & Suppression Ray=noun-phrase=rejected by the `alpha1` lookahead, Tranquil Frillback=em-dash=rejected). The latent risk is false-green direction: a FUTURE card with "choose up to that many." + a `WhenYouDo` reflexive + a non-mana or X-mana repeated cost would parse a modal WITH `dynamic_max_choices` (so `Modal_DynamicMaxDropped` stays silent ⇒ marked supported) yet fall to the generic `repeat_for` path and resolve the modal per-iteration with the wrong cap. Why it matters: this is the exact "fall through to generic path and stay honestly unimplemented" guarantee obligation 2 relies on — it holds for the driver but NOT at the coverage layer for the non-mana subclass. Suggested fix (forward-looking, not required now): when the dynamic cap is `TimesCostPaidThisResolution`, have coverage cross-check the carrying ability's cost is synchronous mana (else flag swallowed), OR track this as a known gap until the driver gains pause-resume plumbing.

---

## Conclusion

All four prior findings are genuinely resolved, each backed by a discriminating test whose non-vacuity I re-measured by reverting the fix and observing the named failure (K=0; X-mana satisfies; P1=20). The fixes introduced no regression (13830/0/7, clippy clean, fmt clean) and are surgical. The two new findings are LOW (one CR-annotation accuracy nit the first review missed; one empty-today latent coverage-honesty divergence). Neither is a blocker or high.

VERDICT: CLEAN
