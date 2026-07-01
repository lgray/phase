# MSH-E plan review — ROUND 2 (independent adversarial re-measure)

**VERDICT: CHANGES-REQUIRED** — BLOCKER 0 · HIGH 2 · MED 3 · LOW 2

Base re-measured: worktree `/home/lgray/vibe-coding/wt-msh-f` @ `aa4e88ec4`
(`feat(engine): Hawkeye, Young Avenger …`). CR text + `data/*.json` are symlinks into
`phase-rs-workdir`. Every load-bearing claim below was re-grepped/re-read in this base; the
plan's tables were trusted for nothing.

Bottom line: **Sub-plan A is sound and the r1 BLOCKER is genuinely fixed** — the `"modal":{`
gate is empirically fully discriminating across the entire 10-card class with ZERO green
regressions. **Sub-plan B's diagnosis, CR work, and seam re-location are all correct**, but two
things must change before implementation: (1) the B3 reflexive-gate hedge must be *resolved* (it is
resolvable now — see HIGH-1), and (2) the B-i flag-clear discriminator is non-vacuous-failing as
written (HIGH-2). Three MED items (B4 header form, A2 serde rationale, B2 continuation scope) and
two LOW items round it out.

---

## Re-measured and CONFIRMED CORRECT (coverage map for the lead)

### Sub-plan A — fully verified
- **10-card class identity + supported/gap** — `jq` over `data/coverage-data.json` for
  `oracle_text ~ /choose up to (x|that many)/i` returns EXACTLY the plan's 10 cards with matching
  `supported`/`gap_count` (Riku f/1, Reap f/1, Heroic Feast t/0, Ruinous f/0, Temporal Firestorm
  t/0, Frillback f/0, Hawkeye f/0, Discordant f/1, Bumi f/1, Suppression f/1).
- **A1 gate — empirically discriminating, ZERO green regressions.** Grepping each card's serialized
  entry in `data/card-data.json` for `"modal":{` and `"dynamic_max_choices":`:
  | card | `"modal":{` | `"dynamic_max_choices"` | detector | result |
  |---|---|---|---|---|
  | Hawkeye | ✅ | ❌ | FIRES | RED ✓ |
  | Ruinous | ✅ | ✅ | silent | greened ✓ |
  | Tranquil Frillback | ✅ | ❌ | FIRES | RED ✓ |
  | Bumi / Riku | ✅ | ❌ | FIRES | already red (gap1) ✓ |
  | Discordant / Reap / Suppression | ❌ | ❌ | silent | already red (gap1) ✓ |
  | **Heroic Feast** | **❌** | ❌ | silent | **GREEN — no regression** ✓ |
  | **Temporal Firestorm** | **❌** | ❌ | silent | **GREEN — no regression** ✓ |
  This matches the plan's A1 table cell-for-cell. The two previously-green cards have **no parsed
  modal node** (their "choose up to that many/X <nouns>" is a non-modal selection clause), so the
  `"modal":{` requirement excludes them. **The r1 A1 BLOCKER is resolved.**
- **Ruinous's dynamic_max serializes as an object** — `"dynamic_max_choices":{"type":"Ref","qty":
  {"type":"CostXPaid"}}` — so the detector's "absence of `"dynamic_max_choices":{`" condition-3 is
  correctly FALSE for Ruinous → detector stays silent → Ruinous is greened by the line-counter, not
  re-reddened by the detector. (This is load-bearing: a non-object serialization would have kept
  Ruinous red.)
- **`dynamic_max_choices` serde** — `ability.rs:12924` `#[serde(default, skip_serializing_if =
  "Option::is_none")]`, field at `:12925`. Omitted when `None`, emitted as object when `Some`.
  A2's "key on absence of `:{`, no `:null` form" is CORRECT.
- **A3 bullet counts** (`oracle_text` grep): Heroic Feast 0, Temporal Firestorm 0, Hawkeye 3,
  Ruinous 4, Frillback 3. The fold logic (`count_effective_oracle_lines`, `coverage.rs:5377-5434`;
  clears `in_modal` on the next non-bullet line at `:5426-5430`) confirms recognizing the 0-bullet
  greens' header changes their count by 0 → no flip. Hawkeye (3 bullets) would false-green under
  the recognizer ALONE → the detector is genuinely mandatory.
- **Line-counter arithmetic** — `count_effective_parsed_items` (`coverage.rs:5589-5601`, `1` per
  childless item, `1 + children.len()` per parent). From real `parse_details`: Hawkeye
  `[c0,c0,c1]` → 4; oracle 5 → `SilentDrop:4_of_5`. Ruinous `[c1,c1]` → 4; oracle 6 →
  `SilentDrop:4_of_6`. Both match the plan.
- **Wiring** — `is_modal_header_line` (`coverage.rs:5438-5464`, `CHOOSE_PHRASES` `5439-5462`, lacks
  "choose up to x"/"that many"); `check_silent_drops` (`5123-5141`, call `4464`);
  `check_resolver_features` (`4460`); `check_parse_warnings` (`5184-5193`, call `4469`);
  `parse_warning_gap_label` SwallowedClause→`Swallow:{detector}` (`5204`); detector pipeline
  `check_swallowed_clauses` (`swallow_check.rs:78`), early-return on Unimplemented (`:93`),
  `ast_json = serde_json::to_string(parsed)` (`:103`), `json_has_any` (`:1342`), `detect_dynamic_qty`
  template (`:1355`), diagnostics flow `oracle.rs:3978-3981` → `ctx.push_diagnostic` →
  `parse_warnings`. Existing mirror test `count_effective_oracle_lines_recognizes_choose_up_to_four`
  exists at `coverage.rs:11211`. Every cited line is accurate (±1).

### Sub-plan B — verified
- **B seam re-location is correct.** `repeat_for_outermost_with_scope_or_unless`
  (`effects/mod.rs:3962-3967`) requires `player_scope.is_some() || unless_pay.is_some()` (`:3966`);
  Hawkeye has neither → `drive_repeat_for_outermost` (`:3972`) is never entered. The r1 mis-citation
  is properly corrected. (NOTE: actual path is `crates/engine/src/game/effects/mod.rs`; the plan
  abbreviates as `effects/mod.rs` throughout — harmless.)
- **B2 core claim CONFIRMED — the plain Fixed-count path has NO per-iteration optionality.**
  `repeated_full_chain = repeat_for.is_some() && sub_ability.is_some()` (`:5800-5801`); for
  `Fixed(3)` with `member_driven=false, kind_driven=false`, each iteration takes the
  `if repeated_full_chain` branch (`:5854`) → `resolve_ability_chain(full_chain_iteration)` (`:5859`)
  which re-resolves **PayCost AND the modal every iteration**; `return Ok(())` at `:5920-5922`.
  Per-iteration `OptionalEffectChoice` exists only for `kind_driven || (member_driven && optional)`
  (`:5836`, `:5860`). So today Hawkeye (if its up-front gate were accepted) pays once-or-not, then
  runs the chain 3× mandatorily AND resolves the modal 3×. The new driver is genuinely required.
- **Up-front optional gate** (`:5403-5448`): `ability.optional && !has_kind_driven_repeat &&
  !has_member_driven_repeat_after_hydration` → `WaitingFor::OptionalEffectChoice` at `:5441`, stash
  `pending_optional_effect` `:5428`, return `:5447`. Adding `&& !is_repeated_optional_payment` here
  mirrors the existing gate exactly — realizable.
- **`resolve_optional_effect_decision`** (`:1733-1807`): `optional=false` at `:1740`, Accept →
  `resolve_ability_chain` at `:1747`. Confirmed.
- **depth==0 prelude** (`:4619`) with sibling `state.exiled_from_hand_this_resolution = 0` at
  `:4643`. A new `optional_cost_payments_this_resolution` counter cleared here is structurally
  consistent. **(Critical, see MED-3: the post-loop reflexive must run at depth ≥ 1 or this prelude
  wipes K before `modal_choice_for_player` reads it.)**
- **`modal_choice_for_player`** (`ability_utils.rs:543-576`): dynamic clamp
  `effective.max_choices = (resolved.max(0) as usize).min(modal.mode_count)` at `:573` (CR 700.2d),
  and `resolve_quantity(state, expr, player, source_id)` at `:572` takes **no ability** → K MUST
  live on `GameState` (resolution-scoped). The plan's B2a claim is correct.
- **`cost_payment_failed_flag` semantics** — set on failure (`pay.rs:40, 233, 257`;
  `effects/mod.rs:5005`); a **successful** payment (`PaymentOutcome::Paid`, `pay.rs:241`) does NOT
  clear it; cleared per-iteration ONLY inside the player_scope loop (`effects/mod.rs:5078`). The
  comment at `:5068-5077` explicitly states this is "the missing fourth resumption boundary" — i.e.
  `repeated_full_chain` has no per-iteration clear. B3's location claim is correct.
- **WhenYouDo eval** (`effects/mod.rs:6885-6887`):
  `!(matches!(ability.effect, Effect::PayCost{..}) && state.cost_payment_failed_flag)`. Confirmed.
- **B1 inventory** — `QuantityRef` enum at `ability.rs:3954`; the `*Paid/*Count` set is
  `CostXPaid` (`:4500`), `KickerCount` (`:4503`), `AdditionalCostPaymentCount` (`:4507`),
  `AdditionalCostPaymentCountFor` (`:4511`), `ConvokedCreatureCount` (`:4519`) — all cast-time, all
  reading the source object. No `TimesCostPaid*` exists → DOES_NOT_EXIST confirmed. Registration
  points exist: `quantity.rs` CostXPaid scope/dependency arms at `:405/:653/:836`,
  `coverage.rs quantity_ref_feature` at `:6297`.
- **B4 parser seams** — `enum ModalCountSpec { Fixed, DynamicCostX }` (`oracle_modal.rs:1662-1665`);
  count_spec map `:600-609` (`DynamicCostX => (0, usize::MAX, Some(Ref{CostXPaid}))`); dynamic arm
  `value(DynamicCostX, terminated(tag("choose up to x"), not(…"where"…)))` at `:1713-1719` (the
  `not(where)` lookahead correctly excludes Bumi/Riku). The proposed refactor preserves Ruinous
  exactly (see HIGH/MED notes for caveats).
- **"Reuse, no new surface" (interactive/MP/AI/frontend)** — `WaitingFor::AbilityModeChoice`
  (`game_state.rs:3607`) / `OptionalEffectChoice` (`:3630`); routed in `acting_player` (`:4609`) /
  `acting_players` (`:4630`); `GameAction::SelectModes` (`actions.rs:333`) / `DecideOptionalEffect`
  (`:444`); `session.rs` has **0** hits for either WaitingFor variant (routing reuses
  acting_player/acting_players). `game_state.rs:6597` already documents "count across an
  OptionalEffectChoice round-trip" machinery. All reuse claims hold.
- **CR numbers — every one grep-verified in `docs/MagicCompRules.txt`:**
  - **603.12a (line 2659)** — full text read: *"…However, if a resolving spell or ability includes
    a choice to pay a cost multiple times and creates a triggered ability that triggers when that
    payment is made, paying that cost one or more times causes the reflexive triggered ability to
    trigger only once."* — **verbatim the Hawkeye rule.** The load-bearing CR is accurate.
  - **700.2b (3207)** — *"controller of a modal triggered ability chooses the mode(s)… If one of the
    modes would be illegal… that mode can't be chosen. If no mode is chosen, the ability is removed
    from the stack."* Matches the plan's framing.
  - **700.2d (3211)** distinct-modes cap ✓; **700.2e (3213)** *"a player other than their
    controller chooses a mode"* — N/A to Hawkeye ✓; **601.2b (2459)** modal announcement during
    casting — N/A to a reflexive triggered modal ✓. **The plan's B4 CR correction (use 700.2b +
    700.2d; drop 601.2b and 700.2e) is CORRECT.**
  - 603.12 (2656), 700.2 (3203), 700.2a (3205), 107.3m (488), 608.2c (2793), 701.26 (3518),
    701.21 (3449), 119.3 (1065), 122.1 (1178) — all resolve and match.

---

## FINDINGS

### HIGH-1 — B3: the reflexive-gate hedge is RESOLVABLE NOW; the plan must commit (don't ship an either/or)
**Measured.** Hawkeye's reflexive sub-ability, in the exported AST (`data/card-data.json`,
`triggers[0].execute.sub_ability`), has **`effect: { "type":"GenericEffect", … }`** with
`condition: WhenYouDo`, `modal:{min1,max1,count3}`, `mode_abilities:[Net,Explosive,Boomerang]`. The
sub's effect is **NOT `Effect::PayCost`**.

`evaluate_condition(condition, state, ability)` (`effects/mod.rs:6758`) is always passed the ability
that *carries* the condition. There are two real evaluation sites with different bindings:
- **`:5282`** (top-level condition check when an ability is resolved as the chain head): `ability` =
  that ability. If the new driver resolves the reflexive sub **directly** via
  `resolve_ability_chain(sub)`, the `:6886` gate sees `ability.effect = GenericEffect` →
  `matches!(…, PayCost) = false` → returns **true unconditionally, flag-independent.**
- **`:6264`** (sub evaluated as a descendant of its parent chain): `ability` = the **parent**
  (effect = PayCost) → the flag matters. This is how the *current* `repeated_full_chain` gates it.
  Note also the interactive-payment **deferral** path (`:6222-6261`) re-stashes the WhenYouDo sub and
  re-evaluates it at `:5282` — i.e. for human payments the gate already becomes flag-independent.

**Resolution (definitive):** the clean design is to resolve the reflexive sub **directly** (passing
the sub, effect=GenericEffect) at depth ≥ 1, gated solely on `K ≥ 1`. Under that design the `:6885`
guard needs **NO change** and the driver's explicit `K==0` skip is the sole authority. The plan's
deviation #4 ("decide during impl") should be deleted and replaced with: *"the reflexive sub is
resolved directly; `:6886` is unchanged; the K-gate is authoritative."* Shipping the unresolved
either/or under-specifies the single highest-risk seam.
**Fix:** state the committed design (direct-sub resolution + K-gate, no `:6886` change). Keep
`:6886` unchanged for its existing single-payment Guide-of-Souls class (tests at
`effects/mod.rs:10782/10801` exercise the synthetic PayCost-parent path and stay valid).

### HIGH-2 — B-i hostile test ("revert the flag-clear ⇒ reflexive suppressed") is NON-DISCRIMINATING
**Measured.** B-i's hostile column claims: *"Decline iteration 2 AFTER paying iteration 1 ⇒ K==1
and the reflexive STILL fires … revert that [per-iteration flag] clear ⇒ the loop-terminating
decline's flag suppresses the reflexive ⇒ fails."* This does not hold:
1. A **decline** never runs `Effect::PayCost`, so it never sets `cost_payment_failed_flag` (set only
   on payment failure — `pay.rs:40/233/257`).
2. A **successful** iter-1 payment leaves the flag false (Paid does not touch it — `pay.rs:241`),
   and B2(b) clears the flag *before* each payment anyway.
3. Therefore at the post-loop reflexive the flag is already **false**, regardless of the B2(c)
   "clear before the reflexive." Reverting that clear changes nothing → the reflexive fires either
   way → **the test cannot fail on the reverted code → it is vacuous.**

Compounding it: under HIGH-1's committed design (reflexive sub resolved directly, effect=GenericEffect)
the `:6886` gate ignores the flag entirely, so the flag-clear-before-reflexive (B2c) is a **no-op**
and is, in fact, unnecessary. The genuinely load-bearing clear is B2(b) **before each payment** —
because a stale `true` from a *failed* prior iteration would, since Paid never clears it, block the
`K` increment of a later *successful* payment (`if !flag { K += 1 }`), under-sizing the modal cap.
**Fix:** re-target the discriminator. (a) Keep "decline iter2 ⇒ K==1 ⇒ reflexive fires," but make
its discriminating revert the **K-gate** (resolve reflexive iff `K≥1`), not the flag-clear. (b) Add
a *new* discriminating row for B2(b): "iter1 payment fails (e.g. no mana), iter2 accepted+paid ⇒
K==1; revert the clear-before-payment ⇒ stale flag blocks the increment ⇒ K==0 ⇒ modal cap 0 ⇒
fails." (c) Drop the B2(c) flag-clear-before-reflexive or justify it against a real flag-true-with-
K≥1 scenario (which does not exist under the direct-sub design).

### MED-1 — B4: the new "that many" parser arm must match Hawkeye's PERIOD/bare header, not em-dash
**Measured.** Hawkeye's header (oracle): `"When you do, choose up to that many."` — terminated by a
**period, no em-dash**. The plan (B4 and §A-1) repeatedly says *"em-dash form `choose up to that
many —` preferred."* For the **parser** arm in `scan_modal_count_override`, em-dash-only would NOT
match Hawkeye → the modal would keep its `Fixed{1,1}` default (exactly the current
`max_choices:1, no dynamic_max` state in card-data) → **B4 fails for its target card.** The existing
`DynamicCostX` arm does NOT gate on em-dash either — it is `terminated(tag("choose up to x"),
not(…"where"…))` (`:1716-1718`), with the em-dash left in the remainder. The new arm should mirror
that: `tag("choose up to that many")` with a `not(where)`-style guard, matching both Frillback's
em-dash and Hawkeye's period form. (`scan_at_word_boundaries` already scans mid-line, so it locates
the phrase regardless of the trailing punctuation.) The modal-header gate already excludes Heroic
Feast/Temporal Firestorm — they parse no modal node — so a bare tag cannot misfire on them.
**Fix:** specify the parser arm matches the bare "choose up to that many" form (period-terminated),
not em-dash-only. (The §A-1 *line-counter* "em-dash preferred" is harmless — see LOW-2 — but the
**B4 parser** guidance is actively wrong for the target card.)

### MED-2 — A2: the serde justification for the `modal` gate is factually wrong (conclusion still holds)
**Measured.** The plan (A-2 step 2) asserts: *"`modal: Option<ModalChoice>` at `ability.rs:13275`
has no `skip_serializing_if`, so it serializes `"modal":null` when `None` and `"modal":{` only when
`Some`."* That mechanism is wrong. `AbilityDefinition` has a **custom `Serialize` impl**
(`ability.rs:13409`) via the `AbilityDefinitionRepr` proxy (`:13342`), and the proxy's `modal` field
**does carry** `#[serde(skip_serializing_if = "Option::is_none")]` (`:13380-13381`). So when `modal`
is `None` the key is **omitted entirely** (not `"modal":null`). The *conclusion* — `"modal":{`
appears iff a modal node is present — is still correct (omitted-vs-null is immaterial to a substring
test), and the empirical data confirms the gate works. But the cited evidence is false, and the
CR/serde code comment the plan proposes to write would mislead a future reader.
**Fix:** correct the rationale to cite the proxy `skip_serializing_if` at `ability.rs:13381` (modal
omitted when None) rather than a non-existent field-level absence; the gate substring is unchanged.

### MED-3 — B2: the new driver is genuinely new continuation plumbing, not a literal "reuse," and needs depth-≥1 discipline
**Measured.** The plan oscillates between "reuse the stash machinery (`pending_repeat_iteration`,
`:5904-5914`)" and "a new stash carrying `{remaining_iterations, reflexive_sub}`." These are not the
same. `drain_pending_repeat_iteration` (`:893-954`) re-runs the **full chain** per iteration via
`resolve_ability_chain(iter_effective, depth=1)` (`:938`) — it has no concept of per-iteration
`OptionalEffectChoice` for a PayCost-only unit, a `K` counter, an early-stop on decline, or a
once-after-loop reflexive. Reusing it verbatim would re-resolve PayCost+modal per iteration (the bug
being fixed). The new driver therefore needs **its own** stash/drain block (a new GameState field,
or a tagged variant), structurally modeled on `drain_pending_repeat_iteration` but with the
payment/K/reflexive semantics. This is realizable on the existing seams (the
OptionalEffectChoice round-trip `engine_payment_choices.rs:28` + the continuation drain both exist),
but it is the real first-class work — the plan should not imply a drop-in reuse.
Additionally **critical**: the resolution-local `K` counter is cleared at the depth==0 prelude
(`:4619/:4643`). The per-iteration round-trips resume via the drain at **depth 1** (`:938`), so K
survives across iterations; but the implementer must ensure the **post-loop reflexive also runs at
depth ≥ 1** — if it re-enters at depth 0, the prelude wipes K to 0 before `modal_choice_for_player`
(`ability_utils.rs:572`) reads it, collapsing the cap to 0. State this invariant in the plan.
**Fix:** scope the new stash/drain explicitly; add the "reflexive resolves at depth ≥ 1 so the K
counter is not wiped" invariant; drop the "reuse `pending_repeat_iteration`" phrasing or qualify it
as "model on, not reuse."

### LOW-1 — B4: `ModalCountSpec` must drop its `Copy` derive
`ModalCountSpec` derives `Copy` (`oracle_modal.rs:1661`). `QuantityRef` derives only `Clone`, not
`Copy` (`ability.rs:3951`). Changing `DynamicCostX` (unit) → `Dynamic { qty: QuantityRef }` forces
removing `Copy` from `ModalCountSpec`. Impact is small (the value is moved into the `:600` match and
cloned by nom `value()`, both of which only need `Clone`), but it is an unstated refactor step that
will surface as a compile error. List it in the registration-points checklist.

### LOW-2 — minor: A-1 em-dash narrative + one stale line cite
- The §A-1 narrative assumes the line-counter recognizer folds Hawkeye's bullets (5→2) so "the
  detector is the load-bearing gate." With the *em-dash-preferred* recognizer the plan also
  endorses, Hawkeye's period-form header is NOT recognized, so `SilentDrop:4_of_5` keeps Hawkeye red
  instead (and the detector also fires). Hawkeye ends RED either way, so there is no functional
  defect — but the two statements are mutually inconsistent; pick one (recommend: recognizer is
  em-dash/period-agnostic for headers, detector is the honesty gate, both keep Hawkeye red).
- `ConvokedCreatureCount` is cited at `ability.rs:4516`; actual is `:4519` (comment drift). Every
  other B line cite is accurate.

---

## Verification-matrix audit (CLAUDE.md non-vacuity)
- **A-i** (revert `is_modal_header_line` arm ⇒ count 6 not 2) — discriminating; math re-derived. ✓
- **A-ii** (Ruinous flips green; revert A-1 ⇒ `SilentDrop:4_of_6` ⇒ false) — discriminating. ✓
- **A-iii** (detector unit test; revert registration ⇒ no diagnostic; negatives: dynamic_max present
  ⇒ silent / non-modal ⇒ silent / Unimplemented early-return) — discriminating, well-chosen. ✓
- **A-iv** (end-to-end supported-set diff, only Ruinous flips, zero reds) — discriminating. ✓
- **B-i** — K-capture half (revert increment ⇒ K stays 0) is discriminating ✓; **flag-clear hostile
  half is VACUOUS** — see HIGH-2.
- **B-ii** (cap min(K,3), min 0; revert B4 ⇒ cap 1) — discriminating; clamp verified at
  `ability_utils.rs:573`. ✓
- **B-iii** (reflexive once; revert ⇒ damage K× via the per-iteration modal; K==0 ⇒ never offered) —
  discriminating; the "K× damage" baseline is the *current* `repeated_full_chain` behavior
  (`:5854/:5859`), confirmed. ✓
- **B-iv** (each mode resolves) — discriminating. ✓
- **B-v** (B4 emits `dynamic_max` ⇒ detector silent ⇒ Hawkeye green; revert B4 ⇒ red) —
  discriminating. ✓
- **Multi-authority fixture** (two Hawkeye taps, K not leaking) — sound given the depth==0 counter
  clear; reinforces MED-3's depth invariant.

---

## What would make this CLEAN
1. Resolve HIGH-1 in the plan text: reflexive sub resolved directly at depth ≥ 1, K-gate
   authoritative, `:6886` unchanged. Delete deviation #4's either/or.
2. Fix HIGH-2: re-target B-i's discriminators (K-gate for the reflexive; a new clear-before-payment
   row for K-accounting); drop/justify the B2(c) flag-clear-before-reflexive.
3. MED-1: specify the B4 "that many" arm matches the bare/period form (Hawkeye), not em-dash-only.
4. MED-2: correct the `modal` serde rationale to the proxy `skip_serializing_if` (`ability.rs:13381`).
5. MED-3: scope the new stash/drain as first-class; add the depth-≥1 reflexive invariant.
6. LOW-1/LOW-2: add "drop `ModalCountSpec: Copy`" to the checklist; reconcile the A-1 em-dash
   narrative; fix the `ConvokedCreatureCount` line cite.

No fundamental redesign is required. Sub-plan A is implementable as-is modulo MED-2/LOW-2 wording.
Sub-plan B's architecture is correct; it needs the two HIGH resolutions and the MED clarifications
before the implementer starts, primarily so the highest-risk seam (B3) ships decided, not hedged.
