# Combo detector — root-cause analysis + implementation plan
### Making CR 732.2a loop shortcuts work on real decks and real board states

**Date:** 2026-07-14 · **Status:** Plan. Implementation NOT started. **Adversarially reviewed 4×.**
**Branch:** `debug/combo-generator` (fork-only; **never** merge toward `main` — `.planning/` is gitignored).

> ### Evidence standard — read this before trusting any line below
> Every claim is **measured**: by driving the user's exported live board through the real engine, by
> grepping `docs/MagicCompRules.txt`, or by reading `data/card-data.json` / the engine source.
> **This document has been wrong TEN times.** Every single failure was a **code claim asserted from
> memory**; the rules work has held up under four reviews. They are catalogued in **Appendix B** — read
> it, because the guard rails only work if you know what they guard. **Assume there is an eleventh.**

---

## 1. Executive summary

**The combo detector cannot fire in any real game of Magic.** Two live infinite combos on a real
4-player Commander board were verified undetectable. There are **four independent root causes**, and
**no single one of them is sufficient to fix.**

| | Root cause | Where |
|---|---|---|
| **RC-1** | **The fire-time observer predicate is wrong** — it rejects on *"references any typed object filter"*, which every Commander permanent does. It also scans **hidden zones**. | `resource.rs:1451` |
| **RC-2** | **The cover forbids a bounded start-up transient** — it demands recurrence from iteration 0. | `engine.rs:1725` |
| **RC-3** | **The live path arms on ONE bespoke card shape**, and **zero of 53 corpus rows test it.** | `casting_costs.rs:6785` |
| **RC-4** | **Loop equality is keyed on `ObjectId`**, which **CR 400.7** makes rules-wrong. | `game_state.rs:10456` |

**CI is green because the acceptance fixture cannot exist in a real game.** `sprout_swarm_scenario`
(`loop_shortcut.rs:2536`) builds a board with no lands, an empty library, no auras, and a stub
Witherbloom oracle. All four root causes are invisible to it — and RC-3 means *nothing anywhere* is
looking at the live path.

**The two findings that outrank the bug:**

1. **The detector asks a question the rules do not.** It tries to prove *"no ability anywhere could ever
   observe this growth."* **CR 732.2a** asks only whether a sequence *"may be legally taken based on the
   current game state and the predictable results of the sequence of choices"*, and **CR 732.2b** gives
   every other player the right to **accept or shorten**. Interaction is the response window's job, not
   the cover's.
2. **The scan reads hidden zones.** A `Solemn Simulacrum` **in the library** vetoes detection. That is
   illegal twice: by **CR 113.6** (an object's abilities *"usually function only while that object is on
   the battlefield"* — the ability **does not exist** there) and by **CR 400.2** (library and hand are
   **hidden zones**).

> ## ⚠️ Size this honestly: "Only C2 is new" was FALSE — but it is TWO new subsystems, not three.
> Review measured **three**. The **governing constraint** (§4.6) then eliminated one of them:
>
> | Subsystem | Verdict |
> |---|---|
> | **Generalized action driver** (P1) — `drive_recast_iteration` has **8 cast-shaped elements and 1 parameter** | ⚠️ **REWRITE. And a PREREQUISITE — 6 of 15 test rows never arm without it.** |
> | **CR 113.6 zone-of-function predicate** (P2) — **does not exist**; `battlefield_active_triggers` IS the battlefield-hard-coding CR 113.6 forbids | ⚠️ **NEW CODE.** |
> | ~~Fire-time **observer** predicate~~ (P5) | ✅ **NOT a rewrite — mostly a DELETION.** The drive already measures every effect the board produces; the firewall's only job is the **threshold** it is blind to, and **that condition scan already exists** at gate (4). |
>
> **C2 + the P1 driver + the P2 predicate.** That is the honest surface. **A plan that under-states its
> own new surface will be executed as if it were small.**

---

## 2. Reproduction

- **Fixture:** `crates/engine/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json`
  (debug-panel export; wrapped `{gameState, waitingFor, legalActions, turnCheckpoints}`).
- **Harness:** `crates/engine/tests/integration/repro_user_combo.rs`.
- A bare snapshot is insufficient: arming happens **during a cast**, so the repro must drive a real cast.

```
cargo test -p engine --test integration real_board_fixture_is_intact   # PASSES (guards the fixture)
cargo test -p engine --test integration -- --ignored real_board        # FAILS (the bug)
```

**Board:** Witherbloom, the Balancer (Legendary) + 4 untapped green Saproling **tokens** + Kilo, Apogee
Mind (Legendary, enchanted by Freed from the Real) + Relic of Legends + Pentad Prism (1 charge) +
Forests/Islands. Sprout Swarm in hand. `Interactive`, `Priority{P0}`, own turn, empty stack.

**Measured after the driven cast:** `last_recast_context` is armed **correctly**
(`card_id:415, controller:0, from_zone:Hand, uses_buyback:Used, convoke:Some`); every cheap gate at
`engine.rs:445` is green; `waiting_for` stays `Priority{0}`. **The decline is downstream, in the cover.**

### 2.1 The two combos — Oracle text verified in `data/card-data.json` (engine-planner Step 0)

| Card | Oracle text (verbatim from the shipped DB) |
|---|---|
| **Sprout Swarm** | Convoke · Buyback {3} · *"Create a 1/1 green Saproling creature token."* |
| **Witherbloom, the Balancer** | *"Instant and sorcery spells you cast have affinity for creatures."* |
| **Relic of Legends** | *"{T}: Add one mana of any color."* · *"**Tap an untapped legendary creature you control**: Add one mana of any color."* |
| **Kilo, Apogee Mind** | *"**Haste.** Whenever Kilo becomes tapped, proliferate."* |
| **Freed from the Real** | Enchant creature · *"{U}: Tap enchanted creature."* · *"{U}: Untap enchanted creature."* |
| **Pentad Prism** | Sunburst · *"Remove a charge counter from this artifact: Add one mana of any color."* |

**Combo A — Witherbloom + Sprout Swarm (object growth).** Through the casting rules:
- **CR 601.2b** — announce buyback {3} ⇒ base {1}{G} + {3} = **{4}{G}**.
- **CR 601.2f** — **CR 702.41a** affinity (*"costs {1} less to cast for each [text] you control"*) is a
  cost **reduction**; ≥4 creatures ⇒ generic to {0}. **Total cost LOCKS IN.** Remaining: **{G}**.
- **CR 601.2h** — **CR 702.51b**: *"convoke isn't an additional or alternative cost and applies only
  after the total cost … is determined"* ⇒ convoke is a **payment substitution**: tap one untapped
  **green** creature for the {G}.
- Resolve: create a **green, untapped** Saproling; buyback returns the card.

⇒ **Δ(untapped green creatures) = −1 (convoked) + 1 (new green untapped token) = 0.**
⇒ **Δ(creatures) = +1**, so affinity only strengthens. **Legal for all N; the ω-axis is creatures.**

**Combo B — Kilo + Freed + Relic → Pentad Prism (counter growth). ⚠️ It is TWO actions, not one.**
**The tree's own certifying driver** (`corpus.rs:1556`, `drive_offline_kilo_freed_relic`) is:
```rust
run_combo(board, |probe| {
    activate_and_resolve(probe, relic, relic_tap_creature, Some(TargetRef::Object(kilo)));
    activate_and_resolve(probe, freed, freed_untap,        Some(TargetRef::Object(kilo)));
})
```
Two `GameAction::ActivateAbility` at priority. Its own comment pins why: ***"Relic has two mana
abilities; the tap-self one would not fire Kilo's trigger."*** Relic must be activated **standalone**,
selecting the `TapCreatures{Legendary}` cost, to tap **Kilo** (`GameEvent::PermanentTapped`,
`restrictions.rs:756`) and fire the proliferate trigger.

> **Appendix B #6 — my "Combo B is ONE activation" claim was FALSE.** The CR 605.3a nesting story
> (activate Relic's mana ability *inside* Freed's cost payment) is **rules-legal** but **engine-false**:
> even in a `WaitingFor::ManaPayment` window a mana ability is dispatched as its own
> `GameAction::ActivateAbility` (`engine.rs:4867`), and Relic's *tap-self* ability — the one auto-payment
> would pick — **does not tap Kilo and does not fire the trigger.** **There is no single-action encoding
> of this cycle anywhere in the action model.** A single-action arming latch cannot capture it. This
> refutes the previous Phase 5 outright.

⇒ Δ(mana) = 0, Δ(Kilo tapped) = 0, **Δ(charge counters) = +1.** Unbounded counters ⇒ unbounded mana.

> **Counters, not mana, are the ω-axis.** **CR 106.4 / CR 500.5**: *"any unspent mana … empties"* at
> end of each step and phase. Mana is not durable. This is what the shipped
> `loop_states_cover_modulo_counter_growth` already certifies — **build nothing there.**

---

## 3. Root cause

### 3.1 RC-1 — the observer predicate is wrong, and it reads hidden zones

`fire_time_conditions_read_growing_class` (`resource.rs:1451`).

**(a) The predicate itself.** Gate (1) rejects if any live ability
`ability_definition_reads_sibling_mutable` (`ability_scan.rs:3767`). But `ability_scan.rs:2454`:
```rust
TargetFilter::Typed(tf) => Axes { event: true, sibling: true, … }   // UNCONDITIONALLY
```
**`sibling: true` for ANY typed object filter.** Measured consequence — **Intruder Alarm**, whose parsed
trigger is `SetTapState{target: Typed[Creature], scope: All, state: Untap}`, **trips gate (1)** and is
rejected. **Intruder Alarm is CR 732.2a's OWN worked example.** The predicate is not *"reads the growing
class"*; it is *"references any typed object filter"* — which every Commander permanent does.

> **⚠️ Appendix B #8 — my "catch-22" argument was OVER-CLAIMED.** I claimed the *driving* ability
> (Witherbloom's affinity) trips the firewall. **Measured false:** Witherbloom's static parses to
> `modifications: []`, `condition: null`, `mode: CastWithKeyword{Affinity{Creature}}`, and gate (4)
> (`resource.rs:1524`) inspects **only** `condition` and `modifications` — neither trips. The fodder
> cover also deliberately drops `cost_surface_references_growing_class` (`resource.rs:1078`). **The
> conclusion stands but the evidence was wrong: use INTRUDER ALARM, not affinity.**

**(b) The zones.** Gates (1) and (4) scan `state.objects.values()` across **every zone**. Measured trips
on the real board: `Solemn Simulacrum` **(Library)** → a basic **`Forest`** → **`Freed from the Real`**.
Illegal by **CR 113.6** (the ability *does not function* off the battlefield ⇒ scanning it reads an
ability that does not exist) and by **CR 400.2** (hidden zone). **CR 113.6 is the primary authority.**

> **⚠️ Appendix B #9 — "measured trips, in order" is the wrong provenance.**
> `loop_states_cover_modulo_fodder_growth` checks `board_covers_modulo_fodder` **first**
> (`resource.rs:1119`) and returns false before reaching the firewall (`resource.rs:1132`). Because RC-2
> fails that first board cover, **the firewall is never reached on `(cs_n, cs_n₁)`.** Both root causes
> are real and neither alone suffices — but the trips were observed under instrumentation, not on the
> live path.

**R2 is already fixed and committed** (`scan_mana_production`; the `Forest` trip).

### 3.2 RC-2 — the cover forbids a bounded start-up transient — **CONFIRMED, could not be broken**

`engine.rs:1725` requires the cover on **both** pairs:
```rust
loop_states_cover_modulo_fodder_growth(&cs_n,  &cs_n1, &fodder)   // ← FAILS
&& loop_states_cover_modulo_fodder_growth(&cs_n1, &cs_n2, &fodder)
```
Chain, every link measured:
1. `select_convoke_taps` (`mana_payment.rs:436`) does `candidates.sort_by_key(|id| id.0)` and **re-runs
   per drive iteration**.
2. `is_convoke_eligible` (`game_object.rs:2206`) checks **only** controller / battlefield / untapped /
   Creature — **no color preference, no sickness gate**.
3. ⇒ **Witherbloom (402, `["Black","Green"]`, untapped)** is picked over the Saprolings (413+).
4. Witherbloom is still **untapped at `s_n`** because the acceptance test convokes a Saproling
   (`repro_user_combo.rs:108`).
5. **Nothing absorbs the flip.** `normalize_recast_frame` (`engine.rs:1599`) strips only the recast card
   + anaphora; `derived_fodder_class` (`engine.rs:1633`) derives only the Saproling class;
   `fodder_content_eq` (`resource.rs:994`) is content-equality-modulo-`tapped` **against that class**, so
   Witherbloom is a **STABLE ENGINE** object, not fodder — and `object_content_eq`
   (`game_state.rs:10456`) **compares `tapped`.**
6. ⇒ Witherbloom's untapped→tapped flip breaks the stable partition of `board_covers_modulo_fodder`
   (`resource.rs:1049`) ⇒ **`(cs_n, cs_n₁)` cannot cover** ⇒ no offer.

**Bounded:** nothing untaps Witherbloom (Freed enchants **Kilo**, not her). ⇒ the transient is a
**one-time prefix**, and the recurrence from iteration 2 is exact (untapped-green count invariant at 5).

**⚠️ Scope the claim correctly:** *"on any real board"* is **too strong**. Correct: **"on any board where
the driven prefix consumes a non-fodder engine piece."**

**The airtight supporting evidence is the ASYMMETRY between the two callers of the same machinery** —
*not* `WARMUP` (a constant in the same crate by the same authors is corroboration, not independence):

| | transient tolerated | covering pairs required |
|---|---|---|
| **Offline** `run_combo` (`corpus.rs:1179`) | **≥4 cycles** (`WARMUP:2` + failed `STEADY` retries) | **1** |
| **Live** `try_offer_object_growth_shortcut` (`engine.rs:1690`) | **0** | **2, from iteration 0** |

### 3.3 RC-3 — the live path arms on one card shape, and nothing tests it

The live offer fires only when `last_recast_context` is armed (`casting_costs.rs:6785`) — *a
buyback-paid, token-creating recast.* **One card shape.** Every other player-driven loop is invisible.

**`grep -c "WaitingFor::LoopShortcut" crates/engine/src/analysis/corpus.rs` == 0.** All 53 rows are
driven through the **offline** `detect_loop`. **Not one row exercises the live offer path.** The corpus
is *structurally incapable* of catching this bug.

**And the ring cannot substitute.** `loop_detect_ring` stores `Arc<GameState>` **snapshots**, not actions
(`game_state.rs:6939`), and `engine.rs:3081` clears it on **everything except `PassPriority |
OrderTriggers`**. ⇒ **"detect multi-action player loops" and "leave `engine.rs:3081` alone" are mutually
exclusive.** This plan resolves it by **arming**, not by weakening the ring (§6 P1).

### 3.4 RC-4 — loop equality is keyed on `ObjectId`, which CR 400.7 makes rules-wrong

`object_content_eq` (`game_state.rs:10456`) is id-keyed. **CR 400.7** (`zones.rs:132`): *"An object that
changes zones becomes a new object."* A permanent that dies / blinks / bounces returns with a **fresh
`ObjectId`**, so the loop point is never board-identical. This is the `DeferralBucket::ObjectReentry`
bucket. **See §5 — it is smaller and far more dangerous than I first claimed.**

---

## 4. Architecture — the fixed-sequence formulation

### 4.1 CR 732.2a fixes the player's choices. That is the whole design.

> **CR 732.2a** *(verbatim, `docs/MagicCompRules.txt:6372`)*: *"the player with priority may suggest a
> shortcut by **describing a sequence of game choices**, for all players, that **may be legally taken
> based on the current game state and the predictable results of the sequence of choices**. This sequence
> may be **a non-repetitive series of choices, a loop that repeats a specified number of times**, multiple
> loops, or nested loops, **and may even cross multiple turns**. **It can't include conditional actions**…
> **The ending point of this sequence must be a place where a player has priority**…"*

Five deductions, each of which changes code:

- **D1 — a shortcut IS a straight-line action sequence, by rule.** No conditionals ⇒ the proposer commits
  to which creature to convoke, which source to tap, which target to pick. The question is **not** *"is
  this board a linear program?"* (ill-posed). It is:

  > ## **Is this FIXED sequence legally repeatable forever, with constant Δ?**

- **D2 — "a loop that repeats a specified number of times."** The proposer names **N**, and the proposal
  must be legal *"based on the predictable results"* — for **every** iteration. ⇒ **precondition
  non-depletion (C2) IS CR 732.2a**, not an engineering add-on.
- **D3 — "a non-repetitive series of choices, [or] a loop that repeats…"** ⇒ **a shortcut may be a
  non-repetitive PREFIX followed by a loop.** Demanding the loop cover from iteration 0 is **stricter
  than the rule**. That is **RC-2**.
- **D4 — "the ending point must be a place where a player has priority."** The iteration boundary is a
  priority beat **by rule** — the empty-stack settle condition the drive already uses.
- **D5 — "a sequence of game CHOICES" (plural) and "may even cross multiple turns" are LEGAL.**
  Multi-action bodies are **confirmed in three drivers** (`drive_offline_devoted_vizier` corpus.rs:1416,
  `drive_offline_grim_power` :1433, `drive_offline_kilo_freed_relic` :1556). **Excluding turn-crossing
  loops is an ENGINEERING cut, not a rules one — waive it LOUDLY with the CR quote.**

**CR 732.2a's own worked example is an object-growth loop** — Presence of Gond + Intruder Alarm, *"I'll
create a million tokens."* **The rulebook certifies the exact class we cannot detect**, and **RC-1
rejects it** (§3.1a). It is the plan's primary acceptance fixture.

### 4.2 Two rules that prune the design

- **CR 732.4 + CR 104.4b** — *"Loops that contain an optional action don't result in a draw."* Our loops
  contain the proposer's **optional** action ⇒ never a draw ⇒ the engine **offers**. **Already
  implemented**: `no_living_player_has_meaningful_priority_action` (**`engine.rs:2367`**, called at
  `engine.rs:1766`). **Don't rebuild.** CR 732.5/732.6 govern only *mandatory* loops ⇒ **out of scope.**
- **CR 732.3 — fragmented loops.** If repetition needs an **opponent's** independent action, the active
  player must break it ⇒ **reject any sequence requiring an opponent's non-pass action.** Ours need only
  priority passes, which CR 732.2b already lets them decline.

### 4.3 The choice vector is enumerable — from CR 601.2 / 602.2

**CR 602.2b**: activating an ability follows **601.2b–i** identically. So what a fixed sequence must pin
is **closed and checkable**:

| CR | Choice to pin |
|---|---|
| 601.2b | mode · splice · **optional additional/alternative costs (buyback)** · **X** · hybrid · Phyrexian |
| 601.2c | **targets** (and the number) |
| 601.2d | division / distribution |
| 601.2f | order of applying cost **reductions** |
| 601.2g | **which mana abilities to activate** |
| 601.2h | **payment choices — including convoke's tap-set** (CR 702.51b) |

**Measured gap — this is a BLOCKER, not an audit item.** `build_recast_template` emits
`[ConvokeTaps]` or `[]` (`engine.rs:1558`), and `drive_recast_iteration` **explicitly aborts** on the
other five `ConcreteDecision` kinds (`engine.rs:1527`, `return Err(RecastAbort)`). Combo B's cycle opens
`WaitingFor::PayCost{TapCreatures}` (`engine.rs:3947`) — which lands on the `_ => return Err(..)` arm at
`engine.rs:1548`. **The driver cannot drive Combo B at all.** In P1's scope, not §8.

### 4.4 Every failure mode collapses into one: *the fixed sequence becomes ILLEGAL*

All card text verified in `data/card-data.json`.

| Case | The place the sequence draws from | Δ(place) | Verdict |
|---|---|---|---|
| **Sprout Swarm** — convoke (CR 702.51b, a **payment**; no `{T}` on the creature) | untapped **green** creature | −1 + 1 (**token is green & untapped**) = **0** | **ACCEPT** ✅ |
| **Earthcraft** — *"**Tap an untapped creature you control**: Untap target basic land."* → cost on **Earthcraft's own** ability, **no tap symbol** ⇒ **CR 302.6 does not apply** ⇒ a summoning-**SICK** Squirrel is legal fodder | untapped creature (sick or not) | −1 + 1 = **0** | **ACCEPT** ✅ |
| **Cryptolith Rite** — *"Creatures you control have **'{T}: Add one mana of any color.'**"* → the **creature's OWN `{T}`** ability ⇒ **CR 302.6 APPLIES** | **unsick** untapped creature | −1 + 0 (**new token is sick**) = **−1** | **REJECT** ✅ |
| **Presence of Gond + Intruder Alarm** (**CR 732.2a's example**) — `{T}` on the creature ⇒ CR 302.6 applies; Intruder Alarm untaps it | **unsick** untapped enchanted creature | −1 + 1 = **0** | **ACCEPT** ✅ |
| **Manaforge Cinder** — *"{1}: Add {B} or {R}. **Activate no more than three times each turn.**"* | activations remaining | −1 | **REJECT** |
| **Crucible + Zuran Orb** | land plays remaining (**CR 305.2**) | −1 | **REJECT** |
| **Basalt Monolith + Mesmeric Orb** (Four Horsemen **minus Emrakul**) | cards in library (**CR 704.5b**) | −n | **REJECT** |
| **Damping Sphere** — *"Each spell a player casts costs {1} more for each other spell that player has cast this turn."* | — | cost RISES at **CR 601.2f** ⇒ **Δ₁ ≠ Δ₂** | **REJECT** |
| **Solemnity** + proliferate | — | **measured** Δ = 0 counters | **REJECT** |

**CR 302.6** *(verbatim, `:1630`)*: *"A creature's activated ability **with the tap symbol or the untap
symbol in its activation cost** can't be activated unless the creature has been under its controller's
control continuously since their most recent turn began."*

**The engine CAN see this split** (verified): `AbilityCost::Tap` (the `{T}` symbol) vs
`AbilityCost::TapCreatures { requirement, filter }` (`ability.rs:7841`), and CR 302.6 is enforced **only
on the former, against the ability's own `source`**, via `check_summoning_sickness_for_cost` →
`cost_contains_tap_or_untap` (`restrictions.rs:618, 675`). **Phase 4's place-split is implementable.**

> ⚠️ **Appendix B #10 — "Hum of the Radix" was UNSATISFIABLE.** Verified text: *"Each **artifact spell**
> costs {1} more…"*. Sprout Swarm is a **green instant** ⇒ Hum cannot affect it ⇒ **both arms of that §7
> row OFFER.** The card the plan wanted is **Damping Sphere**, which it named in §4.6 and then failed to
> test. Corrected above.

### 4.5 Legality gates ARE consumables

*"3 activations left"*, *"1 land drop left"* (**CR 305.2**), *"unsick creatures"* (**CR 302.6**), *"cards
in library"* (**CR 704.5b**), *"loyalty activations"* (**CR 606.3**) are **resources the fixed sequence
spends**. `project_out_resources` (`resource.rs:2500`) already **deliberately preserves** them — its own
comment: *"blanket-clearing them would erase the gate that makes a once-per-turn … ability
NON-repeatable, **falsely certifying it as infinite**."* Single authority:
`ability_has_per_turn_activation_gate` (**`resource.rs:2848`**).

### 4.6 The four checks

> ## ⭐ THE GOVERNING CONSTRAINT — derive every check from this, or you will over-build
>
> **The player proposes a FIXED loop (CR 732.2a: no conditional actions), and it is impactable ONLY by
> what is CURRENTLY on the board.** And **we DRIVE that fixed sequence on a clone through the real
> reducer.**
>
> ⇒ **Every ability on the battlefield that fires during the loop ALREADY FIRES IN THE DRIVE and ALREADY
> LANDS IN Δ.** Intruder Alarm untapping, affinity reducing, Solemnity preventing, Damping Sphere
> scaling — **the drive saw all of it.** A firewall that re-derives them statically is not conservatism;
> it is **duplicated work that gets the answer wrong** (§3.1a).
>
> **So ask the only question that matters: what can the CURRENT BOARD do that the DRIVE CANNOT SEE?**
> The answer is exhaustive, and it is two things:
>
> | # | Blind spot | Why the drive misses it | Check |
> |---|---|---|---|
> | **1** | **Monotone depletion outside the drive window** | Δ is *constant* for the driven iterations; the sequence dies at iteration 4 (Manaforge's 3/turn, land drops, library, sickness) | **C2** |
> | **2** | **A DISCONTINUITY — a threshold that trips at a future iteration count** | Δ is *constant* until it trips ("when you control 10+ creatures, sacrifice…"); the drive only runs 2–3 | **C3** |
>
> **Everything else is MEASURED.** An effect that **scales** with the growing axis changes Δ between
> iterations ⇒ **C1**. An effect that **reads** the growing axis but does **not** scale yields constant Δ
> ⇒ **HARMLESS** — and rejecting it is precisely how the current predicate rejects the rulebook's own
> example.
>
> ⇒ **C3 is a CONDITION scan, not an OBSERVER scan.** Its job is *thresholds*, nothing more: **a
> fire-time `Comparator` whose operand is the quotiented (growing) axis**, on an ability that
> **functions** (CR 113.6) in the zone it is in. Nothing about hands, libraries, or hypothetical boards —
> **the current board, and only the current board.**

| # | Check | Catches | Status |
|---|---|---|---|
| **C1** | **Δ-constancy** across two **post-transient** pairs | anything that **scales** with the growth (**Damping Sphere**) | drive exists; must skip the transient (RC-2) |
| **C2** | **Place non-depletion** | monotone depletion outside the window (activation limits, land drops, sickness, self-mill) | **new — the only genuinely new logic** |
| **C3** | **Threshold scan** — a fire-time `Comparator` against the growing axis | **discontinuities** | ⚠️ **mostly a DELETION — see below** |
| **C4** | **The shipped triple** — `net_progress_for(caster)` + `has_no_loss_axis` + `driving_resources_non_decreasing` | self-deck, self-damage, adverse scaling | **exists, unchanged** (`engine.rs:1756`) |

**⇒ C3 collapses from a rewrite into a deletion, and the condition scan ALREADY EXISTS.** Measured:
gate **(4)** (`resource.rs:1524`) already inspects **`def.condition`** — the right place. The defect is
gate **(1)**, which scans **EFFECTS** via `ability_definition_reads_sibling_mutable` (`ability_scan.rs:3767`
→ `sibling: true` for any typed filter, `:2454`). **Effects are the drive's job, not the firewall's.**

- **DELETE gate (1)'s effect-scan** — the drive measures effects. *(This alone unrejects Intruder Alarm,
  Suture Priest, and every Commander permanent.)*
- **KEEP gate (4)'s condition scan, but narrow it**: match a `Comparator` **against the growing axis**,
  not the current blanket `if !def.modifications.is_empty() { return true }` (R5) or "any condition".
- **DELETE R3** (activated-ability bodies — an activated ability observes nothing unless *activated*, and
  the fixed sequence pins whether it is), **R5**, and **R6**.

> ⚠️ **§4.6 previously said *"C3 is the one arm three adversarial rounds never broke — keep its logic."*
> That contradicted §3.1 and is refuted (Appendix B #8). But the round-4 conclusion — *"C3 is a rewrite"* —
> **over-corrected.** Under the governing constraint, **C3's kept half (the condition scan) is already in
> the tree; its broken half (the effect scan) should be DELETED, not rebuilt.** *"Only C2 is new"* is
> **restored for C3** — though it remains false for the **CR 113.6 predicate** (P2) and the **generalized
> driver** (P1), which are still real new subsystems.

**Why measurement, not derivation.** Δ cannot be derived from the AST — **replacements rewrite it at
resolution** (Solemnity turns proliferate's AST-Δ of +1 into a true Δ of **0**), and **CR 704.3 / CR
603.3b** put a full SBA + trigger settle between iterations, so a loop that kills its own engine simply
fails to recur. **The drive is the authority; the firewall's ONLY remaining job is the discontinuity the
drive is structurally blind to.**

---

## 5. RC-4 / object identity — the honest picture (⚠️ I was wrong about this too)

> **Appendix B #7 — "generalizing `normalize_recast_frame` lifts all 13 `ObjectReentry` rows and is worth
> more than Phases 1–5 combined" is FALSE.** It lifts **ZERO** of them directly, and the real fix is the
> **riskiest change in the program**. It is **not a quick win and must not be sequenced as one.**

`DeferralBucket::ObjectReentry` is a **coarse bucket over two structurally different failures**:

**Group A — token ACCUMULATION; id churn is NOT the blocker (6 rows).**
Kiki-Jiki + Zealous Conscripts · Splinter Twin + Deceiver Exarch · Midnight Guard + Presence of Gond ·
Scurry Oak + Ivy Lane Denizen · Felidar Guardian + Saheeli · **Earthcraft + Squirrel Nest**.
These are **pure object growth**, and `loop_states_cover_modulo_{object,fodder}_growth` **already exclude
the add-set from id-keyed equality** (`resource.rs:1040`, `:1095`). What actually blocks them:
**Kiki/Twin** — each token carries *"sacrifice at the beginning of the next end step"* ⇒
`state.delayed_triggers` grows ⇒ **gate (6)** rejects on ANY non-empty `delayed_triggers`
(`resource.rs:1577`). **The rest** — **RC-1** (typed-filter gate) and **RC-3** (nothing arms).
⇒ **Phases 1/2/5 lift Group A. Object identity is irrelevant to it.**

**Group B — TRUE re-entry; id churn IS the blocker, and `normalize_recast_frame` is the WRONG fix (7 rows).**
Palinchron + Deadeye · Dockside + Sabertooth · Mikaeus + Triskelion · Food Chain + Eternal Scourge ·
Gravecrawler + Altar + Blood Artist · Karmic Guide + Reveillark + Viscera Seer · Reassembling Skeleton +
Ashnod's + Nim Deathmantle.

`normalize_recast_frame` handles churn by **deleting the object from both frames** — sound **only**
because the recast card is `ctx`-identified **and off the battlefield** (a card in hand, carrying no board
state). **Neither holds for Group B:**

1. **The churning object IS the engine piece.** Deleting Palinchron erases its own board state —
   including `summoning_sick`, **the exact CR 302.6 field C2's place-split depends on.** You would
   project out the thing you are checking.
2. **Id churn contaminates STABLE objects through id-valued fields.** `object_content_eq`
   (`game_state.rs:10470`) compares **`attached_to`, `attachments`, `paired_with`** — all
   `ObjectId`-valued. Palinchron is soulbonded to **Deadeye Navigator**: after the blink, **Deadeye's
   `paired_with` points at a NEW id**, so **Deadeye — a stable, never-moved object — fails content
   equality.** Stripping Palinchron does not fix Deadeye. Same for Nim Deathmantle's `attached_to`.

**The real fix is id-canonicalization of the whole frame** (remap `ObjectId`s to a canonical order **and**
canonicalize every id-valued field) — **a soundness-critical rewrite of the equality core.**
**Content-multiset equality is EXACTLY where a false certificate enters**: two boards can be content-equal
per-object yet differ in **which object the stack, a delayed trigger, or an aura POINTS AT.**

> **Verdict: object identity across a loop cycle is a real, general, unsolved problem that deserves its
> OWN PR with its OWN soundness proof. It is §6 P6 — LAST, not a "Phase 2.5 quick win."**
> The `Quotient` parameterization (one `loop_states_cover(prior, current, &[Quotient])` replacing the four
> `loop_states_cover_modulo_*` siblings) is still the right **shape** — the sibling-cluster smell is real —
> but it must be earned with the canonicalization proof, not asserted as a refactor.

---

## 5b. SPIKE — can we buy the equality core off the shelf? (`egg` / e-graphs, vs. scalarset symmetry reduction)

**Explore this BEFORE committing to P6.** §5 says object identity across a loop cycle is *"the riskiest
change in the program"* and needs *"its own soundness proof."* **Both of those are bad things to
hand-roll.** This section names the two off-the-shelf formalisms and — critically — **the one soundness
asymmetry that decides between them.**

### 5b.1 The soundness asymmetry — read this before evaluating either option

Our equality relation sits on the **only game-ending path**. So its error direction is not symmetric:

| Relation errs… | Meaning | Consequence |
|---|---|---|
| **TOO COARSE** (says *equal* when they are not) | certifies a state recurrence that did not happen | ⛔ **FALSE CERTIFICATE — ends a real game wrongly. CATASTROPHIC.** |
| **TOO FINE** (says *different* when they are equivalent) | misses a real loop | ✅ **false negative — a missed offer. SAFE / fail-closed.** |

**⇒ A coarse relation is only ever admissible as a REJECT filter, never as an ACCEPT decision.**
Everything below hangs on that sentence.

### 5b.2 ⭐ Option A — `egg`'s **e-class analysis over the ABILITY AST**. This is the one that reshapes the plan.

[`egg`](https://docs.rs/egg) ([POPL'21](https://dl.acm.org/doi/pdf/10.1145/3434304)) provides congruence
closure with hashconsing plus **e-class analysis** — a semilattice (`make` / `merge` / `modify`) that is a
tested **monotone abstract-interpretation** framework, co-designed with congruence so the analysis
**cannot drift between equal subterms.** Its own abstract says it exists to *"reduce the need for ad hoc
manipulation."* **Not currently a dependency** (verified: `grep egg Cargo.toml` ⇒ nothing).

**The fit is at the PARSER/AST layer, and it goes at the ROOT of RC-1.**

`ability_scan.rs`'s `Axes { event, sibling, projected }` walk **already IS an abstract interpretation over
the ability AST.** It is hand-rolled, and it is **measurably wrong**: `TargetFilter::Typed(_) => Axes {
sibling: true, .. }` **unconditionally** (`ability_scan.rs:2454`) — which is exactly what rejects
**Intruder Alarm, CR 732.2a's own worked example** (§3.1a). **R1, R3, R5 and R6 are four hand-rolled scans
over that same AST.** They are ad-hoc manipulation, and they are the root cause.

**⇒ Make `Axes` an e-class analysis, and the plan's shape changes:**

| Today | With an e-class `Axes` |
|---|---|
| **P2** — write a new CR 113.6 zone-of-function predicate (**new subsystem**) | a **query** against the analysis |
| **P5** — narrow gate (4) to a Comparator-vs-growing-axis scan (**bespoke scan**) | a **query** against the analysis |
| four `loop_states_cover_modulo_*` siblings each re-deriving firewall + cost-surface | **one** analysis, consulted four ways |
| `sibling: true` drift, undetectable | **congruence forbids** two equal subterms carrying different `Axes` |

**⚠️⚠️ RULES-CORRECTNESS IS PRIMARY — AND THE HAZARD IS THE REWRITE RULES.**
Equality **saturation** rewrites terms into equivalent forms **according to your rewrite rules, and every
rewrite rule is a CR claim.** *"Destroy"* ≠ *"sacrifice"* ≠ *"put into a graveyard"* — they differ under
**regeneration (CR 701.15)**, **indestructible (CR 702.12)**, and **replacement effects (CR 614)**. A
single non-CR-preserving rule **silently changes what every card in the database does**, everywhere. That
is a **far higher-stakes place to be wrong than the detector.**

> ## ⛔ SCOPE THE SPIKE: take the ANALYSIS, take ZERO semantic rewrite rules.
> Build the e-graph from the existing AST with **congruence + `Analysis` only** — **no rewrites**, or only
> rules **proven CR-neutral one at a time, each carrying its own verified CR citation.**
> **This buys the ENTIRE win — a principled, monotone, congruence-consistent `Axes` — at ZERO rules risk,
> because nothing is rewritten.** Equality *saturation* is a **separate, later, opt-in** decision that
> must not ride in on this one.

**Secondary (sound, minor):** a congruence-closure **REJECT-ONLY** pre-filter for state equality — hashes
differ ⇒ fast decline; hashes match ⇒ fall through to the exact check. Sound by §5b.1 (coarse relations may
reject, never accept), and it **could also discharge P1's open DoS pre-gate.** ⚠️ **But see 5b.3: congruence
is the WRONG relation for state equality, so this is a filter and nothing more.**

**Cost flags (measure, do not assume):** the engine **ships to WASM** (`opt-level='z'` + LTO) and **the
detector is on the live in-game path — it cannot be feature-gated out.** Measure the bundle delta.
*(Contrast `analysis/corpus.rs`, which IS `#[cfg(any(test, feature = "combo-verify"))]` and excluded from
the shipped lib.)* If the delta is unacceptable, a bare semilattice is ~30 lines of Rust — **but you lose
the congruence guarantee, which is the half that prevents the `sibling: true` class of bug.**

### 5b.3 Option B — scalarset symmetry reduction (the literature match for RC-4) — **and egg is NOT this**

**The two problems are different and must not be conflated:**

| Problem | Right tool |
|---|---|
| **Ability SEMANTICS** — "does this AST observe/scale-with the growing axis?" | ⭐ **egg e-class analysis** (5b.2) |
| **Board-state EQUALITY** — "is this the same board modulo id churn?" | **scalarset symmetry reduction** (below) — **NOT egg** |

**Why egg is unsound for the second.** Congruence/bisimulation is **coarser than isomorphism** and it
**COLLAPSES MULTIPLICITY**: three identical Saprolings and four identical Saprolings are *the same term*,
so they hashcons to *the same e-class*. **But multiplicity IS the growth axis we are measuring.** Accepting
on congruence certifies iteration N ≡ N+1 **precisely when the token count grew** — i.e. **exactly on every
real loop** — which by §5b.1 is a **false certificate on the game-ending path.** ⛔ **Never accept on
congruence. Reject-only, or not at all.**

**`ObjectId` is a *scalarset*.** Two boards are the same board **up to a permutation of object
identities**; permuting them induces automorphisms of the state graph; the fix is a **canonical
representative per orbit**. This is [Murφ](http://www.cfdvs.iitb.ac.in/download/Docs/verification/tools/murphi/html/murphiinfo.html)'s
symmetry reduction, and it is **exactly** what §5 prescribes (*"remap `ObjectId`s to a canonical order
AND canonicalize every id-valued field"*). Decades-old, with published correctness proofs — which is
precisely what P6's soundness obligation is asking for.
([survey](https://www.doc.ic.ac.uk/~afd/papers/2006/ACMSurvey.pdf))

**And it hands us the safe engineering split — this is the actionable part:**

| Murφ strategy | Property | For us |
|---|---|---|
| **Normalization** (lightweight) | may yield **several** representatives per orbit ⇒ errs **TOO FINE** | ✅ **misses some loops; never certifies a false one. SHIP THIS FIRST.** |
| **Canonicalization** (heavyweight) | **unique** representative per orbit ⇒ **exact** | graph-iso-hard, but **board sizes are tens of objects ⇒ nauty-class tools are effectively free here**. The upgrade path. |

Rust bindings exist: [`graph-canon`](https://github.com/noamteyssier/graph-canon) (nauty),
[`nauty-pet`](https://docs.rs/nauty-pet) (petgraph), [`canonical-form`](https://github.com/avangogo/canonical-form).

⚠️ **Do NOT reach for 1-WL / colour refinement as the equality test.** It errs **coarse** (cannot
distinguish some non-isomorphic graphs) ⇒ **wrong direction** ⇒ false certificate. It is admissible
**only** as a reject filter, same as 5b.2(1).

### 5b.4 The spike, and how it terminates — **run it FIRST; it re-shapes everything downstream**

**Time-box it. It is a decision, not a project.** `egg` is **not a requirement** — if it does not pay,
disregard it and the plan reverts to §6 as written. Deliverable is an answer to exactly four questions:

1. ⭐ **Can `Axes` be expressed as an `egg::Analysis` over the existing ability AST — with NO rewrite
   rules?** Build it, and check it against the two known-wrong verdicts: **Intruder Alarm must NOT be
   rejected** (it is CR 732.2a's own example), and **Gaea's Cradle must STILL fail closed**
   (`for_each_creature_production_still_fails_closed`, the revert-probe-verified guard). **If both hold,
   this option has proven itself on the exact case that broke the hand-rolled walk.**
2. **Do P2 and P5 collapse into queries against it?** *If yes, the plan's honest new surface drops from
   two subsystems + C2 to essentially C2 — the single biggest scope reduction available.*
3. **WASM bundle delta** of adding `egg` to the engine crate. **The detector is on the live in-game path
   and cannot be feature-gated out.**
4. **Confirm no semantic rewrite rules crept in.** ⛔ Any rewrite is a CR claim (5b.2). **A spike that
   ships rewrite rules has failed, regardless of its numbers.**

**Expected outcome (stated so the spike can refute it):**
- **AST layer** → `egg` e-class analysis, **congruence + `Analysis`, zero rewrites.** Fixes RC-1 at the
  root; P2 and P5 collapse into it.
- **State-equality layer** → **NOT egg.** Option B (scalarset) **normalization** as the accept-relation
  (errs fine ⇒ fail-closed), canonicalization as the proven upgrade. Optionally an `egg` congruence hash
  as a **reject-only** pre-filter in front of it.

**What kills this section:** (1) failing — i.e. an `Analysis` cannot reproduce `Axes`' *correct* verdicts
without rewrites — or (3) being unacceptable. **Then disregard `egg` entirely and run §6 as written.**

> **Nothing here changes the DRIVE.** Δ is still measured by executing the fixed sequence on a clone
> (§4.6). This spike replaces (a) the **ad-hoc AST analysis** that is RC-1's root and (b) the **equality
> relation between frames** that §5 proved we cannot hand-roll safely. **Both are analysis, not
> semantics — and the rules stay where they are.**

---

## 6. Implementation plan — **RE-SEQUENCED** (arming is a PREREQUISITE, not the last phase)

> ### ⭐ RUN §5b's SPIKE BEFORE P2 AND P5.
> Both are AST analyses, and **RC-1's root is that the AST analysis (`Axes`) is hand-rolled and wrong.**
> If §5b.4(1) succeeds, **P2 and P5 collapse into queries** against one principled e-class analysis and
> the plan's new surface drops to essentially **C2 alone.** If it fails, run P2/P5 as written below.
> **`egg` is not a requirement — it is a scope reduction to be earned.**

**Six of the fifteen §7 test rows never reach the code they claim to test**, because `engine.rs:445`
gates the entire hook on `last_recast_context.is_some()`. Presence of Gond, Earthcraft, Cryptolith Rite,
Manaforge Cinder, Crucible+Zuran Orb, and Basalt Monolith are **all activation or land-play loops** ⇒
**nothing arms** ⇒ they decline **vacuously** and **no revert-probe can flip them**. **Arming must come
first or the test matrix is theater.**

### P0 — `run_combo_live`: the DUAL of the corpus harness (tests only; no fix)

`corpus.rs:1175` — `run_combo(board, step)`, where **"`step` drives exactly ONE loop iteration's
actions"** — **`step` IS the CR 732.2a fixed sequence.** A human writes it; `detect_loop` merely *judges*
it. The live path must **discover** the same cycle. Build the dual, sharing `ComboRow` / `ComboBoard` /
`step`:

- `ComboDriver::Offline(f)` → route-agnostic `Cycle(f)`; `DRIVERS` (`corpus.rs:673`) stays the single
  source of truth so its meta/partition tests extend for free.
- **`run_combo_live(board, step)`** drives `step` through the **real `apply()` reducer**.
  ⚠️ **First verify `LoopProbe` is not an offline-only abstraction that bypasses `apply()`** — if it is,
  P0 needs redesign. **UNVERIFIED; check before building.**

**The partition has THREE terminals, not two** (CR 104.4b makes this a rules distinction):

| Partition | Rows | Live terminal |
|---|---|---|
| **L-OFFER** — cycle contains ≥1 **optional** player action | the 10 `Offline` drivers + the 13 `ObjectReentry` + 20 `Other` + 1 `ColorConverting` | **must** reach `WaitingFor::LoopShortcut` |
| **L-AUTOWIN** — **mandatory** cascade, no player action | **17** (Sanguine Bond + Exquisite Blood), **18** (Marauding Blight-Priest + Bloodthirsty Conqueror) | **must** reach `WaitingFor::GameOver`; **must NOT offer** (CR 104.4b) |
| **WAIVED — by ENGINEERING, not by rules** | **32** Aggravated Assault + Sword · **33** Combat Celebrant + Helm · **34** Time Sieve + Thopter Assembly | none today — ⚠️ **CR 732.2a explicitly permits these** (*"may even cross multiple turns"*). **Waive LOUDLY, with the CR quote in the exclusion comment.** Silently bucketing them as "offline-only" is exactly the dressing-a-cut-as-a-rule that D5 forbids. |

**The invariant:**
```
certifies_offline  ==  (offers_live XOR auto_wins_live)      // for every non-WAIVED row
```
- **⇒ failing = RC-3** (false negative in real play). **Today all 10 `Offline` rows fail it** — including
  **row 1, Kilo + Freed + Relic**: *the corpus already certifies Combo B offline and has never once
  offered it live.*
- **⇐ failing = UNSOUNDNESS** — the live path certifying what the analyzer rejects. **Must never go red.**

> ⚠️ **FIX THE ASYMMETRY UPWARD, or this invariant will make things worse.** `run_combo` requires **one**
> covering pair after `WARMUP`; the live path requires **two, from iteration 0**. The bi-implication
> therefore applies pressure to **relax the live path to one pair** to go green — degrading the only
> **game-ending** path, and it will look like progress. **Make `run_combo` ALSO require two consecutive
> covering pairs with equal Δ.**

Also: real cards, real libraries, real mana bases; port
`object_growth_51st_sprout_swarm_covers_and_offers` onto them (**it must FAIL today**); add **Presence of
Gond + Intruder Alarm** as a first-class row.

### P1 — RC-3: ONE generalized arming context + driver ⚠️ **THIS IS A REWRITE, AND IT IS A PREREQUISITE**

**Do NOT add `last_activation_context` as a sibling** (sibling-cluster smell). **But do not let the naming
fix disguise the cost** — measured, `drive_recast_iteration` (`engine.rs:1451`) has **eight structural
cast-shaped elements and exactly one parameter (controller)**:
hardcoded `GameAction::CastSpell{payment_mode: Auto}` (:1469) · card re-find by `(card_id, from_zone,
controller)` (:1460) · `DecideOptionalCost{pay: ctx.uses_buyback.pays()}` — **buyback by name** (:1487) ·
`ManaPayment` resolves **`ConvokeTaps` pins only**, every other `ConcreteDecision` ⇒ `Err(RecastAbort)`
(:1527) · `_ => Err(RecastAbort)` (:1548) — **where Combo B's `WaitingFor::PayCost{TapCreatures}` lands** ·
`build_recast_template` emits `[ConvokeTaps]` (:1558) · `normalize_recast_frame` (:1599) ·
`derived_fodder_class` fails closed unless **exactly one** new battlefield object (:1633).
**`RecastContext` has no action field — the action is implied by the type.**

- Build `LoopProbeContext { actions: Vec<GameAction>, controller, decisions }` — **`actions` is a
  SEQUENCE** (CR 732.2a *"choices"*, plural; three drivers are multi-action; **Combo B is two**).
- Build `drive_loop_iteration(&[GameAction])`. **New context + new driver + new
  `PinnedDecision`/`ConcreteDecision` variant ⇒ the `/add-engine-variant` gate is MANDATORY and is a hard
  prerequisite, not a conditional.** Grep `data/engine-inventory.json` first.
- ⚠️ **A NEW CHEAP NECESSARY-CONDITION PRE-GATE IS REQUIRED.** Commit `57b0e537d` bounds shortcut
  **EXECUTION** (`MAX_SHORTCUT_CYCLES` caps the post-acceptance replay) — **not DETECTION.** The pre-offer
  clone-drive is bounded today by exactly one thing: **it almost never runs**
  (`last_recast_context.is_some()`, `engine.rs:449`). Remove that and the drive runs on **every player
  action at every empty-stack priority beat**: 3× full `GameState::clone()` + 2× a cascade whose beat cap
  is `auto_pass_loop_max_iterations` = **`.clamp(500, 10_000)`** (`engine.rs:2413`), each beat re-running
  `flush_layers`. **Without a new pre-gate, the #5672 remote DoS is the deliverable.**
- **Leave `engine.rs:3081` and the ring alone** — arming, not the ring, is the fix (§3.3).

### P2 — RC-1(b): a real CR 113.6 zone-of-function predicate ⚠️ **IT DOES NOT EXIST — NEW CODE**

- `active_trigger_definitions` (`functioning_abilities.rs:391`) implements **NO CR 113.6 logic** — it
  gates only phased-out (CR 702.26b) and non-emblem command zone. `battlefield_active_triggers` (:416) is
  literally `state.battlefield × active_trigger_definitions`. **So "use `battlefield_active_triggers`" IS
  "hard-code battlefield-only"** — the thing CR 113.6 forbids. **The predicate must be written.**
- **CR 113.6's exceptions are live and verified** (`docs/MagicCompRules.txt:771–793`): **113.6b/c**
  (abilities stating their zones), **113.6j** (an activated ability whose cost can't be paid on the
  battlefield — Reassembling Skeleton), **113.6k** (a trigger condition that can't trigger from the
  battlefield), **113.6d/e/f** (cost/play-modifying abilities function **on the stack and in the zone the
  object would be cast from — including the HAND**). **CR 400.2 is about HIDDEN zones; CR 113.6 is about
  FUNCTION. Do not conflate them.**
- **R4's fix is mis-aimed.** `active_replacements` (`functioning_abilities.rs:446`) is **deliberately**
  all-zones, and its doc names the real runtime authority: **`find_applicable_replacements`
  (`game/replacement.rs`)** restricts to `[Battlefield, Command]` + the entering/discarded card. **Share
  THAT predicate.**
- **Permanent guard test:** the verdict must not change when an arbitrary card is added to any library or
  hand.

### P3 — RC-2: tolerate the bounded start-up transient (CR 732.2a D3)

- Drive until the cover holds on **two consecutive pairs with equal Δ**, rather than on the first two.
  **The SKIP is sound** — two consecutive covering pairs at offset *k* is exactly as strong as at offset 0
  (`board_covers_modulo_fodder` already demands exact content equality on the whole stable partition,
  `resource.rs:1040`).
- ⚠️ **DO NOT SHIP THE POPULATION BOUND.** *"Non-fodder population + 2"* is a **heuristic**, and §8 admits
  it while P3 previously shipped it as a theorem. **Use the DoS cap:** drive to the cap, take the first
  *k* with two consecutive equal-Δ covering pairs, **decline loudly on overflow.** The population bound
  buys nothing (the cap already bounds it) and **is the only place in this phase an unsound argument can
  hide.**

### P4 — C2: place non-depletion (**the only phase that was correctly sized**)

| Gate / place | Authority | CR |
|---|---|---|
| `activation_restrictions` | `ability_has_per_turn_activation_gate` (`resource.rs:2848`) | — |
| trigger `OncePerTurn` / `MaxTimesPerTurn` | `project_out_resources` (already preserved) | — |
| loyalty | `loyalty_activation_counts_match` | **CR 606.3** |
| land plays | *(new axis, or exclude loudly)* | **CR 305.2** |
| **summoning sickness — a PLACE SPLIT** | `AbilityCost::Tap` vs `AbilityCost::TapCreatures` (`ability.rs:7841`); enforced only on the former via `cost_contains_tap_or_untap` (`restrictions.rs:675`) | **CR 302.6** |
| library size | `library_delta` (in `has_no_loss_axis`) | **CR 704.5b** |
| **opponent's non-pass action required ⇒ REJECT** | — | **CR 732.3** |

A blanket *"reject any `{T}` cost"* would decline **CR 732.2a's own example** and most creature mana
engines. Exhaustive typed enum + `_ => REJECT` + no-`..` totality guard.

### P5 — C3: the firewall becomes a THRESHOLD scan — **mostly a DELETION** (see §4.6's governing constraint)

**The drive measures every effect the current board produces.** The firewall's only remaining job is the
**discontinuity** the drive is structurally blind to: **a threshold that trips at a future iteration
count.** Everything else it currently does is duplicated work that gets the answer wrong.

- **DELETE gate (1)'s EFFECT scan.** `ability_definition_reads_sibling_mutable` (`ability_scan.rs:3767`)
  → `TargetFilter::Typed(_) => Axes { sibling: true, .. }` **unconditionally** (`:2454`). This rejects
  **Intruder Alarm — CR 732.2a's own worked example** — and **Suture Priest**, and every Commander
  permanent. **Effects are the drive's job.** Re-scoping cannot save it: Intruder Alarm is **on the
  battlefield.**
- **KEEP gate (4)'s CONDITION scan — it is already the right place** (`resource.rs:1524` inspects
  `def.condition`). **Narrow it** to: *a fire-time `Comparator` whose operand is the growing axis*, on an
  ability that **functions** in its zone (CR 113.6, via P2). Replace the blanket
  `if !def.modifications.is_empty() { return true }` (**R5**) and any "any condition ⇒ reject".
- **Retain the `projected` cost axis** and its firewall (`R-e2`, `resource.rs:5052`) — it catches
  `ModifyCost{dynamic_count}` (**Damping Sphere**). *(A scaling cost also moves Δ, so C1 backstops it —
  but keep the axis; belt and braces on the only game-ending path.)*
- **DELETE R3** (activated-ability bodies — an activated ability observes nothing unless *activated*, and
  the fixed sequence **pins** whether it is), and **R6**. **R6 (`delayed_triggers` non-empty ⇒ reject) is
  what blocks Kiki-Jiki and Splinter Twin** (§5 Group A) — every Kiki token carries *"sacrifice it at the
  beginning of the next end step"*. **Deleting R6 is worth 2 corpus rows on its own**, and it is sound
  because the delayed trigger **fires in the drive** and lands in Δ.
- **Soundness note:** this phase is the one place the plan makes the detector **less** conservative.
  Every deletion must be justified by *"the drive measures this"* — and the **⇐ direction of P0's duality
  invariant is its runtime guard.** If a deletion makes the live path certify something `detect_loop`
  rejects, **that is the alarm.**

### P5.5 — SPIKE: buy the equality core, don't hand-roll it (**run this BEFORE P6 — see §5b**)

**Time-boxed decision, not a project.** §5 proved we cannot hand-roll the equality relation safely; §5b
names two off-the-shelf formalisms and the asymmetry that decides between them.

- Evaluate **`egg`** as (i) a **congruence-closure REJECT-ONLY pre-filter** — which, if it works, **also
  discharges P1's open DoS pre-gate**, closing two holes at once — and (ii) the **semilattice** for
  `Quotient` monotonicity.
- **HARD KILL CRITERION: congruence is COARSER than isomorphism and COLLAPSES MULTIPLICITY** (3 identical
  Saprolings and 4 identical Saprolings are the same term ⇒ the same e-class). **Multiplicity IS the
  growth axis.** If the design drifts to *accepting* on congruence, it certifies iteration N ≡ N+1
  exactly when the tokens grew — **a false certificate on the game-ending path. Stop and take Option B.**
- **Measure the WASM bundle delta.** The engine ships to WASM (`opt-level='z'` + LTO) and **the detector
  is on the live in-game path — it cannot be feature-gated out.**
- **Expected outcome, stated so the spike can refute it:** `egg` as a **reject-only pre-filter** + **Option
  B (scalarset) normalization** as the accept-relation.

### P6 — RC-4: object identity across a loop cycle ⚠️ **ITS OWN PR, WITH ITS OWN SOUNDNESS PROOF**

Per §5: **not** a refactor and **not** a quick win. Requires **id-canonicalization of the whole frame**
(remap `ObjectId`s to a canonical order **and** canonicalize every id-valued field: `attached_to`,
`attachments`, `paired_with`, stack targets, delayed-trigger references). **This is where a false
certificate enters** — two boards can be content-equal per-object yet differ in *which object the stack
points at*.

**Take the formalism P5.5 selects — do not invent one.** The literature match is **Murφ scalarset
symmetry reduction** (§5b.3): `ObjectId` **is** a scalarset, and the safe sequencing is **normalization
first** (errs **too fine** ⇒ misses loops ⇒ **fail-closed**) with **canonicalization** (exact; nauty-class,
effectively free at our board sizes) as the proven upgrade. **Never 1-WL / colour refinement as the accept
relation — it errs coarse.**

Ship the `Quotient` parameterization (one `loop_states_cover(prior, current, &[Quotient])` replacing the
four `loop_states_cover_modulo_*` siblings) **here**, earned by the canonicalization proof — not asserted
as a refactor. **Target: §5 Group B (7 rows).**

---

## 7. Verification matrix

⚠️ **Six rows in the previous matrix were VACUOUS** — dominated not by the RNG gate but by the **arming
gate** (`last_recast_context.is_some()`, `engine.rs:445`). **P1 is a prerequisite for every row below
marked †.** Every negative names its **paired positive reach-guard**.

| Claim | Seam | Test | Revert-probe (must FLIP) | Reach-guard / hazard |
|---|---|---|---|---|
| ⭐ **THE DUAL (⇒ coverage)** | `run_combo_live` vs `run_combo` | `certifies_offline ⇒ (offers_live XOR auto_wins)`. **Today 10 certify, 0 offer** | revert P1 ⇒ **every row but Combo A goes red** | **the reach-guard for the whole plan.** Only Combo A + B green ⇒ **did not generalize; do not ship** |
| ⭐ **THE DUAL (⇐ SOUNDNESS)** | ″ | `offers_live ⇒ certifies_offline`. **Must NEVER go red** | — | ⚠️ **first make `run_combo` require 2 covering pairs** — else the invariant pressures the live path to *relax* (B7) |
| **L-AUTOWIN stays autowin** | `interactive_loop_bridge` (`engine.rs:492`) | rows 17/18 reach `GameOver`, **must NOT offer** (CR 104.4b) | — | proves the 3-terminal partition |
| Combo A certifies on a real board | `try_offer_object_growth_shortcut` | `real_board_sprout_swarm_offers_loop_shortcut` (**FAILS today**) | — | — |
| **RC-1 + RC-2 are BOTH required** | — | the acceptance test **must still fail after P2 alone** | — | a green-after-P2 result is a **false positive** |
| **CR 113.6 / 400.2 invariance** | zone predicate | `real_board_verdict_is_invariant_under_hidden_zone_contents` | restore all-zones scan | **asserts the OFFER in every arm** — `assert_eq!` alone passes vacuously as `false==false` |
| **CR 113.6 exceptions preserved** | zone predicate | a **113.6j** (Reassembling Skeleton, graveyard) and a **113.6k** ability are still scanned | hard-code battlefield-only | catches the P2 trap |
| **RC-2 bounded transient** | two-pair cover | verdict invariant under **which green creature the real cast convokes** | restore the `(cs_n,cs_n1)` requirement | **assert the OFFER in every arm** |
| ⭐ **CR 732.2a's own example** † | end-to-end | **Presence of Gond + Intruder Alarm** OFFERS | restore the typed-filter `sibling:true` arm | **this is the C3 discriminator** — it is what proves P5 |
| **C2 sickness (the crux)** † | cost shape (CR 302.6) | ⚠️ **REDESIGNED.** Hold the LOOP fixed, vary ONLY the cost shape: **Earthcraft + Squirrel Nest CERTIFIES**; the same board with Earthcraft's cost replaced by a creature-`{T}` grant DECLINES | collapse the sick/unsick split | ⚠️ **the old pair was VACUOUS: Cryptolith Rite + Squirrel Nest is NOT A LOOP AT ALL** (nothing untaps the land), so it declined for the wrong reason and the split was never consulted |
| **C2 activation gate** † | `ability_has_per_turn_activation_gate` | **Manaforge Cinder** DECLINES | remove the axis | ⚠️ the old reach-guard was incoherent (*"remove the mana source ⇒ OFFER"*). **Specify the loop board.** |
| **C2 land drops** † | `lands_played_this_turn` | **Crucible + Zuran Orb** DECLINES | remove the axis | board minus Crucible must OFFER |
| **C2 fragmented loop** | transition set | a sequence needing an **opponent's** non-pass action DECLINES | drop the check | CR 732.3 |
| **C1 scaled cost** | Δᵢ vs Δᵢ₊₁ | ⚠️ **Damping Sphere**, NOT Hum of the Radix (*"each **artifact** spell"* — cannot affect a green instant; **both arms would OFFER**) | drop the `projected` axis (preserve `R-e2`) | board minus Damping Sphere must OFFER |
| **C4 self-deck** † | `has_no_loss_axis` | **Basalt Monolith + Mesmeric Orb** DECLINES | drop `library_delta >= 0` | ⚠️ **VACUOUS TWICE**: dominated by arming, **and** post-P1 it is a *mill* loop with **no fodder and no counter growth ⇒ no cover applies at all** ⇒ the axis is still never consulted. **Needs a cover before it is a test.** |
| **C4 adverse scaling** | `has_no_loss_axis` | opponent's **Suture Priest** ⇒ Combo A DECLINES | drop `life >= 0` | ⚠️ **VACUOUS today**: Suture Priest's typed filter trips gate (1) ⇒ the cover fails at `engine.rs:1728` **before** the triple at `:1756` runs. **Only valid after P5.** |
| Δ measured, not derived † | drive | **Solemnity** + proliferate DECLINES (true Δ = 0) | derive Δ from the AST | board minus Solemnity must OFFER |
| **Combo B** † | `LoopProbeContext{actions}` | Kilo + Freed + Relic OFFERS — **a TWO-action cycle** | — | assert `engine.rs:3081` + the DoS cap are untouched |
| **DoS** | new pre-gate | generalized arming does **not** regress #5672 | remove the pre-gate | **the drive must not run on every priority beat** |
| Gaea's Cradle stays closed | `scan_mana_production` | `for_each_creature_production_still_fails_closed` (**exists, revert-probe verified**) | collapse count-arms to `Axes::NONE` | `fixed_production_reads_nothing` still passes |
| **Multiplayer** | — | ≥1 criterion exercises **>2 players** (the fixture is 4-player) | — | — |
| Corpus regression | `analysis/corpus.rs` | the 12 driven rows still certify; the partitions hold | — | corpus is **53** rows, not 55 |

---

## 8. Open questions — do NOT hand-wave (this document has been wrong ten times)

1. **Is Δ-constancy + place non-depletion SUFFICIENT?** Manaforge Cinder has Δ₁=Δ₂=Δ₃ and is illegal at
   **4** (C2 catches it, not C1). **Prove no third failure mode exists** — a change at iteration ≥3 that
   neither alters Δ nor depletes a modelled place. **Not attempted. Still a real proof obligation.**
2. **Is `LoopProbe` drivable through `apply()`**, or is it an offline-only abstraction? **If the latter,
   P0's dual is not buildable as specified. UNVERIFIED — check first.**
3. **What blocks the 20 `Other` deferral rows?** **UNVERIFIED** — only `ObjectReentry`(13),
   `ExtraTurnOrCombat`(3) and `ColorConverting`(1) were classified.
4. **What happens AFTER an offer is ACCEPTED?** `materialize_fixed_shortcut` — does the replay correctly
   re-execute the **transient prefix**? **UNVERIFIED.**
5. **Does `Effect::Proliferate` trip the firewall?** i.e. does Kilo's own trigger self-reject? **UNVERIFIED.**
6. **The C3 replacement predicate.** *"Could this replacement apply?"* needs a real event-type × filter
   match. A blanket *"any replacement exists ⇒ reject"* is useless on a Commander board.
7. **P6's canonicalization soundness proof.** Content-multiset equality is where a false certificate
   enters (§5). **This is the proof obligation that gates P6.**

---

## Appendix A — Design principles

1. **Scope every conservatism to the present board and the sequence actually executed** — never to all
   boards reachable from all cards in all decks and hands. Reaching into a library is a **CR 113.6** error
   *and* a **CR 400.2** violation.
2. **The loop must be infinite from the PROPOSER's perspective** (CR 732.2a), then **passed around for
   response** (CR 732.2b). Interaction is the response window's job, not the cover's.
3. **Monotone reads are not hazards.** A firewall rejecting *"references a typed filter"* rejects the
   rulebook's own example.
4. **Measure, don't derive.** Replacements rewrite Δ at resolution; SBAs and triggers settle between
   iterations. Only the drive sees the truth.
5. **Real cards, real libraries, real mana bases** in every combo-detector test.
6. **Read the rule, don't cite it.** Every architectural correction here came from the rule *text*.
7. **The rules work has held; every failure was a CODE claim from memory.** Ten for ten. **Grep before you
   assert, and put the file:line in the sentence.**

## Appendix B — What we got wrong (ten times)

| # | Claim | Reality |
|---|---|---|
| 1 | *"No counter-growth cover exists"* | **FALSE.** `loop_states_cover_modulo_counter_growth` (`resource.rs:1329`) exists, names **Pentad Prism**, is wired into `detect_loop` + `interactive_loop_bridge`, has 4 tests. |
| 2 | *"`ResourceVector` already computes these deltas"* | **FALSE.** No tap-state axis; `mana` summed across all players; growth axes zero under `snapshot`. |
| 3 | *"The payment choice is inexpressible"* | **FALSE.** Witherbloom is **Legendary**; Relic filters on `Legendary`. |
| 4 | *"Convoking Witherbloom is illegal at iteration 2 ⇒ the proposer must SEARCH"* | **FALSE, and it inverted the fix.** `select_convoke_taps` re-runs each iteration; the *place* is non-depleting (Δ=0). The real defect is **RC-2**: a **bounded transient** the cover forbids — which **CR 732.2a explicitly permits**. |
| 5 | *"Gaea's Cradle fail-closes via `repeat_for`"* | **FALSE.** It parses as `AnyOneColor{count: Ref(ObjectCount{Creature,You})}` — caught **only** by `scan_mana_production`. **Do not "simplify" that walker.** |
| 6 | *"Combo B's cycle is ONE activation"* | **FALSE.** `drive_offline_kilo_freed_relic` (`corpus.rs:1556`) takes **TWO** `ActivateAbility` actions. Its comment: *"Relic has two mana abilities; the tap-self one would not fire Kilo's trigger."* The CR 605.3a nesting story is **rules-legal but engine-false** — a mana ability in a `ManaPayment` window is still its own `GameAction` (`engine.rs:4867`). **A single-action arming latch cannot capture it.** |
| 7 | *"Generalizing `normalize_recast_frame` lifts all 13 `ObjectReentry` rows — worth more than Phases 1–5 combined"* | **FALSE.** It lifts **ZERO** directly. 6 rows are blocked by R6/RC-1/RC-3, not id churn. The other 7 need **id-canonicalization** — and stripping the object **does not fix stable objects whose `paired_with`/`attached_to` point at the churned id** (Deadeye Navigator never moves and still fails). **The riskiest change in the program, not a quick win.** |
| 8 | *"C3 is the one arm three rounds never broke — keep its logic"* | **FALSE, and it contradicted §3.1.** `ability_scan.rs:2454` sets `sibling: true` for **any** typed filter ⇒ the predicate rejects **Intruder Alarm — CR 732.2a's own worked example.** **C3 is a rewrite.** |
| 9 | *"Measured trips, in order"* (RC-1) | **Wrong provenance.** `board_covers_modulo_fodder` runs first (`resource.rs:1119`) and returns false before the firewall (`:1132`). Both root causes are real; the trips were seen under instrumentation, not on the live path. |
| 10 | *"Hum of the Radix DECLINES"* | **UNSATISFIABLE.** *"Each **artifact spell** costs {1} more"* — Sprout Swarm is a green instant. **Both arms OFFER.** The card is **Damping Sphere**. |
| — | *"The untap step is CR 502.2"* | **FALSE.** 502.2 is day/night. It is **CR 502.3**. |
| — | An LP / Petri-VAS model would replace the drive | **Unsound.** Δ is not derivable (replacements); legality is not a resource. |
