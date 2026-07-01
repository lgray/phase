# MSH-E Implementation Report — Hawkeye, Master Marksman + The Ruinous Wrecking Crew

Worktree: `/home/lgray/vibe-coding/wt-msh-f` · branch `feat/msh-e-marksman-ruinous` · base `aa4e88ec4`.
Plan: `.planning/msh-rollout/MSH-E-PLAN-r2.md` (r2 review-resolution section authoritative).
Status: **Sub-plan A COMPLETE + verified. Sub-plan B COMPLETE + verified.** No commits, no `git add` (left staged-as-modified).

Cargo run DIRECTLY in this isolated, non-Tilt-watched worktree (as instructed). `data/*.json` are symlinks to the Tilt-watched main worktree and were NOT regenerated (multi-agent safety + plan deviation #6); the end-to-end `coverage-data.json` flips are the orchestrator's `card-data` Tilt job.

---

## Verification counts (cargo-direct, this worktree)

| Gate | Result |
|---|---|
| `cargo fmt` | clean |
| `cargo build -p engine --lib` | OK (exit 0) |
| `cargo clippy -p engine --lib -- -D warnings` | clean (exit 0) |
| `cargo test -p engine --lib` (full) | **13827 passed · 0 failed · 7 ignored** |
| `./scripts/check-parser-combinators.sh` | PASS (exit 0) |
| Parser diff-gate inline grep | only annotated swallow-detector marker scans (allowed) |
| `cargo coverage` | exit 0 (read-only; reflects stale base `card-data.json`) |
| `cargo semantic-audit data/` | exit 0 (reflects stale base; local artifacts removed) |
| `cargo ai-gate` (B mandate) | **0 FAIL · 1 WARN · 2 PASS · 0 NEW · 0 REMOVED** (exit 0) |
| CR-annotation diff gate | **8/8 verified · 0 UNVERIFIED** |

New tests added: **13** (7 for A, 6 for B), all passing. Non-vacuity measured by temporary revert for the four load-bearing discriminators (below).

---

# SUB-PLAN A — The Ruinous Wrecking Crew (coverage-audit fix)

### Per-file diff summary
- `crates/engine/src/game/coverage.rs`
  - `is_modal_header_line` (~5466–5480): added `DYNAMIC_CHOOSE_HEADERS = ["choose up to x", "choose up to that many"]` recognized as a class after the existing `CHOOSE_PHRASES`. CR 700.2 + CR 107.3m. (`game/` line-classifier; parser gate does not scan this dir.)
  - `quantity_ref_human_label` (1487) + `quantity_ref_feature` (6455): new `TimesCostPaidThisResolution` arms (B-shared, see B).
  - New test `count_effective_oracle_lines_folds_dynamic_modal_headers`.
- `crates/engine/src/parser/swallow_check.rs`
  - New detector `detect_modal_dynamic_max_dropped` (1570) registered in `check_swallowed_clauses` (118, after `detect_apnap`). Fires iff (1) `cleaned` has `"choose up to that many"`/`"choose up to x"`; (2) `ast_json` contains `"modal":{`; (3) `ast_json` does NOT contain `"dynamic_max_choices":{`. CR 700.2 + CR 700.2d. Marker scans annotated `// allow-noncombinator` inline (matches sibling detectors).
  - 6 new tests (3 direct synthetic-JSON gate tests + 3 real-parse pipeline tests).

### Tests added (A) + revert-fail discriminators
| Test | Drives | Revert-fail discriminator |
|---|---|---|
| `count_effective_oracle_lines_folds_dynamic_modal_headers` (coverage.rs) | `count_effective_oracle_lines` | Drop `DYNAMIC_CHOOSE_HEADERS` arm ⇒ Ruinous text returns **6 not 2** (MEASURED: `left:6 right:2`), "that many" returns 4 not 1. |
| `modal_dynamic_max_dropped_fires_on_modal_without_dynamic_cap` | `detect_modal_dynamic_max_dropped` (synthetic ast_json) | Remove the `diagnostics.push` / any gate ⇒ no diagnostic. |
| `modal_dynamic_max_dropped_silent_when_dynamic_cap_present` | detector | Revert gate (3) ⇒ fires on a card that captured the cap (Ruinous). |
| `modal_dynamic_max_dropped_silent_without_modal_node` | detector (A1 gate) | Revert gate (2) ⇒ false-fires on non-modal selection clauses (Heroic Feast / Temporal Firestorm). |
| `modal_dynamic_max_dropped_registered_via_real_parse` | `parse_oracle_text` → `check_swallowed_clauses` (real pipeline) | Remove the registration line at swallow_check.rs:118 ⇒ MEASURED: only `DynamicQty` fires, not `Modal_DynamicMaxDropped`. (Uses a "choose up to X, where X is …" modal that B's parser arm does NOT touch — stable across both sub-plans.) |
| `modal_dynamic_max_dropped_silent_on_ruinous` | real parse | Ruinous carries the cap → silent (greened by the line-counter fold). Stable. |
| `modal_dynamic_max_dropped_silent_on_non_modal_selection_clauses` | real parse | Heroic Feast / Temporal Firestorm (no modal node) → silent. No-regression guard. |

The positive detector + registration tests use synthetic JSON / a B-stable "where X is" modal so they remain green across **both** A and B (Hawkeye flips to green under B; a real-Hawkeye "fires" test would have been fragile).

### A end-to-end (deferred to orchestrator Tilt)
A-ii / A-iv (`coverage-data.json` shows Ruinous→`true`, Hawkeye+Frillback→`false`, Heroic Feast/Temporal Firestorm stay `true`, ONLY Ruinous flips green) require a `card-data` regen that would clobber the shared symlinked `data/`. Confirmed via the **`card-data` Tilt resource**, not in-worktree. The in-worktree proof is the unit + real-parse tests on the exact card texts above.

---

# SUB-PLAN B — Hawkeye, Master Marksman (engine + parser feature)

### Per-file diff summary
- `crates/engine/src/types/ability.rs:4529` — `QuantityRef::TimesCostPaidThisResolution` variant (CR 603.12a doc). LOW-1 handled in parser.
- `crates/engine/src/game/quantity.rs`
  - `2899`: value-resolution arm reads `state.optional_cost_payments_this_resolution`.
  - `410`/`659`/`843`: three exhaustive classification arms (`uses_unspent_mana` / `uses_object_count` / entry-perturbation) → `false`, mirroring `ExiledFromHandThisResolution` (resolution-local, never snapshotted/cached).
- `crates/engine/src/game/triggers.rs:6586` — `quantity_ref_refs_cost_paid_object` → `false`.
- `crates/engine/src/game/coverage.rs:1487`/`6455` — human label + `quantity_ref_feature => Handled`.
- `crates/engine/src/types/game_state.rs`
  - `6828`: `optional_cost_payments_this_resolution: u32`, `#[serde(skip, default)]`, eq-EXCLUDED (mirrors `static_gate_truth`, NOT `exiled_from_hand_this_resolution`); init `0` (7684); exclusion comment in `PartialEq` (8160).
  - `994`: `PendingRepeatedOptionalPayment { payment_unit, reflexive, remaining }` struct; field `6485` (`#[serde(default, skip_serializing_if = "Option::is_none")]`), init `7642`, eq-included `8116`.
- `crates/engine/src/parser/oracle_modal.rs`
  - `1661`: `ModalCountSpec` — dropped `Copy` (LOW-1), `DynamicCostX` → `Dynamic { qty: QuantityRef }`.
  - `604`: count-spec map `Dynamic { qty } => (0, usize::MAX, Some(Ref { qty }))`.
  - `1713`/`1730`: `scan_modal_count_override` — CostXPaid arm + NEW `"choose up to that many"` arm matching PERIOD/bare/bullet-terminated form via `not(preceded(multispace0, alt((dashes, alpha1))))` (rejects em-dash + noun-phrase). MED-1.
- `crates/engine/src/game/effects/mod.rs`
  - `3966` `is_repeated_optional_payment`; `3982` `drive_repeated_optional_payment`; `4028` `resolve_repeated_optional_payment_choice` (`pub(super)`); `4085` `finish_repeated_optional_payment`.
  - `4793`: depth==0 prelude clears `optional_cost_payments_this_resolution` (next to `exiled_from_hand_this_resolution`).
  - `5560`: up-front optional gate suppressed for the class (`&& !is_repeated_optional_payment`).
  - `5611`: driver dispatch before the generic `repeat_for` loop.
- `crates/engine/src/game/engine_payment_choices.rs:37` — `handle_optional_effect_choice` routes to `resolve_repeated_optional_payment_choice` when `pending_repeated_optional_payment` is set (else-if before the generic path); shared `set_active_priority`/resume tail unchanged.
- `crates/engine/src/game/mod.rs:82` + `crates/engine/src/game/marksman_tests.rs` — new test module (6 tests).

### Driver semantics (HIGH-1/HIGH-2/MED-1/MED-3/B3 committed decisions)
- Per-iteration `OptionalEffectChoice`; K accumulates only successful payments; decline stops early. Resolution-time mana payment is **synchronous** (`costs.rs` auto-tap path returns Paid/Failed, never Paused) ⇒ flag is read immediately after the payment, no extra drain plumbing.
- Reflexive resolved by **direct sub-resolution at depth ≥ 1** after the loop (HIGH-1); the driver's `K >= 1` skip in `finish_repeated_optional_payment` is the SOLE zero-case authority. **`effects/mod.rs:6885` WhenYouDo gate UNCHANGED** (the reflexive's `GenericEffect` makes it flag-independent).
- Depth ≥ 1 for the reflexive is REQUIRED so the depth==0 prelude doesn't wipe K before `modal_choice_for_player` (`ability_utils.rs:572`) reads it as the dynamic cap (CR 700.2d clamp to `mode_count`).

### Tests added (B) + revert-fail discriminators
| Test | Drives (production path) | Revert-fail discriminator |
|---|---|---|
| `hawkeye_modal_parses_with_resolution_local_dynamic_cap` | `parse_oracle_text` → trigger→reflexive modal | Drop the `"that many"` arm / `Dynamic { qty }` ⇒ modal stays `(1,1)`, `dynamic_max_choices == None`. (Also covered by `oracle_modal` unit tests below.) |
| `pay_three_times_caps_modal_at_three_and_offers_reflexive_once` | resolve trigger via `resolve_ability_chain` (depth 0) → **3× `DecideOptionalEffect` via `apply`** | K==3, modal cap `(0,3)`, pool drained 3→0. Revert K-increment ⇒ K=0 cap 0; revert B4 dynamic-max ⇒ cap pinned `(1,1)`. |
| `pay_twice_then_decline_caps_modal_at_two` | apply: accept,accept,decline | K==2, cap `(0,2)`, loop stops early. |
| `decline_immediately_skips_reflexive_at_k_zero` | apply: decline | **MEASURED revert**: neutralize the `K >= 1` gate ⇒ an `AbilityModeChoice` (cap 0) appears at K=0 ⇒ test fails. Proves the K-gate is the sole zero authority. |
| `k_counts_only_successful_payments` | apply: accept(funded),accept(unfunded),decline | **MEASURED revert**: neutralize the `if !cost_payment_failed_flag` guard ⇒ K=2 not 1 (`left:2 right:1`) ⇒ cap `(0,2)` not `(0,1)` ⇒ fails. Proves the per-iteration flag-clear + increment-guard K accounting. |
| `boomerang_mode_resolves_once_through_apply` | apply: 3×accept, **`SelectModes` via apply** | Boomerang (discard 1 + draw 1) → hand net 0 AND no second `AbilityModeChoice`. Reverting to the generic `repeated_full_chain` (modal per payment) ⇒ modal offered 3×. |

`oracle_modal.rs` unit tests added: `parse_modal_choose_count_up_to_that_many_period_is_dynamic` (period/bare → `Dynamic { TimesCostPaidThisResolution }`) and `parse_modal_choose_count_up_to_that_many_em_dash_and_noun_are_not_dynamic` (Frillback em-dash + Heroic-Feast noun-phrase → fixed default). Existing `..._up_to_x_is_dynamic` updated to `Dynamic { CostXPaid }`.

### Serialized-surface delta (B) — counter is ZERO wire surface
- `optional_cost_payments_this_resolution: u32` carries `#[serde(skip, default)]` → **never serialized** (zero wire surface) and eq-excluded → serialize→deserialize roundtrip equality preserved. Confirmed by the full suite (which includes serde roundtrip tests) staying green.
- `pending_repeated_optional_payment` / `PendingRepeatedOptionalPayment` use `#[serde(default, skip_serializing_if = "Option::is_none")]` (serialize only when `Some`, mirroring `pending_repeat_iteration`/`pending_continuation`) so a save mid-decision survives — eq-included like its siblings. No card-data migration risk (new optional `Some` only on the new card; absent ⇒ default `None`).

### B end-to-end (deferred to orchestrator Tilt)
B-v (after B's parser emits `dynamic_max_choices`, the A-2 detector no longer fires on Hawkeye ⇒ Hawkeye `supported:true`, Frillback stays red) requires a `card-data` regen (orchestrator Tilt). In-worktree proof: `hawkeye_modal_parses_with_resolution_local_dynamic_cap` (Hawkeye now carries the cap ⇒ A-2 gate (3) silent) + the em-dash parser test (Frillback's modal stays Fixed, no cap ⇒ A-2 still fires ⇒ red).

---

## Discriminating-test gate — production-path coverage map

| Behavioral claim | Changed seam/fn | Production entry | Test reaching it | Revert-failing assertion | Sibling/negative |
|---|---|---|---|---|---|
| Dynamic modal header folds bullets | `is_modal_header_line` (coverage.rs:5478) | `count_effective_oracle_lines` (real text) | `count_effective_oracle_lines_folds_dynamic_modal_headers` | count==2 (revert ⇒ 6, MEASURED) | non-modal 0-bullet unchanged; fixed `up to two` still folds |
| Dropped-cap modal stays unsupported | `detect_modal_dynamic_max_dropped` + registration (swallow_check.rs:118/1570) | `parse_oracle_text`→`check_swallowed_clauses` | `..._registered_via_real_parse` | diagnostic present (revert registration ⇒ absent, MEASURED) | dynmax-present silent; no-modal silent; Unimplemented early-return |
| Modal cap = min(K, mode_count) | B4 parser + `modal_choice_for_player` (ability_utils.rs:571) | `apply` `DecideOptionalEffect`×K → `AbilityModeChoice` | `pay_three…`/`pay_twice…`/`k_counts…` | cap `(0,3)`/`(0,2)`/`(0,1)` (revert B4 ⇒ `(1,1)`) | cap never exceeds mode_count |
| K captured; decline stops early | `resolve_repeated_optional_payment_choice` (effects/mod.rs:4028) + counter (6828/4793) | `apply` `DecideOptionalEffect` | `pay_twice_then_decline…` | K==2, no 3rd prompt | K==0 path |
| K counts only successful payments | increment guard (effects/mod.rs:4056) | `apply` (unfunded 2nd payment) | `k_counts_only_successful_payments` | K==1 (revert guard ⇒ 2, MEASURED) | — |
| Reflexive fires once iff K≥1 | `finish_repeated_optional_payment` K-gate (effects/mod.rs:4090) | `apply` decline-all / `SelectModes` | `decline_immediately…` / `boomerang…` / `explosive…` | no modal at K=0 (revert gate ⇒ modal at K=0, MEASURED); no 2nd modal | mode resolution end-to-end: Boomerang (net 0) + **Explosive (targeted −2 life via `SelectModes`+`ChooseTarget`, fix-round)**; Net's CantBlock via shared mode infra (no dedicated test) |

> **Fix-round correction (LOW-1 overclaim):** the original "each mode resolves correctly (Net ⇒ CantBlock, Explosive ⇒ −2 life, Boomerang ⇒ net 0)" wording overstated coverage — only Boomerang (no-target) was exercised end-to-end. The fix-round adds `explosive_targeted_mode_resolves_once_through_apply` (Explosive, a TARGETED mode: `SelectModes`→`ChooseTarget`→stack-resolve → P1 −2 life, asserting damage applies exactly once, not 2×K). Net (CantBlock) is still covered only by shared mode-resolution infrastructure, not a dedicated end-to-end test.
| Modal binds resolution-local cap | B4 `oracle_modal.rs` arm (1730) | `parse_oracle_text` | `hawkeye_modal_parses…` + 2 `oracle_modal` unit tests | `dynamic_max_choices == Some(Ref(TimesCostPaidThisResolution))` | em-dash (Frillback) + noun-phrase rejected |

No production-reachable seam is left covered only by a degenerate fixture: the K∈{0,1,2,3} sweep, fail-then-succeed sweep, and a real `SelectModes` mode resolution all run through `apply`. No shape-only test stands in for runtime semantics.

---

## Maintainer-simulation matrix (B-critical rows)

| Seam / first prod branch | Selected authority | Bound value / when | Binding mode | Storage | Consumer | Invalidation | Hostile rows | serde/protocol |
|---|---|---|---|---|---|---|---|---|
| `is_repeated_optional_payment` first hits `ability.optional` then `matches!(PayCost)`/`repeat_for=Fixed`/`sub.condition=WhenYouDo` | controller (`ability.controller`) | the trigger's controller, at trigger resolution | live | — (predicate) | driver dispatch (5611), gate suppression (5560) | n/a (pure predicate) | non-optional / non-PayCost / non-Fixed / non-WhenYouDo ability ⇒ falls to generic loop (UNREACHABLE for driver) | none |
| `resolve_repeated_optional_payment_choice` (`pending_repeated_optional_payment.is_some()`) | repeated-payment count K | incremented per successful payment during resolution | live counter, snapshotted only by `modal_choice_for_player`'s read | `GameState::optional_cost_payments_this_resolution` (`#[serde(skip)]`) | `resolve_quantity(TimesCostPaidThisResolution)` → modal cap | cleared at depth==0 prelude (next top-level resolution) | two Hawkeye taps same turn ⇒ each resolution starts from cleared K (prelude); decline iter2-after-iter1 ⇒ K=1 reflexive still fires | counter ZERO wire surface; pending state serialized when Some |
| reflexive resolution (`finish_…`, depth 1) | the reflexive `sub_ability` (modal) | resolved once after loop iff K≥1 | live read of K via `modal_choice_for_player` | transient (resolved through chain) | `AbilityModeChoice` → `SelectModes` → `build_chained_resolved` | n/a (single resolution) | K==0 ⇒ reflexive skipped (no modal); pay-fail-only ⇒ K=0 ⇒ skipped | `AbilityModeChoice.modal` carries resolved `max_choices` (existing shape) |
| B4 `oracle_modal` arm | the `QuantityRef` bound to the cap | `TimesCostPaidThisResolution` at parse time | snapshotted into AST `dynamic_max_choices` | `ModalChoice.dynamic_max_choices` | `modal_choice_for_player` (CR 700.2d clamp) | n/a (parse-time) | em-dash (Frillback, condition≠WhenYouDo) + noun-phrase (Heroic Feast / "creatures tapped this way" / "where X is") rejected ⇒ no cap ⇒ unhandled cards stay red (no false green) | `dynamic_max_choices` omitted when None (existing serde) |

No global-rescan concern: K is carried on the resolution-scoped counter and read live by the single consumer; the "you may" duration-bound controller is `ability.controller`, not rescanned.

---

## Parser diff gate

`./scripts/check-parser-combinators.sh` → PASS. Inline supplementary grep flags only:
```
+ let has_dynamic_header = cleaned.contains("choose up to that many") // allow-noncombinator: swallow detector marker scan on classified text
+     || cleaned.contains("choose up to x"); // allow-noncombinator: swallow detector marker scan on classified text
```
These are swallow-DETECTOR marker scans on already-classified text (audit code in `swallow_check.rs`, identical pattern to every existing detector) — explicitly annotated non-dispatch structural use. The new parser arm (`oracle_modal.rs`) is a pure `value()/tag()`+`not(preceded(...))` combinator sibling — zero string dispatch.

---

## CR-annotation diff gate
```
git diff | grep '^+' | grep -oE 'CR [0-9]{3}(\.[0-9]+[a-z]?)?' | sort -u → verify each
```
**8/8 verified present in `docs/MagicCompRules.txt`, 0 UNVERIFIED:** CR 107.3m, CR 117.1, CR 118.1, CR 601.2, CR 603.12a, CR 608.2c, CR 700.2, CR 700.2d. Each cited rule also *describes* the annotated code (603.12a = the exact reflexive-once rule for Hawkeye; 700.2d = the dynamic-cap clamp; 107.3m = cast-X ETB for the shared modal-header class; 608.2c = per-iteration flag-clear ordering).

---

## Judgement calls
1. **A-iii positive/registration tests use synthetic JSON / a B-stable "where X is" modal, not real Hawkeye.** Reason: under Sub-plan B, Hawkeye's parser emits `dynamic_max_choices`, so a real-Hawkeye "detector fires" test would flip to failing once B lands. The plan's A-iii row explicitly anticipates this ("build a `ParsedAbilities`…"); I chose inputs stable across both sub-plans so the A commit is independently green AND survives B.
2. **B4 "that many" arm matches PERIOD/bare form only (rejects em-dash).** MED-1 committed this. It is also the load-bearing no-false-green guard: Tranquil Frillback's `"choose up to that many —"` (em-dash) has a reflexive condition that is NOT `WhenYouDo`, so the new driver would not fix its runtime — matching it would give it a cap (detector silent) without a working driver = false green. The negative lookahead keeps Frillback at the fixed default → A-2 detector still fires → Frillback stays red. Verified by `parse_modal_choose_count_up_to_that_many_em_dash_and_noun_are_not_dynamic`.
3. **Dedicated pending state + handler branch (not reusing `pending_repeat_iteration`).** MED-3 scoped the driver as genuinely-new continuation plumbing; the existing drain has no per-iteration-optional-PayCost / K-counter / once-after-loop-reflexive semantics. The dedicated `PendingRepeatedOptionalPayment` keeps the concern isolated and correct.
4. **Synchronous-payment assumption.** Verified (`costs.rs:437-463`): resolution-time mana payment auto-taps and returns Paid/Failed, never `Paused`/`waiting_for`. So the flag is read immediately after the payment; no mid-payment drain hook is needed for the mana-cost class.
5. **Eq-exclude the counter, eq-include the pending state.** The counter is nonzero only mid-`apply()` and is `#[serde(skip)]`, so eq-excluding it (mirroring `static_gate_truth`) preserves roundtrip equality; the pending state mirrors its serializable siblings.

## Deviations from the plan
- The plan predicate text was `matches!(*ability.effect, …)` (for the boxed `AbilityDefinition.effect`); `ResolvedAbility.effect` is unboxed, so the implemented predicate is `matches!(ability.effect, Effect::PayCost { .. })`. Semantically identical.
- The reflexive resolution path resolves the `sub_ability` directly at depth 1 (HIGH-1), leaving `effects/mod.rs:6885` untouched — as committed.
- A-1 recognizer uses the bare/period forms (`"choose up to x"`, `"choose up to that many"`) rather than the em-dash-preferred forms; required so Hawkeye's PERIOD header folds (LOW-2 says the line-counter em-dash preference is harmless, and the A-2 detector is the honesty gate).

## Risks / what `/review-impl` should watch
1. **Pausing non-mana repeated costs out of scope.** The driver assumes synchronous payment (true for mana). A future "you may pay {discard}/{sacrifice} up to N times" card whose payment Pauses would not be handled by this driver. Not in the current corpus; `is_repeated_optional_payment` still matches `PayCost{..}` generally — a follow-up could either tighten the predicate to mana costs or add a pause-resume drain. Flagged, not blocking.
2. **End-to-end coverage flips (A-ii/A-iv/B-v) are NOT verified in-worktree** (symlinked `data/` must not be regenerated here). The orchestrator must run the `card-data` Tilt resource and confirm: Ruinous→`true`; Hawkeye→`true` (B); Frillback→`false`; Heroic Feast & Temporal Firestorm stay `true`; only Ruinous (A) then Hawkeye (B) flip green, zero cards turn red.
3. **ai-gate WARN** (affinity-mirror avg-turn drift +7.9 turns, 0 W→L/L→W flips) is on a scenario that does not exercise the Hawkeye path; it is sample/seed noise, not a policy regression (0 FAIL). Re-confirm on the orchestrator's paired-seed run if desired.
4. **Frillback honesty depends on its reflexive condition not being `WhenYouDo`** — if a future parser change makes Frillback's "when you pay this cost one or more times" parse to `WhenYouDo`, it would (correctly) become driver-eligible, but its em-dash header would still be rejected by B4, so it would stay red. If both change, re-verify.

## Stop-and-return items
**None.** No measured plan-vs-code contradiction was hit. Both sub-plans implemented surgically and verified.

## Not completed in-worktree (by design / multi-agent safety)
- `card-data` / `coverage-data.json` regeneration (orchestrator Tilt job; symlinked shared data must not be clobbered).
- No commit / no `git add` (left staged-as-modified, per instructions).
