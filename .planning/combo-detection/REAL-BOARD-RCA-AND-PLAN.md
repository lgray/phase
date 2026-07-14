# Combo detector — root-cause analysis + implementation plan
### Making CR 732.2a loop shortcuts work on real decks and real board states

**Date:** 2026-07-13 · **Status:** Plan. Implementation NOT started.
**Branch:** `debug/combo-generator` (fork-only; **never** merge toward `main` — `.planning/` is gitignored upstream).
**Evidence rule:** every claim below is **measured** — by driving the user's exported live board through
the real engine, by grepping `docs/MagicCompRules.txt`, or by reading `data/card-data.json`. This
document has been adversarially reviewed three times and **four of its earlier claims were false**;
they are recorded in **Appendix B**, because the guard rails only work if you know what they guard.

---

## 1. Executive summary

**The combo detector cannot fire in any real game of Magic.** Two live infinite combos on a real
4-player Commander board were verified undetectable. There are **two independent root causes**, and
**fixing either one alone leaves the loop undetected.**

**RC-1 — the observer firewall reads zones where abilities do not function.**
`fire_time_conditions_read_growing_class` (`resource.rs:1468`) scans `state.objects.values()` across
**every zone**. Measured trips, in order, on the real board:

| Trip | Consequence |
|---|---|
| `Solemn Simulacrum` **in the LIBRARY** | a card never drawn, uncastable, **permanently disables detection** |
| a basic **`Forest`** | `Effect::Mana => Axes::CONSERVATIVE` ⇒ a loop can only certify on a board with **zero mana sources** |
| **`Freed from the Real`** | any aura/utility permanent with a creature-referencing **activated** ability |

**RC-2 — the cover forbids a bounded start-up transient.** The detector demands board recurrence on
**both** driven pairs, `(s_n, s_n₁)` **and** `(s_n₁, s_n₂)` (`engine.rs:1728`). On any real board the
**first** driven iteration consumes a *non-fodder engine piece* (it convokes **Witherbloom** itself —
`select_convoke_taps` sorts by ObjectId and Witherbloom is 402, the Saprolings 413+). That is a
one-time, bounded consumption, so the first pair **cannot** cover, and the offer dies. **CR 732.2a
explicitly permits this shape** — see §4.4.

**RC-3 — the live detector arms on exactly ONE bespoke shape, and no test covers the live path at all.**
Measured:
- The **live** offer path fires only when `last_recast_context` is armed — *a buyback-paid,
  token-creating recast* (`casting_costs.rs:6785`). **That is one card shape.** Every other
  player-driven loop — one cast or one activation per iteration: Kiki-Jiki, Splinter Twin,
  Devoted Druid + Vizier, Earthcraft + Squirrel Nest, **Presence of Gond + Intruder Alarm (CR 732.2a's
  own example)** — is **invisible to the live detector**, no matter how correct the covers are.
- The **53-row corpus** (`analysis/corpus.rs`) is driven **entirely through `detect_loop`** (the
  *offline* analyzer) plus `live_mandatory_loop_winner` for two drain cascades.
  `grep -c "WaitingFor::LoopShortcut" crates/engine/src/analysis/corpus.rs` == **0**.
  **Not one corpus row exercises the interactive offer path.** The corpus is *structurally incapable* of
  catching the bug this document is about.

**RC-4 — the COVER layer is a sibling cluster, so every new combo family costs a new cover function.**
Four `loop_states_cover_modulo_*` variants each re-derive the same four invariants around a different
exemption (`resource.rs:784 / 924 / 1095 / 1326`). That is why **37 of 53 corpus rows are deferred**,
and the tree's own `DeferralBucket` says so. The largest bucket, **`ObjectReentry` (13 rows)**, is
**the canonical list of Magic's most famous infinite combos** — Kiki-Jiki, Splinter Twin, Palinchron,
Mikaeus, Dockside, Food Chain, Karmic Guide, Nim Deathmantle, **Earthcraft + Squirrel Nest** — all
blocked by one thing: **ObjectId churn**, which is *rules-wrong* under **CR 400.7** (an object that
changes zones **is a new object**) and which `fodder_content_eq` **already solves** on one path.
**Fixing it is a parameterization, not a feature.** See §4.10 — **this is the highest-leverage change
in the document and it outranks Phases 1–5.**

**RC-3 and RC-4 are the reason this must not become a two-combo patch.** The four checks are already
card-agnostic and `ResourceAxis` already carries ~10 ω-axes; **the narrowness is entirely in the arming
(RC-3) and the covers (RC-4).** See §4.8 and §4.10.

**CI is green because the acceptance fixture cannot exist in a real game.** `sprout_swarm_scenario`
(`loop_shortcut.rs:2536`) builds a board with no lands, an empty library, no auras, and a stub
Witherbloom oracle. RC-1 and RC-2 are both invisible to it — and RC-3 means *nothing anywhere* is
looking.

**Two findings beyond the false negatives, both derived from the rules rather than from the bug:**

1. **The all-zones scan is illegal twice over.** Primarily by **CR 113.6** — *"Abilities of all other
   objects usually function only while that object is on the battlefield"* — a Solemn Simulacrum in
   the library **has no functioning ability at all**, so scanning it is not conservatism, it is reading
   an ability that does not exist. Secondarily by **CR 400.2**: library and hand are **hidden zones**,
   so the verdict is a function of information no player may act on.
2. **The detector asks a question the rules do not.** It tries to prove *"no ability anywhere could
   ever observe this growth."* **CR 732.2a** asks only whether a sequence *"may be legally taken based
   on the current game state and the predictable results of the sequence of choices"*, and **CR 732.2b**
   gives every other player the right to **accept or shorten**. Interaction is the response window's
   job, not the cover's.

---

## 2. Reproduction

- **Fixture:** `crates/engine/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json`
  (debug-panel export; wrapped `{gameState, waitingFor, legalActions, turnCheckpoints}`).
- **Harness:** `crates/engine/tests/integration/repro_user_combo.rs` — `serde_json` → `GameState` →
  `layers::flush_layers` → `GameRunner::from_state`, then drives the same cast the synthetic test drives.
- A bare snapshot is insufficient: arming (`last_recast_context`) happens **during a cast**, so the
  repro must drive a real cast.

```
cargo test -p engine --test integration real_board_fixture_is_intact   # PASSES (guards the fixture)
cargo test -p engine --test integration -- --ignored real_board        # FAILS (the bug)
```

**Board:** Witherbloom, the Balancer (Legendary) + 4 untapped green Saproling **tokens** + Kilo,
Apogee Mind (Legendary, enchanted by Freed from the Real) + Relic of Legends + Pentad Prism
(1 charge) + Forests/Islands. Sprout Swarm in hand. `Interactive`, `Priority{P0}`, own turn, empty stack.

**Measured after the driven cast:**
```
last_recast_context = Some(RecastContext{card_id:415, controller:0, from_zone:Hand,
                                         uses_buyback:Used, convoke:Some(Convoke)})   ← ARMING CORRECT
waiting_for         = Priority{0}                                                      ← NO OFFER
saprolings          = 4 → 5                                                            ← the cast worked
```
Every cheap gate at `engine.rs:445` is green. The decline is downstream, in the cover.

### 2.1 The two combos — Oracle text verified in `data/card-data.json` (engine-planner Step 0)

All text below is **quoted from the shipped card database**, not from memory.

| Card | Oracle text (verbatim) |
|---|---|
| **Sprout Swarm** | Convoke · Buyback {3} · *"Create a 1/1 green Saproling creature token."* |
| **Witherbloom, the Balancer** | *"Instant and sorcery spells you cast have affinity for creatures."* (+ Affinity for creatures, Flying, deathtouch) |
| **Relic of Legends** | *"{T}: Add one mana of any color."* · *"**Tap an untapped legendary creature you control**: Add one mana of any color."* |
| **Kilo, Apogee Mind** | *"**Haste.** Whenever Kilo becomes tapped, proliferate."* |
| **Freed from the Real** | Enchant creature · *"{U}: Tap enchanted creature."* · *"{U}: Untap enchanted creature."* |
| **Pentad Prism** | Sunburst · *"Remove a charge counter from this artifact: Add one mana of any color."* |

**Combo A — Witherbloom + Sprout Swarm (object growth).** Worked through the casting rules:
- **CR 601.2b** — announce the optional additional cost (**buyback {3}**) ⇒ base {1}{G} + {3} = {4}{G}.
- **CR 601.2f** — **CR 702.41a** affinity (*"costs {1} less to cast for each [text] you control"*) is a
  cost **reduction**; with ≥4 creatures the {4} generic goes to {0}. **The total cost then LOCKS IN**
  (*"If effects would change the total cost after this time, they have no effect."*). Remaining: **{G}**.
- **CR 601.2h** — **CR 702.51b**: *"The convoke ability isn't an additional or alternative cost and
  applies only after the total cost of the spell with convoke is determined."* ⇒ convoke is a **payment
  substitution**: tap one untapped **green** creature for the {G}.
- Resolve: create a **green, untapped** Saproling; buyback returns the card to hand.

⇒ **Δ(untapped green creatures you control) = −1 (convoked) + 1 (new green untapped token) = 0.**
⇒ **Δ(creatures you control) = +1**, so affinity only gets *stronger* — the cost stays {G} forever.
**The loop is legal for all N, and the certifiable unbounded axis is creatures/tokens.**

**Combo B — Kilo + Freed from the Real + Relic of Legends → Pentad Prism (counter growth).** The whole
cycle is **one activation**, which the rules make explicit:
- Activate **Freed from the Real**'s *"{U}: Untap enchanted creature"* (**CR 602.2b** ⇒ follow 601.2b–i).
- **CR 601.2g / CR 605.3a** — *"A player may activate an activated mana ability … whenever they are
  casting a spell or activating an ability that requires a mana payment … even if it's in the middle
  of … activating … an ability."* **CR 605.1a** makes Relic's second ability a **mana ability** (no
  target, adds mana, not loyalty). So *inside Freed's cost payment* we activate Relic, **tapping Kilo**.
- Kilo *"becomes tapped"* ⇒ **proliferate triggers**; **CR 603.3b** holds it until a player would next
  receive priority. **CR 601.2h** pays the {U}.
- Priority: the proliferate trigger resolves (+1 charge counter on Pentad Prism), then Freed's ability
  resolves (**Kilo untaps**). Kilo has **haste**, so **CR 302.6** is moot regardless.

⇒ **Δ(mana) = 0, Δ(Kilo tapped) = 0, Δ(charge counters) = +1.** Unbounded counters ⇒ unbounded mana
(Pentad Prism's removal ability is itself a **CR 605.1a** mana ability).

> **Why counters and not mana are the certified axis.** **CR 106.4 / CR 500.5**: *"any unspent mana left
> in a player's mana pool empties"* at the end of each step and phase. **Unbounded mana is not a durable
> resource** — it cannot be the ω-axis of a shortcut that ends at a later priority beat. The durable
> axis is the **charge counters**. This is exactly what the shipped
> `loop_states_cover_modulo_counter_growth` certifies, and it is why adding a "mana growth" axis would
> be *wrong*.

---

## 3. Root cause

### 3.1 RC-1 — the observer firewall (`fire_time_conditions_read_growing_class`, `resource.rs:1468`)

| Defect | Measured | Fatal because |
|---|---|---|
| **R1** Gate (1) trigger scan is all-zones (`active_trigger_definitions`, `functioning_abilities.rs:391`) | trip: `Solemn Simulacrum (Library)` | **CR 113.6** — the ability doesn't function there at all; **CR 400.2** — hidden zone |
| **R2** `Effect::Mana { .. } => Axes::CONSERVATIVE` (`ability_scan.rs:852`) | trip: `Forest` | certification requires a board with **zero mana sources** |
| **R3** Gate (2) scans **activated** ability bodies (`resource.rs:1510`) | trip: `Freed from the Real` | an activated ability observes nothing unless *activated*; the fixed sequence pins whether it is |
| **R4** Gate (3) `active_replacements` is all-zones | reached | same CR 113.6 / 400.2 class as R1 |
| **R5** Gate (4) blanket `if !def.modifications.is_empty() { return true }` (`resource.rs:1539`) | not yet reached | any anthem/aura/equipment |
| **R6** Gate (6) rejects on ANY non-empty `delayed_triggers` (`resource.rs:1582`) | not yet reached | **every Kiki-Jiki token** carries *"Sacrifice it at the beginning of the next end step"* |

Gate (2) *is* correctly battlefield-scoped — **the inconsistency between gates is the tell.**
**R2 is already fixed and committed** (`scan_mana_production`; §6 Phase 1).

**The catch-22.** In **both** combos the ability that *drives* the loop reads the growing axis:
affinity reads the **creature count** (the growing class **is** creatures); proliferate reads
*"permanents with counters"* (the growing axis **is** counters). A firewall phrased *"reject if any
live ability reads the growing class"* is **structurally incompatible with self-referential engines**
— i.e. with most real combos. **The predicate is wrong, not mistuned.**

### 3.2 RC-2 — the cover forbids a bounded start-up transient (`engine.rs:1728`)

The drive produces **three frames** and requires the cover on **both** pairs:
```rust
loop_states_cover_modulo_fodder_growth(&cs_n,  &cs_n1, &fodder)   // ← FAILS on any real board
&& loop_states_cover_modulo_fodder_growth(&cs_n1, &cs_n2, &fodder)
```
`select_convoke_taps` (`mana_payment.rs:394`) does `candidates.sort_by_key(|id| id.0)` and re-runs
**per iteration**. On the real board the first driven iteration therefore taps **Witherbloom (402)**,
not a Saproling — flipping a **non-token, non-fodder legendary creature** from untapped to tapped.
That is not "inert tapped-fodder growth", so `(cs_n, cs_n1)` **cannot cover**.

**This is a bounded transient, not a leak.** Witherbloom is tapped exactly **once**, and there is no
untapper — so from iteration 2 onward only Saprolings are convoked and the recurrence is exact:

| after | untapped green | tapped fodder | total creatures |
|---|---|---|---|
| real cast | 5 | 0 | 6 |
| drive iter 0 (taps **Witherbloom**) | 5 | 0 (+1 tapped **non-fodder**) | 7 |
| drive iter 1 (taps S1) | 5 | 1 | 8 |
| drive iter 2 (taps S2) | 5 | 2 | 9 |

**The untapped-green count is invariant at 5.** The loop is sound; only the **first** driven pair is
transient. The engine measures exactly the wrong pair.

> **This vindicates the instinct that the payment choice must not matter** and that *"the untapped
> token only happens for a number of times equal to the [non-token] green creatures … so that itself
> is not unbounded."* That is precisely a **bounded transient**. The earlier "bias the convoke selector
> toward fodder" idea was treating the symptom, and is correctly abandoned — see Appendix B, false
> claim #4.

### 3.3 Combo B is additionally blocked by the ring — but the ring is **not** what to fix

**`engine.rs:3081`:**
```rust
if !matches!(action, GameAction::PassPriority | GameAction::OrderTriggers { .. }) {
    state.loop_detect_ring.clear();   // "cast/activate/play-land is a deliberate break"
}
```
Combo B is driven by **activating an ability**, so the ring is cleared every iteration and never fills.

**But §2.1 showed Combo B's cycle is exactly ONE activation** — structurally identical to Combo A's
one cast. So the fix is **not** to weaken the ring (that is the **DoS guard**, commit `57b0e537d`,
*"bound loop-shortcut iteration count (remote DoS in #5672)"*). The fix is to **arm on activation the
same way we arm on cast**: a `last_activation_context` sibling of `last_recast_context`, feeding the
same drive. See §6 Phase 5.

> **A counter-growth cover ALREADY EXISTS.** `loop_states_cover_modulo_counter_growth`
> (`resource.rs:1326`) covers strict `Generic`-counter growth; its doc names *"the proliferate/charge
> (**Pentad Prism**) … ω-cover shape"*; it is wired into `detect_loop` (`loop_check.rs:230`) **and**
> `interactive_loop_bridge` (`engine.rs:632`) with four discriminating tests. **Build nothing here.**
> It is simply never *consulted*, because nothing ever arms.

---

## 4. Architecture — the fixed-sequence formulation

### 4.1 CR 732.2a fixes the player's choices. That is the whole design.

> **CR 732.2a** *(verbatim)*: *"the player with priority may suggest a shortcut by **describing a
> sequence of game choices**, for all players, that **may be legally taken based on the current game
> state and the predictable results of the sequence of choices**. This sequence may be **a
> non-repetitive series of choices, a loop that repeats a specified number of times, multiple loops, or
> nested loops**, and may even cross multiple turns. **It can't include conditional actions**, where
> the outcome of a game event determines the next action a player takes. **The ending point of this
> sequence must be a place where a player has priority**…"*

Five load-bearing deductions, each of which changes the code:

- **D1 — a shortcut IS a straight-line action sequence, by rule.** No conditionals ⇒ the proposer
  commits to which creature to convoke, which source to tap, which target to pick. The question is not
  *"is this board a linear program?"* — that is ill-posed (Priest of Titania's `{T}: Add {G} for each
  Elf` is constant-Δ in its own loop and non-constant beside an Elf-token maker). The question is:

  > ## **Is this FIXED sequence legally repeatable forever, with constant Δ?**

- **D2 — "a loop that repeats a specified number of times."** The proposer names **N**, and the
  proposal must be legal *"based on … the predictable results"* — i.e. **legal for every one of the N
  iterations.** ⇒ **precondition non-depletion (C2) IS CR 732.2a**, not an engineering add-on. A
  sequence that becomes illegal at iteration 4 is not a legal proposal for N = 10⁶.

- **D3 — "a non-repetitive series of choices, [or] a loop that repeats…"** ⇒ **a shortcut may be a
  non-repetitive PREFIX followed by a loop.** The engine's demand that the loop cover from iteration 0
  is **stricter than the rule**, and it is **RC-2**. Real boards almost always have a finite pool of
  non-fodder engine pieces that the first few iterations consume once each.

- **D4 — "the ending point … must be a place where a player has priority."** The iteration boundary is
  a priority beat **by rule** — which is exactly the empty-stack `Priority` settle condition the drive
  already uses. It also **forbids** a mid-resolution loop boundary. Nothing to change; now it is
  *justified* rather than incidental.

- **D5 — "multiple loops, or nested loops, and may even cross multiple turns" are LEGAL.** Excluding
  them (§4.6) is an **engineering** decision, not a rules one. Say so honestly; do not dress a scope
  cut as a rules constraint.

**And CR 732.2a's own worked example is an object-growth loop** — Presence of Gond (*"Enchanted creature
has '{T}: Create a 1/1 green Elf Warrior creature token.'"*) + Intruder Alarm (*"Whenever a creature
enters, untap all creatures."*), *"I'll create a million tokens."* **The rulebook certifies the exact
class we are failing to detect.** It must be an acceptance fixture (§7).

### 4.2 Two more rules that prune the design

- **CR 732.4 + CR 104.4b** — *"If a loop contains only mandatory actions, the game is a draw… Loops that
  contain an optional action don't result in a draw."* Our loops contain the proposer's **optional**
  cast/activation ⇒ **never a draw** ⇒ the engine **offers**. **This is already implemented**
  (`no_living_player_has_meaningful_priority_action`, `engine.rs:1765`). Don't rebuild it. CR 732.5 and
  CR 732.6 govern only *mandatory* loops and are **out of scope entirely**.

- **CR 732.3 — fragmented loops.** *"each player involved in the loop performs an independent action
  that results in the same game state being reached multiple times … the active player … must then make
  a different game choice so the loop does not continue."* ⇒ **a sequence whose repetition requires an
  OPPONENT to take a non-pass action is broken by rule and must be rejected.** Our sequences require
  opponents only to *pass priority* — and CR 732.2b already gives them the right to decline that (the
  "shorten" right), which is the response window, not the cover's problem. **This is a new soundness
  constraint the detector does not currently express.**

### 4.3 The choice vector is enumerable — straight out of CR 601.2 / 602.2

**CR 601.2** lists *every* choice made while casting, and **CR 602.2b** says activating an ability
follows **601.2b–i** identically. So the set of things a fixed sequence must pin is **closed and
checkable**, not invented:

| CR | Choice to pin |
|---|---|
| 601.2b | mode · splice · **optional additional/alternative costs (buyback!)** · **X** · hybrid · Phyrexian |
| 601.2c | **targets** (and, if variable, the *number* of targets) |
| 601.2d | division / distribution |
| 601.2f | order of applying cost **reductions** |
| 601.2g | **which mana abilities to activate** (Relic tapping Kilo — CR 605.3a) |
| 601.2h | **payment choices — including convoke's tap-set** (CR 702.51b) |

**Measured gap:** today `build_recast_template` emits `template.decisions == [ConvokeTaps]` and nothing
else (`engine.rs:1771` comment: *"`[ConvokeTaps]` when the recast has convoke, else `[]`"*). Against the
CR 601.2 enumeration that is **incomplete** — modes, X, targets, and mana-ability selection are
unpinned. Any loop whose body makes one of those choices is either mis-driven or silently
non-deterministic. **`DecisionTemplate` completeness must be audited against this table** (§6 Phase 5).

### 4.4 Every failure mode collapses into one: *the fixed sequence becomes ILLEGAL*

All card text verified in `data/card-data.json`.

| Case | The place the sequence draws from | Δ(place) | Verdict |
|---|---|---|---|
| **Sprout Swarm** — convoke (CR 702.51b, a **payment**, no `{T}` on the creature) | untapped **green** creature you control | −1 + 1 (**token is green & untapped**) = **0** | **ACCEPT** ✅ |
| **Earthcraft** — *"**Tap an untapped creature you control**: Untap target basic land."* → **the cost is on Earthcraft's own ability; NO tap symbol ⇒ CR 302.6 does not apply ⇒ a summoning-SICK Squirrel is legal fodder** | untapped creature (sick or not) | −1 + 1 = **0** | **ACCEPT** ✅ (Earthcraft + Squirrel Nest) |
| **Cryptolith Rite** — *"Creatures you control have **'{T}: Add one mana of any color.'**"* → **the creature's OWN `{T}` ability ⇒ CR 302.6 DOES apply** | **unsick** untapped creature | −1 + 0 (**new token is sick**) = **−1** | **REJECT** ✅ |
| **Presence of Gond + Intruder Alarm** (**CR 732.2a's own example**) — `{T}` on the creature ⇒ CR 302.6 applies; Intruder Alarm untaps it | **unsick** untapped enchanted creature | −1 + 1 (**untapped by the trigger**) = **0** | **ACCEPT** ✅ |
| **Manaforge Cinder** — *"{1}: Add {B} or {R}. **Activate no more than three times each turn.**"* | activations remaining (`MaxTimesEachTurn{3}`) | −1 + 0 = **−1** | **REJECT** |
| **Crucible of Worlds + Zuran Orb** | land plays remaining (**CR 305.2**) | −1 | **REJECT** |
| **Basalt Monolith + Mesmeric Orb** (Four Horsemen **minus Emrakul** — deterministic) | cards in library (**CR 104.3c / 704.5b**) | −n | **REJECT** |
| **Hum of the Radix** — *"costs {1} more for each artifact its controller controls"* | — | cost RISES at **CR 601.2f** each iteration ⇒ **Δ₁ ≠ Δ₂** | **REJECT** |
| **Solemnity** + proliferate | — | **measured** Δ = 0 counters ⇒ no progress | **REJECT** |

**CR 302.6** *(verbatim)*: *"A creature's activated ability **with the tap symbol or the untap symbol in
its activation cost** can't be activated unless the creature has been under its controller's control
continuously since their most recent turn began."*

> **Earthcraft vs Cryptolith Rite is the whole design in one pair.** Same board shape, same "tap a
> creature for value" idiom, **opposite verdicts**, and the discriminator is **the shape of the cost in
> the Oracle text** — a `{T}` on the creature's own ability (CR 302.6 applies) versus "tap a creature"
> as a cost on *another* permanent's ability (CR 302.6 does not). It is **not** a resource level and it
> is **not** card-specific. One predicate, three places, four correct verdicts. **That is "build for the
> class."**

### 4.5 The unification: legality gates ARE consumables

*"3 activations left this turn"*, *"1 land drop left"* (**CR 305.2**), *"unsick creatures"* (**CR 302.6**),
*"cards in library"* (**CR 704.5b**), *"loyalty activations"* (**CR 606.3**) are **resources the fixed
sequence spends**. This folds every non-resource legality gate into the *same* sustainability check.

The engine already knows this. `project_out_resources` (`resource.rs:2500+`) **deliberately preserves**
`activated_abilities_this_turn` / `_this_game`, `OncePerTurn` / `MaxTimesPerTurn` trigger limits,
`crew_activated_this_turn`, and loyalty — its own comment:

> *"blanket-clearing them would erase the gate that makes a once-per-turn … ability NON-repeatable,
> **falsely certifying it as infinite**."*

**Single authority:** `ability_has_per_turn_activation_gate` (`resource.rs:2842`).

### 4.6 The four checks

| # | Check | Catches | Status |
|---|---|---|---|
| **C1** | **Δ-constancy** across two **post-transient** pairs | scaled costs (Hum of the Radix, Damping Sphere) | drive exists; **must skip the transient prefix** (RC-2) |
| **C2** | **Place non-depletion** — every place the pinned sequence draws from is non-decreasing under its own Δ | activation limits, land drops, summoning sickness, self-mill | ⚠️ **THE ONLY GENUINELY NEW CODE** |
| **C3** | **Threshold scan** — a `Comparator` against a growth axis | board thresholds that fire **outside** the fixed choices | **exists**; needs only re-**scoping** |
| **C4** | **The shipped triple** — `net_progress_for(caster)` + `has_no_loss_axis(delta)` + `driving_resources_non_decreasing(..)` | self-deck, self-damage, adverse opponent scaling | **exists, unchanged** (`engine.rs:1756`) |

**C3 is the one arm three adversarial rounds never broke.** Every hole routed *around* it — into costs,
activation restrictions, replacements, per-object attributes, and turn-based limits. **Keep its logic;
fix only its scope.**

**Why measurement, not derivation.** Δ **cannot** be derived from the AST: **replacement effects rewrite
it at resolution** (Solemnity's two `Prevent` replacements on `AddCounter` turn proliferate's AST-Δ of
`+1 counter` into a true Δ of **0**). And **CR 704.3 / CR 603.3b** put a full SBA + trigger settle between
iterations, so a loop that kills its own engine (0 toughness, legend rule, controller at 0 life) simply
*fails to recur* and is caught for free. Because C1/C2/C4 read Δ from the **clone-drive**, all of this is
handled without a symbolic model. **The drive is the authority; nothing replaces it.**

### 4.8 Where the generality actually lives — and where it does not (RC-3)

Measured, layer by layer. **This table is the anti-purpose-built audit, and it says the fix belongs in
exactly one layer.**

| Layer | How general is it today? | Evidence |
|---|---|---|
| **ω-axes** (`ResourceAxis`) | **General.** ~10 axes: `TokensCreated`, `CardsDrawn`, `Casts`, `LandfallTriggers`, `CombatPhases`, `ExtraTurns`, `Death/Etb/Ltb/Sac` triggers, poison, … | `analysis/resource.rs` |
| **The four checks** (C1–C4) | **General — card-agnostic by construction.** They read a *measured* Δ; they never look at a card name. | `engine.rs:1756` |
| **Covers** | **Semi-general.** Four exist: `loop_states_cover_modulo_{growth, object_growth, fodder_growth, counter_growth}` | `resource.rs:784 / 924 / 1095 / 1326` |
| **Arming (live path)** | ⛔ **ONE bespoke shape.** `last_recast_context` = a buyback-paid, token-creating recast. | `casting_costs.rs:6785` |
| **Corpus coverage of the live path** | ⛔ **ZERO of 53 rows.** All go through offline `detect_loop`. | `grep -c "WaitingFor::LoopShortcut" corpus.rs` == 0 |

**Therefore: the general pattern is not "detect combo X." It is —**

> **At any empty-stack priority beat following a player action, ask whether repeating that FIXED
> action is legal forever with constant Δ.** Which card produced the action is irrelevant, and no
> layer below arming needs to know.

**The `last_recast_context` / `last_activation_context` sibling pair is the wrong answer** — it is the
exact sibling-cluster smell CLAUDE.md prohibits (*"three or more variants that … differ only in a
context label … is a parameterization that didn't happen"*). Two shapes today become five tomorrow.
**Parameterize on the action; do not proliferate contexts.** See Phase 5.

### 4.9 The DUAL — and the abstraction is already in the tree

**Do not write a second suite. Write the dual of the one that exists.** `corpus.rs:1175` already carries
the exact abstraction:

```rust
/// `step` drives exactly ONE loop iteration's actions.
pub(crate) fn run_combo<S: FnMut(&mut LoopProbe)>(board: ComboBoard, mut step: S)
    -> Option<LoopCertificate>
{
    const WARMUP: usize = 2;                    // ← see (2) below
    const STEADY: usize = 3;
    for _ in 0..WARMUP { step(&mut probe); … }  // burn the transient
    for _ in 0..STEADY {
        let start = …; step(&mut probe); let delta = probe.iteration_delta(); let end = …;
        if let Some(cert) = detect_loop(&start, &end, &delta, controller, false) { return Some(cert) }
    }
    None
}
```

**`step` IS the CR 732.2a fixed sequence.** A human writes it per row; `detect_loop` merely *judges* it.
The live path gets no such gift — it must **discover** the same cycle from arming. That asymmetry is the
entire bug, and it names the dual exactly:

| | who supplies the cycle | who judges | today |
|---|---|---|---|
| **`run_combo`** (offline) | the **test author** (`step`) | `detect_loop` | 12 rows drive |
| **`run_combo_live`** (**the DUAL — to build**) | **the ENGINE must discover it** | `WaitingFor::LoopShortcut` | **0 rows** |

> ## The duality invariant — this is the pattern
> **Same `ComboRow`. Same `ComboBoard`. Same `step` closure. Two observers.**
> For every row whose cycle contains an **optional player action** (CR 732.4 / CR 104.4b — every
> non-drain row), driving `step` through the **real `apply()` reducer** must OFFER **iff** `detect_loop`
> certifies:
>
> - **certifies-offline ∧ ¬offers-live** ⇒ **RC-3** — a false negative in real play. **Today this is
>   ALL 12 driven rows**, including **row 1 = Kilo + Freed + Relic** — *the corpus already certifies
>   Combo B offline and has never once offered it live.*
> - **offers-live ∧ ¬certifies-offline** ⇒ **UNSOUNDNESS** — the live path certifies something the
>   analyzer rejects. Catastrophic; this direction is why the dual must be a **bi-implication**, not a
>   one-way "does it offer" check.
>
> **Zero duplication.** `ComboDriver::Offline(f)` becomes a route-agnostic `Cycle(f)` driven by **both**
> routes; `DRIVERS` (`corpus.rs:673`) stays the single source of truth, and its existing meta/partition
> tests extend to the dual for free.

**Three things this reveals, all measured:**

**(1) The offline harness already tolerates the transient — the live path does not.** `WARMUP = 2`
(`corpus.rs:1180`) burns two cycles *before* measuring. That is **independent confirmation of RC-2**
from the tree's own harness: the offline route was built knowing loops have a bounded start-up
transient, and the live route's two-pair cover requirement (§3.2) forgot.

**(2) `step` can be MULTI-ACTION — so `LoopProbeContext` must carry a SEQUENCE, not an action.**
`drive_offline_devoted_vizier` (row 6, Devoted Druid + Vizier of Remedies) drives **two activations per
cycle**. **CR 732.2a says "a sequence of game choices" — plural.** A `LoopProbeContext { action }` is
therefore wrong on both the rules and the evidence. It must be `{ actions, controller, decisions }`.
This corrects Phase 5 and closes Open Question #8.

**(3) The honest ceiling, already measured by the tree itself.** Only **12 of 53** rows drive; **4** are
card-gated; **37** carry a `DeferralBucket` — *the tree's own accounting of what the detector cannot do*:

| `DeferralBucket` | rows | what it means |
|---|---|---|
| `ObjectReentry` | **13** | a permanent that dies/blinks/bounces returns with a **fresh `ObjectId`**, so id-keyed loop equality sees a different board |
| `Other` | 20 | no bespoke driver on today's in-place loop model |
| `ExtraTurnOrCombat` | 3 | each cycle advances `turn_number` ⇒ not board-identical |
| `ColorConverting` | 1 | per-color net-progress rule rejects it |

**`ObjectReentry` (13 rows — the single largest bucket) is almost certainly already solvable.**
`normalize_recast_frame` (`engine.rs:1724`) exists precisely to *"clear churning token-id bookkeeping
(CR 400.7)"* on the recast path. **Generalizing that normalization is the highest-leverage coverage win
in the entire corpus** — and it is a *pattern* fix (object identity across a loop cycle), not a card fix.
**Measure it in Phase 0; it may be worth more than Phases 1–5 combined.**

### 4.10 RC-4 — the COVER layer is a sibling cluster, and it is why 37 of 53 rows are deferred

**This is the think-ahead section. Everything above fixes the two combos in front of us; this is what
stops us being stuck here again.**

There are **four** cover functions, and they are the textbook sibling cluster CLAUDE.md warns about —
each is *"board equal **modulo** ⟨a different growing thing⟩"*, and each **re-derives the same four
invariants** (fire-time observer firewall, cost-surface scan, stack embedding, inertness) around a
different exemption:

| function | quotients out | `resource.rs` |
|---|---|---|
| `loop_states_cover_modulo_growth` | a narrowed projection + stack growth | 784 |
| `loop_states_cover_modulo_object_growth` | an unobserved object class | 924 |
| `loop_states_cover_modulo_fodder_growth` | an inert **fungible token class** | 1095 |
| `loop_states_cover_modulo_counter_growth` | the `Generic` **counter** class | 1326 |

**⇒ Every new combo family today costs a new cover function.** That is exactly why **37 of 53 corpus
rows are deferred**, and the tree says so itself in `DeferralBucket`.

**And the machinery to fix the biggest bucket is already written.** `loop_states_cover_modulo_fodder_growth`'s
own doc (`resource.rs:1095`):

> *"`fodder_class` is a **CONTENT authority** … compared LIVE each call via `fodder_content_eq` (modulo
> tapped) — **not latched by ObjectId, because fodder tokens are not id-stable**. Covers any inert
> fungible token class (Saproling, Elf Warrior, Thopter, …), **so it builds for the class not a card**."*

Meanwhile `DeferralBucket::ObjectReentry` is defined as: *"a permanent that dies/blinks/bounces … gets a
FRESH `ObjectId` each cycle, so the **id-keyed** per-object loop equality sees a different board."*
**Content-not-ObjectId comparison already exists — it is simply welded to the fodder path.** This is
also just **CR 400.7**: an object that changes zones **is a new object**. Id-keyed loop equality is
therefore *rules-wrong*, not merely limited.

**What that bucket actually contains — the 13 most famous infinite combos in Magic:**

> Kiki-Jiki + Zealous Conscripts · Splinter Twin + Deceiver Exarch · Palinchron + Deadeye Navigator ·
> Mikaeus + Triskelion · Dockside Extortionist + Temur Sabertooth · Food Chain + Eternal Scourge ·
> Karmic Guide + Reveillark + Viscera Seer · Reassembling Skeleton + Ashnod's Altar + Nim Deathmantle ·
> Gravecrawler + Phyrexian Altar + Blood Artist · Felidar Guardian + Saheeli · Scurry Oak + Ivy Lane
> Denizen · Midnight Guard + Presence of Gond · **Earthcraft + Squirrel Nest**

⚠️ **Note the last two.** *Midnight Guard + Presence of Gond* is a sibling of **CR 732.2a's own worked
example**, and **Earthcraft + Squirrel Nest is a hostile fixture in this very plan (§4.4)** — I proposed
it as a *positive* acceptance case without noticing it is **deferred and undetectable**. That is the
purpose-built failure mode catching me in my own document.

#### The pattern: ONE quotient relation, not N covers

```rust
/// CR 732.2a: a loop recurs iff the board is equal MODULO a set of quotients, each of
/// which is a monotone non-decreasing ω-axis. Adding a combo FAMILY = adding a
/// `Quotient` variant + its monotonicity proof — NOT a new cover function.
fn loop_states_cover(prior: &GameState, current: &GameState, q: &[Quotient]) -> bool;

enum Quotient {
    /// CR 400.7: an object that changes zones IS A NEW OBJECT. Compare by CONTENT
    /// class, not ObjectId. Machinery exists: `fodder_content_eq` (resource.rs:1095).
    ObjectIdentity,
    /// Inert fungible object growth (tokens).            → fodder / object growth
    ObjectCount { class: ObjectClass },
    /// Counter growth (Generic: charge / burden).        → counter growth
    CounterCount { kind: CounterKind },
    /// CR 732.2a: a shortcut "may even cross multiple turns".
    /// `ResourceAxis::{ExtraTurns, CombatPhases}` ALREADY EXIST as ω-axes.
    TurnCount,
    CombatCount,
    /// CR 106.4 / CR 500.5: the mana pool empties at end of step — never a durable
    /// residual, so it can always be quotiented out at a step boundary.
    ManaPool,
}
```

**Certification rule.** A loop certifies iff **∃** a quotient set **Q** such that
**(1)** states are equal modulo **Q**; **(2)** every quotiented axis is **monotone non-decreasing**
across the cycle (that is what makes it the ω-axis and not a leak); **(3)** no live observer reads a
quotiented axis (the firewall — now **CR 113.6**-scoped per Phase 1); **(4)** C1–C4 pass. The four
shared invariants factor out **once** instead of being re-derived per cover.

**Measured payoff against the corpus:**

| `Quotient` | unlocks | rows |
|---|---|---|
| `ObjectIdentity` (CR 400.7) | **the entire `ObjectReentry` bucket** | **13** |
| `TurnCount` / `CombatCount` (CR 732.2a *"may cross multiple turns"*) | `ExtraTurnOrCombat` | **3** |
| `ManaPool` (CR 106.4) | `ColorConverting` (Pili-Pala + Grand Architect — restricted-mana accounting) | 1 |
| Phase 5's `{ actions }` **sequence** | the multi-action rows inside `Other` (Basalt Monolith + **Rings of Brighthearth**, Dramatic Reversal + Isochron Scepter, Dualcaster Mage + Twinflame, …) | **≤20, measure** |

**⇒ ~17 of 37 deferrals fall out of ONE parameterization**, before touching the `Other` bucket.

> **This is the "not stuck like this later" contract.** A new combo family costs **one `Quotient`
> variant + its monotonicity proof** — *not* a new cover function, *not* a new arming context, *not* a
> new bespoke driver, *not* a new `DeferralBucket`. Run the **`/add-engine-variant`** gate on `Quotient`
> and it becomes a checklist, not an archaeology expedition.

### 4.7 Explicitly OUT of scope — an engineering cut, NOT a rules constraint (see D5)

- **Nested / multiple loops** and **turn-crossing loops** (Time Vault). **CR 732.2a permits all three.**
  We exclude them because they are rare in real play and expensive to certify; the untap step
  (**CR 502.3** — *"the active player determines which permanents they control will untap. Then they
  untap them all simultaneously"*) has a marking-dependent Δ, and a turn boundary resets the per-turn
  tallies C2 depends on.
- **Special actions** (**CR 116.1**) and land plays, unless `lands_played_this_turn` is modelled as a C2
  precondition (§6 Phase 3).
- **Venture / dungeon** (CR 309) — no axis exists in `ResourceVector`; **Acererak the Archlich** is a real
  EDH loop.

**Every one of these must DECLINE LOUDLY with a logged reason.** A silent decline is indistinguishable
from the bug we are fixing.

---

## 5. Architectural questions (engine-planner Step 4)

**Pattern coverage.** Not one card, not two: the change governs **every** CR 732.2a shortcut. C2 covers
the whole class of *repetition-blocking legality gates* (activation limits, land drops, summoning
sickness, loyalty, crew, per-turn trigger caps). The corpus is **55 combos** (`analysis/corpus.rs`), and
Combo B is already in it (*"Kilo, Apogee Mind + Freed from the Real + Relic of Legends"*, family
`Proliferate`).

**Building blocks (compose; do not re-create).**
- `ability_has_per_turn_activation_gate` (`resource.rs:2842`) — **the single authority** for per-turn gates.
- `project_out_resources` (`resource.rs:2500`) — already preserves the tallies C2 needs.
- `net_progress_for` / `has_no_loss_axis` / `driving_resources_non_decreasing` — C4, untouched.
- `no_living_player_has_meaningful_priority_action` (`engine.rs:1765`) — the **CR 104.4b** optional-loop
  gate. **Already correct.**
- `battlefield_active_triggers` (`functioning_abilities.rs:416`) — the correctly-scoped authority R1 must use.
- `ability_scan::Axes` walk (`sibling`, **`projected`**) — C3's existing engine. **`projected` must be
  preserved**: it is what catches `ModifyCost{dynamic_count}` (Damping Sphere, Hum of the Radix), guarded
  by the in-tree test `R-e2` (`resource.rs:5052`).
- `drive_recast_iteration` (`engine.rs:1469`) + `normalize_recast_frame` + `derived_fodder_class` — the
  measurement authority for C1.
- `loop_states_cover_modulo_counter_growth` (`resource.rs:1326`) — **Combo B's cover already exists.**
- `object_content_eq` / `_gameobject_partition_is_total` / `_gamestate_partition_is_total` — the
  compiler-enforced totality guards that keep the residual-equality check honest.

**Logic placement.** C1/C2/C4 are **analysis** (`crates/engine/src/analysis/`). C3 is an **ability scan**
(`game/ability_scan.rs`). Zone-of-function is a **rules** predicate (`game/functioning_abilities.rs` — the
module is literally named for CR 113.6). No frontend change: the offer already renders
(`LoopShortcutModal.tsx`, mounted at `GamePage.tsx:1710`).

**Rust idioms.** C2's gate set is a **typed enum**, exhaustively matched, with an explicit `_ => REJECT`
default and a no-`..` totality guard so a new `ActivationRestriction` / `Cost` / turn-based limit
**build-breaks** rather than silently failing open (precedent: `_gameobject_partition_is_total`). The
transient-prefix bound (§6 Phase 2) is an `Option<NonZeroU32>`, never a bare `usize` sentinel.

**Extension vs creation.** Extension throughout. C3 keeps its predicate and changes scope. C4 is
untouched. C1 compares deltas the drive already produces. Combo B reuses `drive_recast_iteration` via a
new arming context. **Only C2 is new.**

**Variant discoverability.** If C2 or the activation context introduces an enum variant, run the mandatory
**`/add-engine-variant`** gate and grep `data/engine-inventory.json` first.

**Analogous trace.** Traced the object-growth detector end-to-end: `casting_costs.rs:6785` (arming) →
`engine.rs:445` (offer gate) → `engine.rs:1648` (`try_offer_object_growth_shortcut`) → `engine.rs:1469`
(`drive_recast_iteration`) → `engine.rs:1728` (the two-pair cover) → `resource.rs:1095`
(`loop_states_cover_modulo_fodder_growth`) → `resource.rs:1468` (the firewall) → `engine.rs:1756` (the
shipped triple) → `engine.rs:1765` (CR 104.4b) → `LoopShortcutModal.tsx`. **Phase 5 mirrors this trace
for activations.**

**Identity / provenance contract.** **The pinned choice must be a PLACE, not an ObjectId.** CR 732.2a asks
for *"a sequence of game choices"* describable per-iteration — the human proposal is *"convoke a
Saproling"*, a **place**. Pinning a raw ObjectId is wrong (iteration 2 needs a *different* Saproling);
pinning "any legal choice" is also wrong (that is a **conditional action**, which CR 732.2a forbids, and
it is what lets `select_convoke_taps` silently pick Witherbloom). **The contract: `DecisionTemplate` pins
`(place, deterministic selector)`; C2 requires the place's population to be non-decreasing; the drive
re-resolves the selector each iteration.** The hostile fixture that proves the binding is
**Earthcraft-vs-Cryptolith-Rite** — same selector, different place, opposite verdict.

---

## 6. Implementation plan

Each phase is independently shippable and independently testable.
**Soundness is monotone throughout: C1–C4 can only turn OFFERs into NO-OFFERs.** Phases 1 and 2 are the
two root causes, and **neither alone makes the acceptance test pass** — the implementer should expect the
red test to stay red after Phase 1 and to turn green after Phase 2. **Say so in the PR, or a green-after-
Phase-1 report is a false positive.**

### Phase 0 — `run_combo_live`: the DUAL of the corpus harness (DO THIS FIRST — it is the anti-purpose-built gate)

**This is the single most valuable change in this document.** It must land *before* any fix, because it
is the only thing that can tell you whether a fix generalized. It is not a new suite — it is the **dual**
of `run_combo` (§4.9), sharing the row, the board, and the `step` closure.

1. **Route-agnostify the driver.** `ComboDriver::Offline(f)` → `ComboDriver::Cycle(f)`. `DRIVERS`
   (`corpus.rs:673`) stays the single source of truth; `LiveDrain` stays as-is (mandatory ⇒ CR 732.4).
2. **Build `run_combo_live(board, step) -> Option<LoopShortcutOffer>`** as the mirror of `run_combo`
   (`corpus.rs:1175`): **same `WARMUP`/`STEADY` shape** (the warm-up is what tolerates the bounded
   transient — §4.9(1)), but drive `step` through the **real `apply()` reducer** and observe
   `WaitingFor::LoopShortcut` instead of calling `detect_loop`.
3. **Assert the duality invariant (§4.9) as a BI-IMPLICATION**, per row, for every non-`LiveDrain` row:
   ```
   run_combo(board, step).is_some()  ==  run_combo_live(board, step).is_some()
   ```
   - **Today every driven row fails the ⇒ direction** (12 certify offline, **0** offer live) — including
     **row 1, Kilo + Freed + Relic**, which is *this document's Combo B, already certified offline and
     never once offered live.* That is the RC-3 debt made visible, and it is the non-vacuity proof for
     Phases 1–5.
   - **The ⇐ direction is the soundness guard**: a row that offers live but does not certify offline
     means the live path is certifying something the analyzer rejects. **This direction must NEVER go
     red**, and it is why the dual is a bi-implication and not a one-way "does it offer" check.
4. **Real cards, real libraries, real mana bases.** Add a `GameScenario` builder loading **real Oracle
   text from `card-data.json`** with a real library, and port
   `object_growth_51st_sprout_swarm_covers_and_offers` onto it. **It must FAIL today.**
5. Add **Presence of Gond + Intruder Alarm** — **CR 732.2a's own worked example** — as a first-class row
   with a `step` closure, so it is driven by **both** routes.
6. **Measure the `ObjectReentry` bucket (13 rows — §4.9(3)).** Determine whether generalizing
   `normalize_recast_frame`'s CR 400.7 token-id normalization lifts them. **If it does, that is a larger
   coverage win than Phases 1–5 combined, and it should be re-prioritized ahead of them.**
7. **Review gates.** Reject any combo-detector test with zero lands, an empty library, or a stub oracle.
   **Reject any fix that turns exactly the two combos in this document green and leaves the rest of the
   duality invariant red** — that is a purpose-built patch wearing a plan's clothes.

### Phase 1 — RC-1: zone-scope the observer scans (a RULES fix — ship standalone)

**Primary authority is CR 113.6, not CR 400.2.** A `Solemn Simulacrum` in the library **has no functioning
ability**; scanning it is not conservatism, it is reading an ability that does not exist.

- Gate (1): replace `for obj in state.objects.values() { active_trigger_definitions(..) }` with
  `battlefield_active_triggers(state)`.
- Gates (3), (4), and `cost_surface_references_growing_class`: route through **one CR 113.6
  zone-of-function predicate** in `functioning_abilities.rs`.
- **Do NOT simply hard-code "battlefield-only" — CR 113.6 has eleven exceptions and several are live:**
  **113.6b/c** (abilities that state their zones), **113.6j** (an activated ability whose cost can't be
  paid on the battlefield functions where it can be — Reassembling Skeleton), **113.6k** (a trigger
  condition that can't trigger from the battlefield functions in every zone it can), and **113.6d/e/f**
  (cost- and play-modifying abilities function **on the stack and in the zone the object would be cast
  from — including the HAND**). Battlefield-only would drop legitimate observers. **CR 400.2 is about
  HIDDEN zones; CR 113.6 is about FUNCTION. Do not conflate them.**
- **Permanent guard test:** *the verdict must not change when an arbitrary card is added to any library or
  hand.* A verdict that depends on a hidden zone is a rules violation by construction. This alone would
  have caught Solemn Simulacrum.
- **Already done:** R2 (`scan_mana_production`), committed with a revert-probe-verified guard
  (`ability_scan::mana_production_scan_tests`) proving Gaea's Cradle still fails closed.

### Phase 2 — RC-2: tolerate the bounded start-up transient (**CR 732.2a D3**)

CR 732.2a permits *"a non-repetitive series of choices"* **followed by** *"a loop that repeats a specified
number of times."* The engine must stop requiring the loop to cover from iteration 0.

- Drive until the cover holds on **two consecutive pairs with equal Δ**, rather than on the first two
  pairs. The transient is **provably finite**: each transient iteration consumes one untapped **non-fodder**
  member of the place and never replenishes it, so the prefix is bounded by the place's non-fodder
  population — a **board-derived, present-state-only** bound (no hidden-zone read).
- Bound the search by `min(non_fodder_population + 2, DOS_CAP)`. **Keep the DoS cap** (commit `57b0e537d`).
- Report the prefix length in the certificate so the offer says *"N₀ setup iterations, then ×N"*, matching
  CR 732.2a's own two-part shape.

### Phase 3 — C2: place non-depletion ⚠️ **the only new code**

Model every repetition-blocking legality gate the fixed sequence consumes as a **consumable place**, and
require it non-decreasing under the sequence's own Δ:

| Gate / place | Authority | CR |
|---|---|---|
| `activation_restrictions` (`OnlyOnceEachTurn`, `MaxTimesEachTurn`) | `ability_has_per_turn_activation_gate` | — |
| trigger `OncePerTurn` / `MaxTimesPerTurn` | `project_out_resources` (already preserved) | — |
| loyalty activations | `loyalty_activation_counts_match` | **CR 606.3** |
| land plays (`lands_played_this_turn`) | *(new axis, or exclude land plays loudly)* | **CR 305.2** |
| **summoning sickness — a PLACE SPLIT, not a blanket reject** | cost shape: `{T}` on the creature's own ability ⇒ sick creatures excluded; a "tap a creature" cost on **another** permanent ⇒ they are not | **CR 302.6** |
| library size | `library_delta` (already in `has_no_loss_axis`) | **CR 104.3c / 704.5b** |
| **opponent's non-pass action required** ⇒ **REJECT** | — | **CR 732.3** (fragmented loops) |

**The summoning-sickness place split is the crux.** A blanket *"reject any `{T}` cost"* would decline
**CR 732.2a's own example** (Presence of Gond has `{T}`) and most creature mana engines (Devoted Druid,
Priest of Titania, Bloom Tender, Pili-Pala). The split is driven by **the shape of the cost**, per §4.4.

Exhaustive typed enum + `_ => REJECT` default + no-`..` totality guard.

### Phase 4 — C1 + C3

- **C1:** compare Δ across the two **post-transient** pairs Phase 2 identifies; `Δᵢ != Δᵢ₊₁` ⇒ REJECT.
- **C3:** keep the `Comparator`-vs-growth-axis predicate (unbroken across three review rounds). Change only
  its **scope** (the transition set; never a hidden zone) and **retain the `projected` axis** and its
  firewall — `R-e2` (`resource.rs:5052`) is the precedent to preserve, not delete. Then delete the
  over-broad remainder (R3, R5, R6).

### Phase 2.5 — RC-4: ONE quotient relation, not four covers (**the highest-leverage phase — see §4.10**)

**Sequenced here deliberately: it outranks Phases 3–5 on measured coverage.** It is also the phase that
makes the detector *extensible* rather than merely *correct on two combos*.

- **Parameterize the four `loop_states_cover_modulo_*` siblings into one
  `loop_states_cover(prior, current, &[Quotient])`**, factoring the four shared invariants (fire-time
  firewall, cost surface, stack embedding, inertness) out **once**. Run the mandatory
  **`/add-engine-variant`** gate on `Quotient` and grep `data/engine-inventory.json` first.
- **Ship `Quotient::ObjectIdentity` first (CR 400.7).** Generalize `fodder_content_eq`'s
  content-class comparison (`resource.rs:1095` — *"not latched by ObjectId, because fodder tokens are
  not id-stable"*) off the fodder path. **Target: the 13 `ObjectReentry` rows.** Id-keyed loop equality
  is *rules-wrong*, not merely limited — CR 400.7 says a zone change makes a **new object**.
- Then `Quotient::{TurnCount, CombatCount}` — **CR 732.2a** permits a shortcut that *"may even cross
  multiple turns"*, and `ResourceAxis::{ExtraTurns, CombatPhases}` **already exist**. Target: the 3
  `ExtraTurnOrCombat` rows. **This retires §4.7's turn-crossing scope cut** — which was an engineering
  cut, never a rules one (D5).
- **Every quotient must carry a monotonicity proof**: a quotiented axis that is *not* monotone
  non-decreasing across the cycle is a **leak**, not an ω-axis, and would certify a loop that does not
  recur. **This is the soundness heart of the phase** — the `⇐` direction of the Phase 0 duality
  invariant is its runtime guard.
- **Re-bucket the corpus after each quotient lands.** `DeferralBucket` is the scoreboard; a quotient
  that empties a bucket has earned its keep. Anything still in `Other` (20) is the honest remainder.

### Phase 5 — RC-3: ONE generalized arming context (**the build-to-the-pattern phase**)

**Do NOT add `last_activation_context` as a sibling of `last_recast_context`.** That is the
sibling-cluster smell (§4.8), and it hard-codes the detector to the two combos in this document.

Per §2.1/§3.3, Combo B's cycle is **one activation** (CR 602.2b + CR 605.3a: Relic's mana ability is
activated *inside* Freed's cost payment) — **structurally identical to Combo A's one cast.** Both are
CR 732.2a's *"one pinned announcement + payment, then passes to the ending priority beat."* So:

- **Replace `RecastContext` with one `LoopProbeContext { actions, controller, decisions }`,
  parameterized on the pinned player-action SEQUENCE**, armed at any empty-stack priority beat that
  follows a player action. `RecastContext`'s buyback/convoke fields become *decisions*, not a shape.
  **`actions` is a SEQUENCE, not a single action** — **CR 732.2a** says *"a sequence of game choices"*
  (plural), and the tree agrees: `drive_offline_devoted_vizier` (corpus row 6) drives **two activations
  per cycle** (§4.9(2)). A single-action context is wrong on both the rules and the evidence, and it is
  the mirror-image of the sibling-cluster mistake — under-parameterizing instead of over-proliferating.
- **Generalize `drive_recast_iteration` → `drive_loop_iteration(action)`.** ⚠️ **Cost unknown — this is
  Open Question #6.** `drive_recast_iteration` may be structurally cast-shaped
  (`normalize_recast_frame` strips the self-returning buyback card; `derived_fodder_class` derives the
  token class *from the cast*). If generalizing it is a rewrite rather than a parameterization, the
  plan's cost changes by an order of magnitude. **Settle this before committing to Phase 5.**
- **Reuse the existing four covers** (§4.8). **Build no new cover** — Combo B's
  `loop_states_cover_modulo_counter_growth` already exists.
- **Audit `DecisionTemplate` against the CR 601.2b–h table in §4.3.** Today `template.decisions ==
  [ConvokeTaps]`; an activation loop also needs the **CR 601.2g mana-ability selection** pinned (which
  legendary creature Relic taps). **An unpinned choice is a conditional action, which CR 732.2a forbids.**
- **Leave `engine.rs:3081` and the DoS cap (`57b0e537d`) alone.** ⚠️ Broader arming means the analysis
  runs far more often. **Prove the cheap-gate cascade at `engine.rs:445` still bounds it** — a
  generalized arm that re-opens the remote DoS of #5672 is not shippable. **Open Question #7.**

---

## 7. Verification matrix

Every negative control names its **paired positive reach-guard** — a bare negative an upstream gate can
satisfy vacuously is not a test.

| Claim | Seam | Test | Revert-probe (must FLIP to FAIL) | Positive reach-guard |
|---|---|---|---|---|
| Combo A certifies on a **real** board | `try_offer_object_growth_shortcut` | `real_board_sprout_swarm_offers_loop_shortcut` (exists, **FAILS today**) | — | — |
| **Phases 1 and 2 are BOTH required** | RC-1 + RC-2 | the acceptance test **must still fail after Phase 1 alone** | — | a green-after-Phase-1 result is a **false positive** — investigate, don't celebrate |
| **CR 113.6 / CR 400.2** hidden-zone invariance | firewall scope | `real_board_verdict_is_invariant_under_hidden_zone_contents` (exists) | restore the all-zones scan | **asserts the OFFER in every arm** — `assert_eq!(v₁,v₂)` alone passes vacuously as `false == false` |
| **CR 113.6 exceptions preserved** | zone predicate | a **113.6j** ability (Reassembling Skeleton, graveyard) and a **113.6k** trigger are still scanned | hard-code battlefield-only | — |
| **RC-2** bounded transient | two-pair cover | verdict invariant under **which green creature the real cast convokes** (Witherbloom vs a Saproling) | restore the `(cs_n, cs_n1)` cover requirement | **assert the OFFER in every arm** |
| **CR 732.2a example** | end-to-end | **Presence of Gond + Intruder Alarm** OFFERS | — | this is the **rulebook's own** certified loop |
| **C2** activation gate | `ability_has_per_turn_activation_gate` | **Manaforge Cinder** DECLINES | remove the gate axis | same board minus Manaforge Cinder must **OFFER** |
| **C2** summoning sickness (**the crux**) | cost shape (CR 302.6) | **Cryptolith Rite DECLINES** *and* **Earthcraft + Squirrel Nest CERTIFIES** | collapse the sick/unsick place split | **the pair IS the discriminator** — either alone is vacuous |
| **C2** land drops (CR 305.2) | `lands_played_this_turn` | **Crucible of Worlds + Zuran Orb** DECLINES | remove the axis | board minus Crucible must OFFER |
| **C2** fragmented loop (CR 732.3) | transition set | a sequence needing an **opponent's** non-pass action DECLINES | drop the opponent-action check | a pass-only sequence on the same board must OFFER |
| **C1** scaled cost (CR 601.2f) | Δᵢ vs Δᵢ₊₁ | **Hum of the Radix** DECLINES (preserve `R-e2`) | drop the `projected` axis | board minus Hum must OFFER |
| **C4** self-deck | `has_no_loss_axis` | **Basalt Monolith + Mesmeric Orb** (Four Horsemen **minus Emrakul**) DECLINES | drop `library_delta >= 0` | ⚠️ **full Four Horsemen is NOT discriminating** — it declines on the randomness gate alone (`s_n2.rng.get_word_pos()`) and never reaches this axis |
| **C4** adverse scaling | `has_no_loss_axis` | opponent's **Suture Priest** ⇒ Combo A DECLINES | drop `life >= 0` | board minus Suture Priest must OFFER |
| Δ measured, not derived | drive | **Solemnity** + proliferate DECLINES (true Δ = 0 counters) | derive Δ from the AST | board minus Solemnity must OFFER |
| **Combo B** (Phase 5) | `last_activation_context` | Kilo + Freed + Relic + Pentad Prism OFFERS | — | **the ring and the DoS cap are unchanged** — assert `engine.rs:3081` is untouched |
| **CR 104.4b** optional-loop gate | `no_living_player_has_meaningful_priority_action` | **unchanged** — regression only | — | — |
| Gaea's Cradle stays closed | `scan_mana_production` | `for_each_creature_production_still_fails_closed` (**exists, revert-probe verified**) | collapse count-arms to `Axes::NONE` | `fixed_production_reads_nothing` (Forest) still passes |
| **Multiplayer** | — | ≥1 criterion exercises **>2 players** (the driving fixture is 4-player) | — | — |
| ⭐ **THE DUAL — ⇒ direction (coverage)** | `run_combo_live` vs `run_combo` | for every non-`LiveDrain` row: `certifies_offline ⇒ offers_live`. **Today 12 certify, 0 offer** — incl. **row 1 = Combo B** | revert generalized arming to `last_recast_context` ⇒ **every row but Combo A goes red** | **this row IS the reach-guard for the whole plan.** Only Combo A + Combo B green ⇒ the fix did **not** generalize and **must not ship** |
| ⭐ **THE DUAL — ⇐ direction (SOUNDNESS)** | `run_combo_live` vs `run_combo` | `offers_live ⇒ certifies_offline`. **Must NEVER go red** | — | a live offer the offline analyzer rejects = the detector is ending real games on a **false certificate** |
| ⭐ **RC-4 / `Quotient::ObjectIdentity`** | `loop_states_cover(.., &[Quotient])` | the **13 `ObjectReentry`** rows certify (Kiki-Jiki, Splinter Twin, Palinchron, Mikaeus, Dockside, Food Chain, Karmic Guide, Nim Deathmantle, **Earthcraft + Squirrel Nest**, …) | drop `ObjectIdentity` from the quotient set ⇒ **all 13 go red** | ⚠️ **Earthcraft + Squirrel Nest is a §4.4 hostile fixture that is ITSELF deferred** — it cannot be a positive control until this lands |
| **RC-4 monotonicity (SOUNDNESS)** | each `Quotient` | a quotiented axis that is **not** monotone non-decreasing must **REJECT** | quotient a non-monotone axis ⇒ a non-recurring loop certifies | this is the soundness heart of Phase 2.5; the Phase-0 `⇐` direction is its runtime guard |
| **RC-4 / `Quotient::{TurnCount,CombatCount}`** | ″ | the **3 `ExtraTurnOrCombat`** rows certify (CR 732.2a *"may cross multiple turns"*) | drop the quotients ⇒ all 3 go red | retires the §4.7 turn-crossing scope cut |
| **RC-4 scoreboard** | `DeferralBucket` | re-bucket the corpus after each quotient; **`Other` (20) is the honest remainder** | — | a quotient that empties no bucket has not earned its keep |
| Corpus regression | `analysis/corpus.rs` | the 12 driven rows still certify via `detect_loop`; the 37 `DeferralBucket` + 4 `gated_on` partitions unchanged | — | — |

---

## 8. Open questions — do NOT hand-wave (this document has been wrong four times)

1. **Is Δ-constancy + place non-depletion SUFFICIENT?** Manaforge Cinder has Δ₁ = Δ₂ = Δ₃ and is illegal at
   **4** — caught by C2, not C1. Both are necessary. **Prove no third failure mode exists** (a change at
   iteration ≥3 that neither alters Δ nor depletes a modelled place). Board thresholds are the known
   candidate and are C3's job; enumerate the rest **structurally**, not by recall.
2. **Exhaustiveness of the C2 place set.** It is what three review rounds found — **evidence, not proof**.
   The `_ => REJECT` default + totality guard is what makes that acceptable.
3. **The bound on the transient prefix (Phase 2).** *"Non-fodder population + 2"* is an argument, not a
   theorem. An untapper (Intruder Alarm!) replenishes the non-fodder place — so the prefix can be **0** and
   the bound is loose but safe. **Prove the bound is an upper bound, or fall back to the DoS cap and decline
   loudly on overflow.**
4. **The replacement predicate (R4).** *"Could this replacement apply?"* needs a real event-type × filter
   match. A blanket *"any replacement exists ⇒ reject"* is useless — Commander boards always have
   replacements.
5. **`DecisionTemplate` completeness vs CR 601.2b–h.** §4.3 shows `[ConvokeTaps]` is incomplete. **Which of
   the six choice classes can actually occur inside a certifiable loop body?** Unpinned ⇒ conditional ⇒
   CR 732.2a violation.
6. ⚠️ **Is `drive_recast_iteration` generalizable, or is it a rewrite?** Phase 5 assumes it parameterizes
   on the action. But `normalize_recast_frame` strips the *self-returning buyback card* and
   `derived_fodder_class` derives the token class *from the cast* — both may be structurally cast-shaped.
   **This is the largest cost unknown in the plan. Settle it before committing to Phase 5.**
7. ⚠️ **Does generalized arming re-open the #5672 remote DoS?** Arming on any player action at an
   empty-stack priority beat runs the analysis far more often than today. **Prove the cheap-gate cascade
   at `engine.rs:445` still bounds it.** Commit `57b0e537d` exists for a reason.
8. **Multi-action loop bodies.** CR 732.2a says *"a sequence of game choices"* — **plural**. Devoted Druid +
   Vizier of Remedies is **two activations per cycle**; Basalt Monolith + Rings of Brighthearth likewise.
   The drive today settles **one** action to the next priority beat. **A one-action loop body is a real
   coverage ceiling that this plan does not lift** — name it, measure how much of the corpus it excludes
   (Phase 0's partition will tell you), and decide explicitly. **Do not let it pass silently.**

---

## Appendix A — Design principles

1. **Scope every conservatism to the present board and the sequence actually being executed** — never to all
   possible boards reachable from all cards in all decks and hands. Reaching into a library is a **CR 113.6**
   error (the ability doesn't function) *and* a **CR 400.2** violation (the zone is hidden).
2. **The loop must be infinite from the PROPOSER's perspective** (CR 732.2a), then **passed around for
   response** (CR 732.2b: accept or shorten). Interaction is the response window's job, not the cover's.
3. **Monotone reads are not hazards.** The card that makes a combo infinite is usually the card that reads
   the growing axis (affinity reads creature count; proliferate reads permanents-with-counters). A firewall
   rejecting *"reads the growing class"* is incompatible with the entire family.
4. **Measure, don't derive.** Replacements rewrite Δ at resolution (CR 614); SBAs and triggers settle between
   iterations (CR 704.3 / CR 603.3b). Only the drive sees the truth.
5. **Real cards, real libraries, real mana bases** in every combo-detector test.
6. **Read the rule, don't cite it.** Every architectural correction in this document — the transient prefix
   (CR 732.2a), the Earthcraft/Cryptolith split (CR 302.6), the one-activation shape of Combo B (CR 605.3a),
   the choice-vector enumeration (CR 601.2), counters-not-mana as the ω-axis (CR 106.4) — came from reading
   the rule *text*, and **none** of them came from citing the rule *number*.

## Appendix B — What we got wrong (the record; these are why the guard rails exist)

| Claim | Reality |
|---|---|
| *"No counter-growth cover exists"* | **FALSE.** `loop_states_cover_modulo_counter_growth` (`resource.rs:1326`) exists, names **Pentad Prism** in its doc, is wired into both detection paths, and has 4 tests. Combo B's blocker is **arming** (§3.3), not a missing cover. |
| *"`ResourceVector` already computes these deltas"* | **FALSE.** No tap-state axis at all; `mana` is summed **across all players**; growth axes are event-fed and **zero under `snapshot`**; `delta` diffs two *snapshots* — a **measurement**, not a symbolic effect vector. |
| *"The payment choice is inexpressible (net-0 either way)"* | **FALSE on our own board.** Witherbloom is **Legendary** and Relic of Legends filters costs on `HasSupertype: Legendary`. |
| **#4 — *"Convoking Witherbloom is illegal at iteration 2, so the payment choice is decisive and the proposer must SEARCH for a repeatable sequence."*** | **FALSE, and it inverted the fix.** `select_convoke_taps` **re-runs every iteration**, so iteration 2 simply picks an untapped Saproling; the *place* (untapped green creatures) is non-depleting with **Δ = 0**, and the loop certifies **whichever** creature is convoked. The payment choice was **never** decisive. The real defect is **RC-2**: tapping Witherbloom is a **bounded start-up transient** that the engine's first-pair cover requirement forbids — which **CR 732.2a explicitly permits** (*"a non-repetitive series of choices"* preceding *"a loop that repeats a specified number of times"*). **No proposer search is needed; the transient must be tolerated.** |
| *"Gaea's Cradle fail-closes via `repeat_for`"* | **FALSE.** It parses as `AnyOneColor{count: Ref(ObjectCount{Creature,You})}`; the read lives **inside `ManaProduction`** and is caught **only** by `scan_mana_production` routing count-bearing variants through `scan_quantity_expr`. **Do not "simplify" that walker** — doing so silently enables false certification of unbounded mana. |
| *"The untap step is CR 502.2"* | **FALSE.** CR 502.2 is **day/night**. The untap step is **CR 502.3**. |
| An LP / Petri-VAS model would replace the drive | **Unsound.** Δ is not derivable (replacements), legality is not a resource, and Δ can be marking-dependent. Superseded by §4. |
