# Combo detector: root-cause analysis + remediation plan
### Making loop detection work on real decks and real board states

**Date:** 2026-07-13
**Status:** Investigation complete, implementation NOT started. For maintainer review.
**Evidence:** All claims below were *measured* by driving the user's exported live game state
through the real engine, not inferred. Reproduction harness + method in §2.

> ## ⛔ READ §5.5.7 FIRST — adversarial review found §5.5 UNSAFE AS FIRST WRITTEN
>
> §1–§4 (the RCA) survived adversarial review and is sound. **§5.5 (the target architecture) did
> not.** As first written it was **LESS SOUND than the code it proposes to replace**: it silently
> discarded three safety properties that exist in the tree today, are documented, and are
> test-backed. An implementer following the original §5.5 + §7 could ship a detector that **ends a
> game on a false certificate**.
>
> §5.5.7 (round 1) and **§5.5.8 (round 2)** carry the corrections and **override** anything earlier
> that contradicts them. **THREE claims in this document were measurably FALSE**: B2 (a counter-growth
> cover already exists and names Pentad Prism); the `ResourceVector` reuse claim; and — worst —
> §5.5.2's *"the payment choice is inexpressible"*, which is **false on our own §2 board** because
> **Witherbloom is Legendary and Relic of Legends filters costs on `Legendary`** (§5.5.8-A).
> Do not implement §5.5 without reading §5.5.7, §5.5.8 **and §5.5.9**.
>
> **§5.5.9 is the most important section in this document.** §5.5's progress rule is *strictly weaker*
> than what already ships: it has **no player attribution and no loss veto**, so it would certify a
> loop that **decks and kills its own proposer** (Four Horsemen minus Emrakul — fully deterministic,
> the non-determinism guard never fires). **The fix is not new code — it is the existing
> `net_progress_for(caster)` + `has_no_loss_axis(delta)` + `driving_resources_non_decreasing(..)`
> triple the plan was about to throw away.**

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

**~~B2 — No counter-growth cover.~~ ❌ STRUCK — THIS CLAIM WAS FALSE (adversarial review).**

**The counter-growth cover ALREADY EXISTS and already names this exact combo.**
`loop_states_cover_modulo_counter_growth` (`resource.rs:1326`) covers strict `Generic`-counter
growth, and its doc-comment says verbatim: *"the proliferate/charge (**Pentad Prism**) and burden
(The One Ring) ω-cover shape."* It is wired into **both** `detect_loop` (`loop_check.rs:230`) and
`interactive_loop_bridge` (`engine.rs:632`), with four discriminating tests
(`resource.rs:3116-3204`).

⇒ **Combo B's blocker set is B1 + B3 + B4 — NOT B1 + B2.** The cover is never *consulted* because
B1 clears the ring before it can accumulate. §6.5b is therefore not "subsumed by §5.5"; it is
**already shipped**, and any plan that proposes to build it is duplicating working, tested code.

*(This was my error. It also deflated part of §5.5's justification: the growth-axis taxonomy is
less incomplete than claimed.)*

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
by kind, tokens, life, cards-in-zone.

> ❌ **STRUCK — FALSE:** ~~(`ResourceVector` already computes exactly these deltas.)~~
> Measured (`resource.rs:138-229`): `ResourceVector` has **no untapped-permanent axis and no tap
> state at all**; `mana` is a 6-slot array of *floating pool* mana **summed across ALL players**
> (unusable per-player, and the driving fixture is a 4-player game); and `tokens_created` /
> `cards_drawn` / `casts_this_step` are **event-fed and left ZERO by `snapshot`**. Worse,
> `ResourceVector::delta` diffs two *snapshots* — it is a **measurement**, not a symbolic effect
> vector, and **no symbolic Δ extractor exists in the engine.**
> ⇒ §5.5 is a **new subsystem**, not a reuse. See §5.5.7-E for what this does to the cost estimate.

The certificate is an **ordered sequence of STAGED segments** `[(p₁,x₁), (p₂,x₂), …, (pₙ,xₙ)]` —
each a finite **prefix** `pᵢ ≥ 0` followed by a repeating **cycle** `xᵢ ≥ 0`, where `xᵢ` repeats
until a (linear) resource threshold is met and the next stage begins. Real wins are **sequential**:
*loop for infinite mana → stop → cast the payoff*. A single `(p, x)` pair cannot express that; see
§5.5.5 for why staged (and NOT nested) is the right shape.

Each segment satisfies:

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

### 5.5.5 Certificate shape: STAGED, not nested — and why that keeps it decidable (researched)

CR 732.2a admits four shapes: *"a non-repetitive series of choices, a loop that repeats a specified
number of times, **multiple loops**, or **nested loops**."* We support the first three and
**deliberately DECLINE nested loops**. Justification is empirical and theoretical, and both point the
same way.

**Nested = where Turing-completeness lives.** A *nested* loop whose inner iteration count depends on
the outer loop's evolving state is a counter machine. That is precisely the construct §5.5.3's guard
rejects (a marking-dependent branch / zero-test). Supporting staged-but-not-nested is therefore not a
gap — it is the *same* decidability boundary, expressed at the certificate level.

**Empirically, nesting does not occur in real play.** Searched; found no counterexample:

- The Turing-completeness construction (Churchill, Biderman & Herrick, *Magic: The Gathering is
  Turing Complete*, arXiv:1904.09828 / LIPIcs.FUN.2021.9) requires **deliberately assembled**
  machinery — Rotlung Reanimator / Xathrid Necromancer, **Artificial Evolution** to rewrite creature
  types, phasing as a state toggle — and the authors state it is *"a theoretical result about
  computational possibilities rather than something that would emerge spontaneously during standard
  gameplay,"* requiring *"intentional construction,"* *"precise sequencing,"* and *"specific board
  states maintained across multiple turns."* Its conditional branching comes from **responding to
  game state** — i.e. the zero-test. It is not a board state that arises in play.
- Real combos are single cycles or straight lines: Thassa's Oracle + Demonic Consultation (no loop),
  Kiki-Jiki + Corridor Monitor / Felidar Guardian (one cycle), Underworld Breach + Brain Freeze +
  Lotus Petal (one cycle). (EDHREC / LearnCEDH combo corpora.)
- Wins are **sequential/staged**: loop to accumulate a resource, stop, spend it. Hence the staged
  certificate.

**Tractability is the payoff.** Staged is still LP-able — each stage is an LP and the inter-stage
threshold is a linear constraint. Nested is not. Declining nested is what keeps the whole model
decidable, and it costs ~nothing in coverage.

### 5.5.6 Independent validation: the official notion of "advancement" IS the progress constraint

**CR 732.1c** defers loop handling to the Magic Tournament Rules. The canonical *unshortcuttable*
loop — **Four Horsemen** (Basalt Monolith + Mesmeric Orb + Narcomoeba/Dread Return/Blasting Station)
— is disallowed for reasons that restate this model almost exactly. The judge rationale:

> *"the loop does not add mana to our pool, draw cards, make creature tokens, deal damage, accrue
> energy counters, venture into the dungeon, or do anything else that would count as **advancement**…
> the only detail of the gamestate that changes from one post-reshuffle snapshot to the next is the
> order of those cards in our library… which is unknown information."*

"**Advancement**" is exactly `Σ xₜ·Δₜ > 0` on some growth axis. Four Horsemen is net-zero on every
observable axis ⇒ **the LP rejects it for the official reason**. It is *also* non-deterministic
(shuffling ⇒ unknown library order) ⇒ the §5.5.3 fragment guard rejects it a second, independent
way — and CR 732.2a excludes conditional/random actions by name.

⇒ **Add Four Horsemen as a negative-control acceptance test** (§7): it must be DECLINED, and ideally
for both reasons. A model that certified it would be wrong by the actual tournament rules.

**Also validated: the Monotone class is real and common.** Underworld Breach + Brain Freeze is a
*growing cascade* — storm count rises each cast, so each Brain Freeze mills MORE. `Δ` grows with the
marking, so it is not a constant-`Δ` VAS. It is admissible **only** via the monotonicity lemma
(production non-decreasing in the growth axis ⇒ evaluate `Δ` at the current marking `m₀` as a
conservative LOWER bound; feasible at `m₀` ⇒ feasible at every larger marking). This is not an exotic
corner — it is a top-tier cEDH combo, so the Monotone class must be first-class, not an afterthought.

### 5.5.7 ⛔ REVIEW CORRECTIONS — read before implementing anything in §5.5

Adversarial review (all findings independently re-measured and CONFIRMED). §5.5 as first written was
**less sound than the code it replaces**. These corrections **override** §5.5.1–§5.5.6.

#### A. Is competitive Magic provably an LP? **NO.** LP is a PROPOSER, never the proof.

The honest answer, and the single most important correction in this document. **Δ is not
symbolically derivable from the AST**, for three independent reasons — each backed by a real card:

1. **Replacement effects rewrite Δ at resolution time.** **Solemnity** (two `Prevent` replacements on
   `AddCounter`): proliferate's AST-Δ is `+1 counter`; its TRUE Δ is **0**. Replacements are not
   transitions and cannot be folded into a static Δ.
2. **Δ can depend on the marking (breaks VAS linearity).** **Freed from the Real** —
   *"{U}: Untap enchanted creature"* — has `Δ(untapped) = +1` **only if** that creature is tapped,
   else `0`.
3. **Legality is not a resource.** Activation limits, summoning sickness, the legend rule, loyalty
   (CR 606.3), max hand size — none are resource axes, and none appear in an `AbilityCondition`.

⇒ **The sound architecture is: LP PROPOSES, the existing DRIVE VERIFIES.**
- **LP (proposer):** cheap, fungible, payment-independent. Dissolves A7 and the "which land taps"
  problem *in the proposal*. Its Δ is a **measured** lower bound, not a symbolic derivation.
- **Drive + existing per-object cover (verifier):** sound. It is the ONLY thing that sees
  replacements, sickness, and marking-dependent Δ, because it actually **runs** the cycle.
- **A cycle is certified only if BOTH agree.** The LP may propose garbage; the verifier kills it.
  This preserves every safety property below *and* keeps the LP's benefits. **Never certify on the
  LP alone.**

#### B. MUST-PRESERVE — three in-tree safety properties §5.5 silently discarded

Deleting any of these ships a false certificate. All exist, are documented, and are test-backed.

1. **Repetition-blocking legality gates** — `project_out_resources` (`resource.rs:2500+`)
   *deliberately PRESERVES* `activated_abilities_this_turn` / `_this_game`, `OncePerTurn` /
   `MaxTimesPerTurn` trigger limits, `crew_activated_this_turn`, and loyalty. Its own comment:
   *"blanket-clearing them would erase the gate that makes a once-per-turn … ability NON-repeatable,
   **falsely certifying it as infinite**."* Single authority: `ability_has_per_turn_activation_gate`
   (`resource.rs:2842`).
2. **The `projected` axis and its firewall** (`fire_time_conditions_read_projected_resource`) — §5.5
   never mentions it. It is what catches **Damping Sphere**
   (`StaticMode::ModifyCost{ mode: Raise, dynamic_count: SpellsCastThisTurn }`), guarded by the
   in-tree discriminating test `R-e2` (`resource.rs:5052`). Replacing `sibling: bool` with
   `FragmentClass` while dropping `projected` is a **straight regression**.
3. **The strict per-object board cover.** It is the verifier (see A).

#### C. ❗ H1 (GAME-ENDING) — activation gates are not modelled, and a 2-iteration drive cannot see them

**`Manaforge Cinder`** — *"{1}: Add {B} or {R}. **Activate no more than three times each turn.**"*
Parses TODAY as `activation_restrictions: [MaxTimesEachTurn{count: 3}]`, **`is_mana_ability: true`**
⇒ it *is* a §5.5.1 transition. The LP sees net ≥ 0 and certifies; the drive is **exactly 2
iterations** (`engine.rs:1688-1696`) so it passes; **the loop dies on activation 4.**

**Fix (LP-native, and it is the right shape):** model a per-turn allowance as a **consumable axis**.
A gated ability draws from a finite pool the cycle never refills ⇒ `net < 0` on that axis ⇒ **no
T-invariant exists** ⇒ correctly rejected, with no special case. Any gate that cannot be encoded as
an axis ⇒ **REJECT the transition** (default-deny).

#### D. ❗ H2 (GAME-ENDING) — "untapped creature" is NOT a fungible axis (CR 302.6)

Identical LP Δ ("consume 1 untapped creature ⇒ +1 mana"), **opposite truth**:
- **Earthcraft** — cost `TapCreatures{count:1, filter: Creature/You}`, **no `{T}` on the creature**
  ⇒ sickness-immune (CR 702.51a-style) ⇒ a fresh token CAN pay ⇒ **Earthcraft + Squirrel Nest is
  genuinely infinite.**
- **Cryptolith Rite** — *grants* creatures `{T}: Add one mana` ⇒ **CR 302.6 applies** ⇒ a fresh token
  **cannot** pay this turn ⇒ bounded by the already-unsick count (a finite buffer a 2-iteration drive
  walks straight through whenever it is ≥ 3).

**The discriminator is the SHAPE OF THE COST, not any resource level.**

#### E. ❗ H5 (ROOT CAUSE) — pick the place granularity; §5.5's two requirements were contradictory

§5.5.2's A7-dissolution needs **fungible class places** ("an untapped green creature"). Freed from
the Real and sickness need **per-object places**. §5.5 never chose. **Resolution:**

> **Places are fungible *within an equivalence class defined by every predicate any COST or FILTER on
> this board can test*.** Sickness, legendary-ness, color, creature type, tapped-ness, token-ness are
> all class dimensions. On a small board this is a handful of classes — tractable *and* sound.

This simultaneously fixes H2 (sick vs unsick are different places; the cost shape selects which) and
**Relic of Legends** (*"tap an untapped **legendary** creature"* — a Saproling is **not** a
substitute for Kilo, because `legendary` is a class dimension).

**Cost estimate correction (see the struck `ResourceVector` claim):** there is **no symbolic Δ
extractor** in the engine. Building one re-derives the effect resolver across the no-wildcard
`Effect` match sites; measuring Δ instead costs one clone-drive **per transition, per priority beat**
(and §6.7 wanted this to run *without* the ring, i.e. on **every** beat). "A handful of ability defs
— cheap" **badly understates this.** Size it honestly before committing.

#### F. ❗ H3 — the fragment guard scans the WRONG AST SURFACE (the deepest finding)

Every hole found routes **around** `AbilityCondition`/`StaticCondition`. The reviewer could **not**
break the Comparator-expressed Threshold/ZeroTest arm — *that class is genuinely handled*. The cliffs
live instead in:

| Surface | Real card | Today caught by |
|---|---|---|
| **Costs** (`Cost::Tap` + per-object `summoning_sick`) | Cryptolith Rite | nothing in §5.5 |
| **Activation restrictions** | Manaforge Cinder | `project_out_resources` (B1 above) |
| **Static cost modifiers** (`ModifyCost.dynamic_count`) | Damping Sphere | the **`projected`** axis |
| **Replacements** (rewrite Δ) | Solemnity | only the drive |
| **Per-object attributes** | summoning sickness | only the drive |

⇒ `FragmentClass` must be **exhaustive with an explicit default-REJECT**, must add a **cost-DIRECTION
rule** (a cost that *rises* with a growing axis — Damping Sphere, Thalia, Archon of Emeria — stalls
the loop and must reject), and needs a **COMPOSITION lemma**: the monotonicity argument is stated
*per-transition*, but feasibility depends on the **composite** cost. Affinity (monotone-DOWN) +
Damping Sphere (monotone-UP) are each "monotone"; **the composite is not.** The one-line
"feasible now ⇒ feasible forever" argument holds only for a *single* modifier.

Also: `ZeroTest` is a leaf parameterization of `Threshold` (zero-test = `Comparator::Equal(0)`) ⇒
**sibling-cluster smell** per CLAUDE.md. Use `Threshold { comparator, bound }`. **Run the mandatory
`add-engine-variant` gate and grep `data/engine-inventory.json`** — §5.5 skipped both.

#### G. §6.1's gate-(1) prescription contradicted itself — CR 113.6 runs a–**k**, not a–d

**CR 113.6k** (verified): *"A trigger condition that can't trigger from the battlefield functions in
all zones it can trigger from."* A blanket battlefield-only filter would silently drop legitimate
**public-zone** observers (graveyard-functioning triggers). Use **one zone-of-function predicate**
applied uniformly to gates (1), (3), (4) — the CR 400.2 fix is about **hidden** zones, not
*non-battlefield* zones. Do not conflate them.

#### H. §6.4 must be RE-SCOPED, not dropped — A7 lives in the VERIFIER

The LP dissolving "which permanent paid" in the **proposer** does nothing for the **verifier**, which
still calls `select_convoke_taps` (lowest ObjectId ⇒ taps Witherbloom) and still compares frames.
Even a pure marking comparison is not transient-immune: frame₀ has Witherbloom untapped, frame₁
tapped — any tap-state axis differs. **Drive-until-stable is still required, on the verifier side.**

#### I. Untouched by §5.5 and still live

- **Gates (5), (5b), (6)** (`resource.rs:1544-1589`). Gate **(6)** rejects on ANY non-empty
  `delayed_triggers` / `deferred_triggers` / `pending_trigger` / `epic_effects` — not a zone scan, and
  the LP has no answer for it. It fires on any board with a *"sacrifice it at the beginning of the
  next end step"* delayed trigger — **e.g. every Kiki-Jiki token**. "The observer firewall largely
  dissolves" does **not** cover this.
- **`cost_surface_references_growing_class`** (all-zones) — §5.5 claims it dissolves. It does not: the
  LP *needs* cost to compute Δ, and cost surfaces are exactly where affinity, convoke, and Damping
  Sphere live. **It moves; it does not vanish.**

### 5.5.8 ⛔ ROUND-2 CORRECTIONS — the fungibility argument was FALSE on our own board

Second adversarial pass. All re-measured and CONFIRMED. **Overrides §5.5.2 and §5.5.5–§5.5.7.**

#### A. ❗❗ THE HEADLINE — "the payment choice is inexpressible" is **FALSE**, and Relic of Legends proves it on the §2 board

§5.5.2 claimed: *"convoke consumes an untapped green creature and the token produces one ⇒ net 0
even when Witherbloom is tapped."* **Measured, that is wrong.**

`Relic of Legends`' second ability cost (verbatim from `data/card-data.json`):
```json
{"type":"TapCreatures","count":1,
 "filter":{"type_filters":["Creature"],"controller":"You",
           "properties":[{"type":"HasSupertype","value":"Legendary"}]}}
```
**Witherbloom is Legendary. Kilo is Legendary. The Sprout Swarm token is NONlegendary.** Relic is on
the §2 board (it is Combo B's mana engine). So convoking Witherbloom is:
- net **0** on *untapped green creatures* ✅
- net **−1** on *untapped **legendary** creatures* ❌ — and the Saproling **cannot** substitute,
  because Relic's cost filters on `Legendary`.

**The dilemma (no third horn):**

| | consequence |
|---|---|
| `legendary` is NOT an axis | The LP treats Witherbloom/Kilo and a Saproling as fungible ⇒ it **falsely certifies a Relic of Legends mana engine** (tap a legendary for mana; a token "replaces" it; the token can never pay that cost). **GAME-ENDING.** |
| `legendary` IS an axis | Then the §5.5.2 net-0 claim is false on our own board — convoking Witherbloom net-drains untapped-legendaries every cycle. |

The measured axis granularity that exists today is
`ObjectClass { Creature, Planeswalker, Battle, Player, Other }` (`resource.rs:52-69`) — **no
legendary, no color, no subtype.** A naive LP reusing it lands on the top row.

**RESOLUTION (this is the corrected model — adopt it):**

> **Axes are NOT a fixed enumeration. They are the equivalence classes induced by the `TargetFilter`s
> that appear in COSTS on the present board.** Costs filter on arbitrary predicates: `Legendary`
> (Relic), *color* (convoke's colored pips, CR 702.51a), *total power* (crew), *type* (improvise),
> `nontoken`, `another`. Partition objects under the Boolean algebra those filters generate. Filters
> **overlap** (Witherbloom is green ∧ legendary), so the places are **not disjoint** — emit one
> transition per *(ability × consumed equivalence-class)* and let the LP choose.

On the §2 board the classes are `{legendary-green: Witherbloom}`, `{legendary-nongreen: Kilo}`,
`{nonlegendary-green: Saprolings}`. The LP then **correctly rejects** "convoke Witherbloom" (net −1
on legendary-green) and **certifies** "convoke a Saproling" (net 0) — with *"convoke Witherbloom
once"* falling out **as the prefix** if it is ever needed.

**The corrected claim is STRONGER than the one it replaces.** The payment choice is *not*
irrelevant — it is **decisive**, and the LP's job is to **choose the sustainable one**. That is a
better property than indifference.

**⇒ And it back-propagates to the drive (this VINDICATES §6.4 and kills "DROP" for good).** The
drive must execute **the cycle the LP certified**. `select_convoke_taps` picks lowest-ObjectId =
**Witherbloom** = the *unsustainable* cycle. **`DecisionTemplate` MUST carry the LP's chosen
equivalence class**, or the drive verifies a different cycle than the one certified. (Note: this is
not "bias the selector to make detection work" — which the maintainer rightly rejected. It is
"execute the certificate faithfully," which is a different and necessary thing.)

#### B. ❗ CHANGE 2 corrected — net-delta verification is UNSOUND as stated. Relax, don't replace.

Measured: `object_content_eq` (`game_state.rs:10453+`) compares **`tapped` per object** — but does
**NOT** compare `summoning_sick`.

⇒ **A7 and the Cryptolith Rite false positive (H2) are the same line of code.** The strict per-object
`tapped` compare is the *only* thing currently blocking H2. Relaxing `tapped` into a fungible
class-count **without** modelling `summoning_sick` converts a false *negative* into a false
*positive*.

What board-equality catches that a coarse net-delta drops (all measured, all in-tree):
`phase_status` (CR 702.26 — *"a loop that phases a permanent in and out is a wrongful CR 104.4b
draw"*); `attached_to`/`attachments` (Freed from the Real must **stay on Kilo**); **non-monotone
counter kinds** — `project_object_for_loop` *deliberately preserves* stun/shield/time/fade/age/**lore**
counters (*"consuming one of these is a real board change, not a monotone pump"*), so a loop burning a
**Saga's lore counter** (CR 714.4 ⇒ sacrifice) is an ∞-consume trap; and the **§5.2c ADD set — 14
fields** (`intensity`, `perpetual_mods`, `stickers`, `class_level`, `contraption_sprocket`,
`is_suspected`, `prepared`, `room_unlocks`, `chosen_attributes`, `goaded_by`, `detained_by`,
`casting_permissions`, `saddled_by`, `dealt_deathtouch_damage`) added by a **prior review** with the
comment *"firewall-blind numeric/growable accumulators … a loop body can drift on a stable object."*
A net-delta verifier drops **all fourteen**. (e.g. a Class-levelling loop grows `class_level`, which
**caps at 3** ⇒ bounded ⇒ the net-delta check certifies it anyway.)

> **MINIMUM SAFE FORM — do this instead of "replace the cover":**
> **RELAX equality only on the axes the LP actually models, and KEEP a compiler-enforced
> residual-equality check on the complement.** The existing no-`..` totality guards
> (`_gameobject_partition_is_total` `game_object.rs:1038`, `_gamestate_partition_is_total`
> `game_state.rs:10506`) make this **structurally safe**: a new field cannot be silently dropped.
> This buys A7 **without** buying H2, Relic-of-Legends, the lore/fade ∞-consume trap, or the 14
> accumulators. Small diff, compiler-enforced.

#### C. ❗ H6 (NEW, GAME-ENDING) — land drops are a repetition gate, not a resource (CR 305.2)

**CR 305.2** (verified): *"A player can normally play one land during their turn."* All three cards
verified in `data/card-data.json`: **Crucible of Worlds** (*"You may play lands from your
graveyard"*), **Zuran Orb** (*"Sacrifice a land: You gain 2 life"*), **Azusa, Lost but Seeking**.

Cycle: play a land from the graveyard → tap for mana → sacrifice to Zuran Orb. Net: **lands 0**,
**mana +1**, **life +2** ⇒ sustainable + progressing on two axes ⇒ **the LP certifies infinite mana
and life.** Reality: **bounded at 3 iterations** by the land-drop rule. The **2-iteration drive walks
straight through it.**

§5.5.1's enumeration (*"each activated ability, each mana ability, each castable self-returning
spell"*) omits **land plays** and **special actions (CR 116)**. As literally written that is a false
*negative* — but land plays are canonical shortcut material and any implementer will add them.
**`lands_played_this_turn` must be an axis, or land plays must be excluded explicitly and loudly.**

#### D. §5.5.5's nesting rationale was WRONG (right conclusion, wrong reason)

**Fixed-count nesting is UNROLLABLE** — "inner ×3 per outer" is just `x` with the inner transitions
scaled by 3. It is **not** a counter machine and **must not be declined**. As written, an implementer
reading *"we decline nested loops"* will reject ordinary combos.
**Only a MARKING-DEPENDENT inner count is TC.** Fix the text: *decline marking-dependent inner
counts; absorb fixed-count nesting by unrolling.*

Also, "multiple loops" needs no staging at all: `x` is an order-free **multiset** of firings, so two
mutually-sustaining interleaved cycles are **one** T-invariant. Staging is needed **only** for the
threshold-gated payoff.

#### E. ❗ Internal inconsistency — the fragment guard would reject our own certificate

The staging predicate (*"repeat `x₁` until mana ≥ N"*) **is a linear comparator against a growth
axis** — syntactically the exact construct §5.5.3's Non-monotone arm REJECTS, and §5.5.5 explicitly
equates the two. **State plainly:**

> The fragment guard classifies **CARD ABILITY ASTs** (the transitions). It does **NOT** classify the
> certificate's own staging predicate. The proposer counting to N is sanctioned by CR 732.2a:
> *"a loop that repeats a **specified number of times**."*

Without this sentence a literal implementer rejects his own certificate.

#### F. ❗ `min Σp + Σx` can pick an UNEXECUTABLE cycle — §7 #10 is backwards

The LP is a **relaxation**: it enforces net-sustainability, **not schedulability**. Finding a firing
*sequence* from a marking is VAS reachability, not an LP. So the argmin `x` may be net-≥0 with **no
valid ordering** (an axis dips negative mid-cycle) while a *larger* cycle is the real combo.

⇒ §7 #10 (*"an LP certificate the drive cannot execute is a BUG"*) is **wrong**: a drive failure on
the argmin is the **expected** behavior of a relaxation. Required: a **no-good cut + re-solve loop**
(cut the failed `x`, re-solve, retry, **bounded**). Without it: false negatives on every board with
>1 candidate cycle.

**Concrete:** **Ashnod's Altar** (*"Sacrifice a creature: Add {C}{C}"*). The monotone lemma linearizes
affinity's cost at `m₀`, but the cast happens **mid-cycle** at `m₀ − (creatures sacrificed so far)`, so
the true cost is **higher** than modelled. ⇒ **The monotonicity lemma is stated over cycle boundaries;
costs are evaluated mid-cycle.** It needs the side condition: *the read axis must be non-decreasing at
**every prefix** of the cycle's schedule*, not merely over the whole cycle.

#### G. ❗ Unbounded prefix = a DoS vector we ALREADY FIXED ONCE on this branch

Nothing bounds `Σp`, and §6.7 wanted the LP to run **without the ring** — i.e. on **every priority
beat, for every player**, in a 4-player game: enumerate transitions → measure each Δ (a clone-drive
each, per §5.5.7-A) → LP → drive `p` → drive `x`.

**Commit `57b0e537d`, on this very branch: _"fix(engine): bound loop-shortcut iteration count (remote
DoS in #5672)."_** §6.7 re-opens that exact class.

> **KEEP THE RING (or some cheap arming signal) as a gate.** It costs nothing and it is **not** what
> was broken. **B1 is a false-negative bug, not a reason to delete the gate.** Bound `Σp`.

#### H. §5.5.6 precision (minor)

Four Horsemen's **short** cycle (Monolith untap → Mesmeric Orb mill) is **library-NEGATIVE**, so the
LP rejects it as **unsustainable**, not as non-advancing. The "no advancement" framing only holds over
the **long** cycle including the Emrakul reshuffle. Right answer, slightly wrong reason.

### 5.5.9 ⛔ ROUND-3 — the ONE fix that matters, and it is code we already have

Third adversarial pass. All re-measured and CONFIRMED. **This section is the most important in the
document.**

#### A. ❗❗ THE SINGLE HIGHEST-VALUE FIX — §5.5's progress rule is STRICTLY WEAKER than what ships

§5.5.1 says only:
> *"Sustainability: `≥ 0` on every consumable axis. Progress: `> 0` on at least one growth axis
> (tokens, counters, damage, …)."*

**No player attribution. No loss veto.** The `…` is doing lethal work. What already ships
(`engine.rs:1756-1760`) is a **player-attributed, loss-vetoed triple** — measured:

```rust
// has_no_loss_axis (engine.rs:814)
delta.life.values().all(|&n| n >= 0)
    && delta.library_delta.values().all(|&n| n >= 0)   // ← ANY library net-drain is a LOSS AXIS
    && delta.poison.values().all(|&n| n <= 0)

// net_progress_for(controller) (resource.rs:497) — PLAYER-ATTRIBUTED
if self.mana.iter().any(|&n| n < 0) { return false; }
for (pid, &n) in &self.life { if *pid == controller && n < 0 { return false; } }  // ← controller's OWN life
!self.unbounded_axes_for(controller).is_empty()
```

> **⇒ REPLACE §5.5.1's two-line constraint with the existing triple:**
> `net_progress_for(caster)` + `has_no_loss_axis(delta)` + `driving_resources_non_decreasing(...)`.
> **This is not new code. It is code the plan was about to throw away.**

That one change kills **all** of the following at once:

**B. ❗ D-4 (GAME-ENDING) — "Four Horsemen minus Emrakul" certifies a loop that DECKS AND KILLS the proposer.**
**Basalt Monolith** (mana-neutral: `{T}` for `{C}{C}{C}`, `{3}` to untap) + **Mesmeric Orb**
(*"Whenever a permanent becomes untapped, that permanent's controller mills a card"*). **No Emrakul ⇒
no shuffle ⇒ the non-determinism guard NEVER FIRES.** Fully deterministic. Δ = mana 0, **library −1,
graveyard +1** per cycle. If "cards-in-zone" is a *growth* axis (§5.5.1 lists it), then sustainability
✅ + progress ✅ ⇒ **the LP CERTIFIES**, the proposer mills their entire library and **loses on their
next draw (CR 104.3c)**. The growth axis was **capped at library size** all along.
*Vetoed today by `library_delta >= 0`. §5.5 deleted that veto.*

**C. ❗ D-6 — Suture Priest.** An **opponent's** Suture Priest (*"Whenever a creature an opponent
controls enters, you may have that player lose 1 life"*) drains **the proposer** 1 life per Saproling
⇒ the Sprout Swarm loop **kills its own controller** at 40 iterations. The **2-iteration drive sails
through.** *Vetoed today by `has_no_loss_axis`'s `life >= 0`.*

**D. ❗ C-2 — §5.5 BREAKS OUR OWN §7 NEGATIVE CONTROL.** §7 #7 requires
`object_growth_self_damage_recast_does_not_offer` to keep declining. Under an **unattributed**
"damage" growth axis, **damage dealt to MYSELF counts as progress** ⇒ that control **flips to an
OFFER**. The plan's own regression list breaks.

#### E. ❗ §5.5.6's flagship Monotone example is WRONG — Breach has NO T-invariant

**Underworld Breach** escape = *"exile **three other cards from your graveyard**"*; **Brain Freeze**
= *"target player mills three cards"* (refuelling from **your own library**). ⇒ the cycle is
**`library_delta < 0` for the controller, every iteration.** Under §5.5's **own** sustainability rule
(`≥ 0` on every consumable axis) **the LP REJECTS it.**

⇒ Breach + Brain Freeze is **a bounded PREFIX into a payoff stage** (Grapeshot / Thassa's Oracle
before you deck), **not a Monotone cycle.** It proves *nothing* about the Monotone class, which is the
sole thing §5.5.6 offered it as proof of. The staged shape can express it — *as a prefix* — but the
Monotone claim must be re-argued or dropped.

#### F. ❗ The fragment table FAILS OPEN — and it is a REGRESSION. **Hum of the Radix.**

*"Each artifact spell costs {1} more to cast **for each artifact its controller controls**."*
(verified in `data/card-data.json`) — parses as `ModifyCost { mode: Raise, dynamic_count:
ObjectCount{Artifact} }`. **This is the growing-axis-scaled tax**, and artifacts (Treasures, Clues,
Thopters, Servos) *are* a growth axis.

Where it lands in §5.5.3's four-arm table: **not** `Constant` (Δ is marking-dependent); **not**
`Monotone` (the table says *"non-increasing in cost"* — this **increases**); **not** `Non-monotone`
(there is **no `Comparator`** — it is a linear scale); **not** `Non-deterministic`.
**It falls through a four-arm table with no default ⇒ FAIL-OPEN.**

And it is a **regression**: the engine catches Hum **today** via `resource.rs:1691` (`ModifyCost {
dynamic_count, .. }` is scanned) ⇒ `QuantityRef::ObjectCount ⇒ Axes{sibling: true}` ⇒ rejected, guarded
by the in-tree test **`R-e2`** (`resource.rs:5052`). **§5.5 deletes the axis that catches it and
supplies no replacement arm.**

⇒ Add a **cost-DIRECTION arm** (monotone *raise* on a growth axis ⇒ **REJECT**) **and an explicit
`_ => REJECT` default.** (My earlier suspects were wrong: **Thalia** and **Grand Arbiter** are **flat
+1**, not scaled; **Rule of Law / Archon of Emeria** are hard `PerTurnCastLimit{max:1}` gates enforced
at cast legality, so the drive backstops them.)

#### G. ❗ §5.5.5's definition of "nested" CONTRADICTS §5.5.6 — key on the BRANCH, not the count

§5.5.5 declines *"a nested loop whose **inner iteration count depends on the outer loop's evolving
state**."* **Underworld Breach + Brain Freeze is exactly that** (storm count → copies → mill), and
§5.5.6 **admits it**. Same structure, opposite verdicts, two sections apart.

> **The distinction that actually matters:** a **marking-dependent Δ** (a *scaling count*) is
> **Monotone — ADMIT**. A **branch** (a *comparison* deciding what happens next) is
> **Threshold/ZeroTest — REJECT**. A counter machine needs the **zero-test**, not merely a Δ that
> scales.
> Rewrite: *decline an inner loop whose **termination condition is a comparison against the outer
> loop's evolving state**; a scaling count is not nesting, it is a Monotone Δ.*
> **Fixed-count nesting ("inner ×3") is UNROLLABLE and must NOT be declined.**

(This is the same conflation as §5.5.8-E: the guard classifies **card ASTs**, never the certificate's
own staging predicate.)

#### H. ❗ INFINITE TURNS break the VAS itself — decline loudly (Time Vault)

**Time Vault** + Voltaic Key (real, Vintage) ⇒ infinite turns. CR 732.2a explicitly sanctions
shortcuts that *"may even cross multiple turns"*, so this is a real class, not a corner. But:
- the **untap step** (CR 502.2) has Δ = **`+tapped_count`** — a **marking-dependent, non-constant
  vector**. A VAS transition **must** be a constant additive vector. **This is not a VAS transition.**
- a turn boundary **RESETS** per-turn tallies (`lands_played_this_turn`,
  `activated_abilities_this_turn`). **A reset is not an increment** — also non-VAS.

⇒ **Turn-crossing loops are outside the fragment as modelled. Decline them explicitly in §5.5.3**, or
an implementer will model the untap step as a transition, silently break linearity, and the soundness
argument evaporates.

#### I. ❗ "Advancement ≡ net > 0" FAILS IN BOTH DIRECTIONS

- **Missed advancement (false negative):** the judge quote §5.5.6 relies on lists *"…accrue energy
  counters, **venture into the dungeon**"* — **and "venture" is absent from §5.5's axis set AND from
  `ResourceVector` entirely** (no dungeon axis; CR 309). **Acererak the Archlich** (verified in
  card-data: *"When Acererak enters, if you haven't completed Tomb of Annihilation, return it to its
  owner's hand and venture into the dungeon"*) is a real EDH infinite. **I quoted the definition of
  advancement and then dropped an item off it.** Same class: **experience counters**, **the
  initiative** (CR 309 Undercity), day/night.
- **False advancement (false positive):** **self-mill.** "cards-in-zone" is in the axis list, but
  milling your own library is **anti**-advancement (CR 104.3c) — see B above. And **damage without
  attribution** makes self-damage "progress" — see D.

#### J. §5.5.6's stated REASON for rejecting Four Horsemen was wrong — and the wrong reason hid B

Four Horsemen's cycle is **library-NEGATIVE** (Mesmeric Orb mills per untap), not "net-zero on every
observable axis." It is only net-zero *because Emrakul reshuffles* — which is the non-deterministic
part. So the LP rejects it for **unsustainability**, not non-advancement. **Right answer, wrong
reason — and the wrong reason is exactly what made D-4 (minus-Emrakul) invisible.**

#### K. New required §7 fixtures

- **Four Horsemen MINUS Emrakul** (deterministic self-mill) must **DECLINE**. ⚠️ The existing
  Four-Horsemen control is **NOT discriminating**: full Four Horsemen declines even if only the
  *non-determinism* arm fires, so it never tests the advancement/loss constraint. **This is the
  discriminating fixture, and it is the one that fails today under §5.5's axis set.**
- **Hum of the Radix** on an artifact-growth loop must **DECLINE** (preserve `R-e2`).
- **Acererak the Archlich** (venture) — a **known, logged** coverage gap, not a silent one.
- **Suture Priest** (opponent's) on the Sprout Swarm loop must **DECLINE** (controller loses life).

#### L. CLEAN BILLS (reviewer could not break these)

- **The empirical no-nesting claim** — searched cEDH/Legacy/Vintage/high-power EDH (Isochron+Dramatic
  Reversal, Kiki lines, Food Chain, KCI/Scrap Trawler webs, Doomsday, Breach). All are straight lines,
  single T-invariants, or staged. Artifact-recursion webs *look* nested but are **one order-free
  multiset** `x`. **§5.5.5's conclusion holds; only its definition was wrong** (see G).
- **Floor saturation** — `max(cost − k, 0)` is still non-increasing in `k`; affinity reduces
  **generic only** (colored pips are never reduced), and the colored requirement is what convoke pays.
  **No hole.**
- **The Threshold/ZeroTest reject arm** — still unbroken across three rounds. **Every** hole routes
  *around* the Comparator. **The chosen AST surface is the wrong one.**

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
| **6.4** transient tolerance | **RE-SCOPE TO THE VERIFIER** (was "DROP" — that was WRONG, see §5.5.7-H) | The *drive-until-stable heuristic* is dead: under §5.5 the payment choice is **inexpressible** — convoke consumes "an untapped green creature" and the token produces one, so it is net-0 *even when Witherbloom is tapped* (it was itself an untapped green creature; the count goes 5→4→5 either way). There is no transient to tolerate and no sampling horizon to be too short. **But the concern survives as the `prefix` term `p` of the §5.5.1 certificate** — a warm-up is a CR 732.2a "non-repetitive series of choices", computed analytically instead of chased by driving extra iterations. **Do not drop 6.4 without also changing what "verified" means** (see the §5.5.1 landmine): keeping board-state equality as the verifier re-creates this exact bug. |
| **6.5a** driven detector for activated cycles | **SUBSUMED** | Becomes "enumerate activated abilities as transitions". Not a bespoke second detector. **B1 (ring cleared on deliberate actions) stops mattering** — the LP reads the *board*, not a sampled history, so a player-driven loop needs no ring at all. |
| **6.5b** counter-growth cover | **ALREADY SHIPPED — BUILD NOTHING** | ❌ My B2 claim was FALSE. `loop_states_cover_modulo_counter_growth` (`resource.rs:1326`) exists, names **Pentad Prism** in its doc, is wired into `detect_loop` + `interactive_loop_bridge`, and has 4 discriminating tests. Building this would duplicate working code. See §5.5.7. |

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
3. **Hidden-zone invariance (CR 400.2).** ⚠️ **NON-VACUITY:** must assert `WaitingFor::LoopShortcut`
   in **EVERY arm**, not merely `assert_eq!(v_with, v_without)` — an equality assertion passes
   trivially as `false == false` when both arms decline. (This trap was hit for real in this effort;
   see `real_board_verdict_is_invariant_under_hidden_zone_contents`.) Plus a positive reach-guard:
   the base board must offer independently.
4. **Payment invariance:** for Combo A the verdict is unchanged under any legal convoke tap-set
   (Witherbloom, a Saproling, or a Forest paying the `{G}`). **Same non-vacuity rule as #3: assert
   the OFFER in every arm.**
5. **Realism:** every acceptance test carries real Oracle text, a real library, and a real mana base.
6. **Non-vacuity:** each fix has a revert-probe — deleting it flips a named test from pass to fail.
7. **No false positives:** the existing negative controls still decline
   (`object_growth_no_affinity_does_not_offer`, `object_growth_no_buyback_does_not_offer`,
   `object_growth_random_recast_body_does_not_offer`, `object_growth_self_damage_recast_does_not_offer`,
   `off_mode_capture_leaves_recast_context_none`).

8. **Fragment-certificate controls (§5.5.3 + §5.5.7-F) — REAL CARDS ONLY.**
   ⚠️ The earlier draft quoted **invented oracle text** with no card named, violating CLAUDE.md
   *"Verify the card, not just the rule."* Verified real substitutes, all present in
   `data/card-data.json`:
   - **Activation gate (H1, GAME-ENDING):** **Manaforge Cinder** (`MaxTimesEachTurn{3}`,
     `is_mana_ability`) must be **REJECTED**. Note it survives a 2-iteration drive — so the drive
     alone is NOT a sufficient control.
   - **Summoning sickness (H2, GAME-ENDING):** **Cryptolith Rite** must be **REJECTED** while
     **Earthcraft + Squirrel Nest** must be **CERTIFIED**. Identical LP Δ, opposite truth — this
     pair is the discriminator.
   - **Rising cost / composite monotonicity (H3):** **Damping Sphere** must be **REJECTED**
     (preserve the in-tree `R-e2` test, `resource.rs:5052`). Also **Rule of Law / Arcane Laboratory /
     Eidolon of Rhetoric / Archon of Emeria** (`PerTurnCastLimit{max:1}`).
   - **Replacement rewrites Δ (H4):** **Solemnity** + proliferate must be **REJECTED** (AST-Δ says
     `+1 counter`; true Δ is `0`).
   - **Non-determinism / no advancement:** **Four Horsemen** (Basalt Monolith + Mesmeric Orb) must be
     **DECLINED** — ideally for both reasons (§5.5.6).
   - **Monotone ADMIT (non-vacuity):** affinity and proliferate must be classified **Monotone and
     ADMITTED**. Without this the classifier degenerates into today's firewall and both live combos
     stay undetected — this proves it is neither merely permissive nor merely conservative.

   ⚠️ **A revert-probe is necessary but NOT sufficient.** Each REJECT control needs a **paired
   positive reach-guard** (the same board minus the hazard card must OFFER), else the REJECT may be
   produced by an unrelated upstream gate (gate 4's blanket, gate 6's delayed-trigger check) and the
   control proves nothing.

9. **Soundness guard already landed** (`ability_scan::mana_production_scan_tests`): *"add {G} for each
   creature you control"* (Gaea's Cradle) must stay CONSERVATIVE. Verified discriminating by
   revert-probe: collapsing the count-bearing arms of `scan_mana_production` to `Axes::NONE` flips
   `for_each_creature_production_still_fails_closed` to FAIL while the Forest control still passes.

10. **LP/drive agreement.** Every loop the LP certifies must also survive the clone-drive
    (`m < m'` Karp–Miller witness). ⚠️ **As written this was a property, not a test, and was
    vacuously satisfied by a corpus in which the LP certifies nothing** — it also self-contradicted
    ("a BUG, not an offer": if production silently declines on disagreement, nothing ever fails).
    Requires: a corpus with **≥1 certifying board**, and a **loud** `debug_assert!`/counter on
    disagreement. **Never certify on the LP alone** (§5.5.7-A).

11. **Multiplayer.** At least one criterion must exercise **>2 players** — the entire driving fixture
    is a 4-player Commander board, yet no criterion previously mentioned player count.

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
