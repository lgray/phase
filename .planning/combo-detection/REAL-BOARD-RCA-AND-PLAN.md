# Combo detector: root-cause analysis + remediation plan
### Making loop detection work on real decks and real board states

**Date:** 2026-07-13
**Status:** Investigation complete, implementation NOT started. For maintainer review.
**Evidence:** All claims below were *measured* by driving the user's exported live game state
through the real engine, not inferred. Reproduction harness + method in §2.

---

## 1. Executive summary

The combo detector **cannot fire in any real game of Magic.** Two independent live combos on a
real 4-player Commander board were verified undetectable, and the reasons generalize far beyond
those two cards.

The engine's *arming* logic is correct. The detector then declines inside a chain of
**fail-closed firewalls** that are individually defensible but collectively fatal:

| Measured trip | What killed detection |
|---|---|
| `Solemn Simulacrum (Library)` | A card **in the library** — never drawn, uncastable — permanently disables detection |
| `Forest` | A **basic land** disables detection (`Effect::Mana => Axes::CONSERVATIVE`) |
| `Freed from the Real` | Any **aura/utility permanent** referencing a creature disables detection |
| replacement scan | All-zones replacement scan disables detection |
| `!def.modifications.is_empty()` | Any **anthem/aura/equipment** disables detection |

The reason CI is green is that the acceptance fixture (`sprout_swarm_scenario`,
`loop_shortcut.rs:2536`) builds a board that **cannot exist in a real game**: no lands, empty
library, no auras, and a stripped-down Witherbloom oracle. Every one of the defects above is
invisible to that fixture and fatal the moment real card data is used.

**Two headline findings beyond the false negatives:**

1. **The all-zones scans are a rules violation, not just a bug.** The firewall iterates
   `state.objects.values()` across **library and hand** — which **CR 400.2** defines as *hidden
   zones*. The detector's verdict is therefore a function of information no player may act on
   (including opponents' libraries and hands). This must be removed on **rules** grounds,
   independent of the false-negative problem.

2. **The detector is asking the wrong question.** It currently tries to prove *"no ability
   anywhere in the game could ever observe this growth."* **CR 732.2a** asks only whether the
   sequence *"may be legally taken **based on the current game state and the predictable results**
   of the sequence of choices"*, excluding *"conditional actions, where the outcome of a game event
   determines the next action."* Interaction is **not** the detector's job — **CR 732.2b** gives
   each other player, in turn order, the right to *accept or shorten*. The loop must be infinite
   **from the proposer's perspective**, and is then **passed around for response**.

---

## 2. Reproduction

- **Fixture:** debug-panel export of a live 4-player Commander game
  (`Export Game State` → zip; JSON is wrapped `{gameState, waitingFor, legalActions, turnCheckpoints}`).
- **Harness:** `crates/engine/tests/integration/repro_user_combo.rs` (uncommitted) —
  `serde_json` → `GameState` → `engine::game::layers::flush_layers` → `GameRunner::from_state`,
  then drives the identical cast the passing synthetic test drives.
- **Method:** instrument each `return None` in `try_offer_object_growth_shortcut` and each branch
  of `fire_time_conditions_read_growing_class`, writing tags out-of-band to a file (never a
  panicking test — a red suite is a multi-agent hazard). Re-run, read the tag, fix, repeat.
- **Note:** a bare `GameState` snapshot is not enough. Arming (`last_recast_context`) happens
  *during a cast*, so the repro must **drive a real cast** from the snapshot.

**Board:** Witherbloom, the Balancer + 4 untapped green Saproling **tokens** + Kilo, Apogee Mind
(enchanted by Freed from the Real) + Relic of Legends + Pentad Prism (1 charge) + Forests/Islands.
Sprout Swarm in hand. `loop_detection = Interactive`, `Priority{P0}`, own turn, empty stack.

**Measured after driving the cast:**
```
last_recast_context = Some(RecastContext{ card_id:415, controller:0, from_zone:Hand,
                                          uses_buyback:Used, convoke:Some(Convoke) })   ← arming CORRECT
waiting_for         = Priority{0}                                                        ← NO OFFER
saprolings          = 4 → 5                                                              ← the cast worked
```
Every cheap gate at `engine.rs:445` is green. The decline is inside
`loop_states_cover_modulo_fodder_growth` → `fire_time_conditions_read_growing_class`.

---

## 3. The core design inversion

The detector conflates three separate questions and answers all of them with one static,
whole-universe, fail-closed scan:

| Question | Who should answer it | Scope |
|---|---|---|
| Is the cycle **repeatable and predictable** from the current board? | the detector | **present board state**, actually-executed cycle |
| Does the cycle **actually terminate / change behavior** as it repeats? | the detector, **empirically** (it already drives the loop on a clone!) | the driven frames |
| Can somebody **break** the loop? | **CR 732.2b response window** (accept / shorten) + `no_living_player_has_meaningful_priority_action` | not the cover's job |

Because the third question leaked into the cover, the cover reaches for *every ability on every
object in every zone*, and fail-closes on anything it cannot classify. That is why a library card
and a basic Forest both veto a real infinite loop.

**The detector already drives two real iterations on a clone.** That empirical drive is strictly
stronger evidence than any static "could this ability read |G|?" scan. The static firewall should
shrink to the narrow residue the drive genuinely cannot see (see §6.3).

### 3.1 The catch-22 (why this family can never certify today)

In **both** combos, the ability that *drives* the loop is itself a reader of the growing axis:

- **Sprout Swarm:** Witherbloom's *affinity for creatures* reads the **creature count** — and the
  growing class **is creatures**. The card that makes the loop free is the card that disqualifies it.
- **Pentad Prism:** *proliferate* reads *"permanents with counters"* — and the growing axis **is
  counters**. Same shape.

A firewall phrased as *"reject if any live ability reads the growing class"* is therefore
**structurally incompatible with the entire class of self-referential engines**, which is most real
combos. This is not a tuning problem; the predicate is wrong.

The distinction that actually matters is **monotone-benign vs. behavior-changing**:

- *monotone / saturating* reads — affinity cost reduction (floored at `{0}`), proliferate (only
  ever adds) → **safe**; more of the growing class cannot break the loop.
- *threshold / comparison* reads — "if you control seven or more creatures…", "whenever you control
  exactly N…" → **dangerous**; behavior changes at a cliff a 2-iteration drive cannot see.
- *scaling* reads — an anthem or "deals damage equal to the number of creatures" → changes the
  per-cycle delta, but often still leaves the loop unbounded.

---

## 4. Combo A — Witherbloom, the Balancer + Sprout Swarm (object growth)

**Mechanics (verified vs Scryfall).** Sprout Swarm `{1}{G}`, Convoke, Buyback `{3}`, "Create a 1/1
green Saproling creature token." Witherbloom grants *"Instant and sorcery spells you cast have
affinity for creatures."* With ≥4 creatures the `{4}` generic (base `{1}` + buyback `{3}`) reduces
to `{0}`; convoke taps one green creature for the `{G}`. Each cycle: tap 1, create 1 (untapped),
buyback returns the card ⇒ **+1 creature, zero mana, forever.** Genuinely infinite.

**Path:** object-growth / 4d-ii driven detector. Armed at `casting_costs.rs:6785`
(`samples() && additional_cost_paid && has_buyback && is_token_creating`), offered at
`engine.rs:445`, decided by `try_offer_object_growth_shortcut` (`engine.rs:1648`).

### Defects (each independently fatal)

**A1 — Observer scan is not zone-scoped. (CR 113.6 + CR 400.2)**
`fire_time_conditions_read_growing_class` gate (1) iterates `state.objects.values()` over **every
zone** and calls `active_trigger_definitions`, which is **not zone-scoped** (its only filter is a
Command-zone special case). Gate (4) has the same unscoped shape. Gate (2) *does* scope correctly —
the inconsistency is the tell.
*Measured trip:* `Solemn Simulacrum (Library)`.
CR 113.6: abilities function only on the battlefield (modulo 113.6a–d). CR 400.2: library and hand
are **hidden zones**.
A correctly-scoped sibling authority already exists: `battlefield_active_triggers`.

**A2 — `Effect::Mana { .. } => Axes::CONSERVATIVE`** (`ability_scan.rs:852`).
Every mana ability — including a basic `Forest` (`{T}: Add {G}`) — is classified as reading the
growing class. The module doc concedes the walk *"does not descend into `ManaProduction`"*.
⇒ an object-growth loop could only certify on a board with **zero mana sources**.
*Measured trip:* `Forest`.
**Fix drafted and proven** (committed on `debug/combo-generator`): a `scan_mana_production` walker.
`Fixed`/`Mixed` carry only static color lists ⇒ `Axes::NONE`.

> ⚠️ **SOUNDNESS-CRITICAL — do not "simplify" this walker.** An earlier draft of this document
> claimed the dynamic case is caught by the ability-level `repeat_for`. **That claim was WRONG**
> and is corrected here, because acting on it would reintroduce a game-ending false positive.
>
> **Measured** (`data/card-data.json`): *"{T}: Add {G} for each creature you control"* (Gaea's
> Cradle, Circle of Dreams Druid, Itlimoc) does **not** use `repeat_for` at all. It parses as:
> ```json
> {"type":"Mana","produced":{"type":"AnyOneColor",
>   "count":{"type":"Ref","qty":{"type":"ObjectCount",
>            "filter":{"type_filters":["Creature"],"controller":"You"}}}}}
> ```
> The dynamic board read lives **inside `ManaProduction` as `count: QuantityExpr`**. It is caught
> **only** because the walker routes `AnyOneColor { count, .. } => scan_quantity_expr(count)`, and
> `scan_quantity_ref` maps `QuantityRef::ObjectCount { .. } => Axes { sibling: true, .. }`
> (verified in `ability_scan.rs`).
>
> ⇒ **The per-variant `count` / `TargetFilter` scanning is the entire safety property.** Dropping
> it — e.g. collapsing the count-bearing variants to `Axes::NONE` on the theory that `repeat_for`
> covers them — would let the engine falsely certify **unbounded mana**. Guard this with the
> regression test in §6.0.

This walker is the template for every other blanket fail-close in the file, and the lesson
generalizes: **verify where each variant's dynamic payload actually lives before declaring it
static.**

**A3 — Gate (2) scans ACTIVATED ability bodies.**
An activated ability observes nothing unless a player *activates* it, and the driven cycle never
does. Loop *breakability* is `no_living_player_has_meaningful_priority_action`'s job (and
ultimately CR 732.2b's).
*Measured trip:* `Freed from the Real` (`{U}: Untap enchanted creature` ⇒ `SetTapState` on
`Typed{type_filters:[Creature], properties:[EnchantedBy]}`).
Note this filter is **pinned to a single attached object** and provably cannot scale with |G| —
see A5.

**A4 — Replacement scan is all-zones** (`active_replacements` is deliberately not
battlefield-scoped). *Measured trip:* `3-replacement-execute`.
Zone-of-function is genuinely subtler for replacements (CR 614.12, madness, etc.), so this one
needs a real zone-of-function predicate, not a blanket battlefield filter.

**A5 — The `sibling` axis is semantically too coarse.**
It currently means *"references something that could be a creature"*, not *"observes or scales
with |G|"*. A `Single`-scope effect pinned to a specific object (`EnchantedBy`, attachment,
`SelfRef`) **cannot** scale with the growing pile and must not trip the firewall.

**A6 — Gate (4) blanket `if !def.modifications.is_empty() { return true }`.**
Rejects **any** static continuous modification anywhere. Its own comment concedes *"default-
CONSERVATIVE: no `scan_continuous_modification` walker exists."* ⇒ any anthem/aura/equipment kills
detection. (Not reached in the cascade yet; it will trip once A1/A3/A4 are scoped.)

**A7 — Detection depends on the payment choice (transient intolerance).**
`select_convoke_taps` (`mana_payment.rs:394`) orders candidates by **lowest ObjectId**, so it taps
**Witherbloom (402)** — a green nontoken — before the Saproling tokens (413+). The first driven
cycle therefore taps a **non-fodder** permanent, so (a) the board changes by a nontoken flipping to
tapped, failing pair-A's cover, and (b) `derived_fodder_class` learns the fodder class as
**untapped** (no fodder was tapped that cycle) while *steady-state* growth is a **tapped** token —
so pair B mismatches too. Both pairs fail.

> **Maintainer ruling already given (and it is correct): do not "fix" this by biasing the
> selector.** Tapping nontoken green creatures / Forests is a **bounded transient** — bounded by
> the number of green sources controlled — i.e. a finite prefix, not unbounded behavior. Detection
> must not depend on *which* mana source is chosen. **The detector must tolerate a bounded
> transient** by driving past it to the steady-state cycle. A token-preference patch was written,
> proven, and then **reverted** for this reason.

---

## 5. Combo B — Kilo, Apogee Mind + Freed from the Real + Relic of Legends → Pentad Prism (counter growth)

**Mechanics (verified vs Scryfall).**
- Relic of Legends: *"Tap an untapped legendary creature you control: Add one mana of any color."*
  (Relic does **not** tap itself ⇒ reusable.) Tap **Kilo** (legendary) ⇒ add `{U}`.
- Kilo, Apogee Mind: *"Whenever Kilo becomes tapped, **proliferate**."*
- Freed from the Real (aura on Kilo): *"{U}: Untap enchanted creature."* Spend the `{U}` ⇒ Kilo untaps.

Per cycle: **mana-neutral** (+`{U}` then −`{U}`), Kilo returns to untapped, and **proliferate fires
once**, adding a charge counter to **Pentad Prism** (which already has one). Pentad Prism:
*"Remove a charge counter: Add one mana of any color."*
⇒ **unbounded counters ⇒ unbounded mana.** The growth axis is **counters**, not objects.

### Defects — two are *structural*, not conservatism

**B1 — The ring is wiped by every deliberate action ⇒ player-driven loops are undetectable.**
`engine.rs:3081`:
```rust
if !matches!(action, GameAction::PassPriority | GameAction::OrderTriggers { .. }) {
    state.loop_detect_ring.clear();   // "cast/activate/play-land is a deliberate break"
}
```
This loop is driven by **activating abilities**, so the ring is cleared on **every activation** and
can never accumulate the ≥2 samples detection needs. The ring path can therefore only ever see
**automatic** cascades (mandatory trigger loops). **A player-driven activated-ability loop is
architecturally undetectable** — the single largest class of real Magic combos (untap engines,
mana engines, Splinter Twin, Kiki-Jiki, Freed from the Real, Pili-Pala, …).
There is **no driven detector for activated-ability cycles**: the only driven detector,
`try_offer_object_growth_shortcut`, is hard-wired to buyback+token recasts via `last_recast_context`.

**B2 — No counter-growth cover.**
`loop_states_cover_modulo_growth` (`resource.rs:784`) requires `object_resource_axes_match` —
*strict object damage/counter equality*. This loop **grows counters every cycle**, so the cover
rejects it even if the ring somehow accumulated. The engine has covers for **constant-depth**,
**object growth**, and **fodder growth**, but **none for a monotone-increasing counter axis**. The
growth-axis taxonomy is incomplete.

**B3 — Shared with Combo A:** the same `fire_time_conditions_read_growing_class` firewall gates the
object-growth cover (`resource.rs:963-980`) and the fodder cover (`resource.rs:1126-1131`), and the
same all-zones `cost_surface_references_growing_class` scan applies. Even after B1/B2, this board
(Forests, Freed from the Real, a real library) trips A1–A4 identically.

**B4 — Shared catch-22:** proliferate reads *"permanents with counters"* — the growing axis itself
(see §3.1).

### Shared-cause summary

| Cause | Combo A | Combo B |
|---|---|---|
| All-zones / hidden-zone observer scan (A1) | ✅ | ✅ |
| Blanket `Effect::Mana` conservatism (A2) | ✅ | ✅ |
| Activated-ability bodies scanned (A3) | ✅ | ✅ |
| Blanket static-modification reject (A6) | ✅ | ✅ |
| `sibling` axis too coarse (A5) | ✅ | ✅ |
| Loop engine reads its own growing axis (§3.1) | ✅ affinity | ✅ proliferate |
| Transient intolerance (A7) | ✅ | likely |
| Ring wiped by deliberate actions (B1) | n/a (driven path) | ✅ **blocker** |
| No counter-growth cover (B2) | n/a | ✅ **blocker** |

Fixing §6.1–§6.3 unblocks **A** and removes B3/B4. **B additionally requires §6.4 and §6.5.**

---

## 5.5 TARGET ARCHITECTURE — resource-flow certificate (monotone VAS + LP), not state-recurrence

**This is the maintainer's design direction and it supersedes large parts of §6.** The current
detector asks *"did the board recur?"* and then fail-closes on anything that might have observed the
growth. The right question is a **resource-flow** one, and it is a small linear program.

### 5.5.1 The model

Take the proposer's **present board** and enumerate the repeatable **transitions** available to them:
each activated ability, each mana ability, each castable self-returning spell. Every transition `t`
has an effect vector `Δₜ` over resource axes — mana by color, untapped permanents by class, counters
by kind, tokens, life, cards-in-zone. (`ResourceVector` already computes exactly these deltas.)

The certificate is a **pair `(p, x)`** — a finite **prefix** `p ≥ 0` and a repeating **cycle**
`x ≥ 0` — such that:

- **Sustainability:** `Σ xₜ·Δₜ ≥ 0` on every *consumable* axis — the cycle net-drains nothing.
- **Progress:** `Σ xₜ·Δₜ > 0` on at least one *growth* axis (tokens, counters, damage, …).
- **Feasibility:** starting from the current marking `m₀`, the prefix `p` reaches a marking `m` from
  which `x` is executable in some order with no axis dipping negative mid-cycle (the scheduling half).

This is precisely a **Petri-net T-invariant** (`x`) reached from an initial marking (via `p`).
Rational solutions scale to integers, so plain **LP** settles existence in polynomial time;
minimizing `Σ pₜ + Σ xₜ` yields **the simplest combo**, which is exactly the sequence to propose as
the CR 732.2a shortcut.

> **Why the prefix is first-class, not a wart.** CR 732.2a explicitly sanctions this shape: *"This
> sequence may be a **non-repetitive series of choices**, a loop that repeats a specified number of
> times, multiple loops, or nested loops."* A loop that needs a warm-up — or a payment transient
> before it settles — is a **prefix**, not a defect. This is where the old §6.4 ("tolerate a bounded
> transient") is absorbed: the transient becomes `p`, computed analytically, instead of being
> chased by driving extra iterations and hoping the horizon was long enough.

> **LP alone is NOT a proof.** A T-invariant proves resource-sustainability, not executability from
> the current marking. Resolution: **LP proposes, the existing clone-drive verifies.** Driving
> `p` then `x` on a clone and observing `m < m'` is a Karp–Miller unboundedness witness, and the
> drive already exists (`drive_recast_iteration`).
>
> ⚠️ **IMPLEMENTATION LANDMINE — change what "verified" MEANS.** The drive must verify the **net
> resource delta**, *not* board-state equality. If the LP is implemented while the existing
> frame-equality cover (`loop_states_cover_modulo_*`) is left in place as the verifier, **the
> Witherbloom bug returns verbatim**: the drive still resolves its own convoke payment via
> `select_convoke_taps`, still taps the engine piece, and the concrete frames still fail to match —
> even though the loop is provably sustainable. Board-state equality is precisely the model §5.5
> exists to replace; retaining it as the check would re-import every defect in §4.

### 5.5.2 Why this dissolves the defects rather than patching them

- **A7 (transient / which source pays) stops existing.** The LP consumes *"an untapped green
  creature"*, not `Witherbloom` or `Saproling#413`. Convoke consumes one untapped green creature; the
  created token produces one ⇒ **net 0**. The loop certifies regardless of which permanent the
  payment engine taps. *The finite choice genuinely does not matter.*
- **A2 (mana conservatism) stops existing.** A Forest is a *transition* (`tap ⇒ +{G}`), not an
  observer. The LP only asks whether the cycle is mana-**non-negative**. Arbitrary mana state cannot
  veto a closed loop.
- **A1/A3/A4/A6 (the observer firewall) largely dissolve.** Only abilities that *actually fire in the
  cycle* contribute transitions. A library card contributes none — so the **CR 400.2 hidden-zone
  violation disappears structurally** instead of being patched, and an unactivated activated ability
  is simply an unused transition.
- **The §3.1 catch-22 dissolves.** Affinity and proliferate reading the growing axis is fine: they
  are transitions whose `Δ` depends on the marking, and both are **monotone-benign**. See 5.5.3.

### 5.5.3 The decidability certificate (the Turing-completeness guard) — combinator walk

A VAS is decidable. What buys **Turing-completeness is exactly zero-testing** (inhibitor arcs).
Magic as a whole is TC, so no complete procedure exists in general — but that is the wrong question.
The right question is: **does THIS board's transition system stay inside the decidable fragment?**
That is a *syntactic* property of the parsed abilities, and phase.rs's typed combinator AST is built
to answer it. Classify each transition by an AST walk (the same walker family as
`scan_mana_production`):

| Class | AST signature | Board examples | Verdict |
|---|---|---|---|
| **Constant Δ** | no `QuantityRef` reading a growth axis; no marking-dependent condition | Relic of Legends, Freed from the Real, a Forest | pure VAS ⇒ **LP exact** |
| **Monotone Δ** | `Δ` depends on a growth axis but is *non-increasing in cost* / *non-decreasing in production* | **Affinity** (cost only FALLS as creatures grow); **proliferate** (only ever ADDS) | **admit** — feasible now ⇒ feasible forever (monotonicity lemma) |
| **Non-monotone** | a `Comparator` against a growth-axis `QuantityRef`: threshold cliffs, and **zero-tests** | *"if you control **seven or more** creatures…"*, *"if you control **no** creatures…"* | **REJECT** — this is exactly where TC hides |
| **Non-deterministic** | randomness / conditional actions | coin flip, die roll, random selection | **REJECT** — CR 732.2a excludes these *by name* |

The monotonicity lemma is what makes the two live combos certifiable: **affinity** only ever makes
the recast *cheaper* as the creature count grows, and **proliferate** only ever *adds* counters — so
a cycle feasible at the current marking is feasible at every larger marking. That is a one-line
argument, not a firewall.

**This replaces the `sibling: bool` axis.** `sibling` currently conflates "references a creature"
with "could break the loop". The correct type is a *fragment classification*, e.g.
`enum FragmentClass { Constant, Monotone, Threshold, ZeroTest, NonDeterministic }`, carried on the
existing `Axes` walk. Per CLAUDE.md ("parameterize, don't proliferate") this is one parameterized
axis, not a new boolean per hazard.

**Cost:** the scan is over battlefield permanents + the proposer's OWN castable spells — a handful of
ability defs on a small board. Cheap, and it never reads a hidden zone.

**Expected hit rate:** the known TC constructions need very specific card sets (Rotlung Reanimator /
Artificial Evolution / Illusory Gains …). Real boards land in Constant/Monotone essentially always,
so the guard costs ~no false negatives while making the LP answer **sound and complete for the board
it certifies**, and declining loudly (not silently) when it cannot.

### 5.5.4 Relationship to §6

§6.1–§6.3 remain worth doing as a **tactical unblock** (they are small, and they make the CURRENT
detector work on real boards). But they are not the destination: §6.4 (transient tolerance) and much
of §6.5 **fall out for free** under 5.5 and should not be built as designed. Sequencing guidance is
in §6.7.

---

## 6. Plan

Ordered by dependency. Each phase is independently shippable and independently testable.
Recommend running through `/engine-implementer` (plan → review-plan → implement → review-impl).

### 6.0 — Testing mandate (do this FIRST; it is the reason all of this shipped)

**Combo-detector corpus/acceptance tests MUST run against real card data, a real library, and a
realistic mana base.** The current fixture proves only that the detector works on a board that
cannot exist.

- Add a `GameScenario` builder that loads **real Oracle text from `card-data.json`** (not
  hand-written stub oracles) and populates a **real library** and a **real mana base**.
- Port `object_growth_51st_sprout_swarm_covers_and_offers` onto that builder. **It must fail
  today** — that is the non-vacuity proof for every fix below.
- Add the two live boards in this document as regression fixtures (a trimmed, committed JSON
  export is fine; the harness in §2 already loads that shape).
- **Gate:** a combo-detector acceptance test that contains zero lands, an empty library, or a
  stub oracle should be rejected in review.

### 6.1 — Zone-scope every observer scan (rules-critical; do before anything else)

**This is a CR 400.2 hidden-information fix, not an optimization.** Ship it on its own.

- The firewall must read **only zones whose contents the game may act on**: battlefield, stack,
  and public zones as applicable — **never library or hand** (CR 400.2), and never opponents'
  hidden zones under any circumstances.
- Gate (1): replace `for obj in state.objects.values() { active_trigger_definitions(..) }` with the
  existing correctly-scoped `battlefield_active_triggers(state)`.
- Gate (4): apply the same scoping to `static_definitions`.
- Gate (A4)/`cost_surface_references_growing_class`: introduce a real **zone-of-function**
  predicate (CR 113.6, incl. 113.6a CDAs / 113.6b–c zone-stating / 113.6d cost-modifiers on the
  stack) rather than a blanket battlefield filter, so genuinely off-battlefield-functioning
  abilities are still honored.
- **Add a permanent guard test:** a loop that certifies must *still* certify after an arbitrary
  card is added to any player's library or hand. A detector verdict that changes when a hidden zone
  changes is a rules violation by construction. This test would have caught Solemn Simulacrum.

### 6.2 — Replace blanket fail-closes with real walkers

Pattern is established by the proven `scan_mana_production` fix (§4/A2):

- `Effect::Mana` ⇒ `scan_mana_production` (**already written & proven — in the tree; needs review**).
- `ContinuousModification` ⇒ write `scan_continuous_modification`; retire gate (4)'s
  `!def.modifications.is_empty()` blanket reject (A6).
- Audit `ability_scan.rs` for every remaining `=> Axes::CONSERVATIVE` on a variant that has a
  walkable payload, and descend instead. Keep fail-closed **only** where the payload is genuinely
  unclassifiable.

### 6.3 — Re-found the `sibling` axis on "observes or scales with |G|"

Redefine the predicate the firewall actually needs (this subsumes A3 and A5):

1. **Only abilities that fire without a player choosing** can perturb a driven cycle: triggers that
   actually trigger, replacements that actually apply, active statics. **Exclude activated
   abilities** — nobody activates them inside the cycle (A3). Breakability stays with CR 732.2b +
   `no_living_player_has_meaningful_priority_action`.
2. **Pinned-single-object filters do not scale.** `Single` scope + `EnchantedBy` / attachment /
   `SelfRef` ⇒ **cannot** scale with |G| ⇒ `Axes::NONE` (A5).
3. **Classify by effect on the loop, not by mere reference** (§3.1):
   - *monotone / saturating* (cost reduction floored at `{0}`; proliferate only adds) ⇒ **safe**;
   - *threshold / comparison* against the growing axis ⇒ **reject** (a cliff the drive can't see);
   - *scaling* ⇒ allowed only if the empirical per-cycle delta is stable (see 6.3.4).
4. **Prefer empirical over static.** The detector already drives the cycle. Require the **per-cycle
   delta to be stable across consecutive steady-state cycles**; keep the static scan only for what
   the drive provably cannot observe — chiefly **thresholds** that lie beyond the driven horizon.
   This is what makes the firewall scale to arbitrary boards instead of needing a new special case
   per card.

### 6.4 — Tolerate a bounded transient (A7)

Do **not** bias the payment selector. Instead:

- Drive until the cycle **stabilizes**: keep driving while consecutive frame-pairs disagree, up to a
  bound derived from the finite resources that can produce a transient (e.g. untapped nontoken
  creatures + untapped mana sources — the maintainer's own bound). Then derive the fodder/growth
  class from a **steady-state pair** and check the cover on **consecutive steady pairs**.
- This makes detection independent of *which* legal payment the engine picks — the correct
  invariant, and it generalizes past convoke to every cost with a free choice.
- Keep `select_convoke_taps` deterministic (replay reproducibility) but stop treating its choice as
  semantically load-bearing.

### 6.5 — Close the two structural gaps for player-driven loops (Combo B)

**6.5a — A driven detector for activated-ability cycles.**
Generalize the 4d-ii pattern beyond buyback+token recasts. `last_recast_context` is a *routing
signal* saying "a repeatable player-driven cycle may have just closed." Introduce the analogous
signal for an **activated-ability cycle** (a repeated activation returning the board to a prior
state modulo a growth axis), then reuse the existing drive-on-a-clone machinery.
Do **not** try to fix this by making the ring survive deliberate actions — the ring's clear-on-
deliberate-action is *correct* for its own purpose (it prevents a stale cascade window); the gap is
that **no driven detector exists for this shape**.

**6.5b — A counter-growth cover.**
Add a monotone counter-growth cover alongside object-growth and fodder-growth, and generalize the
axis taxonomy so a growth axis is a *parameter*, not a new hand-written cover per shape
(cf. CLAUDE.md "parameterize, don't proliferate"). Candidate axes already modelled in
`ResourceVector`: counters, tokens, mana, energy, life. Counter growth (proliferate, charge
counters, +1/+1 engines) is a large, common class.

### 6.7 — Sequencing: tactical unblock vs. target architecture (READ BEFORE STARTING)

§5.5 changes what is worth building. Do **not** implement §6.4 and §6.5 as originally written —
they are workarounds for a model that §5.5 replaces.

| Phase | Do it? | Why |
|---|---|---|
| **6.0** real-card corpus | **YES, FIRST** | Independent of architecture. It is the reason all of this shipped, and it is the acceptance gate for everything below. |
| **6.1** zone-scoping | **YES, FIRST** | **Rules fix (CR 400.2)**, not an optimization. Ship standalone regardless of architecture. §5.5 makes it structural later, but the hidden-zone read must stop now. |
| **6.2** walkers replacing blanket fail-closes | **YES** | Directly reusable: §5.5.3's fragment classifier is the *same walker family*. `scan_mana_production` (done) is the template. Not throwaway work. |
| **6.3** re-found the `sibling` axis | **REPLACE** | Do not merely narrow `sibling: bool`. Go straight to §5.5.3's `FragmentClass` — the narrowing and the fragment certificate are the same walk, so building `sibling` twice is waste. |
| **6.4** transient tolerance | **DROP the mechanism, KEEP the concern** | The *drive-until-stable heuristic* is dead: under §5.5 the payment choice is **inexpressible** — convoke consumes "an untapped green creature" and the token produces one, so it is net-0 *even when Witherbloom is tapped* (it was itself an untapped green creature; the count goes 5→4→5 either way). There is no transient to tolerate and no sampling horizon to be too short. **But the concern survives as the `prefix` term `p` of the §5.5.1 certificate** — a warm-up is a CR 732.2a "non-repetitive series of choices", computed analytically instead of chased by driving extra iterations. **Do not drop 6.4 without also changing what "verified" means** (see the §5.5.1 landmine): keeping board-state equality as the verifier re-creates this exact bug. |
| **6.5a** driven detector for activated cycles | **SUBSUMED** | Becomes "enumerate activated abilities as transitions". Not a bespoke second detector. **B1 (ring cleared on deliberate actions) stops mattering** — the LP reads the *board*, not a sampled history, so a player-driven loop needs no ring at all. |
| **6.5b** counter-growth cover | **SUBSUMED** | Counters are just another axis in `Δ`. Adding a third hand-written cover beside object-growth/fodder-growth is the sibling-cluster smell CLAUDE.md forbids; §5.5 makes the growth axis a **parameter**. |

**Recommended order:** 6.0 → 6.1 (ship: rules fix) → 6.2 (ship: walkers) → §5.5 (LP + fragment
certificate, absorbing 6.3/6.4/6.5).

Under §5.5, **both live combos certify for the same reason**: each is a firing vector with net ≥ 0 on
consumables and > 0 on a growth axis (tokens for Sprout Swarm; counters for Pentad Prism), on a board
whose transitions are all Constant or Monotone. That is what "build for the class" looks like here —
one model, two combos, no per-combo code.

### 6.6 — Follow-ups surfaced but out of scope

- **Chrome export is broken:** `exportGameStateDebugZip` (`client/src/services/gameStateExport.ts:63`)
  takes the `showSaveFilePicker` path in Chrome and silently fails (works in Firefox via the anchor
  fallback). The `await` is unguarded — likely an uncaught rejection on user-cancel/permission.
- The 4-player **ring-clearing between opponents' priority passes** hypothesis (a non-sampling beat
  clears the ring; every ability resolution in a 4-player game requires 3 opponents to pass) was
  **not** the cause here — B1 (clear-on-deliberate-action) fires first and is decisive. Re-check it
  only after 6.5a lands.

---

## 7. Acceptance criteria

1. Witherbloom + Sprout Swarm on the **real** exported board ⇒ `WaitingFor::LoopShortcut`
   (`predicted_winner: None`, `WinKind::Advantage`, unbounded axis `TokensCreated`).
2. Kilo + Freed + Relic + Pentad Prism on the **real** exported board ⇒ offer, unbounded axis
   **counters**.
3. **Hidden-zone invariance:** for both, the verdict is unchanged when arbitrary cards are added to
   any library or hand. (CR 400.2 — a detector verdict that depends on a hidden zone is a rules
   violation.)
4. **Payment invariance:** for Combo A, the verdict is unchanged under any legal convoke tap-set
   (Witherbloom, a Saproling, or a Forest paying the `{G}`).
5. **Realism:** every acceptance test carries real Oracle text, a real library, and a real mana base.
6. **Non-vacuity:** each fix has a revert-probe — deleting it flips a named test from pass to fail.
7. **No false positives:** the existing negative controls still decline
   (`object_growth_no_affinity_does_not_offer`, `object_growth_no_buyback_does_not_offer`,
   `object_growth_random_recast_body_does_not_offer`, `object_growth_self_damage_recast_does_not_offer`,
   `off_mode_capture_leaves_recast_context_none`).

8. **Fragment-certificate controls** (§5.5.3) — each must be *discriminating*, i.e. a revert-probe of
   the guard flips it to FAIL:
   - **Threshold:** a board carrying *"if you control seven or more creatures…"* must be **rejected**
     (non-monotone cliff — the drive's 2–3 iteration horizon cannot see it).
   - **Zero-test:** a board carrying *"if you control no creatures…"* must be **rejected** (an
     inhibitor arc — this is the construct that buys Turing-completeness).
   - **Monotone admit (non-vacuity):** affinity and proliferate must be classified **Monotone and
     ADMITTED**, not rejected. Without this the fragment classifier degenerates into today's
     firewall and both live combos stay undetected — so this control proves the classifier is not
     merely permissive *or* merely conservative.

9. **Soundness guard already landed** (`ability_scan::mana_production_scan_tests`): *"add {G} for each
   creature you control"* (Gaea's Cradle) must stay CONSERVATIVE. Verified discriminating by
   revert-probe: collapsing the count-bearing arms of `scan_mana_production` to `Axes::NONE` flips
   `for_each_creature_production_still_fails_closed` to FAIL while the Forest control still passes.

10. **LP/drive agreement:** every loop the LP certifies must also survive the clone-drive
    (`m < m'` Karp–Miller witness). An LP certificate the drive cannot execute is a BUG, not an
    offer — LP proposes, the drive verifies (§5.5.1).

---

## 8. Design principles to carry into the code

> **(1) Scope every conservatism to the present board state and the loop actually being executed —
> never to the space of all possible board states reachable from all cards in all decks and hands.**
>
> The loop must be infinite **from the perspective of the player executing it**, given the current
> visible board and the predictable results of their own choices (CR 732.2a). It is then **passed
> around the table for response** (CR 732.2b: accept or shorten). Interaction is the response
> window's job, not the cover's — and reaching into hidden zones to pre-empt it is both a false
> negative *and* a rules violation (CR 400.2).

> **(2) Ask a resource-flow question, not a state-recurrence question.** "Is there a firing vector
> with net ≥ 0 on consumables and > 0 on a growth axis?" is a small LP over the present board. It is
> decidable, it yields the *simplest* cycle to propose, and it makes the finite choices (which land
> taps, which creature convokes) **inexpressible as a difference** — they cancel in the net vector.

> **(3) Don't fight Turing-completeness — certify the fragment.** Magic is TC, so no complete
> procedure exists. But a VAS is decidable, and TC is bought by exactly one construct: the
> **zero-test**. Classify the board's transitions syntactically over the combinator AST
> (Constant / Monotone / Threshold / ZeroTest / NonDeterministic). Certify the decidable fragment and
> solve it exactly; decline **loudly and narrowly** outside it. That is a precise, small, principled
> conservatism — the opposite of "reject if any card anywhere might look at a creature."

> **(4) Monotone reads are not hazards.** The card that makes a combo infinite is usually the card
> that reads the growing axis (affinity reads creature count; proliferate reads permanents-with-
> counters). A firewall that rejects "reads the growing class" is structurally incompatible with the
> entire family of real combos. What matters is whether the read is *monotone-benign* (cost only
> falls / production only rises ⇒ feasible now implies feasible forever) or a *cliff*.

---

## Appendix — key sites

| Site | Role |
|---|---|
| `casting_costs.rs:6785` | arming (`last_recast_context`) — **correct**, verified live |
| `engine.rs:445-464` | offer gate — **correct**, all preconditions verified green |
| `engine.rs:1648` | `try_offer_object_growth_shortcut` — declines at the cover |
| `engine.rs:1599` / `:1633` | `normalize_recast_frame` / `derived_fodder_class` |
| `engine.rs:3081` | **ring clear on deliberate action** (B1 blocker) |
| `resource.rs:784` | `loop_states_cover_modulo_growth` (strict counters — B2 blocker) |
| `resource.rs:963-980` / `:1126-1131` | object-growth / fodder covers — both call the firewall |
| `resource.rs:1468-1612` | `fire_time_conditions_read_growing_class` — the firewall (A1/A3/A4/A6) |
| `ability_scan.rs:852` | `Effect::Mana => Axes::CONSERVATIVE` (A2) — **fix drafted & proven** |
| `mana_payment.rs:394` | `select_convoke_taps` (lowest-ObjectId order — A7) |
| `functioning_abilities.rs` | `active_trigger_definitions` (unscoped) vs `battlefield_active_triggers` (scoped) |
| `loop_shortcut.rs:2536` | `sprout_swarm_scenario` — the unrealistic fixture that hid all of this |
