# Adversarial architectural review — `COMBO-DETECTOR-LIVE-PLAN.rev3.md`

**Reviewer:** plan-gate sub-agent · **Date:** 2026-07-14
**Code truth:** `/home/lgray/vibe-coding/phase-rs-workdir` @ `efc76ca1b` (`main`), read-only.
**Probe worktree:** `/home/lgray/vibe-coding/combo-probe-wt` @ `efc76ca1b` (warm target; all runtime evidence below was produced there).
**Framework applied:** `.claude/skills/review-engine-plan/SKILL.md` (all 11 required checks) · `.claude/skills/engine-planner/SKILL.md` · `.claude/skills/add-engine-variant/SKILL.md` · `CLAUDE.md`.
**Binding constraints honored:** the rules layer was NOT re-litigated (CR numbers were only mechanically verified). `LoopDetectionMode`'s three variants + both UI toggles were treated as settled (P5 = doc-comment-only).

---

# 1. VERDICT: ⛔ **REJECT**

Rev 3 is the best document this workstream has produced. Its **CR layer is clean** — **every** CR citation resolves at the **exact** line it claims. Every `file:line` it cites into code *that it actually opened* is **CONFIRMED**. Its measurements (M0–M2, M4, M5) **reproduce**.

But it has **five blockers**, and the two worst share a single root cause:

> ## **The two type-declaration ranges the plan got WRONG are exactly the two places its MODEL OF THE TYPE is wrong.**

One blocker makes **P1 a structural no-op** (the canary can never offer, so acceptance criterion #1 cannot pass and criterion #2 passes vacuously). Another is a **false-certificate hole of precisely the `Effect::Token` class Rev 3 congratulates itself for catching**. A third silently changes **live, default-configuration CR 603.3b game behavior** on a path the plan never analyses — while the plan explicitly guards *the same conjunct* one section later.

---

# 2. BLOCKERS

## ⛔ B1 — §P1-b's capture gate is STRUCTURALLY DEAD. The canary can NEVER offer. **MEASURED.**

### Passage (§P1-b)

```rust
state.last_loop_action_context = (state.loop_detection.samples()
    && state.battlefield.len() > battlefield_len_before   // ⛔ THIS
    && source_obj.zone == Zone::Battlefield)
    .then_some(LoopActionContext { .. });
```
…described as *"armed at the **same settle discipline** as the recast"* and *"Cheap (a length compare)"*.

### Evidence

An activated ability goes on the **STACK** (CR 602.2a — verified at `docs/MagicCompRules.txt:2529`). **The token is created on RESOLUTION**, one or more `PassPriority` beats later.

- The `Priority` + `GameAction::ActivateAbility` arm is `crates/engine/src/game/engine.rs:3217-3224`.
- The non-mana branch dispatches to `casting::handle_activate_ability` at **`engine.rs:3286`** — which puts the ability on the stack.
- ⇒ **the battlefield has not grown at the beat the plan evaluates its gate.**

**Test output I produced** — `combo-probe-wt`, `crates/engine/tests/integration/probe_review_r1.rs::r1_battlefield_does_not_grow_at_the_activateability_beat` (**PASSES**, i.e. the defect is confirmed):

```
===== R1: P1-b capture gate, measured at the ActivateAbility beat =====
  battlefield BEFORE ActivateAbility  = 3
  battlefield AFTER  ActivateAbility  = 3
  stack       AFTER  ActivateAbility  = 1
  P1-b GATE (`state.battlefield.len() > battlefield_len_before`) = false
  battlefield AFTER settle (PassPriority beats) = 4
  growth happened at the ActivateAbility beat? false   at a LATER beat? true
```

### Consequences — both fatal to the plan's own §9 acceptance criteria

- **Criterion #1** (`activation_loop_gond_intruder_alarm_offers_shortcut`) **CANNOT PASS.** The context never arms ⇒ bridge (B)'s `last_loop_action_context.is_some()` conjunct (`engine.rs:450`) stays red ⇒ **no offer.** The plan's entire *raison d'être* — M1's *"every conjunct of bridge (B) is green except `last_recast_context`"* — **is not discharged by P1 as written.**
- **Criterion #2** (`activation_loop_without_untapper_does_not_offer`, the ⭐ discriminator) becomes **VACUOUS**: it passes because *nothing offers, ever*. This is exactly what `review-engine-plan` check 9 bars ("a bare negative assertion that an upstream conjunct satisfies vacuously is NOT a test").

### Root cause — the Prime Directive's own pattern

**The plan never cites a `file:line` for the `ActivateAbility` handler.** §P1-b literally says: *"in the `GameAction::ActivateAbility` handler (`game/engine.rs`; **find it via `find_referencing_symbols`**)"*. **The author wrote code for a site they never opened.**

The recast arm it claims to mirror uses a **STATIC** predicate, not a runtime battlefield delta:

```rust
// crates/engine/src/game/casting_costs.rs:6789
let is_token_creating = matches!(ability.effect, crate::types::ability::Effect::Token { .. });
// ...
// crates/engine/src/game/casting_costs.rs:6795
state.last_recast_context = (state.loop_detection.samples()
    && additional_cost_paid && has_buyback && is_token_creating).then_some(RecastContext { .. });
```

⇒ the *"same settle discipline as the recast"* claim is **FALSE**.

### Required revision

Arm the capture on a **static** property of the activated ability's definition (the `Effect::Token`-in-chain predicate, genuinely mirroring `is_token_creating`) — **not** on a battlefield delta measured at a beat where the battlefield provably has not moved. If a dynamic gate is genuinely wanted, it must be evaluated at the **resolution** beat, and the plan must then name the owning reducer site (with `file:line`) and show how the capture survives to bridge (B). **Re-derive P1-b against the real handler and cite it.**

---

## ⛔ B2 — §P2-c is built on a WRONG MODEL OF THE TYPE. `ManaProduction` is a 15-variant ENUM, not a struct with a `count`. Descending "into `count`" **IS** a false certificate.

### Passage (§P2-c)

> *"`game/ability_scan.rs:862` → descend into `ManaProduction`'s `count: QuantityExpr` (`types/ability.rs:1689-1701`)."*
> *"⭐ **P2-c IS WHAT MAKES THE GAEA'S CRADLE NEGATIVE MEAN ANYTHING.**"*

### Evidence (measured myself)

`pub enum ManaProduction` spans **`crates/engine/src/types/ability.rs:1676`–`:1833`** and has **15 variants**.

**The plan's cited range `:1689-1701` is the body of ONE variant** (`Colorless { count: QuantityExpr }` at `:1689`) plus the head of the next (`AnyOneColor` at `:1699`). The author read a single variant's field list and mistook it for the type.

**Three variants have NO `count` field at all — and two of them read game state:**

| Variant | Line | Payload | Under P2-c's "descend `count`" |
|---|---|---|---|
| ☠️ **`DistinctColorsAmongPermanents`** | **`:1810`** | **`{ filter: TargetFilter }` — NO `count`** | ⇒ **`Axes::NONE`**. A **board-aggregate reader** (Faeburrow Elder: mana = # distinct colors among matching permanents) **certified inert.** |
| ☠️ **`TriggerEventManaType`** | **`:1832`** | **unit variant, no fields.** Doc at `:1830`: *"Resolves from `state.current_trigger_event` at resolution time"* | ⇒ **`Axes::NONE`** on the **event** axis. |
| `Mixed` `:1694` · `ChoiceAmongCombinations` `:1784` · `ChoiceAmongExiledColors` `:1774` | | no `count` (the last reads `state.exile_links`) | ⇒ `NONE` |

Three more carry read-bearing fields the descent would elide:
`AnyCombinationOfObjectColors { count, scope: ObjectScope }` `:1749` · `AnyTypeProduceableBy { count, land_filter: TargetFilter }` `:1764` · `AnyOneColorAmongPermanents { count, filter: TargetFilter, .. }` `:1816`.

**The codebase's own classifier proves the hole.** `DistinctColorsAmongPermanents`'s doc (`:1809`) says it *"mirrors the structure of `QuantityRef::DistinctColorsAmongPermanents`"* — which `game/ability_scan.rs:2090-2097` classifies as **`sibling: true` + `scan_target_filter(filter)`**.

⇒ **Today's blanket (`Effect::Mana { .. } => Axes::CONSERVATIVE`, `ability_scan.rs:862`) is CORRECT. P2-c as written REPLACES A CORRECT REJECTION WITH A FALSE ACCEPTANCE.**

**And `Effect::Mana` carries 5 fields, not 1** (`types/ability.rs:10663-10685`):
`produced: ManaProduction` (`:10665`) · `restrictions: Vec<ManaSpendRestriction>` (`:10667`) · `grants: Vec<ManaSpellGrant>` (`:10670`) · `expiry: Option<ManaExpiry>` (`:10674`) · **`target: Option<TargetFilter>` (`:10684`)** — read-bearing (Jeska's Will: *"Add {R} for each card in **target opponent's** hand"*, per the field's own doc at `:10675-10682`). The plan names only `produced`.

### Required revision

P2-c must be an **exhaustive, no-`..` destructure of `Effect::Mana`'s 5 fields** PLUS an **exhaustive, no-`_` match over all 15 `ManaProduction` variants**, with `DistinctColorsAmongPermanents`, `TriggerEventManaType`, `ChoiceAmongExiledColors`, and every `TargetFilter`/`ObjectScope`-bearing variant classified **read-bearing**.

⚠️ **This also invalidates the plan's `gaeas_cradle_mana_ability_still_vetoes` test design**, which asserts the veto arrives *"via the Mana `count` read"* — that is not how Cradle's variant is shaped.

---

## ⛔ B3 — §P2-b and §P2-c silently flip the **LIVE** CR 603.3b trigger-ordering gate. **UNGATED by `loop_detection`. MEASURED A/B.**

### Passage (§P3-b — the plan's own over-edit guard)

> *"`event` at `:2419` **STAYS `true`**. We touch `sibling` ONLY… With `event` still `true`, `ability_uses_event_context` still returns `true` for any `Typed`-bearing ability ⇒ `c2_order_independent` stays `false` ⇒ **CR 603.3b behavior is BYTE-UNCHANGED.**"*

**P2-b and P2-c destroy that exact premise for their own effects — and the plan never notices.**

### Evidence

`Axes::CONSERVATIVE` sets **all three** axes true (`crates/engine/src/game/ability_scan.rs:131-135`):
```rust
const CONSERVATIVE: Axes = Axes { event: true, sibling: true, projected: true };
```
So today, a triggered ability whose body is `Effect::Token` gets `event = true` **solely** from the blanket at `ability_scan.rs:447`. **Descending it to `Axes::NONE` removes the only thing setting it.**

The live gate (`crates/engine/src/game/triggers.rs:3894-3895`, verbatim):
```rust
let c2_order_independent = !crate::game::ability_scan::ability_uses_event_context(&reference)
    && !crate::game::ability_scan::ability_reads_sibling_mutable(&reference);
```

**Test output I produced** — A/B on the SAME harness (`combo-probe-wt`, `crates/engine/src/game/ability_scan.rs::review_probe_r3::r3_token_effect_axes_and_the_c2_ordering_gate`), on a vanilla *"create a 1/1 Soldier"* `Effect::Token` ability. Row 2 is a **revert-probe** in which I temporarily restored `Effect::Token { .. } => Axes::CONSERVATIVE` and re-ran:

| `Effect::Token` arm | `ability_uses_event_context` | `ability_reads_sibling_mutable` | **`c2_order_independent`** |
|---|---|---|---|
| **main baseline** (`=> Axes::CONSERVATIVE`) | `true` | `true` | **`false`** ⇒ engine **PROMPTS** the player to order triggers |
| **with P2-b descent** | `false` | `false` | **`true`** ⇒ engine **AUTO-ORDERS**, **no prompt** |

Raw output, with-P2-b:
```
===== R3: CR 603.3b gate for a vanilla 'create a 1/1 Soldier' ability =====
  ability_uses_event_context   = false
  ability_reads_sibling_mutable = false
  c2_order_independent (triggers.rs:3894) = true
```
Raw output, revert-probe (main baseline):
```
  ability_uses_event_context   = true
  ability_reads_sibling_mutable = true
  c2_order_independent (triggers.rs:3894) = false
```

**Downstream:** the terminal C2 decision is `c2_order_independent && !batch_conflict` (`triggers.rs:3654`), consumed by `group_is_order_independent` (`triggers.rs:3774`) → **`triggers.rs:4155`** (`g.ordered = true` ⇒ **no prompt**) and `triggers.rs:4361`.

**This path is NOT gated by `loop_detection`** — the repo's own test name says so:
`fn pr625_c2_distinct_event_auto_orders_even_when_loop_detection_off()` at **`triggers.rs:23237`**.

⇒ **P2-b changes DEFAULT-CONFIGURATION LIVE GAME BEHAVIOR** for token-bodied triggered abilities in a distinct-event group. The same argument applies to P2-c for non-CR-605.1b mana triggers.

**Nothing in the tree catches it.** `ordering_parity_sweep` (`crates/engine/src/game/triggers_ordering_parity_tests.rs:492`) compares the legacy serde oracle against `ability_rw` — **it never touches `ability_scan`**. There is **no test anywhere** asserting the axes of `Effect::Token` or `Effect::Mana` — in stark contrast to `TargetFilter::Typed`, which HAS a tripwire (`analysis/resource.rs:3926`) that the plan correctly handles.

### Honest status

- The **flip of `c2_order_independent` is MEASURED** (table above).
- The end-to-end **prompt → auto-order** consequence *additionally* requires `batch_conflict == false`. That is a **CODE TRACE, NOT A RUN**: `ability_rw.rs:4671-4700` gives `Effect::Token` a write-only profile ⇒ `source_independent()` ⇒ the distinct-event fast path at `ability_rw.rs:1082-1090` returns `false`. **I did NOT execute that half — UNVERIFIED.**

This does not soften the blocker: the plan **silently moves a live-path soundness conjunct it never analysed**, while explicitly guarding *the same conjunct* one section later.

### Required revision

Add a §P3-b-equivalent section for `Effect::Token` and `Effect::Mana`. Either:
(a) prove the C2 term is inert/unreachable for these shapes with a **runtime** fixture; or
(b) preserve `event: true` on the descended arms and relax only `sibling`/`projected` (the axes the loop covers actually consume); or
(c) accept the CR 603.3b change **explicitly**, with a named runtime test and a `FORGE_TEST_FULL_DB=1 cargo test -p engine ordering_parity_sweep` delta.

**`add-engine-variant` Step 3 mandates exactly this audit. The plan never invokes the skill.**

---

## ⛔ B4 — `Effect::Token.keywords` is NOT read-free. §P2-b bins it under `_`.

### Passage (§P2-b's field table)

> | `name` `types` `colors` **`keywords`** `tapped` `enters_attacking` `supertypes` | — | `_` **with a one-line justification each** |

### Evidence

`Effect::Token.keywords` is `Vec<Keyword>` (`crates/engine/src/types/ability.rs:9557`). **`Keyword` carries dynamic payloads** (`crates/engine/src/types/keywords.rs`, verified):

- **`Mobilize(QuantityExpr)`** — `keywords.rs:859`
- **`Firebending(QuantityExpr)`** — `keywords.rs:939`
- `Enchant(TargetFilter)` — `keywords.rs:645`
- `Craft { cost, materials: TargetFilter, count }` — `keywords.rs:783-788`
- `Ward(WardCost)` → `WardCost::Sacrifice { count, filter: TargetFilter }` — `keywords.rs:534-537`
- `CumulativeUpkeep(AbilityCost)` — `keywords.rs:809`; `Escalate(AbilityCost)` — `keywords.rs:993`
- `Affinity(TypedFilter)` — `keywords.rs:802`

**The file's own code already refuses to treat `Keyword` as inert:** `keyword_cost_reads_growing_class` (`ability_scan.rs:3867`) is a full exhaustive `Keyword` match, and **`modification_grants_growing_cost_keyword` (`ability_scan.rs:4080` — the very precedent §P2-a cites as its model)** routes `ContinuousModification::AddKeyword { keyword } => keyword_cost_reads_growing_class(keyword)`.

⇒ **Binding `keywords` to `_` is the SAME class of error as Rev 2's `attach_to` / `static_abilities` / `enter_with_counters` miss — the one Rev 3 exists to have caught.**

The identical defect appears in §P2-a, which puts **`AddKeyword` (`:19397`) and `RemoveKeyword` (`:19400`)** in the **`Axes::NONE` "read-free structural"** bucket.

*(For the record: the other six dismissed `Effect::Token` fields — `name` `:9547`, `types` `:9553`, `colors` `:9555`, `tapped` `:9559`, `enters_attacking` `:9571`, `supertypes` `:9574` — ARE confirmed read-free. `Effect::Token`'s field list is otherwise **complete**: 14 fields exist, 14 named. ✅)*

### Required revision

Classify `Effect::Token.keywords` and `ContinuousModification::AddKeyword`/`RemoveKeyword` as **read-bearing**, descending each `Keyword` payload (`QuantityExpr` / `TargetFilter` / `AbilityCost` / `TypedFilter`).

---

## ⛔ B5 — `ContinuousModification` has **53** variants, not 41. Ten are unnamed — one carries a `QuantityExpr`. Three "descend depth ≤ 1" variants reach types with **NO walker**.

### Passage (§P2-a)

> *"**Classification of all 41 measured variants** (`types/ability.rs:19350`–`:19599`)"*
> and §5: *"**P2-a** classifies **all 41** `ContinuousModification` variants — every static/aura/anthem/equipment grant in the game."*

### Evidence (I counted them myself)

`pub enum ContinuousModification` spans **`crates/engine/src/types/ability.rs:19349`–`:19711`** (NOT `:19350`–`:19599`). **Variant count: 53.**

The plan names **43** variants — **all 43 exist** ✅ — and **never names these 10**:

| Unnamed variant | Line |
|---|---|
| ☠️ **`AddCounterOnEnter { counter_type: CounterType, count: QuantityExpr, if_type: Option<CoreType> }`** | **`:19686`** |
| `AddSupertype` | `:19663` |
| `AssignNoCombatDamage` | `:19601` |
| `ChangeController` | `:19604` |
| `RemoveManaCost` | `:19710` |
| `RemoveSupertype` | `:19672` |
| `SetBasicLandType` | `:19607` |
| `SetChosenBasicLandType` | `:19621` |
| `SetChosenName` | `:19630` |
| `SetStartingLoyalty` | `:19700` |

☠️ **`AddCounterOnEnter` carries a `QuantityExpr`.** It *looks* structural. An implementer sweeping the 10 unnamed lookalikes into the plan's "read-free structural" bucket **certifies a dynamic counter count as inert**.

*(The plan's no-`_`-wildcard mandate makes the bare omission fail-SAFE — a compile error. The danger is the `Axes::NONE` bucket the implementer will reach for on the way back.)*

**Worse — the "Ability-bearing (descend, depth ≤ 1)" bucket reaches types OUTSIDE the scanner's declared traversal closure, with no walkers:**

- **`CopyValues`** → `CopiableValues` (`types/ability.rs:19303-19318`) holds **`replacement_definitions: Arc<Vec<ReplacementDefinition>>` (`:19316`)** — a type the `ability_scan.rs` module header (`:32-36`) **explicitly names as not-descended and therefore CONSERVATIVE**. A depth-limited descent that stops short of it is a false certificate.
- **`AddStaticMode { mode: StaticMode }`** → `StaticMode` (`crates/engine/src/types/statics.rs:798`) has **119 variants**, many carrying `TargetFilter`. **There is no `scan_static_mode` in `ability_scan.rs`.**
- **`GrantStaticAbility { definition: Box<StaticDefinition> }`** (and `Effect::Token.static_abilities`) → `StaticDefinition` (`types/ability.rs:18412-18472`) carries `mode: StaticMode` (`:18414`) **and `per_player_condition: Option<ParsedCondition>` (`:18454`)** — neither type has a walker.

### Required revision

- Re-count and classify **all 53** variants.
- Demote **`CopyValues`**, **`AddStaticMode`**, and any **`StaticDefinition`** descent to **`Axes::CONSERVATIVE`** until walkers exist for `ReplacementDefinition` / `StaticMode` / `ParsedCondition`. The plan's own §3.2 soundness asymmetry demands it (*"a subtree the walk does not descend into ⇒ fail-closed"*).
- §P2-a's *"all 41"* and §5's Pattern-Coverage claim are **FALSE** and must be corrected. (U2 correctly flagged the classification as unverified — but §5 then *relies* on it anyway.)

---

# 3. MATERIAL GAPS

### G1 — The `add-engine-variant` skill was never run for the new `LoopAction` enum
`review-engine-plan` **check 8** (skill-checklist adherence). The skill's **Step 7 (Serialized-surface audit)** would have caught **G2**; its **Step 3 (classify the variant in the fail-closed ability-scan walker / ability_rw profiler)** would have caught **B3**. The plan proposes a brand-new engine enum and never routes it through the mandated gate.

### G2 — Serialized-surface audit missing for the `RecastContext` → `LoopActionContext` rename *(ranked target (b))*

**The field IS on the serialized surface:**
- `RecastContext` derives `Serialize, Deserialize` (`crates/engine/src/types/game_state.rs:370`, struct at `:371-383`, **5 fields**: `card_id` `:374`, `controller` `:375`, `from_zone` `:377`, `uses_buyback` `:379`, `convoke` `:382`).
- `GameState` derives `Serialize, Deserialize` (`game_state.rs:6788`).
- The field: `#[serde(default, skip_serializing_if = "Option::is_none")] pub last_recast_context: Option<RecastContext>` — `game_state.rs:8619-8620`.
- `GameState` ships **whole** to the frontend: `ClientGameStateRef { state: &'a GameState, derived }` — `crates/engine/src/game/derived_views.rs:286-290`, emitted by `crates/engine-wasm/src/lib.rs:1105`.
- `GameState` is persisted **whole**: `TrustedGameStateEnvelope { state: GameState, .. }` — `game_state.rs:3395`.
- The multiplayer viewer filter (`crates/engine/src/game/visibility.rs:18`) **does NOT redact it**.
- **The engine's own comment says so** — `crates/engine/src/game/casting_costs.rs:6792`: *"`last_recast_context` is `skip_serializing_if=is_none`, so a spurious `Some(..)` in OFF mode would appear in a save/replay/scenario."*

**BUT it is NOT the B6 defect class.** It is a **write-only leaf with no consumer**:
- `grep -rn "last_recast_context\|lastRecastContext\|RecastContext" client/` → **ZERO hits.** (`client/src/adapter/types.ts`'s `interface GameState` does not declare it.) Contrast `LoopDetectionMode`, which IS named at `client/src/adapter/types.ts:2647` and is **sent inbound** by the client.
- `fixtures/adapter-contract/state_update.json`, `game_started.json` → **ZERO hits.**
- `combo-verify` bin / `analysis/corpus.rs` → **ZERO hits.**
- All construct/read sites are in-crate (`crates/engine`), ~17 of them.

**Exposure:** `GameState` has **no `deny_unknown_fields`** and the field carries **`#[serde(default)]`**. So an old save containing `"last_recast_context": {...}` deserializes under the new name by **ignoring the unknown key and defaulting to `None`** — a **SILENT LOSSY DROP** of a pending object-growth offer, not an error. Blast radius is tiny (capture is gated on `loop_detection.samples()`, default OFF).

**Required revision:** the plan must (a) **state this audit** (it currently says nothing about the serialized surface), and (b) either add **`#[serde(alias = "last_recast_context")]`** on the renamed field (one line, zero cost) or explicitly accept the lossy drop in writing.

*(Note for the record: the plan's `LoopActionContext` **does** correctly retain `convoke`. All 5 of `RecastContext`'s fields are accounted for in the proposed reshape. `impl PartialEq for GameState` **does** exclude the field — `game_state.rs:11019-11025` (the exhaustive-destructure guard with `last_recast_context: _`) and `:11046`'s `eq` body never names it — so §P1-a's ONE-SIDED-SAFETY argument for keeping `resource.rs:1444`'s conjunct is **CONFIRMED CORRECT**. ✅)*

### G3 — §P1-c's `min_by_key` re-find *(ranked target (c))*: the two-copies hypothesis is **REFUTED for real cards**, but **REAL for tokens**

**REFUTED for real cards.** `CardId` is minted **per physical card**, from the object-id counter:
`crates/engine/src/game/scenario.rs:229` and `:351` — `let card_id = CardId(self.state.next_object_id);` (same pattern at `analysis/corpus.rs:979`, `:1422`).
⇒ two copies of the same card carry **DIFFERENT** `CardId`s, so `filter(o.card_id == ctx.card_id)` already selects a unique permanent and `min_by_key` is a harmless tiebreaker.

**Test output I produced** — `probe_review_r1.rs::r2_min_by_key_refind_picks_the_wrong_permanent` (my two-bear fixture; the test **FAILS**, i.e. **my hypothesis was refuted and I am reporting it as refuted**):
```
===== R2: P1-c drive re-find with two same-card permanents =====
  decoy_bear id=ObjectId(1) card_id=CardId(1) abilities=0
  real_bear  id=ObjectId(2) card_id=CardId(2) abilities=1
  same card_id? false
  the player ACTIVATED       : ObjectId(2)
  Rev3's re-find SELECTS     : ObjectId(2)
  ⇒ re-find picked the permanent the player activated? true
```

**⛔ BUT: REAL FOR TOKENS.** Every plain token is created with **`CardId(0)`** — `crates/engine/src/game/effects/token.rs:813` and `:1060` (`GameObject::new(id, CardId(0), owner, name, Zone::Battlefield)`); asserted at `token.rs:3360`.

⇒ If the loop's **driving permanent is itself a token** — **the exact classes §P1 claims to cover ("Marneus Calgar, Ivy Lane Denizen chains")** — then `ctx.card_id == CardId(0)`, and the re-find
```rust
filter(|o| o.card_id == ctx.card_id && o.zone == Battlefield && o.controller == ctx.controller)
    .map(|o| o.id).min_by_key(|id| id.0)
```
matches **EVERY TOKEN THAT PLAYER CONTROLS — including the fodder tokens the loop is manufacturing every iteration.** `min_by_key` picks the lowest-id token, which need not be the driver. The wrong permanent is driven ⇒ the wrong Δ is measured.

**Required revision:** re-find by an identity that survives the growth, **or fail closed** when `ctx.card_id == CardId(0)` / the source `is_token`.

### G4 — `LoopAction::Activate { ability_index: usize }` is a **positional index**, not an identity
`review-engine-plan` **check 10** (identity/provenance contract) requires naming the *selected authority type and id/value, binding time, live-vs-snapshotted semantics, invalidation behavior, and a multi-authority hostile fixture*. `ability_index` indexes the **layer-derived** `obj.abilities` vec — the canary's ability exists **only** because an Aura grants it (the probe harness locates it via `granted_ability_index`). If the granted-ability set changes between capture and drive (a second aura, a removal, a layer re-eval), the index silently addresses a **different ability**. The plan owes the hostile fixture: two auras granting two abilities to one creature.

### G5 — §P3-b's consumer-completeness claim is **FALSE**
> *"The **second** consumer of the `sibling` axis is `game/triggers.rs:3893-3894`."*

There are more live `sibling` consumers: `analysis/resource.rs:1512` (replacement `runtime_execute` reads growth class) and `resource.rs:1610` (`stack_entry_reads_growing_class`), plus the `ability_definition_reads_sibling_mutable` family at `resource.rs:1472, 1495, 1519, 1573`. All are `loop_detection`-gated, so **P3 itself is still probably safe** — but the plan's completeness claim is not, and completeness claims are what this workstream keeps getting wrong.

### G6 — U3's merge gate: **SOUND. No finding.** ✅
The plan's `sibling || projected` reasoning holds under audit:
- `fire_time_conditions_read_projected_resource` (`analysis/resource.rs:2152`) scans a static's **`condition` only** — CONFIRMED.
- `resource.rs:1539` (`if !def.modifications.is_empty() { return true; }`) is the file's **only** `modifications.is_empty()` check — CONFIRMED.
⇒ the sibling blanket **is** incidentally the only protection the object/fodder covers have against a projected-reading modification, and vetoing on `sibling` alone **would** open a real hole. The merge gate (*"DO NOT MERGE P2 WITHOUT THIS TEST GOING RED ON THE REVERT"*) is stated strongly enough to block. **Coherent and correctly scoped.**

---

# 4. NITS

- **`triggers.rs:3893-3894`** (the `c2_order_independent` gate) → actual **`:3894-3895`**. **DRIFTED** (one line).
- P4's `object_functions` trap (`game/functioning_abilities.rs:108-116` — *"IT RETURNS `true` FOR A CARD IN THE LIBRARY"*) is **real and correctly identified** ✅. P4's soundness direction (REJECT-safe) is correctly classified, and its honest label (*"clears ZERO of V1/V2/V3"*) is exemplary.
- P6's disambiguation table (scan (6) **firewall** vs `GameState::PartialEq`'s `delayed_triggers` **cover** conjunct) is correct and valuable — keep it.
- §2's *"written `casting_costs.rs:6795`; read `game/engine.rs:450`"* is **exact** ✅.
- DEFERRED-3 (`mandatory` computed at an intra-cycle instant, CR 104.4b) is a genuine find and correctly filed rather than fixed.

---

# 5. CITATION AUDIT

Legend: **CONFIRMED** = line contains what the plan says · **DRIFTED** = right thing, wrong line · **WRONG** = materially incorrect · **UNVERIFIED** = not checked.

## Code citations

| Plan citation | What the plan claims | Status |
|---|---|---|
| `game/engine.rs:450` | bridge (B) reads `last_recast_context` | **CONFIRMED** |
| `game/engine.rs:445-464` | the empty-stack bridge (B) | **CONFIRMED** |
| `game/engine.rs:456` | `WaitingFor::LoopShortcut` emitted | **CONFIRMED** |
| `game/engine.rs:1451` | `drive_recast_iteration` | **CONFIRMED** |
| `game/engine.rs:1599` | `normalize_recast_frame` | **CONFIRMED** |
| `game/engine.rs:1633` | `derived_fodder_class` | **CONFIRMED** |
| `game/engine.rs:1656` | `try_offer_object_growth_shortcut` | **CONFIRMED** |
| `game/engine.rs:1684` | `spell_ability_bears_randomness` static gate | **CONFIRMED** |
| `game/engine.rs:1713` | runtime RNG word-position backstop | **CONFIRMED** |
| `game/engine.rs:1732` | `loop_states_cover_modulo_fodder_growth` call | **CONFIRMED** |
| `game/engine.rs:2323` | the ring sampler | **CONFIRMED** (`state.record_loop_detect_sample();`) |
| `game/engine.rs:2325` | the empty-stack ring clear | **CONFIRMED** (`state.loop_detect_ring.clear();`) |
| `game/engine.rs:3093` | the deliberate-action ring clear | **CONFIRMED** (`state.loop_detect_ring.clear();`) |
| `game/casting_costs.rs:6795` | the ONLY `last_recast_context` setter | **CONFIRMED** |
| `types/game_state.rs:371` | `RecastContext` | **CONFIRMED** |
| `types/game_state.rs:8620` | the `GameState` field | **CONFIRMED** |
| `analysis/resource.rs:924` | `loop_states_cover_modulo_object_growth` | **CONFIRMED** |
| `analysis/resource.rs:1095` | `loop_states_cover_modulo_fodder_growth` | **CONFIRMED** |
| `analysis/resource.rs:1444` | the F1 ONE-SIDED-SAFETY conjunct | **CONFIRMED** |
| `analysis/resource.rs:1457` | `fire_time_conditions_read_growing_class` | **CONFIRMED** |
| `analysis/resource.rs:1539` | the `!def.modifications.is_empty()` blanket | **CONFIRMED** |
| `analysis/resource.rs:2152` | `fire_time_conditions_read_projected_resource` | **CONFIRMED** |
| `analysis/resource.rs:3926` | `event_and_sibling_axes_unchanged_for_typed` | **CONFIRMED** |
| `game/ability_scan.rs:447` | `Effect::Token { .. } => Axes::CONSERVATIVE` | **CONFIRMED** |
| `game/ability_scan.rs:862` | `Effect::Mana { .. } => Axes::CONSERVATIVE` | **CONFIRMED** |
| `game/ability_scan.rs:1606` | `ObjectCount`'s independent `sibling: true` literal | **CONFIRMED** |
| `game/ability_scan.rs:2418-2422` | the `TargetFilter::Typed` arm | **CONFIRMED** |
| `game/ability_scan.rs:4080` | `modification_grants_growing_cost_keyword` | **CONFIRMED** |
| `game/ability_scan.rs:3632` | `ability_definition_axes` | **CONFIRMED** |
| `game/ability_scan.rs:131` (implied by `Axes::CONSERVATIVE`) | all three axes true | **CONFIRMED** |
| `game/triggers.rs:1057` | `trigger_definition_functions_in_zone` is **private** | **CONFIRMED** |
| `game/functioning_abilities.rs:108` | `object_functions` (the blacklisted trap) | **CONFIRMED** |
| `game/functioning_abilities.rs:187` | `static_functions_in_zone` is `pub(crate)` | **CONFIRMED** |
| `game/functioning_abilities.rs:391` | `active_trigger_definitions` | **CONFIRMED** |
| `types/ability.rs:1557` | `PtValue` = `Fixed` / `Variable` / `Quantity` | **CONFIRMED** (`:1557-1561`, exactly 3 variants) |
| `types/ability.rs:9546-9583` | `Effect::Token`'s fields | **CONFIRMED** (14 fields; the plan names all 14) |
| `game/triggers.rs:3893-3894` | the `c2_order_independent` gate | ⚠️ **DRIFTED** → `:3894-3895` |
| ⛔ `types/ability.rs:1689-1701` | *"`ManaProduction`'s `count: QuantityExpr`"* | ⛔ **WRONG** — `ManaProduction` is an **enum** at `:1676-:1833` with **15 variants**; `:1689-1701` is one variant's body. **(B2)** |
| ⛔ `types/ability.rs:19350-19599` | *"all 41 `ContinuousModification` variants"* | ⛔ **WRONG** — enum spans `:19349-:19711` and has **53** variants. **(B5)** |
| `game/triggers.rs:423-437` (`granted_keyword_triggers_in_zone` exemplar) | | **UNVERIFIED** |
| `types/game_state.rs:4458` (`WaitingFor::LoopShortcut`) | | **UNVERIFIED** |
| `analysis/decision_template.rs:281` (`IterationCount`) | | **UNVERIFIED** |
| `types/actions.rs:834 / :841 / :848` (Declare/Respond/Decline) | | **UNVERIFIED** |
| `analysis/loop_check.rs:83 / :223 / :230` | | **UNVERIFIED** |
| `client/src/adapter/types.ts:2647`, `HostSetup.tsx:544`, `analysis/corpus.rs:2039` (P5 details block) | | **UNVERIFIED** (not load-bearing; P5 is doc-only) |

## CR citations — **ALL CONFIRMED, EXACT** ✅

| CR | Plan says | Verified at `docs/MagicCompRules.txt` |
|---|---|---|
| **CR 732.2a** | `:6372` | ✅ `:6372` — *"the player with priority **may** suggest a shortcut by describing a sequence of game choices…"* |
| **CR 732.2a worked example** | `:6373` | ✅ `:6373` — *"A player controls a creature enchanted by **Presence of Gond**, which grants the creature the ability "{T}: Create a 1/1 green Elf Warrior creature token.""* |
| **CR 104.4b** | `:366` | ✅ `:366` |
| **CR 602.1** | `:2514` | ✅ `:2514` — *"Activated abilities have a cost and an effect."* |
| **CR 602.2a** | `:2529` | ✅ `:2529` |
| **CR 602.5a** | `:2543` | ✅ `:2543` |
| **CR 113.6** | `:771` | ✅ `:771` |
| **CR 400.2** | `:1935` | ✅ `:1935` |
| CR 613.1 · CR 400.7 · CR 704.5a | (no line given) | ✅ resolve at `:2958` · `:1950` · (exists) |

> **The rules layer is clean, exactly as the user directive asserted. The code layer failed in precisely the two places the plan quoted a range it had not opened.**

---

# 6. MEASUREMENT AUDIT (M0–M6)

**Commands run** (all in `/home/lgray/vibe-coding/combo-probe-wt`; `cargo fmt --all` run before each reported result):

```bash
cargo test -p engine --test integration probe_canary_gond -- --nocapture --test-threads=1   # M0/M3/M4/M5
cargo test -p engine --test integration probe_review_r1   -- --nocapture --test-threads=1   # B1 (R1), G3 (R2); emits M1/M2 probe lines
cargo test -p engine --lib review_probe_r3 -- --nocapture                                    # B3 (with P2-b, then revert-probed)
```

| | Claim | Status | Actual output I obtained |
|---|---|---|---|
| **M0** | Canary RED; battlefield grows one Elf/iteration; `OFFER=false`, `MARK=false`, `unbounded={}` | ✅ **REPRODUCED** | `OFFER fired? = false` · `MARK fired? = false ({})` · (my R1 run independently: bf `3 → 4` per iteration) |
| **M1** | Bridge (B)'s gate all-green **except** `last_recast_context`; ring=1, stack=1, `mandatory=true` at the bridge beat | ✅ **REPRODUCED** verbatim | `PROBE-M1 bridge(B) gate: Priority=Y stack_empty=Y samples=Y !probe=Y last_recast_context=false` · `PROBE-M1 ENTER interactive_loop_bridge: ring=1 stack=1 mandatory=true bf=4` |
| **M2** | The ring's prior is a same-beat snapshot: `bf_prior == bf_cur` always; `c2_net_progress=false` always | ✅ **REPRODUCED** verbatim | `PROBE-M2 prior[0] bf_prior=4 bf_cur=4 | c1_equal_mod_res=true c1_cover_growth=false c1_cover_counter=false c1_cover_OBJECT=false | c2_net_progress=false | c3_no_loss=true | c4_winkind=Advantage` |
| **M3** | Firewall `true`; limbs = exactly `{S1Trigger, S2BattlefieldBody, S4StaticModifications}` | ⚠️ **NOT REPRODUCED** | The probe worktree ships the **P2/P3 prototype already applied**, so the live run reports the *post*-fix state (= M5). I did **not** re-derive the pre-fix 3-limb list. |
| **M4** | Fodder class is inert; the firewall is the object-growth cover's **only** failing limb | ✅ **REPRODUCED** verbatim | `fodder class: name="Elf Warrior" tapped=false triggers=0 statics=0 abilities=0 keywords=0` — the brief's *"the Elves are the loop's ENGINE"* worry is indeed **REFUTED** |
| **M5** | After P2+P3: firewall `false`, limbs `[]`, both covers `true` — **and the canary STILL does not offer** | ✅ **REPRODUCED** verbatim | `FIREWALL(short-circuit) = false` · `FIREWALL LIMBS = []` · `loop_states_cover_modulo_OBJECT_gr. = true ⭐` · `loop_states_cover_modulo_FODDER_gr. = true ⭐` · `OFFER fired? = false` |
| **M6** | Full suite `16547 passed; 1 failed` (the `Typed` revert-probe) | ⬜ **NOT ATTEMPTED** | I did not run the full suite. The tripwire **`event_and_sibling_axes_unchanged_for_typed` at `analysis/resource.rs:3926` is CONFIRMED to exist**, and P3 does break it — the plan's re-authoring instruction (P3-b) is correct. |

**M0–M2, M4 and M5 all hold. The plan's measurements are HONEST.** What they never covered is **what P1-b's gate does at the beat it actually runs (B1)** and **what P2's descent does to the trigger gate (B3)**. Both are fatal, and both were reachable with the harness the plan itself built.

---

# 7. THE FOUR RANKED TARGETS — FINDINGS

### (a) More false-certificate holes of the `Effect::Token` class — **YES. THREE. The next one is bigger than the last one.**

1. **`Effect::Mana` / `ManaProduction` — B2.** The plan's model of the type is *wrong*: `ManaProduction` is a **15-variant enum** (`types/ability.rs:1676-1833`), not a struct with a `count`. **Three variants have no `count` at all; two of those read game state** (`DistinctColorsAmongPermanents { filter }` `:1810`; `TriggerEventManaType` `:1832`). P2-c's "descend into `count`" hands them `Axes::NONE`. **And `Effect::Mana` has 5 fields, not 1** — including `target: Option<TargetFilter>` (`:10684`).
2. **`Effect::Token.keywords` — B4.** Binned under `_` as read-free. `Keyword` carries `QuantityExpr` (`Mobilize` `keywords.rs:859`, `Firebending` `:939`), `TargetFilter` (`Enchant` `:645`, `Craft` `:785`), `AbilityCost` (`:809`, `:993`). **The same defect class as Rev 2's `attach_to`/`static_abilities`/`enter_with_counters` miss.**
3. **`ContinuousModification` — B5.** **53** variants, not 41 (`:19349-:19711`). **10 unnamed**, one of which (`AddCounterOnEnter` `:19686`) carries a `QuantityExpr`. And the "descend depth ≤ 1" bucket (`CopyValues`, `AddStaticMode`, `GrantStaticAbility`) reaches `ReplacementDefinition`, `StaticMode` (**119 variants**) and `ParsedCondition` — **all outside the scanner's traversal closure, with no walkers.**

**Bonus (not of that class, but worse): B3** — P2-b/P2-c silently flip a **live, ungated CR 603.3b behavior gate**. `Effect::Token`'s field list itself is **complete** (14 named, 14 exist) ✅ and `PtValue` is **exactly as the plan says** (3 variants) ✅.

### (b) Does the `RecastContext` → `LoopActionContext` rename break the SERIALIZED surface (the B6 defect class)?
**IT IS ON THE SERIALIZED SURFACE — BUT IT IS NOT THE B6 DEFECT. MATERIAL GAP (G2), NOT A BLOCKER.**
The field **is** serialized out (`GameState` is `Serialize`, shipped whole via `ClientGameStateRef` and persisted whole via `TrustedGameStateEnvelope`; the viewer filter does not redact it; the engine's own comment at `casting_costs.rs:6792` acknowledges it appears in *"a save/replay/scenario"*). **But unlike `LoopDetectionMode::On`, it has ZERO consumers**: no `client/` hit, no golden-fixture hit, no `combo-verify` hit, and the client never sends it inbound. With `#[serde(default)]` + no `deny_unknown_fields`, an old save **silently drops** it. **Required:** state the audit + add `#[serde(alias = "last_recast_context")]` (one line) or accept the drop in writing. **The plan currently says nothing at all about the serialized surface — that omission is the finding.**

### (c) Can P1-c's `min_by_key` re-find drive the WRONG permanent with two copies?
**YOUR HYPOTHESIS IS REFUTED FOR REAL CARDS. IT IS REAL FOR TOKENS. (G3)**
`CardId` is minted **per physical card** (`scenario.rs:229`), so two copies carry **different** `CardId`s — measured: my two-bear fixture produced `CardId(1)`/`CardId(2)` and the re-find **picked correctly** (my `assert_ne!` FAILED; I report it as refuted). **BUT every plain token gets `CardId(0)`** (`effects/token.rs:813`, `:1060`). If the loop's driver is **itself a token** — *the exact classes §P1 names: "Marneus Calgar, Ivy Lane Denizen chains"* — the filter matches **every token that player controls, including the fodder the loop is manufacturing**, and `min_by_key` picks the lowest-id token. **Fail closed on `CardId(0)` / `is_token`, or re-find by a surviving identity.**

### (d) Is `activation_loop_without_untapper_does_not_offer` DISCRIMINATING or VACUOUS?
**VACUOUS — and not for a subtle reason.**
Given **B1**, the capture gate never arms, so **nothing ever offers**. The negative therefore passes *for free*, and the paired positive (criterion #1) **cannot pass at all**. This is precisely the `review-engine-plan` check-9 failure ("an early-return makes it pass vacuously").
**Its underlying MECHANISM is sound** — without Intruder Alarm, the bear stays tapped, the 2nd `apply_action(ActivateAbility)` on the clone is illegal ⇒ `Err(RecastAbort)` ⇒ no offer — **but only if the drive is ever reached.** Once B1 is fixed, this test becomes genuinely discriminating (it exercises the drive, not an upstream short-circuit). **As the plan stands, it is worthless, and so is acceptance criterion #2.**

---

# 8. RESIDUAL ASSUMPTIONS I COULD NOT DISCHARGE

1. **B3's second half — NOT ATTEMPTED (runtime).** `batch_conflict == false` for token/mana-bodied triggers is a **code trace, not a run** (`ability_rw.rs:4671-4700` write-only profile → `source_independent()` → distinct-event fast path at `ability_rw.rs:1082-1090` returns `false`). **The `c2_order_independent` flip itself IS measured.**
2. **No named printed card** was verified for the distinct-event token-trigger group shape in B3. The **group shape is CONFIRMED reachable in code** (`triggers.rs:23237`'s existing test); a specific card pair is **UNVERIFIED**.
3. **U1** (scan (3)'s replacement zone-of-function authority; `ReplacementDefinition` has no zone field) — **NOT ATTEMPTED.** The plan already flags it and correctly defers to the implementer.
4. **U5** (P6's descent on real Commander boards) — **NOT ATTEMPTED.**
5. **U6** (perf of the P1-b capture gate) — **NOT ATTEMPTED.** Moot until B1 is fixed. *(Note: `state.battlefield` IS `im::Vector<ObjectId>` — `types/game_state.rs:6818` — so `.len()` is genuinely O(1); the plan's cost claim is right, the semantics are not.)*
6. **U3's fixture** (`SetDynamicPower{Ref(LifeTotal)}` revert-probe) — **NOT ATTEMPTED, deliberately**, per the review mandate (a follow-on round will discharge it). I audited only the **reasoning** and the **gate's strength**: both are **sound** (see G6).
7. **`ContinuousModification`'s 10 unnamed variants:** I confirmed **existence, line numbers, and `AddCounterOnEnter`'s `QuantityExpr` payload**. I did **NOT** independently audit the read surface of the other 9 (they appear structural).
8. **M3's pre-fix 3-limb firewall list** — **NOT REPRODUCED** (the probe worktree ships P2/P3 applied).
9. **M6's full-suite count** — **NOT ATTEMPTED.**

---

# 9. WHAT I CHANGED IN `combo-probe-wt`, AND WHY

## Final state

```
$ git -C /home/lgray/vibe-coding/combo-probe-wt diff --stat
 crates/engine/src/analysis/resource.rs  | 242 +++++++++++++++++++++++++++++++-
 crates/engine/src/game/ability_scan.rs  | 126 ++++++++++++++++-
 crates/engine/src/game/engine.rs        |  42 ++++++
 crates/engine/tests/integration/main.rs |   2 +
 4 files changed, 409 insertions(+), 3 deletions(-)

untracked:
  crates/engine/tests/integration/probe_canary_gond.rs   ← INTACT (291 lines). NOT deleted, NOT modified.
  crates/engine/tests/integration/probe_review_r1.rs     ← NEW (mine)
  .probe-warmbuild.log                                    ← pre-existing, not mine
```

## My drift vs your 356-insertion baseline: **+52 / +1 / one new file**

| Change | Size | What it probes |
|---|---|---|
| **`crates/engine/src/game/ability_scan.rs`** — appended `#[cfg(test)] mod review_probe_r3` | **+52** (74 → 126) | **B3.** Builds a vanilla `Effect::Token` (`create a 1/1 Soldier`, `count: Fixed(1)`, `owner: Controller`, everything else empty) as a `ResolvedAbility`, then evaluates `ability_uses_event_context` / `ability_reads_sibling_mutable` and reconstructs the exact `c2_order_independent` expression from `triggers.rs:3894-3895`. **This is the A/B that proves P2-b flips the live CR 603.3b gate from PROMPT to AUTO-ORDER.** (I ran it once with your P2-b descent in place → `c2 = true`; then **revert-probed** by temporarily restoring `Effect::Token { .. } => Axes::CONSERVATIVE` → `c2 = false`; then **fully restored** your prototype.) |
| **`crates/engine/tests/integration/main.rs`** — one `mod` line | **+1** (1 → 2) | `mod probe_review_r1;` |
| **`crates/engine/tests/integration/probe_review_r1.rs`** — NEW file, 2 tests | untracked | **`r1_battlefield_does_not_grow_at_the_activateability_beat`** (**PASSES** — this is **B1**: measures `state.battlefield.len()` immediately before/after `apply(ActivateAbility)` on the real canary board and shows the growth happens at a *later* `PassPriority` beat, so P1-b's gate is structurally dead). **`r2_min_by_key_refind_picks_the_wrong_permanent`** (**FAILS BY DESIGN** — this is **G3**: its `assert_ne!` encodes the hypothesis that the re-find picks the wrong permanent with two same-card copies; **the failure IS the refutation**, and it also surfaced the `CardId`-minting fact that led me to the *real* token-`CardId(0)` hazard). |

## Restoration verified

- **`Effect::Token` descent is INTACT** — `// PROBE P2-b: descend. Exhaustive destructure, no `..`.` present at `ability_scan.rs:447`. No `REVERT-PROBE` line remains anywhere.
- **P3 is INTACT** — `sibling: false, // PROBE P3` present at `ability_scan.rs:2457`.
- **`probe_canary_gond.rs` is INTACT** — 291 lines, unmodified, **not deleted**.
- `cargo fmt --all` run in the probe worktree before every reported build result.

## Main checkout — **NO `crates/` FILES** ✅

```
$ git -C /home/lgray/vibe-coding/phase-rs-workdir status --short -uno
 M client/src/wasm/engine_wasm.d.ts        ← PRE-EXISTING, not mine
```
**Zero `crates/` edits in main. Zero edits to any plan document.**

---

# 10. REQUIRED BEFORE RE-REVIEW

1. **Re-derive §P1-b against the real `ActivateAbility` handler** (`game/engine.rs:3217-3286`) and **cite it**. B1 is not a tweak — it is the phase.
2. **Rewrite §P2-c** against `ManaProduction`'s real shape (15-variant enum) and `Effect::Mana`'s 5 fields. Fix the `gaeas_cradle_mana_ability_still_vetoes` test design accordingly.
3. **Add the CR 603.3b analysis for `Effect::Token` and `Effect::Mana`** (B3), with a runtime fixture and a `FORGE_TEST_FULL_DB=1 cargo test -p engine ordering_parity_sweep` delta.
4. **Classify `Effect::Token.keywords` and `ContinuousModification::AddKeyword`/`RemoveKeyword` as read-bearing**; **demote `CopyValues` / `AddStaticMode` / `GrantStaticAbility` to `Axes::CONSERVATIVE`** until their walkers exist.
5. **Re-count `ContinuousModification` (53)** and classify all 53. Correct §P2-a's "all 41" and §5's Pattern-Coverage claim.
6. **Run the `add-engine-variant` skill for `LoopAction`** — Steps 3 (walker/profiler classification) and 7 (serialized-surface audit) in particular.
7. **Fail closed in §P1-c's re-find** when `ctx.card_id == CardId(0)` / the source `is_token` (G3), and pin the ability by identity rather than positional index (G4).
