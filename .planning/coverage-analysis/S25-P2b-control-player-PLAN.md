# S25-P2b — Control-Another-Player (Secret of Bloodbending, The Dominion Bracelet)

Planner: P2b (xhigh). Worktree `/home/lgray/vibe-coding/s25-impl-wt` @ `330a1f18b`.
Skills applied: `add-engine-variant` (variant gate, §5), `add-engine-effect` (lifecycle, §3/§4),
`oracle-parser` (nom mandate, §3/§4), `card-test` (discriminating tests, §7).

---

## 0. Scope reality — the fan-out map was WRONG; this is an EXTENSION

The S25 map labeled P2b "control-another-player CR 723 = largest subsystem / green-field."
**That is false.** Control-another-player is already implemented and unit-tested:

- `Effect::ControlNextTurn { target, grant_extra_turn_after }` — `crates/engine/src/types/ability.rs:8886`.
  Two fields only; **no duration/window field → full-next-turn control only (CR 723.1).**
- Resolver `control_next_turn::resolve` — `crates/engine/src/game/effects/control_next_turn.rs:5`;
  dispatched (frozen file, `{ .. }`) at `crates/engine/src/game/effects/mod.rs:2992`. Pushes a
  `ScheduledTurnControl { target_player, controller, grant_extra_turn_after }`
  (`crates/engine/src/types/game_state.rs:7657`) onto `state.scheduled_turn_controls`.
- Runtime turn-piloting: `crates/engine/src/game/turn_control.rs` — `authorized_submitter_for_player`
  (:28) routes the controlled player's action submissions to the controller (CR 723.5); `priority_seat`
  (:21); `turn_decision_maker` (:8).
- Activation/release state machine (`turns.rs`):
  - **Activate**: `start_next_turn` sets `turn_decision_controller = Some(scheduled.controller)` when the
    new active player matches a scheduled control — `crates/engine/src/game/turns.rs:539-549`.
  - **Release**: `start_next_turn` clears `turn_decision_controller = None` and retains-out the
    scheduled entry at the *next* turn boundary — `crates/engine/src/game/turns.rs:461-476`
    (CR 723.1 "the effect doesn't end until the beginning of the next turn").
  - 723.1a last-wins overwrite: resolver `retain` at `control_next_turn.rs:33`; also
    `turns.rs:6951` test `newest_scheduled_control_for_target_takes_precedence`.
- Parser already lowers "you control target {player|opponent} during {that player's|their|its} next
  turn" to `ControlNextTurn` — shared suffix `try_parse_control_next_turn_suffix`,
  `crates/engine/src/parser/oracle_effect/imperative.rs:295-328`. Proof: **The Dominion Bracelet's**
  own granted clause "You control target opponent during their next turn" already parses to
  `Effect::ControlNextTurn { target: Opponent }` (see §2 card-data evidence).

**The genuinely-new capability P2b needs is narrow: CR 723.2 phase-scoped ("next combat phase")
limited-duration control** — which `ControlNextTurn` cannot express and the runtime cannot release
mid-turn. See §3.

### 0.1 Split / ROI recommendation (up front)

**Recommend Option B (split), executed as:**

1. **Increment 1 — The Dominion Bracelet — SHIP NOW.** A single parser wiring change (§2). Everything
   else (the +1/+1 static, the granted `ControlNextTurn`, `{15}`+Exile-self cost, "Activate only as a
   sorcery", the `{X}`-less-by-power `CostReduction` *type* and its *runtime*) already exists. **+1
   supported card, zero new variants, ~5 lines + test.** Highest ROI in the tranche.

2. **Increment 2 — Secret of Bloodbending — DEFER via strict-failure (recommended) OR full build.**
   The card is **all-or-nothing for coverage**: its *default* branch is "control during their next
   **combat phase**" (CR 723.2 phase window), which requires real (bounded) `turn_control`
   phase-boundary activation+release infra that does not exist today (§3). The phase-scoped-player-
   control class is **exactly one card** (measured, §1). A partial fix of only the cost-paid "next
   turn instead" branch yields **zero coverage** (the card stays unsupported while the base branch is
   a gap) — so there is no cheap middle. **Given the 1-card class, recommend tagging the combat-phase
   branch strict-failure (loud `Unimplemented`, coverage waits) and NOT adding the `window`
   parameterization until a second phase-scoped card justifies the runtime** (CLAUDE.md: "the
   strict-failure tag is the right place to leave coverage waiting while the architecture wins").
   The full-build alternative (§3.4) is architecturally clean and specified here so the driver can
   fund it if this specific card is wanted supported; do NOT ship a silent runtime stub (prohibited
   by `add-engine-variant` anti-patterns).

---

## 1. Class enumeration (measured from `data/card-data.json`, 35,397 cards)

Query: oracle matching `you control (target )?(that )?(opponent|player)` ∪ `during (their|its) next
(turn|combat)`. Manual declassification of attack-restriction false positives (City Hall, The Second
Doctor, Willie Lumpkin — "can't attack you during their next turn", NOT player control).

**Player-control class, by CR 723 sub-rule:**

| Card | Clause | Slot | Status |
|---|---|---|---|
| Mindslaver | "control target player during that player's next turn" | `ControlNextTurn` | ✅ supported |
| Worst Fears | same + "Exile Worst Fears" | `ControlNextTurn`+`ChangeZone` | ✅ supported |
| Sorin Markov (−7) | "control target player during that player's next turn" | `ControlNextTurn` | ✅ supported |
| The Dominion Bracelet | granted "control target opponent during their next turn" | `ControlNextTurn` | ⚠️ 1 parser gap (§2) |
| Secret of Bloodbending (paid) | "control that player during their next turn instead" | `ControlNextTurn` | ⚠️ parser gap (§3.2) |
| **Secret of Bloodbending (base)** | **"control target opponent during their next combat phase"** | **none (723.2 phase window)** | ❌ infra gap (§3) |
| Word of Command | "control that player until [spell] finishes resolving" | none (723.2 spell window) | ❌ out of scope |
| Clocknapper | "Steal that phase from target player during their next turn" | none (phase-steal, different mechanic) | ❌ out of scope |

**Conclusion for ROI:** full-turn control (723.1) = 5 cards, all served by the existing
`ControlNextTurn`. **Phase-scoped control (723.2 combat-phase window) = exactly ONE card**
(Secret of Bloodbending base branch). Word of Command (spell-resolution window) and Opposition Agent
are the CR-cited 723.2 pair (`docs/MagicCompRules.txt:6165`) but use *different* windows and are not
this tranche. "Bumi" cards exist (`bumi bash`, `bumi, eclectic earthbender`, `bumi, king of three
trials`, `bumi, unleashed`, …) but **none are control-another-player cards** — the map's "+Bumi"
note is a red herring; no additional P2b card.

---

## 2. The Dominion Bracelet — ONE parser gap; ship it

**Oracle:** `Equipped creature gets +1/+1 and has "{15}, Exile The Dominion Bracelet: You control
target opponent during their next turn. This ability costs {X} less to activate, where X is this
creature's power. Activate only as a sorcery." (You see all cards…) / Equip {1}`

### 2.1 What already parses (card-data.json `static_abilities`, verified)

The equipment static grant is **almost fully modeled** (contrary to the map's framing). Current
`static_abilities[0].modifications`:

- `AddPower 1`, `AddToughness 1` — the +1/+1. ✅
- `GrantAbility { definition: Activated }` with:
  - `effect: ControlNextTurn { target: Typed(controller=Opponent), grant_extra_turn_after: false }` ✅
  - `cost: Composite[ Mana{15}, Exile{ filter: SelfRef, count: 1 } ]` — `{15}` + exile-self-as-cost ✅
  - `activation_restrictions: [ AsSorcery ]` — "Activate only as a sorcery" ✅
  - `sub_ability: Unimplemented { name: "this", description: "This ability costs {X} less to activate,
    where X is ~'s power" }` ❌ **← the ONLY gap.**

`Equip {1}` parses as a separate `Attach` ability. ✅ The equipment-granted-quoted-ability infra is
robust: 35/36 sampled equipment/aura granting a quoted activated ability produce `GrantAbility`
(e.g. **Deconstruction Hammer** — "Equipped creature gets +1/+1 and has \"{3}, {T}, Sacrifice
Deconstruction Hammer: Destroy target artifact or enchantment.\"" — a near-exact structural twin that
parses clean, including the sacrifice-self cost).

### 2.2 The gap — self-cost-reduction is not stripped inside a granted quoted ability

The building blocks all exist:

- **Type**: `CostReduction { amount_per: u32, count: QuantityExpr, condition: Option<ParsedCondition> }`
  — `crates/engine/src/types/ability.rs:14027`; carried on `AbilityDefinition.cost_reduction`
  (`ability.rs:14143`).
- **Parser for the phrase**: `try_parse_cost_reduction` (`crates/engine/src/parser/oracle_cost.rs:1222`)
  → routes a `{X}` amount to `try_parse_dynamic_x_cost_reduction` (`:1359`) →
  `parse_dynamic_x_clause` (`crates/engine/src/parser/oracle_static/shared.rs:2247`) →
  `parse_quantity_ref`. The doc-comment at `oracle_cost.rs:1354-1356` **explicitly names "The
  Dominion Bracelet"** as intended-covered by this path.
- **"~'s power" → `QuantityRef::Power { scope: ObjectScope::Source }`**: `parse_self_power_ref`
  (`crates/engine/src/parser/oracle_nom/quantity.rs:1951`), reachable from `parse_quantity_ref`
  (wired at `quantity.rs:608-609`). Handles "its power" / "~'s power" / "this creature's power".
- **The stripper**: `split_trailing_self_cost_reduction` (`crates/engine/src/parser/oracle.rs:4754`)
  splits on `". this ability costs "` and returns `(activation_line, CostReduction)`.
- **Runtime**: `apply_cost_reduction` (`crates/engine/src/game/casting.rs:14550-14573`) reads
  `ability_def.cost_reduction`, resolves `reduce_by = amount_per * resolve_quantity(count, …,
  source_id)`, floors the generic component. For a granted ability `source_id` = the **equipped
  creature**, so `Power{Source}` correctly reads the equipped creature's (LKI-aware) power. **Runtime
  is already correct — no engine change needed.**

**Root cause:** `split_trailing_self_cost_reduction` is wired **only** into `try_parse_equip`
(`oracle.rs:4595`). The generic quoted-activated-ability parser
`parse_quoted_ability` (`crates/engine/src/parser/oracle_static/grammar.rs:1205`) — the cost-separator
branch at `grammar.rs:1259-1284` — parses `effect_text` via `parse_effect_chain_with_context`
**without** stripping the trailing "This ability costs …" sentence, so it degrades into the
`Unimplemented` sub_ability seen above.

### 2.3 Minimal delta (parser-only, no variant, nom-compliant)

In `parse_quoted_ability`'s cost-separator branch (`grammar.rs:1259-1284`), **after**
`strip_activated_constraints` (which pulls `AsSorcery`, `:1268`) and **before**
`parse_effect_chain_with_context` (`:1279`):

```rust
// CR 601.2f + CR 602.2b: a granted activated ability may carry a trailing
// self-cost-reduction sentence ("This ability costs {X} less to activate, where X
// is ~'s power"). Strip it with the single authority used by standalone/equip
// abilities so it becomes `cost_reduction`, not an Unimplemented sub-clause.
let (effect_text, cost_reduction) =
    crate::parser::oracle::split_trailing_self_cost_reduction(effect_text);
```
then `def.cost_reduction = cost_reduction;` after `def` is built. (`split_trailing_self_cost_reduction`
needs `pub(crate)` visibility across the `oracle` → `oracle_static::grammar` module boundary — verify;
it is currently `fn` in `oracle.rs`.)

Ordering check (verified against the oracle text): `strip_activated_constraints` removes the trailing
"Activate only as a sorcery."; the remaining `effect_text` ends with "…where X is ~'s power." so the
`". this ability costs "` split lands correctly. Reuses 100% existing combinators — **no new
`tag()`/string-matching**, satisfies the nom mandate.

**Class covered:** every "gets +N/+M and has \"…: … . This ability costs {X} less to activate, where X
is <QuantityRef>. …\"" equipment/aura (the doc-comment's stated class: Survey Mechan, The Dominion
Bracelet, and future cards), not one card.

### 2.4 Residual verification (do in impl, not a blocker)

- Confirm `apply_cost_reduction` is invoked on the *granted-ability activation* path (callers
  `casting.rs:13267`, `:13497`) so the reduction actually fires when the equipped creature activates.
  The §7 runtime test proves this end-to-end.
- Confirm `GrantAbility` copies `cost_reduction` intact through layer application (it clones the whole
  `AbilityDefinition`, so expected yes).

---

## 3. Secret of Bloodbending — the hard part (CR 723.2 phase-scoped control)

**Oracle:** `As an additional cost to cast this spell, you may waterbend {10}. You control target
opponent during their next combat phase. If this spell's additional cost was paid, you control that
player during their next turn instead. (You see all cards…) Exile Secret of Bloodbending.`

### 3.1 What already parses / exists

- `waterbend {10}` optional additional cost — `AbilityCost::Waterbend` exists
  (`crates/engine/src/types/ability.rs:7149`); optional-additional-cost machinery present. ✅
- Self-exile "Exile Secret of Bloodbending" → `ChangeZone { destination: Exile, target: SelfRef }`. ✅
- The conditional split is recognized: current card-data shows outer ability + a `sub_ability` gated
  by `condition: AdditionalCostPaidInstead` (`crates/engine/src/types/ability.rs:14885`; runtime
  eval present across `game/casting.rs`, `game/effects/mod.rs`, …). The "instead" replacement
  semantics (run paid-branch INSTEAD of base) must be verified at runtime (§3.5). ✅ structurally.
- **Both control branches are currently `Unimplemented`** (base → `Unimplemented{name:"phase"}`;
  paid → `Unimplemented{name:"you","you control that player during their next turn"}`).

### 3.2 Parser gaps (two)

1. **Combat-phase window.** `try_parse_control_next_turn_suffix`
   (`imperative.rs:295-308`) hard-codes the duration as `terminated(alt(("that player's","their",
   "its")), tag(" next turn"))`. "…during their next **combat phase**" fails the `" next turn"` tag →
   `Unimplemented{name:"phase"}`. Needs a duration axis: `alt((tag(" next turn"), tag(" next combat
   phase")))` returning a window discriminant. **Only emit the combat-phase window if §3.4 runtime is
   built** (else strict-fail — do not emit a variant the runtime can't release).
2. **Anaphoric "that player" target.** The paid branch "you control **that player** during their next
   turn" fails because "that player" (`oracle_nom/target.rs:381`, `oracle_target.rs:164`) lowers to
   `TargetFilter::TriggeringPlayer` — correct in a *trigger* body, but Secret is a *sorcery* where
   "that player" is anaphoric to the base clause's targeted opponent, not a triggering player. The
   paid sub_ability must bind its control to the **same spell target** as the base clause. Needs the
   conditional/instead sub-branch to inherit the parent's `target` (shared-target anaphora), not
   re-resolve "that player" independently.

### 3.3 Runtime infra reality — turn_control releases ONLY at turn boundaries

Traced (`turns.rs`): control **activates** in `start_next_turn` (`:539-549`) keyed on the new
`active_player`, and **releases** in `start_next_turn` (`:461-476`) at the *following* turn boundary.
**There is no phase-boundary activation or release path.** Phase transitions run through
`enter_phase` (`turns.rs:147`) → `drain_pending_phase_transition_progress` (`:194`) →
`finish_enter_phase` (`:416`); combat-phase entry is already tracked
(`combat_phases_started_this_turn`, `enter_phase:151-154`; `Phase::is_combat`, `phase.rs:49`).
`finish_enter_phase` (`:416-446`) is the natural phase-start hook. **Phase-boundary release is the
real, currently-missing infra cost.**

### 3.4 Full-build design (if funded) — parameterize, don't proliferate

Type change (variant gate verdict in §5): add a window discriminant to `ControlNextTurn`:

```rust
ControlNextTurn {
    #[serde(default = "default_target_filter_any")] target: TargetFilter,
    #[serde(default)] grant_extra_turn_after: bool,
    #[serde(default)] window: ControlWindow,   // NEW — serde-default = NextTurn (all existing data)
}
pub enum ControlWindow { #[default] NextTurn, NextCombatPhase }   // CR 723.1 / CR 723.2
```
Mirror `window` onto `ScheduledTurnControl` (`game_state.rs:7657`, serde-default `NextTurn`).

Runtime (`turns.rs`, **non-frozen**):
- **Activate (phase window)** in `finish_enter_phase`: when `next == Phase::BeginCombat`,
  `active_player == scheduled.target_player`, and `scheduled.window == NextCombatPhase`, set
  `turn_decision_controller = Some(controller)`. This is "next combat phase" — for a sorcery cast on
  the caster's turn the opponent's next combat phase is on the opponent's next turn, but the hook is
  general (fires at the target's next `BeginCombat` regardless of turn). Guard against re-firing on a
  later combat phase (consume the scheduled entry on activation, or track an "activated" flag).
- **Release (phase window)** in `finish_enter_phase` (or `advance_phase`/`end_combat_phase_to_postcombat`,
  `turns.rs:110`): when leaving the combat phase (entering the first non-combat phase, i.e. `next ==
  Phase::PostCombatMain` per `end_combat_phase_to_postcombat:129`) while a `NextCombatPhase` control is
  active, set `turn_decision_controller = None` and drop the scheduled entry. CR 723.2: limited
  duration — no "beginning of next turn" release.
- The existing turn-boundary activate/release paths stay for `NextTurn` (must NOT clobber a live
  phase-window control; gate the `turns.rs:461` block on `window == NextTurn`).
- `authorized_submitter_for_player` (`turn_control.rs:28`) needs **no change** — it already routes on
  `turn_decision_controller`; setting it during combat is sufficient to pilot the controlled combat
  phase (CR 723.3: controlled player is still the active player).

Dual-branch lowering: base → `ControlNextTurn { target: opponent, window: NextCombatPhase }`; paid
"instead" sub_ability (gated `AdditionalCostPaidInstead`) → `ControlNextTurn { target: <shared>,
window: NextTurn }`. Classify the new `Effect`-field/`ControlWindow` in the fail-closed walker
(`crates/engine/src/game/ability_scan.rs:514` destructures `ControlNextTurn` — add the `window` arm;
default-safe as it does not create a choice `WaitingFor`).

Estimated cost (full build): ~1 small enum + 2 field additions (serde-defaulted) + ~2 `turns.rs`
hooks (~40-70 lines) + resolver read + 2 parser fixes + ability_scan arm + tests. **Bounded, but for
a 1-card class.**

### 3.5 Recommended path for Secret — strict-failure the combat-phase branch (default)

Given the measured 1-card class (§1) and that the card is **all-or-nothing** (fixing only the paid
branch leaves it unsupported, so §3.2-item-2 alone buys no coverage):

- **Do NOT add `ControlWindow` yet.** Leave "…during their next combat phase" as a loud
  `Effect::unimplemented("phase", …)` (already is) — strict-failure, coverage waits, architecture
  stays clean. Record the §3.4 design here so it is ready when a second phase-scoped card lands.
- Secret of Bloodbending stays **unsupported** (its exile + waterbend cost still parse; the control
  effect is the honest gap). This is the CLAUDE.md-endorsed outcome for a near-one-off requiring real
  infra.
- **Full build is the alternative** only if the driver explicitly wants this card supported and funds
  §3.4. There is no honest partial that yields coverage.

---

## 4. add-engine-effect lifecycle deltas (summary)

| Stage | Dominion Bracelet | Secret (only if §3.4 funded) |
|---|---|---|
| types | none | `+ ControlWindow`, `+ window` on `ControlNextTurn` & `ScheduledTurnControl` (serde-default) |
| parser | wire `split_trailing_self_cost_reduction` into `parse_quoted_ability` | combat-phase suffix + shared-target "that player"; dual-branch lowering |
| resolver | none | `control_next_turn::resolve` reads+stores `window` (non-frozen) |
| turn runtime | none | phase-boundary activate/release in `turns.rs` (non-frozen) |
| targeting/mp-filter | none | none (player target already redacted per RNG-seed precedent; verify controlled-turn visibility already handled by `turn_decision_controller`) |
| frontend | none | none (turn piloting already surfaced via `turn_decision_controller`/`derived_views.rs:1115`) |
| AI | none | none new (ControlNextTurn already AI-classified) |
| ability_scan | none | classify `window` axis (`ability_scan.rs:514`) |
| tests | §7 | §7 |

---

## 5. Variant-gate verdict (`add-engine-variant`, Stage 1-3)

**Proposal:** express "control during next combat phase" (CR 723.2 window) vs "control during next
turn" (CR 723.1).

- **Stage 1 — Existence.** `ControlNextTurn` carries only `{target, grant_extra_turn_after}`
  (`ability.rs:8886`); `ScheduledTurnControl` carries no window (`game_state.rs:7657`); no
  `ControlWindow`/`ControlDuration`/`ControlScope` enum exists (grep of `types/ability.rs`).
  Verdict: **DOES_NOT_EXIST** for the phase window.
- **Stage 2 — Parameterization.** A sibling `Effect::ControlNextCombatPhase` would differ from
  `ControlNextTurn` on exactly one axis — the *control window/duration* — the classic
  "differ only in a scope/duration dimension" smell. **Verdict: REFACTOR_FIRST → parameterize** with a
  `window: ControlWindow { NextTurn, NextCombatPhase }` field on the existing `ControlNextTurn`, NOT a
  new sibling. (CLAUDE.md categorical-boundary note: full-turn 723.1 and limited-duration 723.2 are
  the same CR concept parameterized by duration.)
- **Stage 3 — Categorical boundary.** The parameterization axis (control window) lies entirely within
  **CR 723** (control another player; 723.1 vs 723.2 are subsections). **Verdict: WITHIN_SECTION.**
- **APPROVED as parameterization** *iff the §3.4 runtime is built in the same commit* (no silent
  stub). **If Secret is strict-failed (§3.5, recommended), the extension is NOT taken** — the
  combat-phase branch stays `Unimplemented` and `ControlWindow` is deferred with its runtime.
  Frozen-file check: `effects/mod.rs` dispatch uses `Effect::ControlNextTurn { .. }` (`:2992`) and
  `{ target, .. }` (`:4931`) — a new field does **not** touch the frozen file. Serialized-surface:
  `ControlNextTurn` and `ScheduledTurnControl` appear in card-data + game state; both new fields must
  be `#[serde(default)]` (= `NextTurn`) so all existing fixtures/data load unchanged.

**Dominion Bracelet needs NO variant** (parser-only; all types pre-exist).

---

## 6. Frozen-file & CR-annotation compliance

- **Frozen** (design edits forbidden): `game/effects/mod.rs`, `game/filter.rs`,
  `game/effects/delayed_trigger.rs`. **None are touched** by either increment (dispatch in
  `effects/mod.rs` is `{ .. }`; all edits land in `parser/…`, `types/ability.rs`, `types/game_state.rs`,
  `game/effects/control_next_turn.rs`, `game/turns.rs`, `game/ability_scan.rs` — all non-frozen).
- **CR annotations** (grep-verified in `docs/MagicCompRules.txt`):
  - `docs/MagicCompRules.txt:6159` — **723.1** full-turn control ("controlled during the entire turn;
    the effect doesn't end until the beginning of the next turn").
  - `:6161` — **723.1a** last-wins overwrite. `:6163` — **723.1b** skipped-turn pending waits.
  - `:6165` — **723.2** "Two cards (Word of Command and Opposition Agent) allow a player to control
    another player for a limited duration." (NOTE: this CR-text revision predates Secret of
    Bloodbending; annotate the phase branch `CR 723.2` as a limited-duration control and add a code
    comment that the enumerated-card list is non-exhaustive for newer sets.)
  - `:6167` — **723.3** controlled player is still the active player; objects keep normal controllers.
  - `CR 601.2f` (self-referential cost reduction) + `CR 602.2b` (activation cost) for the Bracelet's
    `CostReduction` (grep-verify before writing).
- **Nom mandate:** Bracelet fix reuses existing combinators only. Secret's suffix/window changes must
  extend the existing `alt()`/`tag()` composition in `try_parse_control_next_turn_suffix` — a single
  `alt((tag(" next turn"), tag(" next combat phase")))` axis, no full-string enumeration, no
  `contains`/`split_once` dispatch.

---

## 7. Test plan (discriminating, revert-to-red; `card-test` recipe)

Use `GameScenario` + `GameRunner::cast(...).resolve()`; assert on `CastOutcome`/state deltas, not
AST-internal flags.

### 7.1 Anti-hollow-win — end-to-end control PILOT (covers BOTH cards' next-turn claim)
The existing turn-control tests (`turns.rs:6887-6981`) drive `start_next_turn` on a hand-planted
`scheduled_turn_controls` — they prove the activation/release/priority-handoff state machine but
**never exercise the cast→resolve→schedule chain**. `casting_tests.rs:26056`
(`is_mana_ability_classifier_authoritative`) is a classifier unit test, not a pilot. **Add** a full
cast-pipeline test:
- Cast **Worst Fears** (sorcery, no cost — simplest) targeting the opponent; resolve. Assert
  `scheduled_turn_controls` gained the entry (parse→resolve chain).
- Advance to the opponent's next turn (`start_next_turn`). Assert `turn_decision_controller ==
  caster` AND `priority_player == caster` (controller genuinely pilots) AND opponent is still
  `active_player` (CR 723.3).
- Advance one more turn boundary. Assert `turn_decision_controller == None` and the schedule cleared
  (release). **Revert-to-red:** breaking the resolver's `scheduled_turn_controls.push` or the
  `turns.rs:548` activation must fail this test.

### 7.2 The Dominion Bracelet (Increment 1)
- **Parser (unit, discriminating):** parse the granted quoted ability; assert
  `def.cost_reduction == Some(CostReduction { amount_per: 1, count: Ref(Power{Source}), condition:
  None })` AND `def.sub_ability` no longer carries the `Unimplemented{name:"this"}` clause. **Revert:
  with the §2.3 wiring removed, the field is `None` and the Unimplemented sub_ability returns** —
  non-vacuous.
- **Runtime (cast pipeline):** equip a creature of known power P; activate the granted `{15}, Exile`
  ability; assert the mana paid = `max(0, 15 − P)` (generic floored at 0), the equipment is exiled,
  and a `ScheduledTurnControl` targeting the opponent is created. Vary P (e.g. P=0, P=5, P=20→cost 0)
  to prove the reduction is dynamic and floors. **Revert:** a static `{15}` (no reduction) fails the
  P>0 case.

### 7.3 Secret of Bloodbending
- **If strict-failed (§3.5, recommended):** assert the base clause remains a loud
  `Effect::Unimplemented { name: "phase", .. }` (coverage-honest gap) and the exile still parses — a
  guard test documenting the deliberate deferral (prevents a silent-stub regression).
- **If full-built (§3.4):** (a) parser: base → `ControlNextTurn { window: NextCombatPhase }`, paid
  sub_ability → `ControlNextTurn { window: NextTurn }` bound to the **same** target; (b) runtime: cast
  WITHOUT waterbend → opponent's next combat phase: `turn_decision_controller == caster` at
  `BeginCombat`, released at `PostCombatMain`, and NOT active outside combat; cast WITH waterbend {10}
  → full-turn control (release at next turn boundary). **Revert:** removing the `finish_enter_phase`
  release hook leaves `turn_decision_controller` set into `PostCombatMain` — fails.

---

## Risks / open questions for /review-engine-plan

1. **`split_trailing_self_cost_reduction` visibility & ordering** (§2.3): it is a private `fn` in
   `oracle.rs`; the fix needs it `pub(crate)` for `oracle_static::grammar`. Confirm the split runs
   AFTER `strip_activated_constraints` and that no other trailing sentence (reminder text is already
   stripped upstream) breaks the `". this ability costs "` anchor. Confirm `apply_cost_reduction`
   fires on the granted-ability activation path (callers `casting.rs:13267/:13497`).
2. **"instead" replacement semantics** (§3.1/§3.5): does a `sub_ability` gated by
   `AdditionalCostPaidInstead` actually run the paid branch *in place of* the base branch (not in
   addition)? Trace the resolver before relying on it for Secret's dual-branch — mis-wiring could run
   both controls.
3. **ROI call on Secret** (§0.1/§3.5): planner recommends strict-failure (1-card class, real
   infra). Confirm the driver/tranche does not have a second phase-scoped-control card in flight that
   would change the class size and flip the verdict to full-build.
4. **Anaphoric "that player" in a sorcery** (§3.2-2): binding the paid sub-branch to the parent
   spell's target (vs `TriggeringPlayer`) is the correct model — verify the conditional sub_ability
   target-inheritance mechanism exists (or is the deeper of the two parser fixes).
5. **Phase-window re-fire guard** (§3.4): "next combat phase" = the FIRST combat phase the target
   takes; the activation hook must consume/flag the schedule so a second combat phase (extra-combat
   effects) does not re-trigger control. Confirm interaction with `extra_phases`/`combat_phases_
   started_this_turn`.
6. **`ability_scan.rs:514` full-destructure**: verify whether the `ControlNextTurn` arm uses `..`; if
   it fully destructures, the new `window` field forces an edit there (non-frozen, expected).
