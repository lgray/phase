# Combo detector — the LIVE plan

**2026-07-14 · Revision 3.** Supersedes `COMBO-DETECTOR-PLAN-REVISED.md` (Rev 2), which it **absorbs**.

> ## ⭐ **THIS IS THE FIRST REVISION IN THIS WORKSTREAM THAT RAN CODE.**
>
> Rev 1 and Rev 2 were static traces. **Every number below marked ✅ MEASURED came out of a `cargo test` run against
> `main @ efc76ca1b` in a throwaway worktree (`combo-probe-wt`), driving CR 732.2a's own worked example through the
> real production reducer.** The probe harness is `crates/engine/tests/integration/probe_canary_gond.rs` (probe-only,
> **not** part of this plan's change set).
>
> **The measurements CONFIRMED Rev 2's firewall diagnosis EXACTLY — and REFUTED both hypotheses the planning brief
> asked me to test, plus two of the four layers in the brief's own model.** Read §1 before anything else.

---

## 0. What we are implementing *(preserved from Rev 1/Rev 2 — unchanged, still correct)*

**Shortcutting a loop is OPTIONAL.** CR 732.2a (`docs/MagicCompRules.txt:6372`): the player *"**may** suggest a
shortcut."* Nobody is compelled to propose one, and no opponent is compelled to accept the count.

> ## ⇒ **THE DETECTOR STAYS OPT-IN. Turning it on IS the table agreeing to use the optional shortcut rule.**
> **`LoopDetectionMode::Off` is not dead code — it is the "we are not shortcutting loops in this game" setting, and it
> must remain the default.**

**USER DIRECTIVE (binding, 2026-07-14):** *"Keep off/on/interactive for the combo detector for now. It helps us
separate concerns."* ⇒ **`LoopDetectionMode` keeps all three variants and both UI toggles. P5 is doc-comment-only.
Do not re-open it.**

**PRESERVED SOUNDNESS RULE (Rev 2, verbatim):** *"A coarse relation may **REJECT**, never **ACCEPT**."* A coarse
relation ⇒ a false certificate ⇒ **a real game ends wrongly.** Too fine ⇒ a missed offer ⇒ safe.

**The spec, in five stages** *(preserved from Rev 1/Rev 2 — unchanged)*:

| | Stage | Rule |
|---|---|---|
| **1** | **CAPTURE** the player's performed actions, **as FIXED choices**. | CR 732.2a: *"can't include conditional actions"* |
| **2** | **REPEAT** that exact sequence; determine whether it yields an **unbounded resource**. | CR 732.1b |
| **3** | **CLASSIFY** — ADVANTAGE / WIN / DRAW. | CR 704.5a · CR 104.4b |
| **4** | **PRESENT** it; pass priority around the table. | CR 732.2b/c |
| **5** | If accepted and un-interacted-with: **emit the certificate and APPLY.** | CR 732.2a |

> ### ⭐ The omission that is the whole design — and it is correct
> **The spec says "repeat the ACTIONS." It never says the game state must return to where it started.** CR 732.2a's own
> worked example — **Presence of Gond + Intruder Alarm** (`docs/MagicCompRules.txt:6373`) — **ADDS A TOKEN EVERY
> ITERATION.** Its state provably never recurs, and the rules shortcut it a million times.

---

## 1. ⛔⛔ §1 — WHAT THE MEASUREMENTS SAID. **TWO OF THE FOUR LAYERS DISSOLVED.**

### 1.1 The canary, driven live

**Fixture** (`probe_canary_gond.rs`): Presence of Gond (Aura) enchanting a vanilla 2/2 + Intruder Alarm, **all on the
battlefield**, 2 players, `loop_detection = Interactive`. Oracle text byte-copied from `data/card-data.json`
(`jq -r '.["presence of gond"].oracle_text'`). Driven through the **real** reducer: `GameAction::ActivateAbility` →
`PassPriority` to settle → repeat.

**✅ MEASURED — M0.** The loop RUNS. Battlefield grows by exactly one Elf Warrior per iteration:

```
frame[0] bf=3   frame[1] bf=4   frame[2] bf=5   frame[3] bf=6
OFFER fired? = false      MARK fired? = false      unbounded_resources = {}
```

⇒ **The canary is RED today.** (Prediction confirmed.)

### 1.2 ⛔ **THE RING CAN NEVER SEE THIS LOOP. `bf_prior == bf_cur` AT EVERY SINGLE BRIDGE ENTRY.**

**✅ MEASURED — M1/M2.** The ring's occupancy across ONE iteration:

| beat | action | `ring` | why *(measured)* |
|---|---|---|---|
| B0 | `ActivateAbility` | **0** | `engine.rs:3089-3093` — **deliberate-action clear** (`ActivateAbility` is not `PassPriority`/`OrderTriggers`) |
| B1 | `PassPriority` → ability resolves, Intruder Alarm trigger placed | **1** | `engine.rs:2323` sampler fires (`resolved_this_beat` ∧ stack non-empty ∧ non-shrinking ∧ `Priority{active}`) |
| B2 | `PassPriority` (handoff) | **1** | leave-intact |
| B3 | `PassPriority` → trigger resolves, stack drains | **0** | `engine.rs:2325` — **empty-stack clear** (`!stack.is_empty()` fails ⇒ `else` branch) |

> ## ⇒ **TWO INDEPENDENT RING-KILLERS PER ITERATION. Either alone is fatal.**
> ## ⇒ **The ring's ONLY prior is a SAME-BEAT snapshot of the current state.** Measured at every bridge entry, all 3 iterations:
>
> ```
> PROBE-M1 ENTER interactive_loop_bridge: ring=1 stack=1 mandatory=true bf=4
> PROBE-M2   prior[0] bf_prior=4 bf_cur=4 | c1_equal_mod_res=true c1_cover_growth=false
>            c1_cover_counter=false c1_cover_OBJECT=false | c2_net_progress=false
>            | c3_no_loss=true | c4_winkind=Advantage
> ```
>
> **`bf_prior == bf_cur`. Always. `c2_net_progress = false`. Always.** The delta is structurally **zero**.

**This is not a bug in the ring — it is the ring's DESIGN.** `engine.rs:3075-3077`, verbatim: *"Any deliberate non-pass
action (cast / activate / play-land) **breaks a self-refilling mandatory cascade**, so the accumulated detection window
is stale and must be dropped."* **Correct for its own subject matter.** The ring is an instrument for **mandatory
self-refilling cascades** — loops driven by `PassPriority` alone.

> ## ⛔ **CR 732.2a's SUBJECT MATTER IS THE OPPOSITE:** *"a player… may suggest a shortcut by describing a **sequence of
> game choices**"* — **a player deliberately doing the same thing over and over.** The ring excludes that class **by
> construction.** That is precisely why **bridge (B)** exists.

### 1.3 ⭐⭐ **THE CANARY IS *ONE CONJUNCT* FROM THE ONLY PATH THAT OFFERS.**

There are two live entry points, **duals on stack-emptiness**:

| | **(A) the RING bridge** `engine.rs:322-435` | **(B) the EMPTY-STACK bridge** `engine.rs:445-464` |
|---|---|---|
| gate | `Priority` ∧ `samples()` ∧ **`!stack.is_empty()`** ∧ `!ring.is_empty()` ∧ `!probe` | `Priority` ∧ `samples()` ∧ **`stack.is_empty()`** ∧ `!probe` ∧ **`last_recast_context.is_some()`** |
| emits an OFFER for object growth? | **NO** (Path C → `mark_unbounded_loop` only) | ✅ **YES** — `WaitingFor::LoopShortcut`, `engine.rs:456` |
| reaches the object-growth covers + the firewall? | **NO** | ✅ **YES** — `try_offer_object_growth_shortcut` → `engine.rs:1732` |

**✅ MEASURED — M1.** Bridge (B)'s gate at every one of the canary's settle beats:

```
PROBE-M1 bridge(B) gate: Priority=Y stack_empty=Y samples=Y !probe=Y last_recast_context=false
```

> ## ⇒ **EVERY CONJUNCT OF BRIDGE (B) IS GREEN EXCEPT `last_recast_context`.**
>
> `last_recast_context` has **exactly one setter** — `game/casting_costs.rs:6795`:
> ```rust
> state.last_recast_context = (state.loop_detection.samples()
>     && additional_cost_paid && has_buyback && is_token_creating).then_some(RecastContext { .. });
> ```
> **It arms ONLY on a buyback-paid, token-creating SPELL** (the Sprout Swarm shape). **The canary casts nothing.**

### 1.4 ✅ **M3 — THE FIREWALL: REV 2's §3.3 IS CONFIRMED, EXACTLY, THROUGH THE GAME'S OWN FRAMES**

Two **real, consecutive, live settle frames** captured from the drive, fed to the object-growth cover, which feeds
`fire_time_conditions_read_growing_class` (`analysis/resource.rs:1457`) at `:968`. **Per-limb, with scan (4) split into
its `condition` and `!modifications.is_empty()` branches, exactly as Rev 2's P1 demanded:**

```
FIREWALL(short-circuit) = true
FIREWALL LIMBS = [
    "S1Trigger.execute[Intruder Alarm|Battlefield]",
    "S2BattlefieldBody[Test Bear]",
    "S4StaticModifications[Presence of Gond|Battlefield|n=1]",
]
```

> ## ⇒ **EXACTLY `{S1Trigger, S2BattlefieldBody, S4StaticModifications}` — and NOT `S3Replacement`, NOT
> `S4StaticCondition`, NOT `S5*`, NOT `S5b`, NOT `S6DeferredStores`.**
> ## ⇒ **Rev 2's falsifiable prediction is CONFIRMED. Its three-veto table (§3.3) is MEASURED-TRUE.**

### 1.5 ✅ **M4 — THE FIREWALL IS THE COVER'S *ONLY* FAILING LIMB**

`loop_states_cover_modulo_object_growth` (`resource.rs:924`), sub-limbs on the same live frames:

```
object-growth cover LIMBS: [ "grown_ids n=1", "FAIL: FIREWALL fire_time_conditions_read_growing_class" ]
```

**Every other limb PASSES**: `board_covers` ✓, `object_resource_axes_match` ✓, `loyalty_activation_counts_match` ✓,
`eq_except_growable` ✓, **`grown_objects_are_inert` ✓**, `stack_entry_reads_growing_class` ✓,
`cost_surface_references_growing_class` ✓.

**The brief worried the Elf Warriors "are UNTAPPED and are the loop's ENGINE… not inert fodder." ⛔ REFUTED.** Measured
fodder class:

```
fodder class: name="Elf Warrior" tapped=false triggers=0 statics=0 abilities=0 keywords=0
```

The Elf is a **pure vanilla token**. It satisfies `object_is_inert` (`resource.rs:1380`) outright. **Intruder Alarm's
trigger lives on the ALARM, not on the Elf** — the Elf is passive. And `fodder_content_eq` (`resource.rs:1372`) compares
**modulo tapped**, which is a *relaxation*, not a requirement. ⇒ **the Elf is a valid class for BOTH covers.**

### 1.6 ⭐⭐⭐ **M5 — THE CHAIN CLOSES. REV 2's P2 + P3 ARE EXACTLY RIGHT AND MEASURABLY SUFFICIENT.**

Rev 2's P2 (Token/Mana descend + `scan_continuous_modification`) and P3 (`Typed` `sibling: false`) were implemented in
the probe worktree and the **same live frames** re-measured:

| | before | **after P2 + P3** |
|---|---|---|
| `fire_time_conditions_read_growing_class` | `true` | ✅ **`false`** |
| firewall limbs | 3 | ✅ **`[]` — ZERO** |
| `loop_states_cover_modulo_object_growth` | `false` | ✅ **`true`** |
| `loop_states_cover_modulo_fodder_growth` | `false` | ✅ **`true`** |
| **the canary OFFERS?** | no | ❌ **STILL NO** — bridge (B) still blocked on `last_recast_context` |

> ## ⇒ **P2 + P3 fully discharge the firewall. The covers certify the canary. The SOLE remaining gap is REACH.**

### 1.7 ✅ **M6 — SOUNDNESS REGRESSION: `16547 passed; 1 failed`**

The single failure is **exactly the one Rev 2 named** (`resource.rs:3926`):

```
analysis::resource::tests::event_and_sibling_axes_unchanged_for_typed ... FAILED
  panicked at crates/engine/src/analysis/resource.rs:3945: "the Typed arm keeps sibling:true"
```

**It is a revert-probe. It goes red BY DESIGN and must be RE-AUTHORED (P3-b), never deleted or `#[ignore]`d.**
⇒ **No behavioral rejection test flipped to accept. No new pass was previously a REJECT.**

### 1.8 ⛔ **THE CORRECTED MODEL — the brief's Layers 2 and 4 DO NOT EXIST**

| Brief's layer | Verdict | Evidence |
|---|---|---|
| **L1 REACH** | ✅ **REAL — and it IS `last_recast_context`** | M1: bridge (B)'s sole red conjunct |
| **L2 RECURRENCE** *(Path C's covers can't tolerate object growth)* | ⛔ **DISSOLVED — NOT A DEFECT** | M4: the object-growth cover passes **every limb but the firewall**. M2: Path C's disjunct is irrelevant — its `prior` **is** the current frame, so `cover_OBJECT = false` and `is_net_progress = false` regardless. |
| **L3 FIREWALL** | ✅ **REAL — Rev 2's P2+P3, CONFIRMED and SUFFICIENT** | M3 + M5 |
| **L4 PRESENT** *(Path C marks, doesn't offer)* | ⛔ **MOOT** | Bridge **(B) OFFERS** (`engine.rs:456`). The canary's home is bridge (B). Path C is never reached — **`mandatory = true`** (M1) ⇒ `if !mandatory` is false. |

#### ⛔ H1 — **REFUTED.**
The brief hypothesized: *"the RING bridge already SEES the canary… Layer 1 is out of scope."* **The literal sub-claim is
TRUE** (`PROBE-M1 ENTER interactive_loop_bridge` fires) **but the conclusion is FALSE.** The ring can never hold a
cross-iteration prior (`bf_prior == bf_cur`, measured, every entry), and `mandatory=true` means Path C never runs.
**Layer 1 is REAL, it is in scope, and it is the largest phase in this plan.**

#### ⛔ H2 — **REFUTED, twice over.**
The brief hypothesized: *"Layer 2's fix is adding `|| loop_states_cover_modulo_object_growth(prior, state)` to Path C's
disjunct."* **Measured `c1_cover_OBJECT = false` at every Path C-eligible entry** (the cover requires *strict* growth —
`resource.rs:947` `if grown_ids.is_empty() { return false; }` — and prior==current ⇒ no growth). **The disjunct is a
measured no-op.** And Path C is unreachable anyway (`mandatory=true`). **The code comment at `engine.rs:588-589` —
*"lights up under 4a-live with no further edit"* — is, as the brief suspected, a CLAIM. It is FALSE.**

---

## 2. What already exists — **DO NOT REBUILD ANY OF THIS** *(Rev 2's table, re-measured, corrected)*

| Stage | On `main` | Where *(measured)* |
|---|---|---|
| **1 · capture** | `last_recast_context` | **written** `game/casting_costs.rs:6795`; **read** `game/engine.rs:450` |
| **2 · repeat** | **Real replay on a clone** — 2 iterations / 3 settle frames, re-entrancy-guarded | `game/engine.rs:1688-1696`; driver `drive_recast_iteration` `:1451` |
| **2 · unbounded (LIVE)** | `loop_states_cover_modulo_fodder_growth` — **the only cover on a live reducer path** | `analysis/resource.rs:1095`, called `game/engine.rs:1732` |
| **2 · unbounded (OFFLINE)** | `loop_states_cover_modulo_object_growth` (`:924`) · `_counter_growth` (`:1326`) — called only from `detect_loop` | `analysis/loop_check.rs:223`/`:230` |
| **3 · classify** | Path A `:498` · Path B `:536` · Path C `:577` · `WinKind` | `game/engine.rs` · `analysis/loop_check.rs:83` |
| **4 · present** | `WaitingFor::LoopShortcut` · `IterationCount` | `types/game_state.rs:4458` · `analysis/decision_template.rs:281` |
| **4 · accept/decline/interact** | `DeclareShortcut` `:834` · `RespondToShortcut` `:841` · `DeclineShortcut` `:848` | `types/actions.rs` |
| **5 · apply** | `apply_confirmed_shortcut` → `apply_until_lethal_shortcut` / `materialize_fixed_shortcut` | `game/engine.rs:855`, `:906`, `:1325` |
| **— · determinism** | **static** `spell_ability_bears_randomness` (`engine.rs:1684`) + **runtime** RNG word-position delta (**`engine.rs:1713`**) | |

> ## ⇒ **The pipeline is complete end to end. It is NOT a capability problem. The detector cannot be REACHED — and now
> we have MEASURED which conjunct.**

---

## 3. The architectural spine *(preserved from Rev 2 — still correct, now measured)*

The three vetoes are **blankets that refuse to descend**:

- `Effect::Token { .. } => Axes::CONSERVATIVE` (`ability_scan.rs:447`) — never looks at **what the token is**.
- `Effect::Mana { .. } => Axes::CONSERVATIVE` (`ability_scan.rs:862`) — never looks at its **`count`**.
- `!def.modifications.is_empty() => true` (`resource.rs:1539`) — never looks at **what the modification does**.

A *sound, general* descent means reasoning about arbitrary ability programs. **We do not have to solve that problem.**
Compose: **battlefield-only** (CR 113.6 + CR 400.2) · **no nested loops** · **not Turing-complete** · **and the TWO
CONCRETELY OBSERVED ITERATIONS the clone-drive already produces** — and the question collapses to:

> # **"Does any BATTLEFIELD ability's fire-time condition read THE SPECIFIC AXIS that THIS OBSERVED loop grows?"**

**Decidable. Bounded. Per-axis.** Answering it **IS** `scan_continuous_modification`, which the engine's own comment
(`resource.rs:1452-55`) says nobody ever built.

### 3.1 ⛔ NON-GOALS — state these and hold them
- ❌ No fixpoint / abstract interpretation / e-graphs / symbolic execution. **Ever.**
- ❌ No general program analysis of ability bodies. The walk is a **finite, exhaustive, single-pass AST match**.
- ❌ **No nested-loop support.** Grant-realization depth is **1**; a grant-of-a-grant is **fail-closed ⇒ REJECT**.
- ❌ No Turing-complete combo class. Out of scope by construction, forever.
- ❌ **No new cover.** (M4: none is needed.)
- ❌ **No change to the ring, the sampler, or the deliberate-action clear.** (§1.2: they are correct for their class.)

### 3.2 ⭐ The soundness asymmetry is PRESERVED, not traded away
Narrowing the input class is what **BUYS** the precision; it does not spend safety to get it. Any shape outside the
recognized class — deeper-than-depth-1 grants, non-battlefield function, an unclassifiable axis, a **new enum variant** —
**still fails closed ⇒ REJECT.** The walker's match is **exhaustive with no `_` wildcard**, so a future
`ContinuousModification` variant **fails to compile** until it is classified.

---

## 4. THE PHASES — with DIRECTION OF SOUNDNESS

> ⚠️ **Three phases move the detector toward ACCEPT. Those are the phases that can emit a false certificate and end a
> real game wrongly. Review every line of them twice.**

| Phase | What | Direction | Canary? |
|---|---|---|---|
| **P1** | REACH — capture + drive a repeated **ACTIVATION** | ⚠️ **ACCEPT** | ✅ **required** |
| **P2** | FIREWALL — make the blankets DESCEND | ⚠️ **ACCEPT** | ✅ **required** |
| **P3** | `Typed` NAMES a type; it does not COUNT one | ⚠️ **ACCEPT** | ✅ **required** |
| **P4** | CR 400.2 hidden-zone leak | ✅ **REJECT (safe)** | ❌ clears nothing |
| **P5** | `LoopDetectionMode` doc comment | ⚪ neutral | ❌ |
| **P6** | scan (6) delayed-trigger blanket | ⚠️ **ACCEPT** | ❌ (real boards, not the canary) |

> ## ⇒ **If you only ship three phases, ship P1 + P2 + P3. They are the canary, and nothing else is.**
> **Ship order: P2 → P3 → P1.** P2/P3 are self-contained and independently testable at the cover layer (M5 proves it).
> P1 lands last, on a firewall that already lets the canary through — so P1's own tests measure **only** P1.

---

### ⭐ P1 — **REACH: the ACTIVATED-ABILITY dual of the recast capture** *(NEW — this phase did not exist in Rev 1 or Rev 2)*

**The class.** CR 602.1 (`docs/MagicCompRules.txt:2514`): *"Activated abilities have a cost and an effect."* CR 732.2a's
worked example is an **activated-ability** loop in which **no spell is ever cast**. Today the engine can only capture a
repeated **CastSpell**. **This phase makes it capture a repeated `ActivateAbility` too** — which is the *other* half of
"a player repeats an action."

**Card count.** Every token-creating activated ability that a board can sustain: Presence of Gond + Intruder Alarm,
Marneus Calgar, Ivy Lane Denizen chains, Sprout Swarm's activated cousins, every `{T}: create a token` + untapper.
**Hundreds of Commander/Modern combos.** The recast capture covers the *spell* half; this covers the *ability* half.
**Together they are the CR 732.2a action space.**

#### P1-a — **PARAMETERIZE, DON'T PROLIFERATE.** `RecastContext` → `LoopActionContext`

⛔ **DO NOT add a sibling `last_activation_context` field.** Two reasons, one of them a **soundness hazard**:

1. **CLAUDE.md's parameterization rule.** `RecastContext` and an activation context are two **leaf parameterizations of
   one structural axis**: *the repeated action that drives the loop.* A sibling field is the sibling-cluster smell.
2. ⛔⛔ **THE SOUNDNESS HAZARD.** `analysis/resource.rs:1444` (inside `eq_except_growable`) carries a hand-added
   ONE-SIDED-SAFETY conjunct:
   ```rust
   a == b && … && a.last_recast_context == b.last_recast_context
   ```
   with a 12-line comment (`:1432-1440`) explaining that `impl PartialEq for GameState` **EXCLUDES** this field, and that
   **excluding a heterogeneous recast context from the cover compare is the fail-DANGEROUS direction.** A **new sibling
   field would be excluded from `PartialEq` too and would NOT be added here** — reintroducing exactly the hole that
   comment exists to close. **Parameterizing into the existing field inherits the protection for free.**

**In `types/game_state.rs`** (at `RecastContext`, `:371`):

```rust
/// CR 732.2a: the repeated ACTION that drives a captured loop. Two leaf shapes of one axis —
/// a re-cast spell (CR 601.2a) and a re-activated ability (CR 602.2a). Exhaustive, no wildcard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopAction {
    /// CR 601.2a + CR 702.27a: a self-returning (buyback) recast.
    Recast { from_zone: Zone, uses_buyback: BuybackUsage },
    /// CR 602.2a: a re-activated ability of a battlefield permanent.
    Activate { ability_index: usize },
}

pub struct LoopActionContext {   // was: RecastContext
    pub card_id: CardId,
    pub controller: PlayerId,
    pub action: LoopAction,       // ← the parameterization
    pub convoke: Option<ConvokeMode>,
}
```
Rename the field `last_recast_context` → **`last_loop_action_context`**. ⛔ **Update `resource.rs:1444`'s conjunct to the
new name and KEEP IT** — it is the F1 ONE-SIDED-SAFETY discriminator, and the `impl PartialEq for GameState` still omits
the field.

#### P1-b — the setter

`game/casting_costs.rs:6795` keeps its recast arm **byte-unchanged** (now constructing `LoopAction::Recast`).

**NEW arm** — in the `GameAction::ActivateAbility` handler (`game/engine.rs`; find it via
`find_referencing_symbols` on `GameAction::ActivateAbility`), armed at the **same settle discipline** as the recast:

```rust
// CR 602.2a + CR 732.2a: capture a repeated activation as the loop's driving action.
// Gated to the OBJECT-GROWTH class exactly as the recast arm is (`is_token_creating`):
// the activation must have GROWN THE BATTLEFIELD. Cheap (a length compare), and it is
// precisely the class the object-growth cover can certify — so the clone-drive runs ~never.
state.last_loop_action_context = (state.loop_detection.samples()
    && state.battlefield.len() > battlefield_len_before
    && source_obj.zone == Zone::Battlefield)              // CR 602.5a: activated abilities function on the battlefield
    .then_some(LoopActionContext { card_id, controller, action: LoopAction::Activate { ability_index }, convoke: None });
```

> ⛔ **DO NOT try to statically prove the ability is re-activatable.** The canary's `{T}` cost is only repeatable because
> Intruder Alarm untaps. **The clone-drive IS the oracle** — `drive_*_iteration` re-applies the real `ActivateAbility`
> through `apply_action`; if the second activation is illegal it returns `Err(RecastAbort)` and **no offer is made.**
> Fail-closed, and it costs nothing.

#### P1-c — the drive

**`drive_recast_iteration` (`game/engine.rs:1451`) is ~90% generic already.** Measured: everything from `:1481` (the
`beat_cap` loop) down — the `ManaPayment`/convoke arm, the `Priority`+empty-stack **settle boundary** (`:1537-1541`,
`if clone.stack.is_empty() { return Ok(()); }`), and the fail-closed `_ => return Err(RecastAbort)` — is **action-agnostic.**

**Only two things are spell-specific:**
- the opening `apply_action(CastSpell { .. })` (`:1469-1480`),
- the `WaitingFor::OptionalCostChoice` (buyback) arm (`:1485-1497`).

⇒ **Rename to `drive_loop_action_iteration` and dispatch the OPENER on `ctx.action`:**

```rust
match &ctx.action {
    LoopAction::Recast { from_zone, uses_buyback } => { /* :1460-1480 VERBATIM */ }
    // CR 602.2a: re-find the source LIVE by card identity (it is a stable battlefield
    // permanent, but re-finding matches the recast arm's CR 400.7 discipline).
    LoopAction::Activate { ability_index } => {
        let source_id = clone.objects.values()
            .filter(|o| o.card_id == ctx.card_id && o.zone == Zone::Battlefield
                     && o.controller == ctx.controller)
            .map(|o| o.id).min_by_key(|id| id.0).ok_or(RecastAbort)?;
        apply_action(clone, ctx.controller,
            GameAction::ActivateAbility { source_id, ability_index: *ability_index }, None)
            .map_err(|_| RecastAbort)?;
    }
}
```
The `OptionalCostChoice` arm becomes `LoopAction::Recast`-only; under `Activate` it is **fail-closed abort** (an
activation that opens an optional-cost window is not a pinned shortcut — CR 732.2a *"can't include conditional actions"*).

#### P1-d — the hook

`try_offer_object_growth_shortcut` (`game/engine.rs:1656`) — **every downstream line is reused unchanged**: the RNG
word-position backstop (`:1713`), `derived_fodder_class` (`:1633`), `normalize_recast_frame` (`:1599`), the fodder cover
(`:1732`), the sign checks, the certificate. **Two edits only:**

1. `let ctx = state.last_loop_action_context.clone()?;`
2. The **static randomness gate** (`:1684`, `spell_ability_bears_randomness` on `combined_spell_ability_def`) is
   spell-only. **Under `LoopAction::Activate`, scan the ACTIVATED ABILITY's definition instead** — the same
   `ability_scan` authority, applied to `source_obj.abilities[ability_index]`:
   ```rust
   // CR 705.1 / CR 706.1a / CR 701.9a: reject a randomness-bearing repeated action STATICALLY,
   // before driving. Same authority, different subject. (The runtime RNG word-position
   // backstop at :1713 remains the complete gate and is action-agnostic.)
   ```
   ⛔ **Do NOT skip this and lean on the runtime backstop alone.** Fail-closed on an undeterminable ability.
3. `normalize_recast_frame` strips the self-returning recast card. Under `Activate` there is no such card — the source is
   a **stable battlefield permanent that compares by id**. ⇒ the strip is a **no-op**; keep the
   `last_created_token_ids` clear (it churns per cycle for both shapes). **Guard this with an assertion, not an
   assumption** (see the P1 tests).

#### P1 — tests *(named, in `crates/engine/tests/integration/loop_shortcut_activation.rs`; add `mod` to `tests/integration/main.rs`)*

| Test | Asserts | **Revert-probe (must FLIP to FAIL)** |
|---|---|---|
| `activation_loop_gond_intruder_alarm_offers_shortcut` | the canary reaches `WaitingFor::LoopShortcut` with `unbounded` naming `TokensCreated` | delete the `LoopAction::Activate` setter arm ⇒ no offer |
| `activation_loop_declare_shortcut_materializes_n_tokens` | `DeclareShortcut { count: Fixed(50) }` ⇒ battlefield grows by exactly 50 Elves | — |
| `activation_loop_without_untapper_does_not_offer` | **Presence of Gond alone (no Intruder Alarm)** ⇒ the 2nd drive iteration is illegal ⇒ **NO offer** | ⭐ **the DISCRIMINATOR.** This is what proves the drive is the oracle and the offer is not arming on "any token-making activation." |
| `activation_loop_randomness_bearing_ability_does_not_offer` | a `{T}: flip a coin, if heads create a token` ability + untapper ⇒ **NO offer** (CR 732.2a *no conditional actions*) | delete the P1-d static gate ⇒ this must still fail via the `:1713` RNG backstop; if it does NOT, the backstop is broken |
| `activation_loop_heterogeneous_context_does_not_cover` | two DIFFERENT `ability_index` values across cycles ⇒ `eq_except_growable`'s context conjunct rejects | ⭐ delete the `resource.rs:1444` conjunct ⇒ **must FLIP to a false certificate.** This is the P1-a soundness proof. |

---

### ⭐ P2 — **Make the blankets DESCEND** *(Rev 2's P2 — CONFIRMED by M3/M5; CORRECTED in P2-b)*

#### P2-a — `scan_continuous_modification` — **the walker the code says does not exist**

**New**, in `game/ability_scan.rs`, beside the shipped precedent `modification_grants_growing_cost_keyword` (`:4080`):

```rust
/// CR 613.1: a continuous modification's read surface. EXHAUSTIVE, NO `_` WILDCARD —
/// a future `ContinuousModification` variant FAILS TO COMPILE until it is classified.
fn scan_continuous_modification(m: &ContinuousModification, depth: u8) -> Axes { … }

pub(crate) fn continuous_modification_reads_sibling_mutable(m: &ContinuousModification) -> bool;
pub(crate) fn continuous_modification_reads_projected_resource(m: &ContinuousModification) -> bool;
```

**Classification of all 41 measured variants** (`types/ability.rs:19350`–`:19599`):

| Class | Variants | Verdict |
|---|---|---|
| **Read-free structural** | `SetName` `AddKeyword` `RemoveKeyword` `RemoveAllAbilities` `AddType` `RemoveType` `AddSubtype` `RemoveSubtype` `SetCardTypes` `RemoveAllSubtypes` `AddAllCreatureTypes` `AddAllBasicLandTypes` `AddAllLandTypes` `AddChosenSubtype` `AddChosenColor` `AddChosenKeyword` `RemoveChosenKeyword` `SetColor` `AddColor` `SwitchPowerToughness` `AssignDamageFromToughness` `AssignDamageAsThoughUnblocked` | `Axes::NONE` |
| ⭐ **Fixed P/T (the ANTHEM class)** | `AddPower{value:i32}` `AddToughness{value:i32}` `SetPower{value:i32}` `SetToughness{value:i32}` (`:19385`–`:19394`) | **`Axes::NONE`** — *an anthem READS NOTHING.* It **applies to** each member of a growing class; it does not **read** a mutable aggregate. **This corrects the firewall doc's own wrong justification** (`resource.rs:1452-55`). |
| **Quantity-bearing** | `SetDynamicPower` `SetDynamicToughness` `SetPowerDynamic` `SetToughnessDynamic` `AddDynamicPower` `AddDynamicToughness` `AddDynamicKeyword` — all `{ value: QuantityExpr }` (`:19483`–`:19516`) | **`scan_quantity_expr(value)`** ⇒ an `ObjectCount`-keyed dynamic P/T correctly **VETOES** via `ability_scan.rs:1606` |
| **Ability-bearing (descend, depth ≤ 1)** | `GrantAbility{definition}` (`:19403`) `GrantStaticAbility` `GrantTrigger` `AddStaticMode` `CopyValues` | recurse via the **existing** `ability_definition_axes` (`:3632`) / `scan_static_condition` (`:2926`); **`depth > 0` ⇒ `Axes::CONSERVATIVE`** (§3.1: no nested grants) |
| **Fail-closed** | `GrantAllActivatedAbilitiesOf` `GrantAllTriggeredAbilitiesOf` `RetainPrintedTriggerFromSource` `RetainPrintedAbilityFromSource` `AddKeywordWithDerivedCost{derivation}` | **`Axes::CONSERVATIVE`** — the source set is not statically known |

#### P2-b — `Effect::Token { .. }` DESCENDS *(clears **V2**)* — ⛔ **REV 2 UNDER-COUNTED THE FIELDS**

`game/ability_scan.rs:447`: `Effect::Token { .. } => Axes::CONSERVATIVE` → **exhaustive destructure, NO `..`**, per the
`resolved_ability_axes` precedent (`:148-155`).

> ## ⛔⛔ **REV 2's P2-b NAMES ONLY `count`, `power`, `toughness`, `owner`. MEASURED (`types/ability.rs:9546-9581`),
> `Effect::Token` HAS *THREE MORE* READ-BEARING FIELDS. Missing one is the FALSE-CERTIFICATE direction.**

**The complete field set, measured:**

| field | type | verdict |
|---|---|---|
| `power` / `toughness` | `PtValue` (`:1557` — `Fixed` / `Variable` / **`Quantity(QuantityExpr)`**) | **descend** via a new `scan_pt_value` (`Fixed`/`Variable` ⇒ `NONE`; `Quantity(q)` ⇒ `scan_quantity_expr(q)`) |
| `count` | `QuantityExpr` | **descend** `scan_quantity_expr` |
| `owner` | `TargetFilter` | **descend** `scan_target_filter` |
| ⭐ `attach_to` | `Option<TargetFilter>` (`:9568`) | ⛔ **MISSED BY REV 2 — descend** `scan_target_filter` |
| ⭐ `static_abilities` | `Vec<StaticDefinition>` (`:9576`) | ⛔ **MISSED BY REV 2 — descend** the condition **and** the modifications, **via P2-a's walker** |
| ⭐ `enter_with_counters` | `Vec<(CounterType, QuantityExpr)>` (`:9581`) | ⛔ **MISSED BY REV 2 — descend** `scan_quantity_expr` |
| `name` `types` `colors` `keywords` `tapped` `enters_attacking` `supertypes` | — | `_` **with a one-line justification each** |

⇒ Presence of Gond's token (`count: Fixed(1)`, `power`/`toughness: Fixed(1)`, `owner: Controller`, the rest empty) ⇒
✅ **`Axes::NONE`** *(measured in the probe)*.
⇒ *"Create X tokens, where X is the number of creatures you control"* (`count: Ref(ObjectCount{..})`) ⇒ **still VETOES**
via `:1606`. **This is the class boundary, and it is drawn in the right place.**

#### P2-c — `Effect::Mana { .. }` DESCENDS *(clears NOTHING on the canary — and is MANDATORY anyway)*

`game/ability_scan.rs:862` → descend into `ManaProduction`'s `count: QuantityExpr` (`types/ability.rs:1689-1701`).

> ## ⭐ **P2-c IS WHAT MAKES THE GAEA'S CRADLE NEGATIVE MEAN ANYTHING.** Gaea's Cradle (`{T}: Add {G} for each creature
> you control`) **is present** in `data/card-data.json` (`jq 'has("gaea'\''s cradle")'` → `true`; the root object is
> **lowercase**-keyed — the review report's exact-case probe was wrong). Without P2-c, the "Cradle still REJECTs"
> acceptance criterion is **vacuous** — it would reject via the blanket, not via the read. **Rev 1 shipped exactly that
> vacuous criterion.**

#### P2-d — ⛔⛔ **THE SAFETY-CRITICAL WIRING. DO NOT SKIP. DO NOT SIMPLIFY.**

Replace `analysis/resource.rs:1539`:

```rust
if !def.modifications.is_empty() { return true; }                      // ⛔ BEFORE (blanket)
```
```rust
// CR 613.1: descend — a modification vetoes iff it READS a mutable aggregate
// (sibling) or a projected player resource. A fixed anthem reads NEITHER.
if def.modifications.iter().any(|m| {
    scan::continuous_modification_reads_sibling_mutable(m)
        || scan::continuous_modification_reads_projected_resource(m)   // ⛔⛔ BOTH AXES
}) { return true; }                                                     // ✅ AFTER
```

> ## ⛔ **THE VETO MUST BE `sibling || projected`, NOT `sibling` ALONE. AN IMPLEMENTER WILL GET THIS WRONG.**
>
> **Measured:** the projected-axis twin `fire_time_conditions_read_projected_resource` (`analysis/resource.rs:2152`)
> scans a static's **`condition` ONLY** (`:2199-2207`) — it has **NO `modifications` scan at all.** And the blanket at
> `:1539` is the **only** `modifications.is_empty()` check in the file. **The sibling blanket is therefore
> *incidentally* providing the ONLY protection the object/fodder covers (`:968`, `:1131`) have against a
> projected-reading modification** — e.g. `SetDynamicPower { value: Ref(LifeTotal) }`.
>
> **Veto on `sibling` alone and you REMOVE that protection ⇒ a real FALSE-CERTIFICATE hole ⇒ a real game ends wrongly.**
> Vetoing on `sibling || projected` preserves **exactly** the protection the blanket gave and removes **only** the
> vetoes where the modification reads **neither** axis — which is precisely the anthem / fixed-token-grant class the
> canary needs. **One-sided safety, held.**
>
> *(Whether the projected twin at `:2152` should grow its own `modifications` scan is a **separate, pre-existing** latent
> gap on the `:784` ω-cover. **OUT OF SCOPE — filed as DEFERRED-2, §7.**)*

#### P2 — tests *(in `crates/engine/src/game/ability_scan.rs` `#[cfg(test)]` + `analysis/resource.rs` `#[cfg(test)]`)*

| Test | Asserts | **Revert-probe** |
|---|---|---|
| `fixed_anthem_modification_reads_nothing` | `AddPower{2}` ⇒ `Axes::NONE` on both axes | — |
| `dynamic_pt_modification_reads_sibling` | `SetDynamicPower{ Ref(ObjectCount{..}) }` ⇒ `sibling == true` | flip the `Quantity-bearing` arm to `NONE` ⇒ FAIL |
| `token_effect_with_fixed_count_reads_nothing` | Presence of Gond's exact parsed `Effect::Token` ⇒ `Axes::NONE` | — |
| `token_effect_with_objectcount_count_reads_sibling` | `count: Ref(ObjectCount)` ⇒ `sibling == true` | — |
| ⭐ `token_effect_with_dynamic_enter_counters_reads_sibling` | `enter_with_counters: [(P1P1, Ref(ObjectCount))]` ⇒ `sibling == true` | ⛔ **the P2-b correction's discriminator.** Bind `enter_with_counters` to `_` ⇒ **must FLIP to a false NONE.** |
| ⭐ `token_effect_with_dynamic_static_ability_reads_sibling` | `static_abilities: [StaticDefinition{ modifications: [SetDynamicPower{Ref(ObjectCount)}] }]` ⇒ `sibling == true` | ⛔ same — bind `static_abilities` to `_` ⇒ **must FLIP.** |
| ⭐⭐ `projected_reading_modification_still_vetoes_the_firewall` | a battlefield static with `modifications: [SetDynamicPower{ Ref(LifeTotal) }]` (no condition) ⇒ `fire_time_conditions_read_growing_class == true` | ⛔⛔ **THE P2-d DISCRIMINATOR. Drop the `|| …reads_projected_resource(m)` term ⇒ this test MUST FLIP TO FAIL.** If it does not flip, the term is dead and the plan's most dangerous line is unproven. **DO NOT MERGE P2 WITHOUT THIS TEST GOING RED ON THE REVERT.** |
| `gaeas_cradle_mana_ability_still_vetoes` | real Gaea's Cradle from `card-data.json` on the battlefield ⇒ firewall `true` **via the Mana `count` read**, not via a blanket | ⛔ revert P2-c ⇒ must still pass (blanket) — so **additionally assert `Axes.sibling == true` on the parsed `Effect::Mana` itself**, which the blanket cannot produce. **A pass-only assertion here is VACUOUS.** |

---

### P3 — **`Typed` NAMES a type; it does not COUNT one** *(ONE LINE — clears **V1**)*

`game/ability_scan.rs:2418-2422`, measured verbatim:

```rust
TargetFilter::Typed(tf) => Axes {
    event: true,                                    // :2419  ⛔ UNCHANGED — see P3-b
    sibling: true,                                  // :2420  ⇒  sibling: false
    projected: typed_filter_reads_projected(tf),    // :2421  unchanged
},
```

**A `Typed` filter is a PREDICATE — `"creature"`, `"creature you control"`. It SELECTS a set. It does not read the set's
CARDINALITY.** Counting is `QuantityRef::ObjectCount`, and that is a **different node**.

#### ⭐ Why this cannot open the catastrophic hole — **STRUCTURALLY IMPOSSIBLE, and MEASURED**

`game/ability_scan.rs:1603-1611`, verbatim:

```rust
QuantityRef::ObjectCount { filter } => {
    let mut acc = Axes { event: false, sibling: true, projected: false };  // :1606 ← INDEPENDENT LITERAL
    acc = acc.or(scan_target_filter(filter));                              // :1609 ← .or() only ADDS
    acc
}
```

> ## ⇒ **`ObjectCount`'s `sibling: true` at `:1606` is its OWN literal. It does NOT come from `scan_target_filter`.**
> ## ⇒ **Flipping `:2420` PROVABLY CANNOT un-reject the counting class.** Identically for `ObjectCountDistinct` (`:1618`)
> and `ObjectCountBySharedQuality` (`:1631`).
>
> **The COUNT-vs-NAME distinction is ALREADY ENCODED IN THE CODE** — at `:1606`/`:1618`/`:1631` (count) versus `:2420`
> (name). **P3 does not invent it. P3 stops `:2420` from lying about it.**

#### P3-b — ⛔ the over-edit guard and the live second consumer

1. **`event` at `:2419` STAYS `true`. We touch `sibling` ONLY.** The **second** consumer of the `sibling` axis is
   `game/triggers.rs:3893-3894` (CR 603.3b trigger auto-resolve):
   ```rust
   let c2_order_independent = !ability_scan::ability_uses_event_context(&reference)
       && !ability_scan::ability_reads_sibling_mutable(&reference);
   ```
   The gate is **`!event && !sibling`**. With `event` still `true`, `ability_uses_event_context` still returns `true` for
   any `Typed`-bearing ability ⇒ `c2_order_independent` stays `false` ⇒ **CR 603.3b behavior is BYTE-UNCHANGED.**
   ✅ **MEASURED (M6): the full suite is green on this axis — 16547 passing, zero trigger regressions.**
2. ✅ **MEASURED (M6): `analysis::resource::tests::event_and_sibling_axes_unchanged_for_typed` (`resource.rs:3926`) GOES
   RED** — `panicked at :3945: "the Typed arm keeps sibling:true"`. **It is a revert-probe and it must be RE-AUTHORED,
   not deleted, not `#[ignore]`d.** Its own doc (`:3922-3925`) states the probe for this exact flip.
   **Re-author** → rename to `event_axis_unchanged_for_typed_sibling_axis_relaxed`; keep the `event: true` assertion
   **verbatim** (it is the `triggers.rs:3893` contract and is now the *load-bearing half*); **invert** the `sibling`
   assertion with a comment pointing at `:1606` as the reason it is safe.

---

### P4 — Scope the covers to **VISIBLE, FUNCTIONING** objects *(the CR 400.2 / CR 113.6 rules fix)*

> ## **HONEST LABEL: this clears ZERO of V1/V2/V3.** ✅ **MEASURED (M3): every veto is BATTLEFIELD-resident.** It ships
> because it is a **real information leak**, not because the canary needs it. **Direction: REJECT (safe).**

Three of the seven scans iterate `state.objects.values()` over **every zone, including library and hand**:

- **scan (1)** `resource.rs:1460` → `functioning_abilities::active_trigger_definitions` (`:391`), whose filter is: *if
  Command-zone non-emblem, defer; otherwise* **bare `true`** (`:405-409`). **No zone filter. A trigger def on a card in
  the LIBRARY is returned as "active."**
- **scan (3)** `resource.rs:1501` → `active_replacements` (`:446`), filtered **only** by `object_functions` (`:108-116`).
- **scan (4)** `resource.rs:1527` — `state.objects.values()`, all zones.

**Two independent rules, either one fatal:**
1. **CR 113.6** (`docs/MagicCompRules.txt:771`) — a non-instant/sorcery object's abilities *"usually function only while
   that object is on the battlefield."* **Scanning a library card's ability is not conservatism — it is scanning an
   ability that does not exist.**
2. ⭐ **CR 400.2** (`:1935`) — ***"Library and hand are hidden zones."*** The offer's **presence or absence is itself
   observable to every player.** If it depends on hidden-zone contents, **the engine leaks hidden information into
   observable game state. That is not a conservative approximation. It is a rules violation.**

#### ⛔ CALL THE ENGINE'S RUNTIME FUNCTIONING AUTHORITY. **DO NOT HAND-ROLL A ZONE LIST.**

| Authority | Where | Note |
|---|---|---|
| `trigger_definition_functions_in_zone` | **`game/triggers.rs:1057`** | ⚠️ **PRIVATE — widen to `pub(crate)`** |
| `static_functions_in_zone` | **`game/functioning_abilities.rs:187`** | already `pub(crate)` ✅ |

**⭐ THE EXEMPLAR — `granted_keyword_triggers_in_zone` (`game/triggers.rs:423-437`) already does exactly this, at `:434`:**
```rust
synthesize_granted_keyword_triggers(obj, keywords.iter())
    .into_iter()
    .filter(|(_, def)| trigger_definition_functions_in_zone(def, obj.zone))   // :434  ⭐
```
**This is precisely why scan (5b) is NOT a leak while (1)/(3)/(4) are.** *(Measured: (5) reads
`state.transient_continuous_effects`, not `state.objects`, so it has no zone surface; (6) is P6.)*

#### ⛔⛔ **THE TRAP — BLACKLISTED BY NAME**
> ## **`functioning_abilities::object_functions` (`game/functioning_abilities.rs:108`) IS NOT A ZONE-OF-FUNCTION AUTHORITY.**
> **Measured `:108-116`:** it checks **phased-out** and **Command-zone-non-emblem**, then `return true`.
> ## ⇒ **IT RETURNS `true` FOR A CARD IN THE LIBRARY.**
> **An implementer WILL reach for it, because scan (3) already calls it.** Calling it reintroduces the exact bug this
> phase exists to fix.

#### The changes
- **scan (1)** `resource.rs:1461` — filter by `triggers::trigger_definition_functions_in_zone(def, obj.zone)`.
- **scan (3)** `resource.rs:1501` — filter by the replacement's zone of function. ⚠️ **`ReplacementDefinition` has no
  measured zone field** ⇒ **UNVERIFIED (§8-U1): the implementer must first determine the correct authority.** If none
  exists, **restrict to `Zone::Battlefield | Zone::Graveyard`** *(CR 113.6b — graveyards are PUBLIC and some replacements
  function there)* and **file the gap.** ⛔ Do NOT hand-roll a broader list.
- **scan (4)** `resource.rs:1527` — filter `obj.static_definitions.iter_all()` by
  `functioning_abilities::static_functions_in_zone(obj, def)`.

⚠️ **`active_trigger_definitions` is a SHARED authority.** The live trigger pipeline has **its own** zone gate at
`game/triggers.rs:1040` (`source_has_trigger_in_zone`). ⇒ **Fix the FIREWALL's scope. Do NOT change
`active_trigger_definitions`' contract.**

#### P4 — test *(the class property, as an assertion)*
`hidden_zone_content_does_not_change_the_offer` — take the canary board, **shuffle an arbitrary loud card (e.g. a
`SetDynamicPower{Ref(ObjectCount)}` anthem) into P0's LIBRARY**, and assert the offer is **byte-identical**.
⛔ **Revert-probe: without P4 this test FAILS** (the library card vetoes). **A test that passes both ways is vacuous.**

---

### P5 — `LoopDetectionMode`: ⛔ **KEEP ALL THREE MODES. TOUCH NOTHING.** *(USER DIRECTIVE)*

> *"Keep off/on/interactive for the combo detector for now. It helps us separate concerns."*

**⇒ OUT OF SCOPE.** Keep the enum, all three variants, both player-facing toggles, the `?loop=` URL param.
**The three modes are load-bearing FOR THIS WORK:** `Off` (default; the CR 732.2a opt-in) · **`On`** (auto-resolve — lets
us exercise *classification* in isolation from the *offer* machinery) · `Interactive` (the full offer path).

**The only change P5 makes is a DOC COMMENT** — `types/game_state.rs`, at the `On` variant:

```rust
/// ANALYSIS-shaped (CR 732.2a): auto-resolves a lethal drain WITHOUT an offer and without an
/// opponent response window (`game/engine.rs:420-427`). Correct for its offline consumer — the
/// `combo-verify` corpus classifier (`analysis/corpus.rs:2039`) — which asks "does this deck
/// contain a lethal loop?" rather than playing a game. See DEFERRED-1.
```

**⇒ DEFERRED-1 (filed, NOT fixed here):** whether `On` should remain *player-selectable in a real game* is a live rules
question — the drain path auto-wins with no offer, which CR 732.2a does not sanction as a *game* mode. **Deferred
deliberately. It is a FRONTEND question first.**

<details><summary><b>Measured evidence retained</b> — why deleting <code>On</code> was never a refactor</summary>

Deleting `On` breaks: the shipped `combo-verify` binary (`crates/engine/Cargo.toml:61-64` ← `analysis/corpus.rs:2039`);
two live UI toggles (`HostSetup.tsx:544`, `GameSetupPage.tsx`, + `client/src/game/loopDetectionMode.ts`'s `?loop=` param);
the wire form (`client/src/adapter/types.ts:2647` ⇒ WS protocol + WASM bridge + saved games + localStorage); and the
`GOLDEN_ON` byte-identity golden in `tests/integration/loop_shortcut.rs`.

**And Rev 1's premise was measurably wrong:** the object-growth bridge gates on **`.samples()`**
(`types/game_state.rs:5975`: `matches!(self, On | Interactive)`), **not** on the mode match ⇒ **`On` ALREADY OFFERS on the
object-growth path.** The rules defect is confined to the **drain** path (`engine.rs:356-430`).
</details>

**P5 touches no Rust, no TypeScript, and no test.**

---

### P6 — scan (6): the delayed-trigger blanket *(**IN SCOPE**, small — ⚠️ ACCEPT direction)*

`analysis/resource.rs:1582-1589`, measured:
```rust
if !state.delayed_triggers.is_empty() || !state.deferred_triggers.is_empty()
    || state.pending_trigger.is_some() || state.pending_trigger_order.is_some()
    || !state.epic_effects.is_empty() { return true; }
```

**Any real Commander board with ONE live delayed trigger** — *"at the beginning of the next end step, …"* — **dies here.**
✅ **MEASURED (M3): it does NOT veto the canary** (`S6DeferredStores` absent from the limb list — a clean board at the
priority beat). **That is why it is P6, not P2.** Once P2's walker exists this is a cheap descent: scan each store's
**ability body** with the existing `ability_definition_reads_sibling_mutable`, rather than blanket-rejecting on
**non-emptiness**. **Veto on `sibling || projected`, exactly as P2-d.**

> ## ⛔⛔ **DISAMBIGUATION — AN IMPLEMENTER WILL CONFLATE THESE. THEY ARE DIFFERENT MECHANISMS.**
>
> | | **scan (6)** — `resource.rs:1582` | **`GameState::PartialEq`'s `delayed_triggers` conjunct** |
> |---|---|---|
> | What it does | **VETOES** the detection if the store is **NON-EMPTY** | **COMPARES** the store between two frames to decide whether the board **RECURRED** |
> | Direction | a **firewall** (rejects) | a **cover** (equality) |
> | This plan | **P6 RELAXES it** (descend, don't blanket) | ## ⛔ **DO NOT TOUCH. DO NOT RELAX.** It is what stops us certifying a loop **whose growth axis dies at the next end step.** |

---

## 5. Mandatory architectural sections *(`/engine-planner` Step 4)*

**Pattern Coverage.** **This is a WALKER and a CAPTURE GENERALIZATION, not a card fix.**
- **P1** covers **every repeated activated ability that grows the board** — the *other half* of CR 732.2a's action space
  (the recast capture already covers the spell half). Hundreds of Commander/Modern token engines.
- **P2-a** classifies **all 41** `ContinuousModification` variants — every static/aura/anthem/equipment grant in the game.
- **P2-b** covers **every token-creating ability**; **P2-c** every **mana** ability.
- **P3**'s one line covers **every `Typed` target filter in the engine** — the single most common filter node there is.
- **Card count: the entire enchantment / aura / anthem / token / mana surface — thousands.**
- **The canary is an ACCEPTANCE TEST, not a GOAL.** Every phase discharges a **class** property (§4's test tables).

**Building Blocks.** Compose from what exists — **no new analysis machinery, and (per M4) NO NEW COVER**:
`ability_scan::scan_quantity_expr` (`:2112`) · `scan_quantity_ref` (`:1603`) · `scan_target_filter` (`:2418`) ·
`scan_static_condition` (`:2926`) · `ability_definition_axes` (`:3632`) · `ability_definition_reads_sibling_mutable`
(`:3737`) · `Axes` + `Axes::or` (`:137-143`) · `triggers::trigger_definition_functions_in_zone` (`:1057`) ·
`functioning_abilities::static_functions_in_zone` (`:187`) · `drive_recast_iteration` (`engine.rs:1451` — **90%
action-agnostic already**) · `derived_fodder_class` (`:1633`) · `normalize_recast_frame` (`:1599`) ·
`loop_states_cover_modulo_fodder_growth` (`resource.rs:1095`) · the RNG word-position backstop (`engine.rs:1713`).
**Two new helpers, both justified:** `scan_continuous_modification` (the walker `resource.rs:1452-55` says is missing;
shipped sibling to mirror: `modification_grants_growing_cost_keyword`, `ability_scan.rs:4080`) and `scan_pt_value`
(P2-b's `PtValue::Quantity` arm has no existing scanner).

**Logic Placement.** **All AST classification lives in `game/ability_scan.rs`** — the file that owns the `Axes` walk; it
is the only module that may know a variant's read surface. **`analysis/resource.rs` only CONSUMES `bool`s** — it must
never learn an enum's shape (today's `:1539` blanket **is** exactly that leak, and P2-d removes it). **Zone-of-function
lives in `game/triggers.rs` / `game/functioning_abilities.rs`** and is **called**, never mirrored (P4). **The loop-action
capture is a `types/game_state.rs` TYPE + a `game/engine.rs` reducer arm** — the transport layers see nothing. **Frontend:
zero changes** (P5 is a doc comment).

**Rust Idioms.**
- **Exhaustive `match`, NO `_` wildcard** in `scan_continuous_modification` — a future variant **must fail to compile**
  until classified. *(Contrast the shipped `modification_grants_growing_cost_keyword`, `ability_scan.rs:4080-4088`, which
  **does** use `_ => false` — acceptable for **its** question, **FORBIDDEN** for ours: ours is the ACCEPT direction.)*
- **Exhaustive destructure, no `..`**, in the `Effect::Token` arm — this is precisely how Rev 2 missed three fields.
  Precedent: `resolved_ability_axes` (`ability_scan.rs:148-155`).
- **`LoopAction` is a typed enum, not a `bool`/`Option` pair** — per CLAUDE.md's parameterize-don't-proliferate rule, and
  because it inherits `resource.rs:1444`'s ONE-SIDED-SAFETY conjunct for free (P1-a).
- **Reuse `RecastAbort`** — do not introduce a second abort type.

**CR Annotations.** Every number below was **grep-verified against `docs/MagicCompRules.txt`** before being written here:
CR 732.2a (`:6372`) · CR 104.4b (`:366`) · CR 602.1 (`:2514`) · CR 602.2a (`:2529`) · CR 602.5a (`:2543`) · CR 113.6
(`:771`) · CR 400.2 (`:1935`). Rev 2's citations are carried forward under the user's RULES-SETTLED directive.

---

## 6. The file-by-file change set

| File | Phase | Change |
|---|---|---|
| `types/game_state.rs` | **P1-a** | `RecastContext` → `LoopActionContext` + new `LoopAction` enum; field `last_recast_context` → `last_loop_action_context`. **P5**: one doc comment on `LoopDetectionMode::On`. |
| `game/casting_costs.rs` | **P1-b** | `:6795` — construct `LoopAction::Recast`. Semantics byte-unchanged. |
| `game/engine.rs` | **P1-b/c/d** | new `ActivateAbility` capture arm; `drive_recast_iteration` (`:1451`) → `drive_loop_action_iteration` with an `action`-dispatched opener; `try_offer_object_growth_shortcut` (`:1656`) reads the new field + action-dispatched static randomness gate. |
| `game/ability_scan.rs` | **P2-a/b/c, P3** | new `scan_continuous_modification` + `scan_pt_value` + the two `continuous_modification_reads_*` accessors; `Effect::Token` (`:447`) descends (**all 7 read-bearing fields**); `Effect::Mana` (`:862`) descends; `TargetFilter::Typed` (`:2420`) `sibling: false`. |
| `analysis/resource.rs` | **P2-d, P4, P6, P3-b** | `:1539` blanket → `sibling \|\| projected` descent; `:1444` conjunct renamed (**KEEP IT**); scans (1)/(3)/(4) zone-scoped; scan (6) descends; **re-author** `event_and_sibling_axes_unchanged_for_typed` (`:3926`). |
| `game/triggers.rs` | **P4** | `trigger_definition_functions_in_zone` (`:1057`) private → `pub(crate)`. **No behavior change.** |
| `tests/integration/loop_shortcut_activation.rs` | **P1** | **NEW** — 5 named tests (§4-P1). Add `mod` to `tests/integration/main.rs`. |

**Not touched, deliberately:** the ring, the sampler (`engine.rs:2323`), the deliberate-action clear (`engine.rs:3093`),
Path A/B/C, `GameState::PartialEq`, any cover, any frontend file.

---

## 7. Deferred / filed — **NOT fixed here**

- **DEFERRED-1 — `LoopDetectionMode::On` in a real game.** The drain path auto-wins with no offer window (CR 732.2a does
  not sanction that as a *game* mode). **User-deferred. Frontend question first.**
- **DEFERRED-2 — `fire_time_conditions_read_projected_resource` (`resource.rs:2152`) has no `modifications` scan.**
  Pre-existing latent gap on the `:784` ω-cover. P2-d **preserves** today's incidental protection on the object/fodder
  covers; it does not extend the projected twin.
- ⭐ **DEFERRED-3 — NEW, FOUND BY MEASUREMENT: `mandatory` is computed at an INTRA-CYCLE INSTANT, not over the CYCLE.**
  ✅ **MEASURED (M1):** at the canary's bridge beat `no_living_player_has_meaningful_priority_action(state)` returns
  **`true`** — because the bear is **tapped at that instant**. But **the loop IS optional**: the player chooses to
  activate again once Intruder Alarm untaps it. **CR 104.4b (`docs/MagicCompRules.txt:366`), verbatim: *"Loops that
  contain an optional action don't result in a draw."*** ⇒ the mandatory-ness of a *loop* must be evaluated **over the
  cycle**, not at one beat inside it. **This feeds the Path B DRAW gate (`engine.rs:536`) and is therefore a live
  false-DRAW hazard.** It is **not currently exploitable via the ring** (§1.2: the ring's delta is always zero, so Path B's
  `is_net_progress` conjunct rejects first), which is why it is **filed, not fixed** — but **it must be analysed before
  anything widens the ring or relaxes Path B.** ⛔ **Do not fix it inside this reachability plan.**

---

## 8. UNVERIFIED — **things I did NOT measure**

| # | Claim | Why unverified |
|---|---|---|
| **U1** | **Scan (3)'s replacement zone-of-function authority.** `ReplacementDefinition` has no measured zone field. | I did not find one. **P4's implementer must determine it first** and file the gap if none exists. |
| **U2** | **P2-a's classification of the 36 `ContinuousModification` variants the canary does not exercise.** | The probe's walker used a **narrow, fail-CLOSED wildcard** (`GrantAbility` descends, fixed-P/T ⇒ `NONE`, `_ => CONSERVATIVE`). It was sufficient to close the canary, but **the shipped walker MUST be exhaustive with no wildcard** and each of the 41 arms is a per-variant judgement I traced from `types/ability.rs` but did **not** execute. |
| **U3** | **The P2-d `projected` term is load-bearing.** | The argument is a **code trace** (`resource.rs:2152` scans `condition` only; `:1539` is the file's only `modifications.is_empty()` check). **I did NOT build the `SetDynamicPower{Ref(LifeTotal)}` fixture and revert-probe it.** ⛔ **That is the single most safety-critical unmeasured claim in this plan. The P2 test `projected_reading_modification_still_vetoes_the_firewall` exists to discharge it, and P2 MUST NOT MERGE until that test is shown to FLIP TO FAIL on the revert.** |
| **U4** | **P1's clone-drive terminates and settles for an activation.** | The drive's settle boundary (`engine.rs:1537-1541`) is action-agnostic **by inspection**, and the canary's iteration provably ends at an empty-stack `Priority` beat (✅ measured, M0: `stack=0` at every settle). But **I did not run `drive_loop_action_iteration` — it does not exist yet.** |
| **U5** | **P6's descent on real Commander boards.** | Not measured. Scan (6) is absent from the canary's limb list (✅ M3), which is all I verified. |
| **U6** | **Perf of the P1-b capture gate.** | `battlefield.len() > before` is O(1), and the drive is gated behind it — but I did not benchmark a token-heavy board. |

---

## 9. Acceptance criteria

1. ✅ **The canary OFFERS.** `activation_loop_gond_intruder_alarm_offers_shortcut` reaches `WaitingFor::LoopShortcut`.
2. ✅ **The canary's negative twin does NOT offer.** `activation_loop_without_untapper_does_not_offer` (Presence of Gond
   with **no** Intruder Alarm). **Without this, criterion 1 is vacuous.**
3. ✅ **Gaea's Cradle STILL REJECTS — via the `Effect::Mana` `count` read, not via a blanket** (P2-c). The test must
   assert the **axis**, not just the rejection.
4. ✅ **`projected_reading_modification_still_vetoes_the_firewall` FLIPS TO FAIL when the `|| …projected` term is deleted.**
5. ✅ **`hidden_zone_content_does_not_change_the_offer` FLIPS TO FAIL when P4 is reverted.**
6. ✅ **The full suite is green** — with `event_and_sibling_axes_unchanged_for_typed` **re-authored**, not deleted.
   *(Baseline measured: `16547 passed; 1 failed`, the 1 being exactly that revert-probe.)*
7. ✅ **`cargo fmt --all`** + Tilt `clippy` / `test-engine` / `card-data` green.
