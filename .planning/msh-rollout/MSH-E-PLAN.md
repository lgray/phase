# MSH-E Implementation Plan — Hawkeye, Master Marksman + The Ruinous Wrecking Crew

Branch: `feat/msh-e-marksman-ruinous` (worktree `/home/lgray/vibe-coding/wt-msh-f`, off main `5eca83b8c` v0.7.0).
Planning only. All evidence below is measured from the worktree at this commit.

---

## Audit findings (measured — the gap is RUNTIME-classification, not parser)

Both cards: `gap_count=0`, **every** `parse_details` node `supported:true`, yet card
`supported:false`. I traced the per-card support predicate (`analyze_coverage`,
`coverage.rs:4418-4465`). The flip is produced by **`check_silent_drops`**
(`coverage.rs:4458` → impl `5117-5135`), NOT by `check_resolver_features`, NOT by any
swallow detector, NOT by `check_triggers/replacements` (those only flag `Effect::Unimplemented`;
neither card has any).

### The shared mechanism
`check_silent_drops` flags a card when `count_effective_oracle_lines > count_effective_parsed_items`.
`count_effective_oracle_lines` (`coverage.rs:5371`) folds a modal's bullet lines into its
header **only when `is_modal_header_line` recognizes the header**. `is_modal_header_line`
(`coverage.rs:5432-5458`) matches `"choose up to one".."ten"`, `"choose any number"`,
`"choose x."` — but **NOT the dynamic forms `"choose up to X"` or `"choose up to that many"`**.
So the bullets are counted as independent Oracle lines, inflating the count past the parsed-item
count. `count_effective_parsed_items` (`coverage.rs` `fn count_effective_parsed_items`) counts
each top-level item + its DIRECT children (one level deep).

**Hawkeye** — parse_details top level = `[FirstStrike, Reach, Taps-trigger(children=[PayCost])]`
→ `effective_parsed = 1+1+(1+1) = 4`. Oracle lines: `"First strike, reach"`(1),
`"Trick Arrows — …choose up to that many."`(2, header NOT recognized → counted, not folded),
3 bullets(3,4,5) → `effective_oracle = 5`. `5 > 4` → **`SilentDrop:4_of_5`** → `supported:false`.

**Ruinous** — parse_details top level = `[ChangesZone-trigger(children=[Modal]),
Moved-replacement(children=[PutCounter])]` → `effective_parsed = (1+1)+(1+1) = 4`. Oracle lines:
`"…enters with X +1/+1 counters…"`(1), `"When …enters, choose up to X —"`(2, header NOT
recognized), 4 bullets(3,4,5,6) → `effective_oracle = 6`. `6 > 4` → **`SilentDrop:4_of_6`**
→ `supported:false`.

### Why Ruinous is STILL flagged after PR #4186 (`23c50148a`)
#4186 ("dynamic modal max (choose up to X)") correctly taught the **parser** to emit
`modal.dynamic_max_choices = Some(Ref(CostXPaid))` and the **runtime** to resolve+clamp it
(`modal_choice_for_player`, `ability_utils.rs:543-576`; serialized AST confirms
`dynamic_max_choices:{type:Ref, qty:{type:CostXPaid}}`). It **never updated the coverage
silent-drop line-counter** (`is_modal_header_line`) to know `"choose up to X"` is a modal
header. So Ruinous parses correctly, resolves correctly, but the coverage audit still
over-counts its Oracle lines. The #4186 follow-up commit message even names Hawkeye as a
"separate follow-up (PR2)".

### Runtime reality per card (measured — determines false-green risk)
**Ruinous — runtime FULLY WORKS; the gap is purely the coverage marker.** All four ETB modes
and the counters resolve:
- Mode "Destroy target token": `FilterProp::Token => obj.is_token` (`game/filter.rs:3179`),
  consumed by `matches_target_filter` in the Destroy resolver. ✓
- Mode "Each player sacrifices a creature of their choice":
  `Sacrifice{ target:Typed[Creature], count:1, player_scope:Some(All) }`. The Sacrifice path
  iterates matching players in APNAP order and rebinds the controller so EACH player chooses
  from their own permanents (`effects/mod.rs:4037-4090`; test
  `player_scope_all_sacrifice_iterates_each_player`, `sacrifice.rs`). The `Typed[Creature]`
  field is a per-player eligibility filter, not a shared target. ✓
- Mode "Target opponent loses 2 life" / "Discard then draw": standard, supported. ✓
- Replacement "enters with X +1/+1 counters": `PutCounter{ count:Ref(CostXPaid), target:SelfRef }`.
  `cost_x_paid` is stamped on the object at `finalize_cast` (`casting_costs.rs:5506-5510`, CR
  107.3m) and read by `quantity.rs:2837-2843`; Walking Ballista test (`engine.rs:7901-7978`)
  proves cast X → that many +1/+1 counters before SBAs. ✓
- modal `dynamic_max_choices:Ref(CostXPaid)` resolved + clamped to `mode_count` at runtime. ✓

  ⟹ **Ruinous needs only the coverage line-counter fix. No engine/parser change. No false green.**

**Hawkeye — the coverage marker is active, but it MASKS genuine semantic gaps. Fixing only the
coverage marker would produce a FALSE GREEN.** Real AST (`card-data.json`):
`PayCost{ optional:true, repeat_for:Fixed(3),
  sub_ability: GenericEffect{ condition:WhenYouDo,
    modal:{ min_choices:1, max_choices:1, mode_count:3, dynamic_max_choices:None },
    mode_abilities:[CantBlock(target creature), DealDamage(2,target player), Discard+Draw] } }`.
Three measured defects:
1. **Modal max is fixed `(1,1)`**, not "up to that many". The dynamic count (= times `{1}`
   paid) is dropped; the player would be forced to choose exactly one mode.
2. **Structural misparse**: `repeat_for:Fixed(3)` is on the def whose `sub_ability` is the
   modal. `drive_repeat_for_outermost`→`resolve_chain_body` resolves effect **and** its
   sub-chain per iteration (`effects/mod.rs:3991`), so the modal would be re-resolved on every
   payment. CR 603.12a requires the reflexive to resolve **once**.
3. **No QuantityRef** expresses resolution-local "number of times a cost was paid" (every
   `*Paid`/`*Count` ref — `CostXPaid`, `KickerCount`, `AdditionalCostPaymentCount` — is
   cast-time, read from object/cast tallies; `quantity.rs`). Plus a latent **bug**:
   `cost_payment_failed_flag` is not cleared between `repeat_for` iterations
   (`effects/mod.rs ~3988-4008`; contrast the per-iteration clear at `~5078`), so a declined
   earlier payment wrongly suppresses `WhenYouDo` (`effects/mod.rs:6885-6887`).

  ⟹ **Hawkeye needs a real engine+parser feature; the coverage marker must stay RED until that
  feature lands.**

### CR basis (every number grep-verified in `docs/MagicCompRules.txt`)
- **CR 603.12 / 603.12a** (line 2656/2659): reflexive "when you do"; *"if a resolving spell or
  ability includes a choice to pay a cost multiple times and creates a triggered ability that
  triggers when that payment is made, paying that cost one or more times causes the reflexive
  triggered ability to trigger only once."* — exactly Hawkeye.
- **CR 700.2 / 700.2b / 700.2d** (3203/3207/3211): modal triggered ability; controller chooses
  mode(s); can't choose more distinct modes than exist (the dynamic-max ceiling).
- **CR 107.3m** (488): an ETB ability/replacement's X = the spell's chosen cast X — Ruinous.
- **CR 601.2b** (2459) + **700.2a/b**: an illegal mode can't be chosen.
- **CR 701.26** Tap (3518); **CR 701.21** Sacrifice (3449); **CR 119.3** lose life (1065);
  **CR 122.1** counter (1178).

---

## Decision: SPLIT into two independently-committable sub-plans

The two cards' **runtime gaps are independent**: Ruinous has none; Hawkeye has a large one.
They share exactly one root cause (the coverage line-counter), which is small and lands in
Sub-plan A together with a discriminating honesty guard that keeps Hawkeye RED until Sub-plan B
makes it genuinely correct. This yields two clean commits and never ships a false green.

- **Sub-plan A — Ruinous (coverage-only).** Ships first; immediately turns Ruinous green
  (verified correct). Adds the guard.
- **Sub-plan B — Hawkeye (engine + parser feature).** Ships after; turns Hawkeye green with
  correct semantics, at which point the guard clears for Hawkeye automatically.

---

# Sub-plan A — The Ruinous Wrecking Crew (coverage-audit fix)

### Scope
Make the coverage silent-drop line-counter recognize **dynamic modal headers** as a class, and
add one discriminating swallow detector so a card whose dynamic modal max was *dropped* stays
unsupported. Net coverage deltas: Ruinous → `supported:true`; Hawkeye → still `supported:false`
(now held by the principled guard, not by a phrase-list gap).

### Files & changes
1. **`crates/engine/src/game/coverage.rs` — `is_modal_header_line` (5432-5458).**
   Extend recognition to the dynamic header forms. Compose, don't enumerate permutations:
   recognize a `"choose up to "` prefix followed by `"x"`, `"that many"`, or any already-listed
   number word, plus the existing fixed phrases. Keep the existing `&[&str]` list approach
   (this is `game/`, not `parser/` — the nom mandate does not apply here; the function is a
   coverage line-classifier, and matching the file's established `lower.contains(p)` idiom is
   the consistent choice). Add entries/branch for `"choose up to x"`, `"choose up to that many"`.
   - CR annotation: `// CR 700.2: a modal header ("choose up to …") plus its bulleted modes is
     one logical unit; the line-counter folds the bullets into the header so the parsed modal
     (1 parent + N children) is not miscounted as N+1 dropped lines.`
   - Effect: both Ruinous (`choose up to X`) and Hawkeye (`choose up to that many`) headers now
     fold → silent-drop guard no longer fires on header recognition alone.

2. **`crates/engine/src/parser/swallow_check.rs` — new detector `detect_modal_dynamic_max_dropped`.**
   Register it in `check_swallowed_clauses` (after the existing detectors, ~line 117). It fires
   iff: the cleaned Oracle text contains a dynamic modal header marker (`"choose up to that many"`
   OR a `"choose up to x"` header — require the `"choose "` lead to avoid `"up to X target"`
   false positives) **AND** no parsed modal in the AST carries `dynamic_max_choices: Some(_)`
   (introspect the precomputed `ast_json` for `"dynamic_max_choices":{` / `"dynamic_max_choices":null`,
   mirroring the existing `ast_json`-introspection detectors). Emit
   `OracleDiagnostic::SwallowedClause{ detector:"Modal_DynamicMaxDropped", … }`.
   - `parse_warning_gap_label` (`coverage.rs:5189`) already maps `SwallowedClause{detector}` →
     `"Swallow:{detector}"`, and `check_parse_warnings` already folds that into `missing` — so
     **no coverage.rs support-predicate change is needed**; the new detector participates
     automatically via `face.parse_warnings`.
   - Discrimination: Ruinous's modal has `dynamic_max_choices:Some(Ref CostXPaid)` → no fire →
     green. Hawkeye's modal has `dynamic_max_choices:None` with `max_choices(1) < mode_count(3)`
     → fire → stays red. (`check_swallowed_clauses` early-returns when any ability is
     `Unimplemented`, so this never double-reports.)
   - CR annotation: `// CR 700.2 + CR 700.2d: a "choose up to X / up to that many" modal whose
     dynamic cap was not captured (dynamic_max_choices == None) silently mis-sizes the modal;
     surface it so coverage stays honest.`

### Pattern coverage
The line-counter fix covers **every** card with a dynamic modal header (all #4186 "choose up to
X" cast-X cards + future "choose up to that many" cards) — `grep -c "choose up to [Xx]" data/card-data.json`
to size the class at implementation time; MSH alone has ≥2. The detector covers the **class** of
"dynamic modal header parsed but dynamic max dropped" misparses (any card where a future parser
regression silently fixes the cap to a constant). Neither is a one-card patch.

### Verification matrix (Sub-plan A)
| Claim | Seam | Production entry | Test (discriminating revert-fail) | Hostile/negative |
|---|---|---|---|---|
| Ruinous becomes supported | `is_modal_header_line` | coverage report over real `card-data` | Unit test on `count_effective_oracle_lines("…choose up to X —\n• …\n• …")` returns the folded count (header + 1), NOT header+N. Revert (drop the new arm) → count = header+N, test fails. | Fixed `"choose up to two"` of 2 modes still folds (regression guard); a non-modal `"up to X target creatures"` line is NOT treated as a header. |
| Hawkeye stays unsupported | `detect_modal_dynamic_max_dropped` | swallow-check over real AST | Build a `ParsedAbilities` whose modal has `dynamic_max_choices:None` + oracle `"choose up to that many"` → detector pushes `Modal_DynamicMaxDropped`. Revert (remove detector) → no diagnostic, Hawkeye false-greens; test fails. | A modal with `dynamic_max_choices:Some` + same oracle → detector silent (proves it keys on the AST cap, not just the phrase). A card with no modal header → silent. |
| End-to-end coverage delta | `analyze_coverage` | `card-data` Tilt resource | After the change, `jq '.cards[]|select(.card_name=="The Ruinous Wrecking Crew").supported'` == `true`; Hawkeye == `false`. | Diff the full supported set before/after; assert ONLY intended cards flip (no unrelated card turns red). |

**Non-vacuity evidence:** each test names the exact line that, when reverted, flips the
assertion. The Ruinous→true / Hawkeye→false pair is the discriminator that proves the fix folds
headers *without* greening an un-implemented dynamic modal.

### Commit
`fix(coverage): fold dynamic "choose up to X / that many" modal headers; flag dropped dynamic max`
(Sub-plan A is self-contained: parser+runtime for Ruinous already exist on main.)

---

# Sub-plan B — Hawkeye, Master Marksman (engine + parser feature)

### Card / rules
`First strike, reach` (already supported). `Trick Arrows — Whenever Hawkeye becomes tapped, you
may pay {1} up to three times. When you do, choose up to that many. • Net — Target creature can't
block this turn. • Explosive — Hawkeye deals 2 damage to target player. • Boomerang — Discard a
card, then draw a card.`

Semantics (CR 701.26 Taps trigger; CR 603.12a; CR 700.2/700.2d): on Hawkeye becoming tapped, the
controller may pay `{1}` up to three times. Let **K** = number of times paid (0..3). Paying one
or more times creates the reflexive **once** (CR 603.12a); the controller then chooses **up to K**
modes (cap = `min(K, 3)`, CR 700.2d). If K = 0, the reflexive does nothing.

### Analogous trace (hard gate)
Closest precedents (traced end-to-end):
- **Dynamic modal max** (#4186, `23c50148a`): `ModalChoice.dynamic_max_choices:Option<QuantityExpr>`
  → `modal_choice_for_player` (`ability_utils.rs:543-576`) resolves+clamps to `mode_count` →
  `WaitingFor::AbilityModeChoice` (`game_state.rs:3526`) → `GameAction::SelectModes`
  (`actions.rs:333`) → `handle_ability_mode_choice` (`engine_modes.rs:19`) →
  `build_chained_resolved` (`ability_utils.rs:201`) → `resolve_ability_chain`. **Hawkeye reuses
  this entire spine** — it only needs a *different QuantityExpr* feeding `dynamic_max_choices`.
- **Optional cost during resolution**: `Effect::PayCost` (`effects/pay.rs`) + `optional` +
  `AbilityCondition::WhenYouDo` (`effects/mod.rs:6885`), gated by `cost_payment_failed_flag`.
- **Counted repeat with per-iteration sub-resolution**: `repeat_for` driver
  `drive_repeat_for_outermost`/`resolve_chain_body` (`effects/mod.rs:3972-4011`).
- **Interactive modal flow**: `/add-interactive-effect` — `WaitingFor::AbilityModeChoice`
  spine already wired through engine handler, AI candidates (`candidates.rs:1287`), MP
  `acting_player` (`session.rs`), and `client/.../ModeChoiceModal.tsx`.

### The four pieces

**B1 — New QuantityRef variant (gated).** Add `QuantityRef::TimesCostPaidThisResolution`
(resolution-local count of optional cost payments made during the current resolution).
Run the `/add-engine-variant` gate:
- *Parameterization filter:* could this be a leaf-parameterization of an existing variant?
  `AdditionalCostPaymentCount`/`KickerCount`/`CostXPaid` are all **cast-time** tallies read from
  the spell/object (CR 601.2 announcement). This is **resolution-local** (CR 603.12a / 608.2c),
  read from a transient per-resolution counter. Different categorical boundary (cast vs.
  resolution) ⟹ a sibling variant, **not** a parameterization of the cast-time refs. Documented.
- *Existence:* `grep '"TimesCostPaid'` / `'OptionalCostPaymentCount'` in
  `data/engine-inventory.json` — absent (confirmed: inventory `*Paid`/`*Count` set is all
  cast-time). New variant justified.
- Registration points (lockstep):
  - `crates/engine/src/types/ability.rs` — add the enum variant with a CR-annotated doc
    (`// CR 603.12a: number of times the controller paid the repeated optional cost during this
    ability's resolution.`).
  - `crates/engine/src/game/quantity.rs` — `resolve_quantity` arm reads the new resolution-scoped
    counter (B2).
  - `crates/engine/src/game/coverage.rs` — `quantity_ref_feature` (6291) classify as `Handled`.
  - `crates/engine/src/parser/oracle_quantity.rs` — produce it from `"that many"` (B4).
  - serde + any exhaustive `match` over `QuantityRef` (compiler will enumerate them).

**B2 — Bounded optional repeated payment + count capture + flag fix.**
- Add a resolution-scoped counter to `GameState`, e.g. `optional_cost_payments_this_resolution:
  u32`, cleared at `depth == 0` in `resolve_ability_chain` alongside the existing accumulators
  (`last_revealed_ids` et al.). `resolve_quantity(TimesCostPaidThisResolution)` reads it.
- In the `repeat_for` driver, when the iterated effect is an **optional `Effect::PayCost`**:
  (a) clear `cost_payment_failed_flag` **before each iteration** (mirror the per-iteration clear
  at `effects/mod.rs ~5078`; fixes the carryover bug — a general correctness fix for all
  `repeat_for + optional cost` cards); (b) on a successful payment, increment the counter; (c) on
  a declined payment, **stop the loop early** (CR: "up to three times" lets the player stop). The
  iteration cap (3) is the `repeat_for` bound.
- CR annotations: `// CR 603.12a: count successful payments; declining ends the repeated
  optional payment early.` and `// CR 608.2c: per-iteration cost-failure flag is reset so an
  earlier decline can't suppress a later reflexive.`

**B3 — Reflexive resolves once + WhenYouDo gate.** Restructure so the `WhenYouDo` modal is **not**
re-resolved per payment (current AST nests it under the repeated def — `resolve_chain_body`
repeats the sub-chain, `effects/mod.rs:3991`). Target structure: the bounded optional PayCost is
the repeated unit; the reflexive modal runs **once after** the loop. Implementation seam choice
(decide during impl, both measured): either (i) parser emits the reflexive as a sibling/following
ability outside the repeated def, or (ii) the repeat driver repeats only the effect for the
optional-PayCost case and resolves the `sub_ability` once after the loop. Update the `WhenYouDo`
evaluation (`effects/mod.rs:6885`) so for a repeated optional payment the reflexive fires iff
`K >= 1` (CR 603.12a), not "last payment didn't fail".

**B4 — Parser (nom only) + modal wiring.** In the trigger/modal parse path
(`oracle_trigger.rs` / modal builder), for "choose up to that many" set `min_choices = 0` and
`dynamic_max_choices = Some(QuantityExpr::Ref(QuantityRef::TimesCostPaidThisResolution))`, and
emit the restructured reflexive (B3). All detection/dispatch via existing combinators
(`parse_for_each_clause`, `parse_quantity_ref` in `oracle_quantity.rs`, the modal-count
combinators); add a `"that many"` arm to the existing dynamic-modal-max combinator (sibling of
#4186's `"choose up to x"` arm) — **no `contains()`/`find()` dispatch**. The runtime cap is then
handled unchanged by `modal_choice_for_player` (clamps `min(K, mode_count)`, CR 700.2d) — reusing
#4186's machinery end-to-end.

### Registration points checklist (B, per /add-engine-effect + /add-interactive-effect)
- types: `QuantityRef` variant (`ability.rs`); `GameState` counter field (`game_state.rs`).
- parser: `oracle_quantity.rs` (`that many` → ref), modal builder (min 0 + dynamic max),
  trigger restructure (`oracle_trigger.rs`). Nom combinators only.
- resolver: `quantity.rs` (resolve new ref); `effects/mod.rs` repeat driver (count + early-stop +
  flag clear + reflexive-once); `effects/pay.rs` (increment hook on success).
- targeting: per-mode targets already handled (`build_target_slots_labelled`,
  `ability_utils.rs:386`); no change. Net (CantBlock target creature), Explosive (DealDamage
  target player) reuse existing target slots.
- interactive WaitingFor/GameAction: **reuse** `WaitingFor::AbilityModeChoice` + `SelectModes`
  (already wired) and `WaitingFor::OptionalCostChoice` + `DecideOptionalCost` (already wired,
  `game_state.rs:3369`, `actions.rs:336`). Verify the repeated optional prompt re-enters per
  iteration.
- MP filter: `acting_player`/`acting_players` arms for `OptionalCostChoice` + `AbilityModeChoice`
  already exist (`session.rs`); no new routing.
- AI: `candidates.rs` already generates `SelectModes` combinations (1287) and `DecideOptionalCost`
  yes/no; verify it offers decline across repeated iterations and that the modal-cap K is read off
  the resolved `WaitingFor` (it is — runtime resolves dynamic max before emitting the choice).
- frontend: `ModeChoiceModal.tsx` + optional-cost UI already render these; no new component.
- AI gate: behavior change touches resolution → run `cargo ai-gate` per CLAUDE.md.

### Pattern coverage (B)
Covers the **class** "pay {cost} up to N times during resolution; a reflexive scales with the
number of payments" (CR 603.12a) — not just Hawkeye. The new `TimesCostPaidThisResolution` ref +
bounded-optional-repeat-with-count + reflexive-once are reusable for every future card of this
shape; the `repeat_for`/`cost_payment_failed_flag` per-iteration reset fixes a latent bug for all
`repeat_for + optional cost` cards today.

### Verification matrix (B) — runtime is the KEY (resolver-flagged card)
| Claim | Seam | Production entry | Test (discriminating revert-fail) | Hostile/negative |
|---|---|---|---|---|
| K captured from repeated payment | repeat driver + counter | cast/tap Hawkeye, pay `{1}` thrice via real `DecideOptionalCost` | After 3 payments, `resolve_quantity(TimesCostPaidThisResolution)` (read through the modal cap) == 3; pay twice → 2; pay 0 → 0. Revert the increment → cap stays 0; X=3 case fails. | Decline on iteration 2 after paying iteration 1 → K==1 and reflexive STILL fires (proves the flag-carryover fix). |
| Modal cap = min(K, 3) | `modal_choice_for_player` | drive the tap trigger → `AbilityModeChoice` | Pay 2 → `effective.max_choices == 2`, `min_choices == 0`; pay 3 → cap 3 (==mode_count). Revert dynamic-max wiring (back to fixed 1,1) → cap 1; test fails. | Pay > mode_count is impossible here (bound 3 == modes 3); assert cap never exceeds 3. |
| Reflexive fires exactly once | `WhenYouDo` eval | full resolution, pay K≥1 | Each mode's effect applies AT MOST once per resolution (e.g. choose Explosive once → exactly 2 damage; not 2×K). Revert reflexive-once (keep modal nested under repeat) → damage applied K times; test fails. | K==0 → modal NEVER offered, no mode resolves (WhenYouDo skip). |
| Each mode resolves correctly | mode resolvers | choose each mode via `SelectModes` | Net → target creature gains CantBlock until EOT; Explosive → target player −2 life; Boomerang → hand size net 0 (discard 1, draw 1). Each assertion reverts on dropping that mode wiring. | Choosing fewer modes than K (e.g. K=3, choose 1) resolves only the chosen one. |
| Coverage honest | swallow detector (A) + parser | `card-data` resource | With B's parser emitting `dynamic_max_choices:Some`, `Modal_DynamicMaxDropped` no longer fires; Hawkeye `supported:true`. Revert B's parser change → detector fires → `false`. | Confirm no other card flips. |

**Non-vacuity evidence:** every row names the exact code whose reversion flips the assertion; the
K∈{0,2,3} sweep + the "Explosive once, not 2×K" assertion are the discriminators that separate
"modal driven by payment count" from both the old fixed-(1,1) behavior and the misparse that
repeats the modal.

### Card-test harness notes (per /card-test)
Use `GameScenario` + `GameRunner::cast(...).resolve()`; reach the tap-trigger via a real tap
(e.g. an attack or a tapping cost), then drive `DecideOptionalCost`×K and `SelectModes`. Do NOT
hand-construct `WaitingFor` for the KEY claims (production-path requirement). Submit the empty
modal choice (K≥1, choose 0) and the K=0 path through the real `GameAction`s. Assert via
`CastOutcome`/state deltas, not AST flags.

### Commits (B, may be one or two)
1. (optional split) `fix(engine): reset cost-payment-failed flag per repeat_for iteration` — the
   general bug fix, independently testable.
2. `feat(engine): repeated optional payment count + reflexive modal (Hawkeye, Master Marksman)` —
   the `TimesCostPaidThisResolution` ref, count capture/early-stop, reflexive-once, parser
   restructure, modal wiring, coverage `"that many"` header recognition.

---

## Architectural sections (both sub-plans)

**Pattern Coverage.** A: every dynamic-modal-header card (line-counter) + every dropped-dynamic-max
misparse (detector). B: every "pay {cost} up to N times → scale a reflexive by the count" card +
every `repeat_for + optional cost` card (flag fix). No single-card special cases.

**Building Blocks.** Reuse: `modal_choice_for_player`, `build_chained_resolved`,
`build_target_slots_labelled` (`ability_utils.rs`); `WaitingFor::AbilityModeChoice`/`SelectModes`,
`WaitingFor::OptionalCostChoice`/`DecideOptionalCost`; `Effect::PayCost` + `AbilityCondition::WhenYouDo`;
`resolve_quantity` (`quantity.rs`); `parse_quantity_ref`/modal-count combinators
(`oracle_quantity.rs`); coverage `ast_json` introspection pattern (`swallow_check.rs`). New: one
`QuantityRef` variant, one `GameState` counter, one swallow detector — each justified above.

**Logic Placement.** Coverage classifier + honesty detector → `coverage.rs`/`swallow_check.rs`
(audit layer). Payment count, early-stop, flag reset, reflexive-once → engine resolver
(`effects/`). Quantity resolution → `quantity.rs`. Text→AST (min 0, dynamic max ref, reflexive
restructure) → parser. Modal cap clamp → already in engine (`ability_utils.rs`). Frontend
unchanged (display only).

**Rust Idioms.** New `QuantityRef` is a typed enum variant (not a bool/sentinel); exhaustive
`match` in `resolve_quantity`/`quantity_ref_feature` (compiler enforces classification). Detector
introspects typed AST/serialized fields, not verbatim Oracle strings. Parser composes existing
combinators (`alt`/`tag` sibling arm for `"that many"`), never `contains()` dispatch.

**Nom Compliance.** Only B touches `parser/`. All detection/dispatch via combinators: `"that many"`
is a new `alt()` arm on #4186's dynamic-modal-max combinator; the quantity ref via
`parse_quantity_ref`. The coverage `is_modal_header_line` and the swallow detector live in
`game/`/`parser/swallow_check.rs` (classifier/audit code, the file's established `contains` idiom,
flagged `// allow-noncombinator` like its siblings) — not parser dispatch.

**Extension vs Creation.** A extends an existing classifier + adds a sibling detector. B extends
the #4186 dynamic-modal-max spine with a new quantity source; the only genuinely new primitives
are the resolution-local count ref + counter, justified by CR 603.12a's distinct categorical
boundary (resolution vs cast time).

**Variant Discoverability.** `data/engine-inventory.json` consulted; the `*Paid`/`*Count`
QuantityRef set is entirely cast-time — no resolution-local payment-count ref exists. `/add-engine-variant`
gate run for `TimesCostPaidThisResolution` (parameterization filter + categorical-boundary +
existence) — result: new sibling variant, documented above.

**Analogous Trace.** A: `is_modal_header_line`→`count_effective_oracle_lines`→`check_silent_drops`
→`analyze_coverage`; detector mirrors `detect_dynamic_qty` (`swallow_check.rs`). B: traced #4186's
dynamic-modal-max end-to-end (`ability.rs ModalChoice` → `ability_utils.rs:543` →
`game_state.rs:3526 AbilityModeChoice` → `actions.rs:333 SelectModes` → `engine_modes.rs:19` →
`ability_utils.rs:201 build_chained_resolved` → `effects/mod.rs resolve_ability_chain`), plus the
`repeat_for` driver (`effects/mod.rs:3972`) and `Effect::PayCost`/`WhenYouDo` (`effects/pay.rs`,
`effects/mod.rs:6885`).

**Identity / Provenance Contract.** B's "that many" binds the modal cap to a *resolution-local*
value: source phrase "choose up to that many" → authority = count of successful optional `{1}`
payments in THIS resolution → bound at resolution time (live read via `resolve_quantity` when
`modal_choice_for_player` builds the cap) → stored in the resolution-scoped `GameState` counter,
cleared at `depth==0` → consumed by `modal_choice_for_player` (clamped to `mode_count`, CR 700.2d)
→ invalidated when the resolution ends (counter reset). Multi-authority hostile fixture: a *second*
Hawkeye tap in the same turn (or another repeated-payment source resolving in between) must NOT
leak K — each resolution starts from a cleared counter; the test resolves two tap triggers with
different K and asserts each modal cap matches its own K. Ruinous's CostXPaid provenance (cast X →
`cost_x_paid` on the object, CR 107.3m) is pre-existing and already tested (Walking Ballista).

---

## Deviations / risks
1. **False-green avoidance is the core constraint.** Recognizing dynamic headers in the
   line-counter alone would green Hawkeye prematurely; the `Modal_DynamicMaxDropped` detector is
   the mandatory honesty guard. (Lighter alternative considered and rejected: add only
   `"choose up to X"` to the phrase list and rely on `"that many"` staying unrecognized — fragile,
   depends on a list gap; the detector is the principled choice.)
2. **Detector breadth.** `Modal_DynamicMaxDropped` may also (correctly) flag redefined-X modal
   cards (Bumi/Riku — `"choose up to X, where X is …"`, dynamic max deliberately left `None` by
   #4186's follow-up). Those are already unsupported (the "where X is" redefinition is unparsed);
   the detector makes the reason explicit and does not regress a green card. Verify by diffing the
   supported set.
3. **B structural restructure (B3) is the highest-risk item.** The current parse nests the modal
   under the repeated PayCost. Whether to fix in parser (emit reflexive as a following sibling) or
   in the repeat driver (resolve sub once after the loop) is an implementation decision; both seams
   are identified with line numbers. The "Explosive once, not 2×K" test is the guard against
   getting this wrong.
4. **Verification cadence (CLAUDE.md risk-scaled).** A is parser/coverage-classifier only → run
   `cargo fmt --all`, the parser combinator gate + targeted coverage assertions, let Tilt
   (`card-data`, `test-engine`) confirm; do not block on full clippy/test for the small change. B
   is engine plumbing + resolution + AI → full gate: `cargo fmt --all`, then
   `./scripts/tilt-wait.sh clippy test-engine card-data` (+ `test-ai`), `cargo ai-gate`, and the
   runtime tests above before marking fixed. Do NOT run `cargo build/clippy/test` directly (target
   locks) — read Tilt resources.
5. **mtgish is out of scope** (dormant) — no changes there.
6. Card-data regeneration is the implementer's responsibility via the `card-data` Tilt resource;
   the symlinked `data/*.json` in this worktree must not be hand-regenerated during planning.
