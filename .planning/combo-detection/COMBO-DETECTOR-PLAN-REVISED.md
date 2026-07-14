# Combo detector — the plan, REVISED

**2026-07-14 · Revision 2.** Supersedes `COMBO-DETECTOR-PLAN.md` (REJECTed by `/review-engine-plan`, 8 blockers).

> **Every code citation below was measured by the author against `main` @ `efc76ca1b`.** Anything not personally
> measured is listed in **§11 UNVERIFIED**. Citations carried over from Revision 1 were **re-measured, not trusted**
> — 7 of its rows were wrong, and one of the review's own rows was wrong too (§11).
>
> **`combo-docs-wt` is 768 commits behind `main`. No code fact in this document comes from it.**

---

## 0. What we are implementing *(preserved from Rev 1 — unchanged, still correct)*

**Shortcutting a loop is OPTIONAL.** CR 732.2a: the player *"**may** suggest a shortcut."* Nobody is compelled to
propose one, and no opponent is compelled to accept the proposed count.

> ## ⇒ **THE DETECTOR STAYS OPT-IN. Turning it on IS the table agreeing to use the optional shortcut rule.**
> **`LoopDetectionMode::Off` is not dead code and is not a wart — it is the "we are not shortcutting loops in this
> game" setting, and it must remain the default.** *(Measured default: `HostSetup.tsx:227` `remembered?.loopDetection ?? { type: "Off" }`; `GameSetupPage.tsx:92` `useState<LoopDetectionMode>({ type: "Off" })`.)*

**Not gated on Rules Enforcement Level.** The *no-conditional-actions* core is in **CR 732.2a** itself. The regime is
identical at Regular / Competitive / Professional and in casual play. **No "tournament mode." Nothing to gate.**

---

## 1. The spec — the whole feature, in five stages *(preserved from Rev 1 — unchanged, still correct)*

| | Stage | Rule |
|---|---|---|
| **1** | **CAPTURE** the player's performed actions, **as FIXED choices** — a loop is a *sequence of actions*, not a decision tree. | CR 732.2a: *"a sequence of game choices… **can't include conditional actions**."* |
| **2** | **REPEAT** that exact sequence; determine whether it yields an **unbounded resource**. | CR 732.1b |
| **3** | **CLASSIFY** — **ADVANTAGE** or **WIN**, *and **DRAW** (CR 104.4b — a mandatory loop with no way to stop)*. | CR 704.5a · CR 104.4b |
| **4** | **PRESENT** it. If accepted, **pass priority around the table** so every opponent may interact or shorten it. | CR 732.2b/c |
| **5** | If accepted and un-interacted-with: **emit the certificate and APPLY the state changes.** | CR 732.2a |

> ### ⭐ The omission that is the whole design — and it is correct
> **The spec says "repeat the ACTIONS." It never says the game state must return to where it started.** Neither does
> the rulebook. **CR 732.2a's own worked example — Presence of Gond + Intruder Alarm
> (`docs/MagicCompRules.txt:6373`) — ADDS A TOKEN EVERY ITERATION**, so its state provably never recurs, and the
> rules shortcut it a million times.
>
> ## ⇒ **A detector that requires STATE RECURRENCE must reject the rulebook's own worked example.**

---

## 2. What already exists — **DO NOT REBUILD ANY OF THIS**

**Rev 1's conclusion here SURVIVES. Rev 1's citation table did not — it is rebuilt below from measurement.**

| Stage | On `main` | Where *(measured)* |
|---|---|---|
| **1 · capture** | `last_recast_context` | **written** at `game/casting_costs.rs:6795`, `types/game_state.rs:10581`; **read** at `game/engine.rs:450` |
| **2 · repeat** | **Real replay on a clone** — 2 iterations / 3 settle frames, re-entrancy-guarded; `drive_recast_iteration` genuinely re-applies `CastSpell` via `apply_action` | `game/engine.rs:1688-1696` (`let _probe = SimulationProbeGuard::enter(); … state.clone(); drive_recast_iteration(…, 0); … drive_recast_iteration(…, 1)`) |
| **2 · unbounded (LIVE)** | ⭐ `loop_states_cover_modulo_fodder_growth` — **the only cover on a live reducer path** | `analysis/resource.rs:1095`, **called from `game/engine.rs:1732`** |
| **2 · unbounded (OFFLINE ONLY)** | `loop_states_cover_modulo_object_growth` (`:924`) · `loop_states_cover_modulo_counter_growth` (`:1326`) — called **only** from `detect_loop` (`analysis/loop_check.rs:223`/`:230`), which states at `:221`: *"This is the OFFLINE classifier only — no live/reducer path."* | `analysis/resource.rs:924`, `:1326` |
| **2 · unbounded (ω / stack-growth)** | `loop_states_cover_modulo_growth` | `analysis/resource.rs:784` (**not** the counter cover — that is `:1326`) |
| **3 · win / draw / advantage** | Path A `:498` · **Path B gate `:536`** · `WinKind::Advantage` | `game/engine.rs:498`, `:536` · `analysis/loop_check.rs:107` (the **enum** opens at `:83`) |
| **4 · present** | `WaitingFor::LoopShortcut` · `IterationCount` | `types/game_state.rs:4458` · `analysis/decision_template.rs:281` (the **enum**; `:203` is the *field*) |
| **4 · accept / decline / interact** | `DeclareShortcut` `types/actions.rs:834` · `RespondToShortcut` `:841` · `DeclineShortcut` `:848` | |
| **5 · apply** | `apply_confirmed_shortcut` → `apply_until_lethal_shortcut` / `materialize_fixed_shortcut` | `game/engine.rs:855`, `:906`, `:1325` |
| **— · reject non-deterministic loops** | **static** gate `spell_ability_bears_randomness` (`game/engine.rs:1684`) + ⭐ **runtime backstop** — an RNG **word-position** delta across the drive | ⭐ **`game/engine.rs:1713`** (`if s_n2.rng.get_word_pos() != state.rng.get_word_pos() { return None; }`) |

> **Rev 1 cited `ability_scan.rs:4407` as the "runtime" determinism gate. It is not.** Its own doc at `:4406` calls
> `effect_is_randomness_bearing` *"the **static**, compile-time-exhaustive half."* Rev 1 pointed at the static gate
> **twice** and never cited the real runtime backstop. **The runtime backstop is `game/engine.rs:1713`.**

> ## ⇒ **The pipeline is complete end to end. It is NOT a capability problem. The detector cannot be REACHED.**

---

## 3. ⛔⛔ THE ROOT CAUSE — re-derived by tracing CR 732.2a's OWN worked board

**The fire-time firewall** is `fire_time_conditions_read_growing_class` (`analysis/resource.rs:1457`). It has **SEVEN**
scans — (1), (2), (3), (4), (5), (5b), (6) — not four. It is a **disjunction**: any one scan returning `true` vetoes
the entire detection.

### 3.1 Step 0 gate — the canary board is REAL (verified against Scryfall, not memory)

`https://api.scryfall.com/cards/named?exact=…`, fetched 2026-07-14; **byte-identical** to `data/card-data.json`:

| Card | Oracle text (Scryfall == card-data.json) | in `card-data.json`? |
|---|---|---|
| **Presence of Gond** | `Enchant creature` / `Enchanted creature has "{T}: Create a 1/1 green Elf Warrior creature token."` | ✅ PRESENT |
| **Intruder Alarm** | `Creatures don't untap during their controllers' untap steps.` / `Whenever a creature enters, untap all creatures.` | ✅ PRESENT |
| **Gaea's Cradle** | `{T}: Add {G} for each creature you control.` | ✅ **PRESENT** |

> ⚠️ **The review report asserted "Gaea's Cradle is absent from `card-data.json`." That is WRONG.** Measured:
> `jq 'has("gaea'"'"'s cradle")' data/card-data.json` → `true`. (The root object is keyed by **lowercase** card name;
> an exact-case probe returns absent.) **Gaea's Cradle can be, and will be, used as a real-card fixture.**

### 3.2 The board: Presence of Gond enchanting a creature + Intruder Alarm. **ALL THREE PERMANENTS ARE ON THE BATTLEFIELD.**

Their **measured** parsed ASTs (`jq '.["…"]' data/card-data.json`):

- **Presence of Gond** → `static_abilities[0] = { mode: Continuous, affected: Typed{Creature, EnchantedBy}, modifications: [ GrantAbility { definition: Activated { effect: Token{ name:"Elf Warrior", power:Fixed(1), toughness:Fixed(1), count:Fixed(1), owner:Controller }, cost: Tap } } ] }`
- **Intruder Alarm** → `triggers[0] = { mode: ChangesZone, destination: Battlefield, trigger_zones:["Battlefield"], condition: null, execute: { effect: SetTapState { target: Typed{Creature}, scope: All, state: Untap } } }`; `static_abilities[0] = { mode: CantUntap, modifications: [] }`
- **The enchanted creature** → after the layer pass, `obj.abilities` **contains** the granted `{T}: Create a 1/1 Elf Warrior token`. **Measured**: `game/layers.rs:5332-5342`, `ContinuousModification::GrantAbility { definition } => { … Arc::make_mut(&mut obj.abilities).push(granted); }`

### 3.3 ⛔ THREE INDEPENDENT VETOES. ALL THREE ARE ON THE BATTLEFIELD.

| # | Firewall scan | The object | The chain, measured | Cleared by |
|---|---|---|---|---|
| **V1** | **(1)** trigger `execute` bodies — `resource.rs:1460-1476` | **Intruder Alarm** (battlefield) | `def.execute` → `ability_definition_reads_sibling_mutable` → `Effect::SetTapState{ target: Typed(Creature) }` → `scan_target_filter` → **`ability_scan.rs:2420` `sibling: true`** | **P3** |
| **V2** | **(2)** every ability body on a functioning **battlefield** permanent — `resource.rs:1488-1499` | **the enchanted creature** (battlefield) | `obj.abilities` (granted, per `layers.rs:5342`) → `Effect::Token { .. }` → **`ability_scan.rs:447` `=> Axes::CONSERVATIVE`** (⇒ `sibling: true` via `:131-135`) | **P2** |
| **V3** | **(4)** condition-gated statics — `resource.rs:1527-1543` | **Presence of Gond** (battlefield) | `def.modifications` is non-empty → **`resource.rs:1539` `if !def.modifications.is_empty() { return true; }`** — a raw blanket that **never reads `sibling` at all**, firing on a **visible, functioning, battlefield** permanent | **P2** |

> ## ⇒ **ALL THREE VETOES ARE BATTLEFIELD-RESIDENT.**
> ## ⇒ **THE HIDDEN-ZONE (CR 400.2) FIX UNBLOCKS *NOTHING*. Rev 1's P2 header — "(the rules fix — AND the reachability fix)" — IS FALSE.**
> ## ⇒ **Rev 1's causal story AND its phase ordering were both wrong. This revision re-orders them.**

### 3.4 The real missing building block — **the code names it itself**

`analysis/resource.rs:1452-1455`, verbatim from the firewall's own doc comment:

> *"(4) condition-gated statics — condition plus any live continuous modification (**default-CONSERVATIVE: no
> `scan_continuous_modification` walker exists**, and an anthem/P-T grant applies to and scales with the growing
> class)"*

**That walker is the reachability fix.** V3 is it directly; V2 is the same granted ability seen from the other side.

**And the doc's own justification for the blanket is measurably wrong.** It says *"an anthem/P-T grant applies to and
scales with the growing class."* Measured (`types/ability.rs`):

- `AddPower { value: i32 }` (`:19385`) · `AddToughness { value: i32 }` (`:19388`) · `SetPower`/`SetToughness` (`:19391`/`:19394`) — **fixed `i32`. An anthem READS NOTHING.** It *applies to* each member of a growing class; it does not *read* a mutable aggregate. Reading is what the `sibling` axis means (`ability_scan.rs:114-116`).
- The variants that genuinely read are the **dynamic** ones: `SetDynamicPower`/`SetDynamicToughness`/`SetPowerDynamic`/`SetToughnessDynamic`/`AddDynamicPower`/`AddDynamicToughness`/`AddDynamicKeyword`, each `{ value: QuantityExpr }` (`:19483`–`:19516`). **Those** are what must veto.

### 3.5 The CR 400.2 hidden-zone leak is REAL — and it is a **rules fix that unblocks nothing**

Three of the seven scans iterate `state.objects.values()` over **every zone, including library and hand**:

- **scan (1)** `resource.rs:1460` → `functioning_abilities::active_trigger_definitions` (`:391`). Despite the name, its
  filter is: *if Command-zone non-emblem, defer to `non_emblem_command_zone_trigger_functions`; otherwise* **bare
  `true`** (`:405-409`). **No zone filter. A trigger def on a card in the LIBRARY is returned as "active."**
- **scan (3)** `resource.rs:1501` → `functioning_abilities::active_replacements` (`:446`), filtered **only** by
  `object_functions` (`:108-116`) — phased-out + Command-zone **only**. ⇒ **returns `true` for a LIBRARY card.**
- **scan (4)** `resource.rs:1527` — `state.objects.values()`, all zones.

Two independent rules, either one fatal:
1. **CR 113.6** (`docs/MagicCompRules.txt:771`) — a non-instant/sorcery object's abilities *"usually function only while
   that object is on the battlefield."* **An ability in the library does not function. Scanning it is not conservatism
   — it is scanning an ability that does not exist.**
2. ⭐ **CR 400.2** (`:1935`, verbatim) — ***"Library and hand are hidden zones."*** The offer's **presence or absence is
   itself observable to every player.** If it depends on hidden-zone contents, **the engine leaks hidden information
   into observable game state.** **That is not a conservative approximation. It is a rules violation.**

**Ship it (P4) — but label it honestly. It clears none of V1/V2/V3.**

### 3.6 The composition failure — corrected counts and the **right model**

**Rev 1 said "84 sites … a one-sided ratchet." Both the count and the model are wrong.** Measured in
`game/ability_scan.rs`:

| | Rev 1 | **Measured** | |
|---|---|---|---|
| `Axes::CONSERVATIVE` | 54 | **51** | 54 `grep` hits − **3 in comments** (`:36`, `:41`, `:5059`) |
| `sibling: true` | 30 | **29** | 30 `grep` hits − **the `CONSERVATIVE` definition itself** at `:133` |
| **TOTAL** | *"84"* | **80** | |

> ## ⛔ **AND THEY ARE NOT DISJOINT: `Axes::CONSERVATIVE` *CONTAINS* `sibling: true`** (`:131-135`).

**They are two OPPOSITE things and get OPPOSITE treatment:**

| | **BLANKET** (`Axes::CONSERVATIVE`) | **LITERAL** (`sibling: true` inside an arm) |
|---|---|---|
| Count | **51** | **29** |
| The walk | **DOES NOT DESCEND** | **DOES DESCEND** (`.or(scan_…(child))`) |
| Meaning | *"we did not look inside"* | *"this arm reads the sibling axis, **and** here are its children"* |
| Treatment | **Make it descend** (P2) | **Leave it alone** — with **exactly one** exception (P3) |

**Where the 29 real literals live (measured, by enclosing fn):**

| fn | count | lines | Does it COUNT a mutable set? |
|---|---|---|---|
| `scan_quantity_ref` | **20** | `:1606, 1618, 1631, 1645, 1657, 1681, 1690, 1699, 1708, 1717, 1726, 1735, 1744, 1753, 1767, 1779, 1788, 1895, 2093, 2102` | ✅ **YES** — `ObjectCount`, `ObjectCountDistinct`, … |
| `scan_trigger_condition` | 4 | `:2608, 2695, 2796, 2803` | ✅ YES |
| `scan_static_condition` | 3 | `:2930, 2976, 2982` | ✅ YES |
| `scan_player_filter` | 1 | `:3341` | ✅ YES |
| **`scan_target_filter`** | **1** | **`:2420`** | ❌ **NO — it merely NAMES a type.** |

> ## ⇒ **28 of the 29 literals ALREADY encode "counts a mutable set." Only `:2420` names-without-counting.**
> ## ⇒ **P3 IS A ONE-LINE CHANGE. The real work is the 51 blankets — and only the 3 on the canary's path.**
> ## ⛔ **DO NOT SWEEP THE 80 SITES UNIFORMLY. That is the catastrophic direction** — a uniform flip would strip
> the 20 counting literals in `scan_quantity_ref` and hand the detector a **false certificate**.

---

## 4. The architectural spine — why a **decidable** predicate exists at all

All three vetoes are **blankets that refuse to descend**:

- `Effect::Token { .. } => Axes::CONSERVATIVE` — never looks at **what the token is**.
- `Effect::Mana { .. } => Axes::CONSERVATIVE` — never looks at its **`count`**.
- `!def.modifications.is_empty() => true` — never looks at **what the modification does**.

They are blanket because a **sound, general** descent means reasoning about arbitrary ability programs — an unbounded
static-analysis problem. **We do not have to solve that problem.** Compose:

- **battlefield-only** — CR 113.6 + CR 400.2 (§3.5),
- **no nested loops** — a nested loop is a slow-play violation in real play; it is not a shape the detector must ACCEPT,
- **not Turing-complete** — the overwhelming majority of presented combos are not,
- **and the TWO CONCRETELY OBSERVED ITERATIONS the drive already produces** — `game/engine.rs:1688-1696`, a **genuine
  replay on a clone**, not a static re-derivation.

…and the question collapses from *"could ANY ability ANYWHERE read ANY mutable set?"* (unanswerable ⇒ blanket ⇒ always
veto) to:

> # **"Does any BATTLEFIELD ability's fire-time condition read THE SPECIFIC AXIS that THIS OBSERVED loop grows?"**

**Decidable. Bounded. Per-axis.** Answering it **IS** the `scan_continuous_modification` walker the engine's own
comment (`resource.rs:1452-55`) says nobody ever built.

### 4.1 ⛔ NON-GOALS — state these, and hold them

- ❌ **No fixpoint / abstract interpretation / `egg` / e-graphs / symbolic execution.** None. Ever.
- ❌ **No general program analysis of ability bodies.** The walk is a **finite, exhaustive, single-pass AST match**.
- ❌ **No nested-loop support.** Grant-realization depth is **1**: a granted ability's body is scanned; an ability
  granted *by* a granted ability is **fail-closed ⇒ REJECT** (§5, P2-a, depth guard).
- ❌ **No Turing-complete combo class.** Out of scope, by construction, forever.

### 4.2 ⭐ The soundness asymmetry is PRESERVED, not traded away

**Narrowing the input class is what BUYS the precision. It does not spend safety to get it.** Any shape outside the
recognized class — deeper-than-depth-1 grants, non-battlefield function, an axis the walker cannot classify, a new
enum variant — **still fails closed ⇒ REJECT.** The walker's match is **exhaustive with no `_` wildcard**, so a future
`ContinuousModification` variant **fails to compile** until it is classified. *(Contrast the shipped precedent
`modification_grants_growing_cost_keyword`, `ability_scan.rs:4080-4088`, which **does** use `_ => false` — acceptable
for its question, **forbidden** for ours. See §6 Rust Idioms.)*

**This is the plan's warrant for containing the only phases that move the detector toward ACCEPT.**

---

## 5. The phases

> **Ordering rationale:** P1 measures. **P2 + P3 are the reachability fix** — together they, and only they, make the
> canary green. **P4 is an independent rules fix that unblocks nothing** (a real CR 400.2 information leak — it ships
> because it is a real defect, not because the canary needs it). **P5 is doc-only** — `LoopDetectionMode` is a
> **separate concern**, kept whole by user directive. P6 is scoped-in and small.
>
> ## ⇒ **If you only ship two phases, ship P2 and P3. Everything else is independent of the canary.**

### P1 — **PER-LIMB** probe *(replaces Rev 1's stub-everything P1)*

**Rev 1's P1 stubbed the WHOLE firewall to always-accept. That probe cannot answer the only question it exists to
answer:** stubbing everything bypasses all three vetoes ⇒ GREEN ⇒ the plan concludes *"the covers are the only
blocker"* and proceeds to fix **the one limb that was not responsible.** It is a **false confirmation** by construction.

**Instead: instrument each limb SEPARATELY.** In a throwaway worktree cut from `main` (own `target/` ⇒ no Tilt lock
contention), add a temporary `#[cfg(test)]`-only instrumented copy of `fire_time_conditions_read_growing_class` that
returns **which limb fired**, not `bool`:

```rust
#[cfg(test)]
#[derive(Debug, PartialEq)]
enum FirewallVeto { S1Trigger, S2BattlefieldBody, S3Replacement,
                    S4StaticCondition, S4StaticModifications,  // ⛔ the two branches of (4) SEPARATELY
                    S5Transient, S5bGrantedKeyword, S6DeferredStores, None }
```

⛔ **Within scan (4), the `condition` branch (`:1531-1538`) and the `!modifications.is_empty()` branch (`:1539-1541`)
MUST be reported separately.** They are different defects with different fixes; collapsing them reproduces exactly the
mis-diagnosis this phase exists to prevent.

**Fixture:** the real CR 732.2a board — `presence of gond` + `intruder alarm` + a vanilla creature, all battlefield,
built with `GameScenario` from **verbatim `card-data.json` Oracle text** (`/card-test` recipe).

**Predicted result (this plan's falsifiable claim):** the probe reports **exactly `{S1Trigger, S2BattlefieldBody,
S4StaticModifications}`** and **not** `S3Replacement`, `S5*`, or `S6DeferredStores`.

**⛔ THE RED PLAN — what we do if the prediction is wrong (Rev 1 said only "instrument it"):**

| Probe result | Diagnosis | Action |
|---|---|---|
| **Exactly the 3 predicted** | §3.3 confirmed. | **Proceed to P2 + P3 as written.** |
| **A predicted limb does NOT fire** | §3.3 over-counts; that limb's fix is unnecessary. | **Delete that phase's sub-step.** Do **not** "fix" a limb that does not fire — that is a purpose-built patch. Re-run §3.3's trace against the measured AST and amend §3. |
| **An UNPREDICTED limb fires** (e.g. `S6DeferredStores`) | §3.3 under-counts. **The fixture's board is not clean** (S6) or an authority is broader than measured. | **STOP. Do not proceed to P2.** Dump the firing object's id + zone + the offending def. Add the limb to §3.3 and re-scope. **A new limb is a new phase, not a bolt-on.** |
| **`None`** (firewall does not veto) — GREEN before any change | §3 is wholly wrong; the blocker is **downstream** (the cover, the drive, or the reach to `try_offer_object_growth_shortcut`). | **STOP. This plan is void.** Bisect the live path `engine.rs:445-451` (the six-conjunct gate) → `:1684` → `:1713` → `:1732`, reporting which conjunct/return-`None` kills the offer. |

**P1 leaves no production change. Its entire output is a measurement.**

---

### P2 — ⭐ **Make the blankets DESCEND** *(THE reachability fix — clears V2 + V3)*

#### P2-a — `scan_continuous_modification` — **the walker the code says does not exist**

**New**, in `game/ability_scan.rs` (beside the shipped precedent `modification_grants_growing_cost_keyword` at `:4080`):

```rust
/// CR 613.1: a continuous modification's read surface. Exhaustive, NO `_` wildcard —
/// a future `ContinuousModification` variant fails to compile until classified.
fn scan_continuous_modification(m: &ContinuousModification, depth: u8) -> Axes { … }

pub(crate) fn continuous_modification_reads_sibling_mutable(m: &ContinuousModification) -> bool;
pub(crate) fn continuous_modification_reads_projected_resource(m: &ContinuousModification) -> bool;
```

**Classification of all 41 measured variants** (`types/ability.rs:19350`–`:19599`):

| Class | Variants | Verdict |
|---|---|---|
| **Read-free structural** | `SetName` `AddKeyword` `RemoveKeyword` `RemoveAllAbilities` `AddType` `RemoveType` `AddSubtype` `RemoveSubtype` `SetCardTypes` `RemoveAllSubtypes` `AddAllCreatureTypes` `AddAllBasicLandTypes` `AddAllLandTypes` `AddChosenSubtype` `AddChosenColor` `AddChosenKeyword` `RemoveChosenKeyword` `SetColor` `AddColor` `SwitchPowerToughness` `AssignDamageFromToughness` `AssignDamageAsThoughUnblocked` | `Axes::NONE` |
| ⭐ **Fixed P/T (the ANTHEM class)** | `AddPower{value:i32}` `AddToughness{value:i32}` `SetPower{value:i32}` `SetToughness{value:i32}` | **`Axes::NONE`** — *reads nothing.* **This is the correction to the doc's own wrong justification (§3.4).** |
| **Quantity-bearing** | `SetDynamicPower` `SetDynamicToughness` `SetPowerDynamic` `SetToughnessDynamic` `AddDynamicPower` `AddDynamicToughness` `AddDynamicKeyword` — all `{ value: QuantityExpr }` | **`scan_quantity_expr(value)`** ⇒ an `ObjectCount`-keyed dynamic P/T correctly **VETOES** (`:1606`) |
| **Ability-bearing (descend, depth ≤ 1)** | `GrantAbility{definition}` `GrantStaticAbility{definition}` `GrantTrigger` `AddStaticMode` `CopyValues` | recurse into the granted body via the **existing** `ability_definition_axes` / `scan_static_condition`; **`depth > 0` ⇒ `Axes::CONSERVATIVE`** (§4.1 non-goal: no nested grants) |
| **Fail-closed** | `GrantAllActivatedAbilitiesOf` `GrantAllTriggeredAbilitiesOf` `RetainPrintedTriggerFromSource` `RetainPrintedAbilityFromSource` `AddKeywordWithDerivedCost{derivation:CostDerivation}` | **`Axes::CONSERVATIVE`** — the source set is not statically known |

#### P2-b — `Effect::Token { .. }` DESCENDS *(clears **V2**)*

`game/ability_scan.rs:447`: `Effect::Token { .. } => Axes::CONSERVATIVE` → descend into its read-bearing fields
(measured, `types/ability.rs:9546-9560+`): `count: QuantityExpr`, `power: PtValue`, `toughness: PtValue`,
`owner: TargetFilter`. The read-free fields (`name`, `types`, `colors`, `keywords`, `tapped`, `enters_attacking`) bind
to `_` with a one-line justification each — **exhaustive destructure, no `..`**, per the `resolved_ability_axes`
precedent (`ability_scan.rs:148-155`).

⇒ Presence of Gond's token (`count: Fixed(1)`, `power/toughness: Fixed(1)`, `owner: Controller`) ⇒ **`Axes::NONE`.**
⇒ A token whose `count` is `Ref(ObjectCount{…})` — *"create X tokens, where X is the number of creatures you control"* —
**still VETOES** via `:1606`. **This is the class boundary, and it is drawn in the right place.**

#### P2-c — `Effect::Mana { .. }` DESCENDS *(clears NOTHING on the canary — and is MANDATORY anyway)*

`game/ability_scan.rs:862`: `Effect::Mana { .. } => Axes::CONSERVATIVE` → descend into `ManaProduction`'s
`count: QuantityExpr` (`types/ability.rs:1689-1701`: `Colorless{count}`, `AnyOneColor{count}`, …).

> ## ⭐ **P2-c IS WHAT MAKES THE GAEA'S CRADLE NEGATIVE MEAN ANYTHING.** See §5-P3 and §7-C4. **Without it the
> acceptance criterion is vacuous** — and Rev 1 shipped exactly that vacuous criterion.

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

> ## ⛔ **THE VETO MUST BE `sibling || projected`, NOT `sibling` ALONE. An implementer WILL get this wrong.**
>
> **Measured:** the projected-axis twin `fire_time_conditions_read_projected_resource` (`analysis/resource.rs:2152`)
> scans a static's **`condition` ONLY** (`:2199-2207`) — it has **NO `modifications` scan at all**. And the blanket at
> `:1539` is the **only** `modifications.is_empty()` check in the file. **The sibling blanket is therefore
> *incidentally* providing the ONLY protection the object/fodder covers (`:968`, `:1131`) have against a
> projected-reading modification** — e.g. `SetDynamicPower { value: Ref(LifeTotal) }`.
>
> **Veto on `sibling` alone and you REMOVE that protection ⇒ a real FALSE-CERTIFICATE hole ⇒ a real game ends
> wrongly.** Vetoing on `sibling || projected` preserves **exactly** the protection the blanket gave, and removes
> **only** the vetoes where the modification reads **neither** axis — which is precisely the anthem / fixed-token-grant
> class the canary needs. **One-sided safety, held.**
>
> *(Whether the projected twin at `:2152` should grow its own `modifications` scan is a **separate, pre-existing**
> latent gap on the `:784`/`:831` ω-cover. **OUT OF SCOPE — see §8.**)*

---

### P3 — **`Typed` NAMES a type; it does not COUNT one** *(ONE LINE — clears **V1**)*

`game/ability_scan.rs:2418-2422`, measured verbatim:

```rust
TargetFilter::Typed(tf) => Axes {
    event: true,                                    // :2419  ⛔ UNCHANGED — see below
    sibling: true,                                  // :2420  ⇒  sibling: false
    projected: typed_filter_reads_projected(tf),    // :2421  unchanged
},
```

**A `Typed` filter is a PREDICATE — `"creature"`, `"creature you control"`. It selects a set. It does not read the
set's CARDINALITY.** Counting is `QuantityRef::ObjectCount`, and that is a **different node**.

#### ⭐ Why this does NOT open the catastrophic hole — **STRUCTURALLY IMPOSSIBLE, and MEASURED**

Rev 1 warned: *"a flip that un-rejects BOTH [Intruder Alarm and Gaea's Cradle] is a HOLE."* **That fear is unfounded,
and the code proves it.** `game/ability_scan.rs:1603-1611`, verbatim:

```rust
QuantityRef::ObjectCount { filter } => {
    let mut acc = Axes { event: false, sibling: true, projected: false };  // :1606 ← INDEPENDENT LITERAL
    acc = acc.or(scan_target_filter(filter));                              // :1609 ← .or() only ADDS
    acc
}
```

> ## ⇒ **`ObjectCount`'s `sibling: true` at `:1606` is its OWN literal. It does NOT come from `scan_target_filter`.**
> ## ⇒ **Flipping `:2420` PROVABLY CANNOT un-reject the counting class.** Identically for `ObjectCountDistinct`
> (`:1618`) and `ObjectCountBySharedQuality` (`:1631`).
>
> **The COUNT-vs-NAME distinction Rev 1 asked for is ALREADY ENCODED IN THE CODE** — at `:1606`/`:1618`/`:1631`
> (count) versus `:2420` (name). **P3 does not invent it. P3 stops `:2420` from lying about it.**

#### ⛔ B5 — the over-edit guard and the live second consumer

1. **`event` at `:2419` STAYS `true`. We touch `sibling` ONLY.** *Why that is safe w.r.t. the live consumer:* the
   **second** consumer of the `sibling` axis is `game/triggers.rs:3893-3894` (CR 603.3b trigger auto-resolve):
   ```rust
   let c2_order_independent = !ability_scan::ability_uses_event_context(&reference)
       && !ability_scan::ability_reads_sibling_mutable(&reference);
   ```
   The gate is **`!event && !sibling`**. With `event` still `true` at `:2419`, `ability_uses_event_context` still
   returns `true` for any `Typed`-bearing ability ⇒ **`c2_order_independent` stays `false` ⇒ the CR 603.3b auto-resolve
   behavior is BYTE-UNCHANGED.** *(This is why the comment at `:2416` treating the two as a pair must not be obeyed
   literally — they have different consumers.)*
2. **`analysis/resource.rs:3926 event_and_sibling_axes_unchanged_for_typed` GOES RED. It must be RE-AUTHORED, not
   deleted, not `#[ignore]`d.** Its doc at `:3922-3925` literally states the revert-probe for this exact flip:
   *"Revert-probe: setting the arm's `event`/`sibling` to `false` flips these."* **Re-author** → rename to
   `event_axis_unchanged_for_typed_sibling_axis_relaxed`, keep the `event: true` assertion **verbatim** (it is the
   `triggers.rs:3893` contract, and it is now the *load-bearing half*), and **invert** the `sibling` assertion with a
   comment pointing at `:1606` as the reason it is safe.

---

### P4 — Scope the covers to **VISIBLE, FUNCTIONING** objects *(the CR 400.2/113.6 rules fix — **unblocks NOTHING**)*

**Honest label:** this clears **zero** of V1/V2/V3 (§3.3). It ships because it is a **real information leak** (§3.5),
not because the canary needs it.

#### ⛔ CALL THE ENGINE'S RUNTIME FUNCTIONING AUTHORITY. **DO NOT HAND-ROLL A ZONE LIST.**

**The authorities exist, measured:**

| Authority | Where | Signature |
|---|---|---|
| `trigger_definition_functions_in_zone` | **`game/triggers.rs:1057`** | `fn (def: &TriggerDefinition, zone: Zone) -> bool` — ⚠️ **PRIVATE. Must be widened to `pub(crate)`.** |
| `static_functions_in_zone` | **`game/functioning_abilities.rs:187`** | `pub(crate) fn (obj: &GameObject, def: &StaticDefinition) -> bool` — already `pub(crate)` ✅ |

#### ⭐ **THE EXEMPLAR TO MIRROR — trace it and P4 writes itself**

`granted_keyword_triggers_in_zone` (**`game/triggers.rs:423-437`**) **already does exactly this**, at **`:434`**:

```rust
synthesize_granted_keyword_triggers(obj, keywords.iter())
    .into_iter()
    .filter(|(_, def)| trigger_definition_functions_in_zone(def, obj.zone))   // :434  ⭐
```

> **This is precisely why scan (5b) is NOT a leak while scans (1)/(3)/(4) are.** *(M1: the plan's pick of (1)(3)(4) as
> the leaks is **CORRECT**; Rev 1's claim of "four scans" was not — there are **seven**, and (5)/(5b)/(6) were never
> mentioned. (5b) is already zone-filtered; (5) reads `state.transient_continuous_effects`, not `state.objects`, so it
> has no zone surface; (6) is §5-P6.)*

#### ⛔⛔ **THE TRAP — BLACKLISTED BY NAME**

> ## **`functioning_abilities::object_functions` (`game/functioning_abilities.rs:108`) IS NOT A ZONE-OF-FUNCTION AUTHORITY.**
> **Measured, `:108-116`:** it checks **phased-out** and **Command-zone-non-emblem**, then `return true`.
> ## ⇒ **IT RETURNS `true` FOR A CARD IN THE LIBRARY.**
> **An implementer WILL reach for it, because scan (3) already calls it** (via `active_replacements`, `:446`).
> **Calling it reintroduces the exact bug this phase exists to fix.**

#### The changes

- **scan (1)** `resource.rs:1461` — filter `active_trigger_definitions(state, obj)` by
  `triggers::trigger_definition_functions_in_zone(def, obj.zone)`.
- **scan (3)** `resource.rs:1501` — filter `active_replacements(state)` by the replacement's zone of function.
  ⚠️ **`ReplacementDefinition` has no measured zone field** (`active_replacements`' own doc says its scan is
  *"future-proofed for per-replacement zones but no current caller needs it"*). **⇒ UNVERIFIED (§11-U1): the
  implementer must first determine the correct authority for replacement zone-of-function.** If none exists,
  **restrict to `obj.zone == Zone::Battlefield || obj.zone == Zone::Graveyard`** *(CR 113.6b — graveyards are PUBLIC
  and some replacements do function there)* **and file the gap.** ⛔ **Do NOT hand-roll a broader list.**
- **scan (4)** `resource.rs:1527` — filter `obj.static_definitions.iter_all()` by
  `functioning_abilities::static_functions_in_zone(obj, def)`.

⚠️ **`active_trigger_definitions` is a SHARED authority.** The live trigger pipeline has **its own** zone gate at
`game/triggers.rs:1040` (`source_has_trigger_in_zone`, which composes `active_trigger_definitions` **with**
`trigger_definition_functions_in_zone`). ⇒ **Fix the FIREWALL's scope. Do NOT change `active_trigger_definitions`'
contract.**

**Class property (the CR 400.2 property, stated as an assertion):** *the verdict is **invariant under ANY hidden-zone
content**.* Shuffle an arbitrary card into a library ⇒ **the offer must not change.**

---

### P5 — `LoopDetectionMode`: ⛔ **KEEP ALL THREE MODES. TOUCH NOTHING. (USER DIRECTIVE, 2026-07-14.)**

> ## ⭐ **USER DIRECTIVE — this supersedes both Rev 1's P4 and this plan's own first draft of P5:**
> > *"Keep off/on/interactive for the combo detector for now. It helps us separate concerns."*
>
> **⇒ `LoopDetectionMode` is OUT OF SCOPE for this plan.** Keep the enum. Keep all three variants. **Keep the two
> player-facing toggles and the `?loop=` URL param exactly as they are.** This plan is about **REACHABILITY** — the
> mode taxonomy is a **different concern**, and bundling a frontend/UX change into the reachability fix is precisely
> the coupling the directive exists to prevent.
>
> **The three modes are load-bearing FOR THIS WORK:** `Off` (default; the CR 732.2a opt-in of §0) · **`On` (auto-resolve
> — lets us exercise the detector's *classification* in isolation from the *offer* machinery)** · `Interactive` (the
> full offer path). **P1's per-limb probe and P2/P3's canary tests are easier to write and to read with `On` available**
> — a green `On` run isolates "did the firewall veto?" from "did the offer/response plumbing work?".

**The only change P5 makes is a DOC COMMENT** — `types/game_state.rs` (at the `On` variant):

```rust
/// ANALYSIS-shaped (CR 732.2a): auto-resolves a lethal drain WITHOUT an offer and without an
/// opponent response window (`game/engine.rs:420-427`). Correct for its offline consumer — the
/// `combo-verify` corpus classifier (`analysis/corpus.rs:2039`) — which asks "does this deck
/// contain a lethal loop?" rather than playing a game. See DEFERRED-1 below.
```

**⇒ DEFERRED-1 (filed, NOT fixed here):** whether `On` should remain *player-selectable* in a real game is a **live
rules question** — the drain path (`game/engine.rs:356-430`) auto-wins with no offer, which CR 732.2a does not
sanction as a *game* mode. **The user has deferred it deliberately. Do not silently resolve it inside this plan.**
It is a **standalone follow-up**, and it is a **frontend** question first (which options the two segmented controls
expose), not an engine one.

<details>
<summary><b>Measured evidence retained</b> — why deleting <code>On</code> was never a refactor (Rev 1's P4), preserved so the deferred follow-up does not re-derive it</summary>

**Rev 1's P4 ("`On` → DELETE") is NOT a refactor. Measured, it breaks:**

| Consumer | Measured |
|---|---|
| **The shipped `combo-verify` binary** | `crates/engine/Cargo.toml:61-64` (`[[bin]] name = "combo-verify"`, `required-features = ["combo-verify"]`) ← `analysis/corpus.rs:2039` `state.loop_detection = LoopDetectionMode::On;` |
| **Two live UI toggles** | `client/src/components/lobby/HostSetup.tsx:544` · `client/src/pages/GameSetupPage.tsx` (+`client/src/game/loopDetectionMode.ts` — a `?loop=` **URL param** with its own parser/serializer) |
| **The wire form** | `client/src/adapter/types.ts:2647` `export type LoopDetectionMode = …` (a `{"type":"On"}` tagged union) ⇒ **WS protocol + WASM bridge + saved games + localStorage** (`HostSetup.tsx:227` reads `remembered?.loopDetection`) |
| **A byte-identity golden** | `tests/integration/loop_shortcut.rs` `GOLDEN_ON` — pins the `On` arm's **exact `Vec<GameEvent>`** |

**And Rev 1's PREMISE is measurably wrong.** The object-growth bridge gates on **`.samples()`**, not on the mode match:

- `game/engine.rs:445-451`: `… && state.loop_detection.samples() && …` → `try_offer_object_growth_shortcut(state)` →
  sets `WaitingFor::LoopShortcut`.
- `types/game_state.rs:5975-5977`: `pub fn samples(self) -> bool { matches!(self, LoopDetectionMode::On | LoopDetectionMode::Interactive) }`

> ## ⇒ **`On` ALREADY OFFERS on the object-growth path — the canary's own path.** The rules defect is **confined to
> the DRAIN path** (`game/engine.rs:356-430`), whose `LoopDetectionMode::On` arm pushes `GameEvent::GameOver` directly
> (`:420-427`) — **an auto-win with NO offer and NO opponent response window ⇒ rules-wrong per CR 732.2a.**

**And Rev 1's PREMISE was measurably wrong too.** The object-growth bridge — **the canary's own path** — gates on
**`.samples()`**, not on the mode match (`game/engine.rs:445-451`; `types/game_state.rs:5975-5977`:
`matches!(self, On | Interactive)`) ⇒ **`On` ALREADY OFFERS on the object-growth path.** The auto-win is confined to the
**drain** path (`game/engine.rs:356-430`, `GameEvent::GameOver` pushed directly at `:420-427`).

**⇒ Any future attempt to delete or re-semantic `On` must first discharge: the `combo-verify` binary, the wire form
(WS + WASM + saved games + localStorage), the two toggles, the `?loop=` param, the 7 locale keys, and `GOLDEN_ON`
(which pins the `On` arm's exact `Vec<GameEvent>` and would break BY DESIGN).**

</details>

**P5 touches no Rust, no TypeScript, and no test. It adds one doc comment and files DEFERRED-1.**

---

### P6 — scan (6): the delayed-trigger blanket *(**IN SCOPE**, small — M2)*

`analysis/resource.rs:1582-1589`, measured:

```rust
if !state.delayed_triggers.is_empty() || !state.deferred_triggers.is_empty()
    || state.pending_trigger.is_some() || state.pending_trigger_order.is_some()
    || !state.epic_effects.is_empty() { return true; }
```

**Any real Commander board with ONE live delayed trigger** — *"at the beginning of the next end step, …"* — **dies
here.** This does **not** veto the canary (a clean board at the priority beat), which is why it is **P6, not P2**. But
the user's steer is about **real boards**, and once P2's walker exists this is a **cheap** descent: scan each store's
**ability body** with the **existing** `ability_definition_reads_sibling_mutable`, rather than blanket-rejecting on
**non-emptiness**. Veto on `sibling || projected`, exactly as P2-d.

> ## ⛔⛔ **DISAMBIGUATION — AN IMPLEMENTER WILL CONFLATE THESE. THEY ARE DIFFERENT MECHANISMS.**
>
> | | **scan (6)** — `resource.rs:1582` | **`GameState::PartialEq`'s `delayed_triggers` conjunct** |
> |---|---|---|
> | What it does | **VETOES** the detection if the store is **NON-EMPTY** | **COMPARES** the store between two frames to decide whether the board **RECURRED** |
> | Direction | a **firewall** (rejects) | a **cover** (equality) |
> | This plan | **P6 RELAXES it** (descend, don't blanket) | ## ⛔ **DO NOT TOUCH. DO NOT RELAX.** It is what stops us certifying a loop **whose growth axis dies at the next end step.** |

---

## 6. Mandatory architectural sections *(`/engine-planner` Step 4)*

**Pattern Coverage.** **This is a WALKER, not a card fix.** P2-a classifies **all 41** `ContinuousModification`
variants — every static/aura/anthem/equipment grant in the game. P2-b covers **every token-creating ability**; P2-c
**every mana ability**. P3's one line covers **every `Typed` target filter in the engine** — the single most common
filter node there is. **Card count: the entire enchantment/aura/anthem/token/mana surface, i.e. thousands.** The two
combos are **canaries, not goals** (§9).

**Building Blocks.** Compose from what exists — **no new analysis machinery**:
`ability_scan::scan_quantity_expr` · `scan_quantity_ref` · `scan_target_filter` · `scan_static_condition` ·
`ability_definition_reads_sibling_mutable` · `Axes` + `Axes::or` (`:137-143`) · `Axes::NONE`/`CONSERVATIVE`
(`:124-135`) · `triggers::trigger_definition_functions_in_zone` (`:1057`) ·
`functioning_abilities::static_functions_in_zone` (`:187`). **The one new helper — `scan_continuous_modification` — is
the walker the engine's own comment (`resource.rs:1452-55`) says is missing, and it has a shipped sibling to mirror:
`modification_grants_growing_cost_keyword` (`ability_scan.rs:4080`), already consumed per-modification at
`resource.rs:1641-1646`.**

**Logic Placement.** **All AST classification lives in `game/ability_scan.rs`** — the file that owns the `Axes` walk;
it is the only module that may know a variant's read surface. **`analysis/resource.rs` only CONSUMES `bool`s** — it
must never learn an enum's shape (today's `:1539` blanket is exactly that leak, and P2-d removes it). **Zone-of-function
lives in `game/triggers.rs` / `game/functioning_abilities.rs`** and is **called**, never mirrored (P4). **Frontend
(P5) is presentation-only** — it removes an *option*, computes nothing.

**Rust Idioms.** **Exhaustive `match`, NO `_` wildcard** in `scan_continuous_modification` — a future variant **must
fail to compile** until classified (the "structural completeness, not self-reported allowlists" rule). **Exhaustive
destructure, no `..` rest pattern** in the `Effect::Token` arm, mirroring `resolved_ability_axes`
(`ability_scan.rs:148-155`), so a future `Token` field fails to compile until classified. **Typed enums, not bools:**
the verdict is an `Axes` (3 axes), never a `bool` — that is precisely why P2-d can veto on `sibling || projected`.
`depth: u8` guards grant recursion (§4.1). ⚠️ **The precedent `modification_grants_growing_cost_keyword` uses
`_ => false`; we must NOT copy that** — fail-open is acceptable for its question, forbidden for ours.

**Nom Compliance.** ⛔ **N/A — NOT APPLICABLE. No file under `crates/engine/src/parser/` is touched by any phase.**
Every card in this plan **already parses correctly** (§3.2 ASTs measured from `data/card-data.json`). **This is a
pure analysis/firewall change. There is no parser work, and none should be invented.**

**Extension vs Creation.** **EXTENSION, on every axis.** The `Axes` walk exists; we add one arm-family to it. The
per-`ContinuousModification` classifier pattern exists (`ability_scan.rs:4080`, consumed at `resource.rs:1641-1646`);
we add its sibling for the read axes. The zone-of-function authorities exist (`triggers.rs:1057`,
`functioning_abilities.rs:187`) and are already composed exactly as P4 needs at `triggers.rs:434`. **No new pattern is
created anywhere in this plan.**

**Analogous Trace *(hard gate)*.** ⭐ **Traced `modification_grants_growing_cost_keyword` end-to-end** — the shipped
per-`ContinuousModification` classifier, which is P2-a's exact structural twin:
`types/ability.rs:19350` (`enum ContinuousModification`) → `game/ability_scan.rs:4080-4088`
(`pub(crate) fn modification_grants_growing_cost_keyword(m: &ContinuousModification) -> bool`, a per-variant `match`) →
consumed per-modification at `analysis/resource.rs:1641-1646`
(`for def in obj.static_definitions.iter_all() { if def.modifications.iter().any(scan::modification_grants_growing_cost_keyword) { return true; } }`)
→ gating the same cover family. **P2-a/P2-d replicate this exact shape for the read axes** — same file, same
consumption site pattern, same `.iter().any(…)` idiom. *(Second trace, for P4: `granted_keyword_triggers_in_zone`,
`game/triggers.rs:423-437`, which composes `synthesize_granted_keyword_triggers` with
`trigger_definition_functions_in_zone` at `:434` — the exemplar P4 mirrors.)*

**Variant Discoverability.** ⛔ **NO NEW ENUM VARIANT IS ADDED BY ANY PHASE.** `/add-engine-variant` **does not
apply**. *(The one new type is a `#[cfg(test)]`-only probe enum in P1, which never reaches production.)*

**Identity / Provenance Contract.** The growth axis is **not assumed — it is MEASURED from the drive's Δ between two
concretely observed iterations**: `derived_fodder_class(&s_n, &s_n1)` (`game/engine.rs:1720`), normalized through
`project_object_for_loop` (`:1721`) — the same projection the cover applies to its frames. **Binding time:** at
detection, from the **replayed clone** (`:1688-1696`), never from the live state. **Live vs snapshotted:**
snapshotted — `s_n`/`s_n1`/`s_n2` are `state.clone()`s driven under `SimulationProbeGuard`; the live state is never
mutated by detection. **Storage:** the `fodder` local, consumed by `loop_states_cover_modulo_fodder_growth`
(`:1732`). **Invalidation:** the offer is re-validated at consumption (proposer + winner, CR 800.4a,
`game/engine.rs:855`). **Multi-authority hostile fixture:** §7-C6 (two grant sources on one host).

---

## 7. Verification matrix — claim → **named test** → **revert-failing assertion**

⛔ **Production entry point.** The live path is `apply()` → `WaitingFor::LoopShortcut` → `GameAction::DeclareShortcut`.
**Helper-only tests are BARRED** (`/review-engine-plan` check 9). Every runtime test below drives `apply()` via
`GameRunner`, sets `runner.state_mut().loop_detection = LoopDetectionMode::Interactive` (the shape measured at
`tests/integration/loop_shortcut.rs:100`), and asserts on `WaitingFor`. **New tests go in
`crates/engine/tests/integration/` with a `mod` line in `tests/integration/main.rs`** (CLAUDE.md).

**New file: `crates/engine/tests/integration/loop_firewall_descent.rs`.** Unit-level `Axes` guards go in
`game/ability_scan.rs`'s existing `#[cfg(test)] mod tests`, beside `sibling_mutable_axis_discriminates` (`:5114`).

| # | Claim | Seam changed | Test (**named**) | ⭐ Revert-failing assertion — *delete the term → this FLIPS TO FAIL* |
|---|---|---|---|---|
| **C1** | ⭐ **THE CANARY GOES GREEN.** CR 732.2a's own worked board offers a shortcut. | P2+P3 (all three vetoes) | `loop_firewall_descent.rs::cr_732_2a_worked_example_offers_shortcut` — **verbatim Oracle** Presence of Gond + Intruder Alarm + vanilla creature, all battlefield; `Interactive`; drive via `apply()` | `assert!(matches!(runner.state().waiting_for, WaitingFor::LoopShortcut { .. }))`. **Revert ANY ONE of P2-b / P2-d / P3 ⇒ the corresponding veto (V2/V3/V1) re-fires ⇒ FAIL.** ⇒ **all three are individually load-bearing.** |
| **C2** | V3 alone: a fixed **anthem** does not veto. | `resource.rs:1539` | `ability_scan.rs::anthem_modification_reads_no_axis` | `assert!(!continuous_modification_reads_sibling_mutable(&AddPower{value:1}))`. **Revert P2-d ⇒ the blanket returns ⇒ FAIL.** |
| **C3** | ⭐ **NEGATIVE (V3 hostile):** a **dynamic** P/T modification **STILL VETOES**. | P2-a | `ability_scan.rs::dynamic_pt_modification_reads_sibling` | `assert!(continuous_modification_reads_sibling_mutable(&SetDynamicPower{ value: Ref(ObjectCount{Typed(Creature)}) }))`. **Delete the `scan_quantity_expr(value)` call from the `SetDynamic*` arm ⇒ FAIL.** **Reach-guard:** paired with C2 on the **same** fn — C2 proves the fn CAN return `false`, so C3's `true` is not vacuous. |
| **C4** | ⭐⭐ **THE DISCRIMINATING NEGATIVE — a REAL card, on the line the walker draws.** **Gaea's Cradle MUST STILL FAIL CLOSED.** | P2-c | `loop_firewall_descent.rs::gaeas_cradle_still_fails_closed` — the **real card** (`card-data.json`, "gaea's cradle", **PRESENT**), verbatim `{T}: Add {G} for each creature you control.` | `assert!(fire_time_conditions_read_growing_class(&state))` — via the production cover. ⭐ **REVERT-PROBE: delete the `sibling: true` at `ability_scan.rs:1606` (the `ObjectCount` literal) ⇒ Gaea's Cradle UN-REJECTS ⇒ THIS ASSERTION FLIPS TO FAIL.** ⇒ **NON-VACUOUS and DISCRIMINATING.** |
| **C4′** | ⭐ **PROOF THAT TODAY'S GUARD IS VACUOUS** *(the B4 finding, as a test)* | — | same test, run **on `main` before P2-c** | On `main`, the C4 revert-probe **does NOT flip** — `Effect::Mana{..} => CONSERVATIVE` (`:862`) **dominates** and Gaea's Cradle fails closed **regardless** of `:1606`. ⇒ **P2-c is what MAKES the negative meaningful.** Record this measurement in the PR body. |
| **C5** | **P3 does not un-reject the counting class.** | `ability_scan.rs:2420` | `ability_scan.rs::typed_names_but_object_count_counts` | `assert!(!ability_reads_sibling_mutable(&ability_with_target(Typed(Creature))))` **AND** `assert!(ability_reads_sibling_mutable(&ability_with_amount(ObjectCount{Typed(Creature)})))` — **the same `Typed` node under both**. **Revert `:1606` ⇒ the second FAILS.** ⇒ the two are independent, as §5-P3 claims. |
| **C6** | **HOSTILE / multi-authority:** **two** grant sources on **one** host. | P2-a depth guard | `loop_firewall_descent.rs::two_grants_one_host_still_offers` — Presence of Gond **+** a second aura granting a **fixed-token** ability to the **same** creature | Offer still raised (both grants are read-free). **Delete the `.iter().any(…)` in P2-d and hard-code `modifications[0]` ⇒ the second grant is unscanned ⇒ this passes for the WRONG reason** — so pair with C6′. |
| **C6′** | **HOSTILE (the veto direction):** one read-free grant **+** one **dynamic** grant on the same host. | P2-a | `loop_firewall_descent.rs::mixed_grants_one_dynamic_vetoes` | `assert!(!matches!(waiting_for, WaitingFor::LoopShortcut{..}))` — **the dynamic grant MUST veto even though a sibling modification is clean.** **Revert the `.any(…)` to `[0]` ⇒ FAIL.** ⇒ C6 + C6′ jointly pin `.any` over the **whole** vector. |
| **C7** | ⛔⛔ **THE SAFETY CLAIM (P2-d).** A **projected**-reading modification **STILL VETOES**. | `resource.rs:1539` | `ability_scan.rs::projected_modification_still_vetoes` **+** `loop_firewall_descent.rs::life_scaled_grant_does_not_offer` | `assert!(continuous_modification_reads_projected_resource(&SetDynamicPower{ value: Ref(LifeTotal) }))` **AND** the runtime test asserts **NO** `LoopShortcut`. ⭐ **REVERT-PROBE: drop the `|| …reads_projected_resource(m)` disjunct from P2-d ⇒ the life-scaled board OFFERS ⇒ FAIL.** ⇒ **proves the `sibling`-alone veto is a FALSE-CERTIFICATE hole.** |
| **C8** | **P3 does not perturb CR 603.3b** (the live second consumer). | `ability_scan.rs:2419` (**unchanged**) | re-authored `resource.rs::event_axis_unchanged_for_typed_sibling_axis_relaxed` (was `:3926`) | `assert!(ability_uses_event_context(&ability))` — **verbatim-preserved**. **Flip `:2419` `event` to `false` ⇒ FAIL** ⇒ pins the `triggers.rs:3893` `!event && !sibling` contract. |
| **C9** | **CR 400.2:** the verdict is **invariant under any hidden-zone content**. | P4 | `loop_firewall_descent.rs::verdict_invariant_under_library_content` | Build the C1 board twice — once with an empty library, once with **an arbitrary card whose trigger reads `ObjectCount` shuffled into a library**. `assert_eq!(offer_a, offer_b)` **and** both are `LoopShortcut`. **Revert P4's scan-(1) zone filter ⇒ the second board's library trigger vetoes ⇒ `assert_eq!` FAILS.** ⇒ non-vacuous **only** because the planted card **would** veto if scanned. |
| **C10** | **P5:** no path auto-wins **without an offer**; `Off` fully restores pre-feature behavior. | frontend only | existing `tests/integration/loop_shortcut.rs` (**`GOLDEN_ON` must stay GREEN** — P5 touches no engine code) + `client` type-check | `GOLDEN_ON` byte-identity holds ⇒ **proof P5 is engine-inert.** Frontend: the segmented control renders **exactly two** options. |

**Coverage-status impact:** ⛔ **NONE.** No parser change ⇒ no card's supported/unsupported status moves. *(The
CI card-data coverage-regression check must still be **run**, to prove exactly that.)*

---

## 8. Soundness — the rule that outranks everything above

> # **A coarse relation may REJECT, never ACCEPT.**
> **Too coarse ⇒ a false certificate ⇒ a real game ends wrongly. Too fine ⇒ a missed offer ⇒ safe.**
> *(Preserved VERBATIM from Rev 1 §6. It was right. It is the only thing that outranks the canary.)*

**Rev 1's risk TABLE, however, was wrong. Corrected:**

| Phase | Moves toward ACCEPT? | Real risk |
|---|---|---|
| **P1** (probe) | No | **NOT artifact-free.** Its conclusion is the sole input to a decision rule. Rev 1's stub-everything probe **could not answer its own question** — hence the per-limb rewrite + the explicit RED plan (§5-P1). |
| **P2** (descent) | ## ⭐ **YES — this is the one.** | ⛔ **HIGHEST RISK. REVIEW EVERY LINE TWICE.** The false-certificate hole lives at **P2-d** (`sibling` alone vs `sibling \|\| projected`). **C7 is the test that stands between this plan and a wrongly-ended game.** |
| **P3** (one line) | **YES** | **Lower than it looks** — bounded by the **measured** proof at `ability_scan.rs:1606` that the counting class carries its **own** literal (§5-P3). Blast radius: the live consumer at `triggers.rs:3893`, pinned **byte-unchanged** by C8. |
| **P4** (zones) | **ZERO on any real board.** It removes scans of abilities that **DO NOT FUNCTION** and **MAY NOT BE SEEN**. | Removing them removes **noise**, not **safety** — *but that argument holds only if the zone predicate is exactly right,* which is why P4 **forbids hand-rolling it** and blacklists `object_functions` **by name**. |
| **P5** (modes) | No — relaxes no rejection | **NOT neutral, and NOT "safe to not double-review."** Rev 1's delete-the-variant would have broken a shipped binary, the WS protocol, the WASM bridge (**silently → `Off`, no error**), saved games, localStorage, 2 UI toggles and 7 locale keys. **It was the phase most likely to break users.** The frontend-only fix (§5-P5) is what makes it genuinely low-risk. |
| **P6** (scan 6) | **YES** (relaxes a veto) | Same `sibling \|\| projected` discipline as P2-d. **Must not be conflated with `GameState::PartialEq`** (§5-P6). |

### ⛔ Do not "fix" these — they are already correct *(corrected per B7)*

- ⛔ **No non-determinism gate.** It exists **twice**: **static** — `game/engine.rs:1684` (`spell_ability_bears_randomness`); ⭐ **runtime** — **`game/engine.rs:1713`** (RNG **word-position** delta across the drive). **Rev 1 cited the static one twice and never cited the runtime backstop.** *(`ability_scan.rs:4407` `effect_is_randomness_bearing` is, per its own doc at `:4406`, **"the static, compile-time-exhaustive half"** — NOT the runtime gate.)*
- ⛔ **No REL / tournament-mode toggle.** §0.
- ⛔ **Do not relax `GameState::PartialEq`'s `delayed_triggers` conjunct.** *(≠ firewall scan (6). §5-P6.)*
- ⛔ **Do not DELETE the covers. Make them PRECISE.** A fail-closed default consumed as a **precise predicate** is the defect — not the fail-closed default itself.
- ⛔ **Do not add a `modifications` scan to the PROJECTED twin (`resource.rs:2152`).** It is a **separate, pre-existing** latent gap on the `:784`/`:831` ω-cover. **OUT OF SCOPE.** *(P2-d makes it **harmless** for the object/fodder covers by vetoing on `projected` too. File it; do not fix it here.)*
- ⛔ **DO NOT SWEEP THE 80 SITES UNIFORMLY** (§3.6). **The 51 blankets and the 29 literals get OPPOSITE treatment,** and 28 of the 29 literals are **load-bearing**.

---

## 9. Acceptance — class-level, not card-level *(preserved from Rev 1 §5 — the doctrine was right)*

> ## **The two combos are CANARIES, not GOALS.** A change that turns them green **without discharging a class
> property** is exactly the purpose-built patch this plan exists to prevent.

| Phase | Class property discharged | Pinned by |
|---|---|---|
| **P2** | A continuous modification vetoes **iff it READS** a mutable aggregate or a projected resource — **not** merely because it exists. **All 41 variants classified; a new one cannot compile unclassified.** | C2 · C3 · **C7** · C6/C6′ |
| **P3** | A filter that **NAMES** a type does not read the `sibling` axis; one that **COUNTS** a set does. | C5 · C8 |
| **P4** | The verdict is **invariant under ANY hidden-zone content** (CR 400.2). | C9 |
| **P5** | `Off` fully restores pre-feature behavior; **no player-selectable mode auto-wins without an offer** (CR 732.2a). | C10 |
| **P6** | A deferred ability body vetoes **iff it READS** the growing axis — not because the store is non-empty. | (mirrors C2/C7) |

---

## 10. ⚠️ Design decisions I own — **team-lead, overrule me here if you disagree**

- **D1 — P2-d vetoes on `sibling \|\| projected`, not `sibling` alone.** *(§5-P2-d.)* This is the **single most
  important line in the plan.** Measured basis: `resource.rs:1539` is the **only** `modifications.is_empty()` check in
  the file, and the projected twin (`:2152`) has **no `modifications` scan at all** (`:2199-2207` scans `condition`
  only) ⇒ the blanket is *incidentally* the only projected-axis protection the object/fodder covers have. **If you
  think I have over-fitted, the cheap disproof is C7's revert-probe.** I would rather be over-conservative here than
  ship a false certificate.
- **D2 — Grant-realization depth is capped at 1** (`depth > 0 ⇒ CONSERVATIVE`). Justified by the user's "no nested
  loops" steer (§4.1). **This is a deliberate, stated incompleteness**, not an oversight. If you want depth-2, say so
  — but it costs the "decidable and bounded" warrant.
- **D3 — ~~P5 keeps `LoopDetectionMode::On` and fixes the FRONTEND instead.~~ ⛔ SETTLED BY USER DIRECTIVE — NOT MY
  CALL, AND NOT OPEN.** *(2026-07-14: **"Keep off/on/interactive for the combo detector for now. It helps us separate
  concerns."**)* My draft proposed removing `On` from the two player-facing toggles. **The user has kept all three
  modes.** P5 is now **doc-comment-only**; the player-selectability question is **DEFERRED-1** (§5-P5), a standalone
  follow-up. **`On` is also a working tool for THIS plan** — it exercises the detector's *classification* in isolation
  from the *offer* machinery, which makes P1's per-limb probe and P2/P3's canary cheaper to write. **Separating those
  concerns is the point. Do not re-open this inside this plan.**
- **D4 — P4 (CR 400.2) ships even though it unblocks nothing.** It is a **real information leak**. But it is
  **honestly labelled**, and if you want it split into its own PR to keep the reachability fix reviewable in
  isolation, **that is a good idea and I would take it.**
- **D5 — P6 is IN scope.** The canary does not need it, but a real Commander board does. **It is small once P2's
  walker exists.** If you would rather ship P2+P3 first and land P6 as a follow-up, **that is defensible** — say so.
- **D6 — P1's RED plan says "STOP, this plan is void" on a `None` result.** That is deliberate. **I would rather kill
  this plan on a measurement than patch around a wrong diagnosis** — which is exactly how this workstream produced 19
  errors.

---

## 11. ⚠️ UNVERIFIED — things I did **NOT** discharge

**Everything else in this document was measured by the author against `main` @ `efc76ca1b`. These were not:**

- **U1 — Replacement zone-of-function (P4, scan (3)).** I found **no** zone-of-function authority for
  `ReplacementDefinition`. `active_replacements`' own doc says its scan is *"future-proofed for per-replacement zones
  but no current caller needs it."* **I did not determine whether `ReplacementDefinition` carries a zone field at
  all.** ⇒ **The implementer must measure this before writing P4's scan-(3) filter.** The plan states the fallback
  (battlefield + graveyard, CR 113.6b) and forbids hand-rolling a broader list.
- **U2 — ~~The exact i18n locale-key set for P5's removed toggle option.~~ ⇒ MOOT.** The user directive keeps all three
  modes and **removes no toggle option**, so **no locale key changes** ⇒ the i18n parity CI gate
  (`resources.test.ts`, all 7 locales) **is not engaged by this plan at all.** *(Retained only as a constraint on
  DEFERRED-1, which WOULD engage it.)*
- **U3 — Whether `ContinuousModification` has variants beyond `:19599`.** I enumerated **41** variants in
  `types/ability.rs:19350`–`:19599`. **I did not read past `:19599` to confirm the enum closes there.** ⇒ **The
  implementer must confirm the full variant list before writing the exhaustive match.** *(The no-wildcard match makes
  this **fail-safe**: a missed variant is a **compile error**, not a silent hole.)*
- **U4 — The precise `#[cfg]` under which `analysis/corpus.rs:2039` compiles into `combo-verify`.** I measured the
  `[[bin]]` (`Cargo.toml:61-64`) and the `LoopDetectionMode::On` assignment (`corpus.rs:2039`). **I did not read the
  enclosing `#[cfg(...)]` attribute myself** — the review reports `#[cfg(any(test, feature = "combo-verify"))]`.
  **P5 does not depend on this** (it touches no Rust), but the "deleting `On` breaks a shipped binary" **argument**
  does.
- **U5 — I did not run ANY test, build, or `cargo` command.** Per the brief, `main` is **READ-ONLY** and Tilt holds the
  target lock. **Every claim in §7 is a PREDICTION to be discharged by the implementer**, not a measurement.
- **U6 — `GOLDEN_ON`'s exact line range** in `tests/integration/loop_shortcut.rs`. I measured its **existence** and
  **purpose** (module doc, `:8-13`). I did not locate the constant itself.

---

## 12. Sources

**`docs/MagicCompRules.txt`** — **CR 104.2a** `:330` · **104.4b** `:366` · **113.6** `:771` · **400.2** `:1935` ·
**704.5a** `:5492` · **732.1b** `:6366` · **732.1c** `:6368` · **732.2a + Example** `:6372`/`:6373` · **732.4** `:6383` ·
**800.4a** `:6408`. *(All ten resolve verbatim — independently confirmed by the reviewer AND by team-lead's
re-measurement against `main` @ `efc76ca1b`. **No NEW CR number is introduced by this revision**, so no new
grep-verification was required; CR 613.1 and CR 603.3b are cited in the plan only where the **existing code** already
carries those annotations.)*

**Scryfall** *(Step 0 hard gate, fetched 2026-07-14)* — `api.scryfall.com/cards/named?exact=` for Presence of Gond,
Intruder Alarm, Gaea's Cradle. **All three match `data/card-data.json` verbatim** (§3.1).

**Supersedes:** `COMBO-DETECTOR-PLAN.md` (Rev 1). **STALE — do not read for code OR rules facts:**
`REAL-BOARD-RCA-AND-PLAN.md`, `PLANNER-BRIEF.md`, `REVIEWER-MANDATE.md`, `ADVERSARY-MANDATE.md`,
`SESSION-HANDOFF.md`, `LOOP-SHORTCUT-SPEC-AND-STATE.md`.
