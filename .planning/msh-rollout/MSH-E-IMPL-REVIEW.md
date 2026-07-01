# MSH-E Implementation Review — Hawkeye, Master Marksman + The Ruinous Wrecking Crew

Reviewer: `mshe-impl-reviewer` (independent; did NOT write this code).
Diff: `git -C /home/lgray/vibe-coding/wt-msh-f diff aa4e88ec4` · branch `feat/msh-e-marksman-ruinous`.
Skill: `/review-impl` (Engine-Implementer Matrix Mode — executor matrix present in IMPL-REPORT).

**Maintainer-Simulation Gate: FAIL** — the K-counter row's serialized-surface / invalidation claim
("counter ZERO wire surface … serialize→deserialize roundtrip equality preserved") is contradicted
by the diff: K (`optional_cost_payments_this_resolution`) must survive the per-iteration
`OptionalEffectChoice` pauses, which cross `apply()` boundaries that the persistence layer serializes
(see HIGH-1). The matrix row treats K like a within-one-apply transient; it is not.

---

## Findings

**[HIGH]** Repeated-payment count K is `#[serde(skip)]` but must survive cross-`apply()` pauses; it is
lost on any serde boundary crossed mid-payment-loop, collapsing the modal cap below the payments
actually made (CR 700.2d). Evidence: `crates/engine/src/types/game_state.rs:6827`
(`#[serde(skip, default)] optional_cost_payments_this_resolution`), eq-excluded at `:8157-8161`. The
field is incremented in `resolve_repeated_optional_payment_choice` across **separate** `apply()`
calls — each `DecideOptionalEffect` fires the next `WaitingFor::OptionalEffectChoice` and returns
(`crates/engine/src/game/effects/mod.rs:4060-4073`), so K is nonzero **at the pause, between
apply() calls**, until the cap is read in `finish_repeated_optional_payment` →
`begin_pending_trigger_target_selection` → `modal_choice_for_player`
(`crates/engine/src/game/ability_utils.rs:571-573`). The sibling continuation
`pending_repeated_optional_payment` IS serialized across that very pause
(`game_state.rs:6485`, `skip_serializing_if = "Option::is_none"`), but K is not — so a state
persisted/restored between payment prompts restores the continuation with K reset to 0. Persistence
round-trips via serde: server `to_persisted`/`from_persisted`
(`crates/server-core/src/session.rs:590,611`) driven by `persist_session_async`
(`crates/phase-server/src/main.rs:2474,2858,2993,…`), and single-player save/load + multiplayer
host-resume (`crates/engine-wasm/src/lib.rs:1146,1201`). The code comment's justification is
factually wrong on two counts: (1) "nonzero only mid-`apply()`" — it is nonzero **across**
apply boundaries at the payment pauses; (2) "reconstructed by re-resolving" — K is **not**
reconstructable (the prior pay/decline choices are gone). The claimed analog `static_gate_truth` is
*derived* state (recomputed by `refresh_static_gate_truth`); the **true** analog is
`exiled_from_hand_this_resolution` (`game_state.rs:6808`, `skip_serializing_if = "is_zero_u32"`,
eq-INCLUDED at `:8159`) — a resolution-local counter the codebase already chose to serialize
*precisely because* it can be observed at a pause. Why it matters: a server crash/restart (the
explicit purpose of the SQLite persistence layer) or a save/load landing between Hawkeye's payment
prompts under-counts K — the player paid the mana but the reflexive modal offers fewer modes than
paid for, a CR 700.2d violation. Suggested fix: mirror `exiled_from_hand_this_resolution` —
`#[serde(default, skip_serializing_if = "is_zero_u32")]` and INCLUDE K in `PartialEq`
(serialize-when-nonzero round-trips faithfully, so eq stays roundtrip-safe); correct the comment.

**[MED]** `is_repeated_optional_payment` admits **any** `Effect::PayCost` cost type, but the driver
assumes the payment is synchronous; a pausing cost corrupts the resolution. Evidence: the predicate
matches `matches!(ability.effect, Effect::PayCost { .. })` with no cost-shape restriction
(`crates/engine/src/game/effects/mod.rs:3966-3974`). Resolution-time `PayCost` can pause: it sets
`WaitingFor::PayAmountChoice` for pay-any-amount mana/energy/life (`crates/engine/src/game/effects/pay.rs:82,109,138`)
and returns `PaymentOutcome::Paused` with a prepended pay-cost continuation for interactive
`DiscardChoice` / replacement-effect payments (`pay.rs:245-254`). The driver does not detect a
paused payment: after `resolve_ability_chain(&payment_unit, …, 1)` it increments K whenever
`!cost_payment_failed_flag` (which is the case for a *pause*, not just success) and then **overwrites**
`state.waiting_for` with the next `OptionalEffectChoice`/reflexive
(`effects/mod.rs:4051-4073`), discarding the payment's own continuation. Why it matters: this is the
exact class the feature advertises ("you may pay {cost} up to N times. When you do, …"); a future
"you may discard/sacrifice a card up to N times" card matches the predicate and would mis-count K and
clobber the interactive payment pause — silently wrong resolution. Latent today only because
Hawkeye's `{1}` mana auto-tap does not pause. Suggested fix: narrow the predicate to non-pausing mana
costs (e.g. `AbilityCost::Mana` without an unannounced X), or detect a paused payment / prepended
pay-cost continuation after the payment and bail before incrementing K — at minimum tighten the
predicate so coverage stays honest about the class it actually handles.

**[LOW]** Targeted modes (Net / Explosive) are never exercised end-to-end; the report overclaims.
Evidence: `crates/engine/src/game/marksman_tests.rs:256-284` (`boomerang_mode_resolves_once_through_apply`)
drives only mode index 2 (Boomerang, no target) through `SelectModes`. The IMPL-REPORT coverage map
and plan B-iv claim "Each mode resolves correctly (Net ⇒ CantBlock, Explosive ⇒ −2 life, Boomerang
⇒ net 0)". The reflexive→`SelectModes`→target-selection hand-off for a *targeted* mode under the new
driver is unverified (it reuses shared infra, so likely correct, but untested for this path). Why it
matters: the report's per-mode claim is not backed by a test. Suggested fix: add a
`SelectModes{indices:[1]}` (Explosive) case asserting P1 loses 2 life, or scope the report claim to
Boomerang.

**[LOW]** `detect_modal_dynamic_max_dropped` can false-RED an unrelated card carrying both a fixed
modal node and a separate non-modal "choose up to X <nouns>" clause. Evidence:
`crates/engine/src/parser/swallow_check.rs:1570-1595` tests whole-AST `"modal":{` and whole-text
`"choose up to x"` independently, so a card with an unrelated fixed modal (ability A) plus a
non-modal "choose up to X creatures" selection clause (ability B) would fire. This is conservative
(false-RED understates coverage, never false-greens), and narrow. Why it matters: minor coverage-
honesty imprecision in the safe direction. Suggested fix: acceptable as-is; note for future
tightening if such a card appears.

---

## Mandatory obligation results

1. **Discriminating runtime test — CONFIRMED.** `marksman_tests.rs` drives K∈{0,1,2,3} through the
   production path: trigger resolved via `resolve_ability_chain(..., 0)` (the same seam production
   uses at `engine_stack.rs:364`), then every `DecideOptionalEffect`/`SelectModes` through `apply`.
   Modal cap = `min(K, mode_count)` is asserted by reading `WaitingFor::AbilityModeChoice.modal`
   (`pay_three…`→(0,3), `pay_twice_then_decline…`→(0,2), `k_counts…`→(0,1), `decline_immediately…`→
   none). Ruinous line-counter fold tested on real oracle text in
   `coverage.rs::count_effective_oracle_lines_folds_dynamic_modal_headers` (Ruinous→2, "that many"→1).

2. **Production-path coverage map — CONFIRMED (with the LOW-1 overclaim).** Every K-sweep behavior
   reaches the real driver via `apply`; the detector registration is exercised through
   `parse_oracle_text → check_swallowed_clauses` (`..._registered_via_real_parse`, which correctly
   yields a real modal node with no `dynamic_max_choices` via the "where X is" guard). The "each mode
   resolves" row is only Boomerang (LOW-1). No runtime-semantics claim rests on a shape-only test.

3. **Maintainer-simulation matrix — REFUTED on the serde/eq row (HIGH-1).** `pending_repeated_optional_payment`
   serde (serialize-when-`Some`, eq-included) correctly mirrors `pending_repeat_iteration` — CONFIRMED.
   The K-counter row claiming zero-wire-surface safety is contradicted: K must survive the same pause
   the continuation survives. The report mirrored the wrong sibling (`static_gate_truth`, derived)
   instead of `exiled_from_hand_this_resolution` (resolution-local counter, serialized-when-nonzero).

4. **Parser coverage honesty — CONFIRMED.** The `"choose up to that many"` arm
   (`oracle_modal.rs:1730-1744`) accepts only the period/bare/bullet-terminated form;
   `not(preceded(multispace0, alt((em-dash, en-dash, hyphen, alpha1))))` rejects Frillback's em-dash
   header and the non-modal noun-phrase clauses (verified by tracing each form + the unit test
   `..._em_dash_and_noun_are_not_dynamic`). Only Hawkeye's period form gets the dynamic cap, and
   Hawkeye has a working WhenYouDo driver (proven by the runtime tests reaching it), so no card is
   given a cap without a driver. All new parser code is nom combinators; the swallow-detector
   `contains` scans are annotated audit code, not parsing dispatch (CI parser gate passed).

5. **CR annotations — CONFIRMED.** 603.12a, 700.2/700.2a/700.2b/700.2d, 117.1, 118.1, 608.2c,
   107.3m, 601.2 all resolve in `docs/MagicCompRules.txt` and describe the annotated code. CR 603.12a
   is verbatim the Hawkeye rule ("paying that cost one or more times causes the reflexive triggered
   ability to trigger only once").

6. **Building-block / architecture — CONFIRMED (with MED-1 caveat).** `QuantityRef::TimesCostPaidThisResolution`
   is a correct leaf reference (no mixed-layer `Fixed` payload), a legitimate new sibling of the
   cast-time `*Count` refs across a distinct CR section (603.12a resolution-local vs 601.2 cast-time);
   exhaustive matches compiler-enforced across the 4 classification arms + value arm.
   `ModalCountSpec::DynamicCostX → Dynamic { qty }` is the correct parameterization (`Copy` dropped).
   The driver (`drive_/resolve_/finish_repeated_optional_payment`) generalizes to the class "repeated
   optional PayCost + WhenYouDo reflexive" — but the predicate over-admits pausing costs it cannot
   correctly drive (MED-1). No mega-effect, no verbatim-string matching, engine owns the logic,
   frontend untouched.

7. **Non-vacuity — CONFIRMED (all 4 measured discriminators hold).** Line-counter (revert
   `DYNAMIC_CHOOSE_HEADERS` ⇒ Ruinous 6≠2): real. Detector registration (revert the
   `check_swallowed_clauses` call ⇒ no diagnostic on the real-parse pipeline): real. K=0 gate
   (revert `K>=1` ⇒ `begin_pending_trigger_target_selection` returns `Some(AbilityModeChoice{max:0})`
   at line 5318 because Hawkeye's modes have legal targets so `unavailable_modes.len() < mode_count`,
   making `modal_cap()` `Some((0,0))` and the `.is_none()` assertion fail — independently verified,
   not vacuous). Increment-guard (revert `if !cost_payment_failed_flag` ⇒ the unfunded 2nd payment
   counts, K=2≠1; verified the unfunded payment fails synchronously with no lands in the scenario).

---

VERDICT: CHANGES REQUIRED (1 high, 1 medium, 2 low)
