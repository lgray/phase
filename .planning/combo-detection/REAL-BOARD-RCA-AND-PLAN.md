# Combo detector — root-cause analysis + implementation plan
### Making CR 732.2a loop shortcuts work on real decks and real board states

**Date:** 2026-07-14 · **Status:** Plan. Implementation NOT started.
**Branch:** `debug/combo-generator` (fork-only; **never** merge toward `main` — `.planning/` is gitignored).

> ### Evidence standard — read this before trusting any line below
> Every claim here is **measured**: by grepping the source, by grepping `docs/MagicCompRules.txt`, by reading
> `data/card-data.json`, or by querying the real board fixture.
> **Every `file:line` in this document was printed by a tool, not recalled.**
>
> ## **This plan has been wrong EIGHTEEN times. Every single failure was a CODE claim asserted from memory. The RULES layer has never once failed — 40/40 CR citations and 32/32 Oracle texts verified across six audits.**
>
> The failures are catalogued in **Appendix B**. Read it: the guard rails only work if you know what they
> guard. **THREE of the seventeen (#15, #16, #17) were committed *while writing this document*.** #15/#16 are
> the **same fabrication**, invented independently by two authors an hour apart. **#17 is this plan asserting
> its own root-cause fix was *"necessary and sufficient"* — refuted by BUILDING IT AND RUNNING IT.** All three
> were caught only because someone re-measured. **That is not an argument against the discipline. It is the
> argument for it.**
>
> **The rule that would have prevented all eighteen: grep before you assert, and put the `file:line` in the
> sentence.** If you cannot verify it, write **UNVERIFIED**. An honest *"I did not reach this"* beats a
> plausible claim that costs a cycle to refute.
>
> **And the two corollaries that were each learned the hard way:**
> - **#15/#17 — A correct reading of the CODE is not a correct claim about the BOARD. Reachability must be
>   measured too.**
> - **#17 — A SINGLE-FIXTURE PROBE CANNOT DISTINGUISH A CLASS FIX FROM A CARD FIX.** A probe on a clean
>   two-card board "proved" a fix that does nothing on the real one. **Only the corpus dual (P2) can tell them
>   apart — which is why P2 is in Tier 1.**
>
> **And the corollary that caught #15:** a correct reading of the *code* is not a correct claim about the
> *board*. **Reachability must be measured too.**

---

# §0 — READ THIS FIRST

*One page. If you read nothing else in this document, read this.*

## ⭐ THE GOVERNING RULE — it outranks everything below

> ## **Combo A and Combo B are ACCEPTANCE TESTS, not GOALS. Every phase fixes a CLASS. A change that turns a combo green without discharging a class property is the purpose-built patch this plan exists to prevent.**

**The combos are CANARIES.** *"`real_board_sprout_swarm_offers_loop_shortcut` goes green"* is **not** an acceptance criterion — it is a smoke alarm. **Every phase gates on a CLASS property**, and if you find yourself doing anything to make a combo green that is not derived from a class property, **STOP: you are writing the bug.**

**This is not theory. It was measured, on this plan, and it caught a false claim in this document's own root-cause section — see Appendix B #17.**

## The four rules that will get you killed

1. **GREP BEFORE YOU ASSERT.** Put the `file:line` in the sentence. **Eighteen errors; every one a code claim from memory.**
2. **A COARSE RELATION MAY *REJECT*, NEVER *ACCEPT*.** (§5b.1.) Too coarse ⇒ **false certificate ⇒ ends a real game wrongly.** Too fine ⇒ a missed offer ⇒ **safe**. The detector sits on the **only game-ending path**.
3. ## ⛔⛔ **P4 AND P7 ARE THE ONLY TWO PHASES THAT MOVE THE DETECTOR IN THE *ACCEPT* DIRECTION.**
   **Every other phase errs safe. These two do not.**
   - **P7's ~88 sites are not 88 chances to un-reject — they are 88 chances to WRONGLY CERTIFY.**
   - **P4 narrows what a REJECT gate scans.** Narrowing a reject gate ⇒ **fewer rejections ⇒ MORE ACCEPTS.**
     **An under-inclusive zone predicate is a FALSE-CERTIFICATE GENERATOR** — and one was written into this
     very document (**#18**). **Review every line of P4 and P7 against rule 2, twice.**
4. **NEVER RELAX `GameState::PartialEq`'s `delayed_triggers` CONJUNCT** (`types/game_state.rs:10875`). It is the trap-antidote (§4.9). An implementer chasing corpus rows *will* be tempted. **That way lies a false certificate.**

## ⭐ The bug, in one sentence

> **CR 732.2a Example** *(verbatim, `docs/MagicCompRules.txt:6373`)*: *"A player controls a creature enchanted
> by **Presence of Gond**… and another player controls **Intruder Alarm**, which reads, in part, 'Whenever a
> creature enters, untap all creatures.' … they may suggest **'I'll create a million tokens'** … repeating that
> sequence 999,999 more times."*

## ⇒ **THE ENGINE'S LOOP DETECTOR REJECTS THE COMPREHENSIVE RULES' OWN WORKED EXAMPLE OF THE RULE IT IMPLEMENTS.**

**That is the bug.** Not *"a combo doesn't fire."* **The rulebook prints the canonical shortcut, names the two
cards, and we decline it.**

## ⛔⛔ And here is WHY it went unnoticed for 88 sites: **THE TEST GATE IS HALF-BUILT.**

**Measured across the entire engine crate:**

| Guard | Exists? | What it pins |
|---|---|---|
| **NEGATIVE — "this MUST trip `sibling`"** | ✅ **YES, and it is discriminating.** `for_each_creature_production_still_fails_closed` (`ability_scan.rs:4840`) — Gaea's Cradle **counts** a mutable creature set ⇒ must fail closed. Its own doc: shipping the mistake *"falsely **CERTIFIES** an unbounded-mana loop — **strictly worse** than the false negatives this walker exists to fix."* |
| **POSITIVE — "this MUST NOT trip `sibling`"** | ⚠️ **ONLY THE TRIVIAL CASE.** The one *"must not trip"* assertion is `assert!(!ability_reads_sibling_mutable(&fixed_drain()))` (`:5215`) — and **`fixed_drain` is `GainLife{ amount: Fixed{1}, player: Controller }`: it references NO OBJECT FILTER AT ALL.** |
| **POSITIVE, DISCRIMINATING — "this REFERENCES a typed object filter but does NOT COUNT it, so it must NOT trip"** | ⛔ **DOES NOT EXIST.** And **`grep -rin "intruder" crates/engine/src/` returns ZERO.** Not a test, not a fixture, not a comment. |

> ## **The exact line P7 turns on — *NAMING a type* vs *COUNTING a mutable set* — is defended by NO existing test.**
> `fixed_drain` cannot defend it: it does **neither**. Gaea's Cradle cannot defend it: it wants `true`.
>
> ## ⇒ **THE CODEBASE CAN STRUCTURALLY DETECT OVER-ACCEPTANCE AND CANNOT DETECT OVER-REJECTION. It is a ONE-SIDED RATCHET — and it ratcheted 88 times.**
>
> **Every one of those 88 fail-closed defaults was FREE.** `sibling: true` costs a contributor nothing and
> trips no guard, so **the conservative arm always won.**
>
> ## ⇒ **RC-1 is not "someone wrote a wrong arm." RC-1 is "the suite only ever defended one side of the line."** **That is the class**, and it reframes P7 from *a fix* into **installing the missing half of a gate that was never symmetric.**

## The bug, in four lines

| | Root cause | Where |
|---|---|---|
| **RC-1** | ⭐ **The `sibling` axis is a FAIL-CLOSED DEFAULT over ~88 sites, consumed as if it were a PRECISE PREDICATE.** Every Commander permanent trips it. | `game/ability_scan.rs` (57 `Axes::CONSERVATIVE` + 31 `sibling: true`); consumed at `analysis/resource.rs:1457` |
| **RC-2** | The cover **forbids a bounded start-up transient** — CR 732.2a explicitly permits one. | `game/engine.rs:1732-1738` |
| **RC-3** | The live path **arms on ONE bespoke card shape**, and **zero corpus rows test it.** | `game/casting_costs.rs:6785` |
| **RC-4** | Loop equality is **id-keyed** — CR 400.7 makes that rules-wrong. | `types/game_state.rs:10428-10435` |

## The six checks

**C1** Δ-constancy · **C2** place non-depletion · **C3** threshold scan · **C4** the shipped triple · **C5** deferred execution (CR 603.7) · **C6** ∞-composition fixpoint.

**The drive measures what RESOLVES in the window. It is structurally blind to what the window SCHEDULES** — that is C5, and it is why Kiki-Jiki defeats C1–C4 simultaneously (§4.6–4.9).

## Landing order — **SEQUENCING, NOT SCOPE-REDUCTION**

> ⛔ **Tiers order DEPENDENCIES. They are NOT permission to ship less.** Every phase discharges a class. **No phase may ever ship on a card-level gate.**

| Tier | Phases | Class gate (**this** is the acceptance criterion) |
|---|---|---|
| **1** | **P2** (the dual) · **P4** (CR 113.6, **all four all-zones gates**) · **P5** (bounded transient) · **P7** (the `sibling` class fix) | **P4:** verdict invariant under **ANY** hidden-zone content · **P5:** verdict invariant under **WHICH** creature the cast convokes · **P7:** **Intruder Alarm un-rejects** (CR 732.2a's own example) **AND Gaea's Cradle stays fail-closed** · **P2:** the corpus dual holds **corpus-wide** |
| **2** | **P0** (mode binary) · **P1** (`LoopOutcome`) · **P3** (generalized arming) | **P3:** an *activation* loop, a *land-play* loop and a *cast* loop **all arm** — not just Combo B |
| **3** | **P6** (C2) · **P8** (C5) · **P9** (C6) · **P10** (RC-4) | soundness · composition · reach |

> ## ⚠️ **P2 IS IN TIER 1, AND IT IS NOT NEGOTIABLE.**
> **P2 (the `run_combo_live` corpus dual) is the ONLY instrument that can tell a CLASS fix from a CARD fix.** Ship Tier 1 without it and *"we built to the pattern"* is an **unverified assertion** — which is Appendix B's failure mode, one level up. **This was demonstrated the hard way: a single-fixture probe "proved" P7 and the real board still declined (#17).**

## Where things live

| You want… | Go to |
|---|---|
| **To EXECUTE** | **§6** (phases) · **§7** (verification matrix) · **§8** (open questions) |
| To understand *why* | §3 (root cause) · §4 (architecture) · §5–§5b (object identity; why `egg` was rejected) |
| **To not repeat a refuted claim** | ⭐ **Appendix B** — **17 errors, every one a code claim from memory.** **Read it before you assert anything.** |

---

## 1. Executive summary

**The combo detector cannot fire in any real game of Magic.** Two live infinite combos on a real 4-player
Commander board were verified undetectable. There are **four independent root causes**, and **no single one
of them is sufficient to fix.**

| | Root cause | Where (verified) |
|---|---|---|
| **RC-1** | **The fire-time observer predicate is wrong** — it rejects on *"references any typed object filter"* (which every Commander permanent does) **and** it carries a **second, unconditional veto on any live continuous modification** — **and** it scans **hidden zones**. | `analysis/resource.rs:1457` (gates **1**, **4**, **5b**) |
| **RC-2** | **The cover forbids a bounded start-up transient** — it demands recurrence from iteration 0. **CR 732.2a explicitly permits** a non-repetitive prefix followed by a loop. | `game/engine.rs:1732-1738` |
| **RC-3** | **The live path arms on ONE bespoke card shape**, and **zero corpus rows test the live path at all.** | `game/casting_costs.rs:6785` |
| **RC-4** | **Loop equality is keyed on `ObjectId`**, which **CR 400.7** makes rules-wrong. | `types/game_state.rs:10428-10435` |

**CI is green because the acceptance fixture builds a board that cannot exist in a real game** — no lands,
empty library, no auras, a stub oracle. All four root causes are invisible to it, and RC-3 means *nothing
anywhere* is looking at the live path.

### The two findings that outrank the bug

1. **The detector asks a question the rules do not.** It tries to prove *"no ability anywhere could ever
   observe this growth."* **CR 732.2a** (`docs/MagicCompRules.txt:6372`) asks only whether a sequence *"may
   be legally taken based on the current game state and the predictable results of the sequence of
   choices"*, and **CR 732.2b** (`:6375`) gives every other player the right to **accept or shorten**.
   Interaction is the response window's job, not the cover's.
2. **The scan reads hidden zones.** A `Solemn Simulacrum` **in the library** vetoes detection. Illegal
   twice: **CR 113.6** (`:771` — an object's abilities *"usually function only while that object is on the
   battlefield"*; the ability **does not exist** there) and **CR 400.2** (`:1935` — library and hand are
   **hidden zones**). **CR 113.6 is the primary authority.** *(Solemn's is an **ETB trigger**, which **can**
   trigger from the battlefield ⇒ exception **113.6k** does **not** rescue it ⇒ this is a pure violation.)*

### ⚠️ The honest new surface — state it ONCE, here, and never contradict it

| Phase | Surface | Size |
|---|---|---|
| **P0** `LoopDetectionMode` → binary | **deletion** of a footgun (user directive) | small |
| **P1** `WinKind` → `LoopOutcome` | **type split** — a soundness boundary (user directive); **prerequisite for C5 v2** | small |
| **P2** `run_combo_live` dual | **tests only**, no fix | medium |
| **P3** generalized arming + driver + DoS pre-gate | ⚠️ **A REWRITE, and a PREREQUISITE** | **large** |
| **P4** CR 113.6 zone-of-function predicate | ⚠️ **NEW CODE — it does not exist** | **medium** |
| **P5** RC-2 bounded transient | narrowing an existing cover | small |
| **P6** **C2** place non-depletion | ⚠️ **SMALLER THAN IT LOOKS — 3 of 5 axes already exist** (§P6) | small–medium |
| **P7** **C3** threshold scan — gates (1) **and** (4) | **two** narrowings; **(a) alone fixes the user's board** | medium |
| **P8** **C5** deferred execution (CR 603.7) | ⚠️ **A NEW CHECK — it REPLACES R6's `delayed_triggers` term** | medium |
| **P9** **C6** ∞-composition fixpoint | ⚠️ **A NEW CHECK** — ~20 lines, no solver; the store exists and has **no reader** | small–medium |
| **P10** RC-4 object identity | ⚠️ **its own PR, its own soundness proof** | **large** |

**The honest surface is: P3 (driver) + P4 (CR 113.6 predicate) + C2 + C5 + C6.** A plan that under-states its
own new surface will be executed as if it were small.

> ## ⛔ **DO NOT ASK "WHAT IS THE MINIMUM TO MAKE THE USER'S COMBO WORK?" THAT QUESTION BUILDS THE BUG.**
> A prior revision of this section answered it, and the answer was **measurably false** (Appendix B #17).
> **Every phase discharges a CLASS. The combos are CANARIES.** See **§0** for the landing order and the
> **class-level acceptance criteria.**
>
> **Tier 1 = P2 + P4 + P5 + P7.** **P2 is in Tier 1 because it is the only instrument that can tell a class
> fix from a card fix** — a lesson learned by shipping a single-fixture probe that "proved" a fix which does
> **nothing** on the real board.

---

## 2. Reproduction

- **Fixture:** `crates/engine/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json` (11 MB
  debug-panel export).
- **Harness:** `crates/engine/tests/integration/repro_user_combo.rs` (`FIXTURE` at `:27-30`).
- A bare snapshot is insufficient: **arming happens during a cast**, so the repro must drive a real cast.

```bash
cargo test -p engine --test integration real_board_fixture_is_intact   # PASSES (guards the fixture)
cargo test -p engine --test integration -- --ignored real_board        # FAILS  (the bug)
```

`#[ignore]`d and **failing**: `real_board_sprout_swarm_offers_loop_shortcut` (`:102`) and
`real_board_verdict_is_invariant_under_hidden_zone_contents` (`:146`). `real_board_fixture_is_intact`
(`:63`) is live and passing.

> **Note:** `repro_user_combo.rs:66` **asserts** `loop_detection == Interactive`; it does not *set* it. The
> value is carried by the fixture JSON (`"loop_detection":{"type":"Interactive"}`). The user had to pass
> `?loop=interactive` to reach the bug at all — see **P0**.

**Board:** Witherbloom, the Balancer (Legendary) + 4 untapped green Saproling **tokens** + Kilo, Apogee Mind
(Legendary, enchanted by Freed from the Real) + Relic of Legends + Pentad Prism (1 charge) + Forests/Islands.
Sprout Swarm in hand. `Interactive`, `Priority{P0}`, own turn, empty stack.

**Measured after the driven cast:** `last_recast_context` is armed **correctly**
(`card_id:415, controller:0, from_zone:Hand, uses_buyback:Used, convoke:Some`); **every cheap gate at
`engine.rs:445-451` is green**; `waiting_for` stays `Priority{0}`. **The decline is downstream, in the cover.**

### 2.1 The two combos — Oracle text verified verbatim in `data/card-data.json`

| Card | Oracle text (verbatim from the shipped DB) |
|---|---|
| **Sprout Swarm** | Convoke · Buyback {3} · *"Create a 1/1 green Saproling creature token."* |
| **Witherbloom, the Balancer** | Affinity for creatures · Flying, deathtouch · *"Instant and sorcery spells you cast have affinity for creatures."* |
| **Relic of Legends** | *"{T}: Add one mana of any color."* · *"**Tap an untapped legendary creature you control**: Add one mana of any color."* |
| **Kilo, Apogee Mind** | *"Haste."* · *"Whenever Kilo becomes tapped, proliferate."* |
| **Freed from the Real** | Enchant creature · *"{U}: Tap enchanted creature."* · *"{U}: Untap enchanted creature."* |
| **Pentad Prism** | Sunburst · *"Remove a charge counter from this artifact: Add one mana of any color."* |

**Combo A — Witherbloom + Sprout Swarm (object growth).** Through the casting rules:
- **CR 601.2b** (`:2459`) — announce buyback {3} ⇒ base {1}{G} + {3} = **{4}{G}**.
- **CR 601.2f** (`:2468`) — **CR 702.41a** (`:4318`) affinity (*"costs {1} less to cast for each [text] you
  control"*) is a cost **reduction**; ≥4 creatures ⇒ generic to {0}. **Total cost LOCKS IN.** Remaining: **{G}**.
- **CR 601.2h** (`:2472`) — **CR 702.51b** (`:4399`): *"convoke isn't an additional or alternative cost and
  applies only after the total cost … is determined"* ⇒ convoke is a **payment substitution**: tap one
  untapped **green** creature for the {G}. *(⇒ convoke **can** tap a summoning-sick creature: it is not the
  creature's own `{T}` ability, so **CR 302.6** (`:1630`) does not apply.)*
- Resolve: create a **green, untapped** Saproling; buyback returns the card.

⇒ **Δ(untapped green creatures) = −1 (convoked) + 1 (new green untapped token) = 0.**
⇒ **Δ(creatures) = +1**, so affinity only strengthens. **Legal for all N; the ω-axis is creatures.**

**Combo B — Kilo + Freed + Relic → Pentad Prism (counter growth). ⚠️ It is TWO actions, not one.**
The tree's own certifying driver — `drive_offline_kilo_freed_relic` (**`analysis/corpus.rs:1537`**) — takes
**two** `activate_and_resolve` calls (`:1559` Relic tap-creature, `:1565` Freed untap), each one
`GameAction::ActivateAbility` (`corpus.rs:1009-1026`). Its own comment pins why: ***"Relic has two mana
abilities; the tap-self one would not fire Kilo's trigger."*** Relic must be activated **standalone**,
selecting the `TapCreatures{Legendary}` cost, to tap **Kilo** and fire the proliferate trigger.

⇒ Δ(mana) = 0, Δ(Kilo tapped) = 0, **Δ(charge counters) = +1.** Unbounded counters ⇒ unbounded mana.

> **Counters, not mana, are the ω-axis.** **CR 106.4** (`:416`) / **CR 500.5** (`:2119`): unspent mana
> **empties at the end of each step and phase**. Mana is not durable. This is what the shipped
> `loop_states_cover_modulo_counter_growth` (**`resource.rs:1326`**) already certifies — **build nothing there.**

---

> ### 📕 **THE RECORD (§3–§5b).** Read this to *understand* a decision or to *reopen* a settled one.
> **To EXECUTE, skip to §6.** Nothing here is optional evidence — but nothing here is an instruction either.
> **Every warning that guards an instruction is repeated AT that instruction.** Argue once; warn everywhere.

## 3. Root cause

### 3.1 RC-1 — ⭐ **`sibling` is a FAIL-CLOSED DEFAULT used as a PRECISE PREDICATE**

> ## ⛔ **THIS SECTION WAS REWRITTEN AFTER A LIVE MEASUREMENT REFUTED IT. See Appendix B #17.**
> A prior revision called RC-1 *"a wrong arm"* and sized its fix at *"~10 lines, already probe-measured."*
> **I ran it on the real board. It is false.** The probe that "proved" the 10-line fix was run on a **clean
> two-card board** — a **vacuous discriminator**, the very thing this plan spends a whole section
> quarantining. **On the real board the 10-line fix changes nothing.**

**MEASURED, in an isolated worktree, by applying the fixes and running the failing test with every decline
point instrumented:**

```
XPROBE: try_offer ENTERED; armed=true          <-- arming is FINE  (⇒ P3 is NOT on Combo A's path)
XPROBE: drives OK                              <-- the driver drives Combo A 3× cleanly
XCOVER: FAIL at fire_time_conditions_read_growing_class
XGATE3: EXECUTE obj="Pentad Prism"    zone=Battlefield     <-- QuantityRef::ManaSpentToCast  (ability_scan.rs:2072)
        ...fix that arm, re-run...
XGATE3: EXECUTE obj="Choked Estuary"  zone=Battlefield     <-- Effect::RevealFromHand        (ability_scan.rs:910)
        ...and there are more behind it.
```

**Fix one arm, the next card appears.** **Pentad Prism is a card in the user's OWN combo.** **Choked Estuary
is a LAND.**

### ⇒ The real root cause, stated as a class

> ## **The `sibling` axis is a FAIL-CLOSED DEFAULT over ~88 sites — and the loop detector consumes it as if it were a PRECISE PREDICATE.**
>
> **Measured surface in `game/ability_scan.rs`: 57 `Axes::CONSERVATIVE` sites + 31 explicit `sibling: true`
> sites.** `Axes::CONSERVATIVE` is the walker's *"I have not analyzed this node"* default — and on the
> `sibling` axis that default **means *"this might read the growing class."*** **On a real Commander board,
> essentially every permanent trips at least one of the 88.**
>
> **The user's board trips it through at least THREE structurally different arms:**
> | Arm | Site | Why it is WRONG |
> |---|---|---|
> | `TargetFilter::Typed` | `ability_scan.rs:2456` | rejects **Intruder Alarm — CR 732.2a's own worked example** |
> | `QuantityRef::ManaSpentToCast` | `ability_scan.rs:2072` | **mana spent to cast is stamped at CAST TIME and immutable** — it *cannot* observe a token count. **(Pentad Prism)** |
> | `Effect::RevealFromHand` | `ability_scan.rs:910` | a hand reveal reads **nothing on the battlefield**. **(Choked Estuary — a land)** |
>
> ## ⇒ **THE FIX IS A CLASS FIX, NOT AN ARM FIX.** Re-derive `sibling` from a **POSITIVE** definition —
> ## ***"does this node read a MUTABLE OBJECT SET whose cardinality the loop changes?"*** — instead of
> ## defaulting to `true` and patching exceptions one card at a time.
>
> **Patching arms card-by-card is whack-a-mole, and it is exactly the purpose-built patch §0 forbids.**

**`sibling` is consumed by `fire_time_conditions_read_growing_class` (`analysis/resource.rs:1457-1591`), a
six-gate scan. Measured, gate by gate — this table is the map for P4 and P7:**

| gate | lines | scans | zone scoping | verdict |
|---|---|---|---|---|
| **(1)** | `1459-1477` | `active_trigger_definitions` → `def.condition` + **`def.execute`** | `state.objects.values()` — **NO zone filter** | ⛔ **wrong predicate + hidden zones** |
| (2) | `1478-1499` | every `obj.abilities` def, any kind | `zone != Battlefield` ⇒ skip | ✅ correctly scoped *(carries a `ponytail:` comment saying so)* |
| **(3)** | `1500-1523` | `active_replacements` → condition + `runtime_execute` + **`execute`** | all-zones — **NO zone filter** | ⛔ **HIDDEN ZONES + the arm that actually blocks the user's board.** ⚠️ **A prior revision listed gate (3) as "all-zones *by design*" and left it OUT of P4. That was WRONG** — the runtime authority (`find_applicable_replacements`, `game/replacement.rs`) restricts to `[Battlefield, Command]`, and **the firewall must share it.** **MEASURED: this is the gate that trips on Pentad Prism and Choked Estuary.** |
| **(4)** | `1524-1543` | `obj.static_definitions.iter_all()` — **statics only** | `state.objects.values()` — **NO zone filter** | ⛔ **UNCONDITIONAL veto + hidden zones** |
| (5) | `1544-1556` | `transient_continuous_effects` → duration + condition | n/a | ok |
| (5b) | `1557-1578` | `granted_keyword_triggers_in_zone` → condition + execute | `state.objects.values()` — **NO zone filter** | ⛔ **hidden zones** |
| **(6)** | `1582-1589` | **FIVE-way OR** (see below) | n/a | **KEEP — see §4.7** |

**(a) Gate (1)'s predicate is wrong.** It rejects if any live ability
`ability_definition_reads_sibling_mutable` (**`game/ability_scan.rs:3767`**). But **`ability_scan.rs:2454-2458`**:
```rust
TargetFilter::Typed(tf) => Axes {
    event: true,                                  // :2455
    sibling: true,                                // :2456   <-- UNCONDITIONAL
    projected: typed_filter_reads_projected(tf),  // :2457
},
```
**`sibling: true` for ANY typed object filter.** Measured consequence — **Intruder Alarm**, whose parsed
trigger is `SetTapState{target: Typed[Creature], scope: All, state: Untap}`, **trips gate (1) and is
rejected. Intruder Alarm is CR 732.2a's OWN worked example** (`docs/MagicCompRules.txt:6373`). The predicate
is not *"reads the growing class"*; it is *"references any typed object filter"* — which every Commander
permanent does.

> ## ⛔ **THE TYPED ARM IS ONE INSTANCE OF THE CLASS — NOT "THE FIX." (Appendix B #17.)**
> A prior revision called this *"a ~10-line root fix, already revert-probe-measured."* **The probe was run on
> a clean two-card board. On the REAL board, fixing this arm alone changes NOTHING** — `ManaSpentToCast`
> (Pentad Prism) takes over, then `RevealFromHand` (Choked Estuary), then the next. **See §3.1's instrumented
> run and P7 for the class fix.**
>
> **What is still TRUE and still useful:** `QuantityRef::ObjectCount` **hard-sets `sibling: true` in its OWN
> arm** (`ability_scan.rs:1593-1601`, flag at `:1596`) — **it genuinely counts a mutable object set, so it
> MUST stay `true` (that is Gaea's Cradle)** — whereas `TargetFilter::Typed` takes its bit from the shared
> child (`:2456`) and **naming a type is not counting one.** **That asymmetry is exactly the positive
> definition P7 generalizes.** And `typed_filter_reads_projected` (`:3113-3122`) **already builds the full
> `Axes` and discards `event` + `sibling`** (returns `acc.projected` at `:3121`) — **return `acc`.** The
> machinery exists; the *classification* is what is wrong.

> **⭐ The `sibling` fix reaches FOUR GATES at once.** Gates (1), (2), (3) and (4) **all bottom out in the
> same `scan_target_filter` / `scan_effect` walk.** So the class fix clears **Intruder Alarm**
> (a *trigger* ⇒ gate 1) **and Freed from the Real** (an *activated ability* ⇒ gate 2 — **the real board's
> actual third veto**) at once.

**(b) ⚠️ Gate (4) has a SECOND, UNCONDITIONAL veto — a LATENT CLASS DEFECT, but NOT a root cause of this bug.**
**`resource.rs:1539`**:
```rust
if !def.modifications.is_empty() {
    return true;                 // <-- NOT condition-gated. ANY live continuous modification vetoes.
}
```
Its own comment (`:1526`) admits it: *"condition + any live continuous modification (**default-CONSERVATIVE**)"*.
**Any battlefield permanent carrying a live continuous modification — an anthem, a lord, a P/T grant — kills
the cover outright.** This is **not** an AST-scan problem, and the `ability_scan.rs:2456` fix **cannot reach
it**.

> ## ⛔ SCOPE THIS CORRECTLY — TWO AUTHORS GOT IT WRONG (Appendix B #15, #16).
> **Both of us claimed gate (4) blocks the user's board because "Freed from the Real is an aura, and auras
> carry modifications." MEASURED ON THE FIXTURE'S REAL `GameObject`:**
> ```
> Freed from the Real | zone: Battlefield | static_definitions: []   <-- EMPTY | n_abilities: 2
> ```
> **Freed carries ZERO static definitions.** Gate (4)'s loop `for def in obj.static_definitions.iter_all()`
> (`:1531`) **has nothing to iterate ⇒ the veto CANNOT fire on it.** *(An aura granting only activated
> abilities has no modifications. Freed trips **gate (2)**, via arm (a).)*
>
> **Board-level measurement, decisive:**
> - Battlefield objects with a non-empty static modification on the user's board: **ZERO**.
> - Objects that **do** trip gate (4): **Alela · Empyrean Eagle · Door of Destinies · Favorable Winds ·
>   Leyline of Transformation** — **all in the LIBRARY** — plus **Kalemne** in the **Command zone**.
>
> ## ⇒ **On this board, gate (4) fires ONLY through the hidden-zone bug (c). P4's CR 113.6 predicate FULLY DISCHARGES it. Arm (a) alone IS sufficient for the user's bug.**
>
> **Arm (b) is still REAL and still ships** — as a **latent class defect**, not a blocker. It is *masked* by
> P4 on this board and **bites the instant any anthem resolves onto the battlefield** — and **Empyrean Eagle,
> Favorable Winds and Door of Destinies are sitting in this very deck's library.** **That is its hostile
> fixture** (§7 row 16). *(It was the old plan's **R5**, dropped during a refactor of the notes — which is
> how a real defect became invisible.)*

**(c) ⛔ The zones — and this is the arm that actually matters.** Gates **(1)**, **(4)** and **(5b)** iterate
`state.objects.values()` with **no zone filter** (only `is_phased_out()`). Measured trips on the real board:
`Solemn Simulacrum` **(Library)**, and the five anthems above **(Library)**. Illegal by **CR 113.6** (`:771`)
and **CR 400.2** (`:1935`). **Gate (2) is already correctly battlefield-scoped — do not "fix" it.**

**R2 is already fixed and committed** (`scan_mana_production`, `ability_scan.rs:2117`; the basic-`Forest`
trip). ⚠️ **`repro_user_combo.rs`'s doc comment (`:18-19`) is STALE** — it still names the `Forest` as one of
three vetoes. **That veto no longer exists.** Fix the comment.

> **⚠️ Appendix B #9 — "measured trips, in order" is the wrong provenance.**
> `loop_states_cover_modulo_fodder_growth` (`resource.rs:1095`) checks `board_covers_modulo_fodder`
> (`:1033`) **first**, at `:1120`, and returns false before reaching the firewall at `:1131`. Because RC-2
> fails that first board cover, **the firewall is never reached on `(cs_n, cs_n₁)`.** Both root causes are
> real and neither alone suffices — but the trips were observed **under instrumentation**, not on the live
> path.

### 3.2 RC-2 — the cover forbids a bounded start-up transient — **CONFIRMED; a reviewer attacked it and could not break it**

**`engine.rs:1732-1738`** requires the cover on **both** pairs:
```rust
loop_states_cover_modulo_fodder_growth(&cs_n,  &cs_n1, &fodder)   // <- FAILS
&& loop_states_cover_modulo_fodder_growth(&cs_n1, &cs_n2, &fodder)
```
Chain, every link measured:
1. `select_convoke_taps` (`game/mana_payment.rs:436`) does `candidates.sort_by_key(|id| id.0)` and
   **re-runs per drive iteration**.
2. `is_convoke_eligible` (`types/game_object.rs:2206`) checks **only** controller / battlefield / untapped /
   Creature — **no color preference, no sickness gate**.
3. ⇒ **Witherbloom (id 402, `["Black","Green"]`, untapped)** is picked over the Saprolings (413+).
4. Witherbloom is still **untapped at `s_n`** because the acceptance test convokes a Saproling
   (`repro_user_combo.rs:108`).
5. **Nothing absorbs the flip.** `normalize_recast_frame` (`engine.rs:1599`) strips only the recast card +
   anaphora; `derived_fodder_class` (`engine.rs:1633`) derives only the Saproling class; `fodder_content_eq`
   (`resource.rs:994`) is content-equality-modulo-`tapped` **against that class** ⇒ Witherbloom is a
   **STABLE ENGINE** object, not fodder — and `object_content_eq` (`game_state.rs:10453`) **compares `tapped`.**
6. ⇒ Witherbloom's untapped→tapped flip breaks the stable partition of `board_covers_modulo_fodder`
   (`resource.rs:1033`) ⇒ **`(cs_n, cs_n₁)` cannot cover** ⇒ no offer.

**Bounded:** nothing untaps Witherbloom (Freed enchants **Kilo**, not her) ⇒ the transient is a **one-time
prefix**, and the recurrence from iteration 2 is exact (untapped-green count invariant at 5).

**⚠️ Scope the claim correctly:** *"on any real board"* is **too strong**. Correct: **"on any board where the
driven prefix consumes a non-fodder engine piece."**

**The airtight evidence is the ASYMMETRY between the two callers of the same machinery** — *not* the `WARMUP`
constant on its own (a constant in the same crate by the same authors is corroboration, not independence):

| | transient tolerated | covering pairs required |
|---|---|---|
| **Offline** `run_combo` (`corpus.rs:1175`; `WARMUP:2` `:1179`, `STEADY:3` `:1180`) | **≥4 cycles** | **1** (`detect_loop` on a single `(start,end)` pair, `:1197`) |
| **Live** `try_offer_object_growth_shortcut` (`engine.rs:1656`) | **0** | **2, from iteration 0** (`:1732-38`) |

⇒ The duality invariant of **P2** would otherwise pressure us to **relax the live path** to go green —
degrading the only game-ending path while looking like progress. **P2 fixes the asymmetry UPWARD: `run_combo`
must ALSO require two consecutive covering pairs.**

### 3.3 RC-3 — the live path arms on one card shape, and nothing tests it

The live offer fires only when `last_recast_context` is armed. **`game/casting_costs.rs:6785-6788`** gates
the capture on `state.loop_detection.samples() && additional_cost_paid && has_buyback && is_token_creating`
— *a buyback-paid, token-creating recast.* **One card shape.** Every other player-driven loop is invisible.

`RecastContext` (**`types/game_state.rs:371-383`**) has **no action field — the action is implied by the
type.**

**`grep -c "WaitingFor::LoopShortcut" crates/engine/src/analysis/corpus.rs` == 0.** All **53** `CORPUS` rows
(`corpus.rs:110-561`) are driven through the **offline** `detect_loop`. **Not one row exercises the live
offer path.** The corpus is *structurally incapable* of catching this bug.

> **Terminology, measured — the old plan conflated these:** `CORPUS` = **53 rows** (`:110-561`). `DRIVERS` =
> **12 rows** (`:673-686`), of which **10 are `Offline`** and 2 are `LiveDrain` (ids 17/18). *"53 drivers"*
> and *"55 rows"* were both wrong.

**And the ring cannot substitute.** `loop_detect_ring` stores `Arc<GameState>` **snapshots**, not actions,
and `engine.rs:3081` clears it on **everything except `PassPriority | OrderTriggers`**. ⇒ *"detect
multi-action player loops"* and *"leave `engine.rs:3081` alone"* are mutually exclusive. **This plan resolves
it by ARMING, not by weakening the ring (P3).**

### 3.4 RC-4 — loop equality is keyed on `ObjectId`, which CR 400.7 makes rules-wrong

**The seam is `objects_content_eq` (`types/game_state.rs:10428-10435`)** — *not* `object_content_eq`
(`:10453`), which takes `(&GameObject, &GameObject)` and **contains no `ObjectId` at all**:
```rust
a.len() == b.len()                                                    // :10432
    && a.iter().all(|(id, x)| b.get(id).is_some_and(|y| object_content_eq(x, y)))   // :10433-34
```
**`b.get(id)` is the id-keyed lookup.** **CR 400.7** (`:1950`): *"An object that moves from one zone to
another becomes a new object with no memory of, or relation to, its previous existence. **This rule has the
following exceptions.**"* — **twelve of them, 400.7a–m (`:1952-1974`). None restore identity**, so the
conclusion stands. *(Cite it **with** its exceptions; quoting it bare invites a fifteenth error.)*

A permanent that dies / blinks / bounces returns with a **fresh `ObjectId`**, so the loop point is never
board-identical. This is the `DeferralBucket::ObjectReentry` bucket (`corpus.rs:93`).

> **⭐ BONUS — P9 SHRINKS.** `objects_content_eq` **already asserts `a.len() == b.len()`** (`:10432`) ⇒
> **multiplicity is already preserved** ⇒ **scalarset normalization need only permute ids.** See §5.

---

## 4. Architecture — the fixed-sequence formulation

### 4.1 CR 732.2a fixes the player's choices. That is the whole design.

> **CR 732.2a** *(verbatim, `docs/MagicCompRules.txt:6372`)*: *"the player with priority may suggest a
> shortcut by **describing a sequence of game choices, for all players**, that **may be legally taken based
> on the current game state and the predictable results of the sequence of choices**. This sequence may be
> **a non-repetitive series of choices, a loop that repeats a specified number of times**, multiple loops, or
> nested loops, **and may even cross multiple turns**. **It can't include conditional actions**… **The ending
> point of this sequence must be a place where a player has priority**…"*

Five deductions, each of which changes code:

- **D1 — a shortcut IS a straight-line action sequence, by rule.** No conditionals ⇒ the proposer commits to
  which creature to convoke, which source to tap, which target to pick. The question is **not** *"is this
  board a linear program?"* (ill-posed). It is:

  > ## **Is this FIXED sequence legally repeatable forever, with constant Δ?**

- **D2 — "a loop that repeats a specified number of times."** The proposer names **N**, and the proposal must
  be legal *"based on the predictable results"* — for **every** iteration. ⇒ **precondition non-depletion
  (C2) IS CR 732.2a**, not an engineering add-on.
- **D3 — "a non-repetitive series of choices, [or] a loop that repeats…"** ⇒ **a shortcut may be a
  non-repetitive PREFIX followed by a loop.** Demanding the cover from iteration 0 is **stricter than the
  rule.** That is **RC-2**.
- **D4 — "the ending point must be a place where a player has priority."** The iteration boundary is a
  priority beat **by rule** — the empty-stack settle condition the drive already uses. ⭐ **This is also the
  premise of C5: a loop that ends at a priority beat NEVER ADVANCES THE PHASE.**
- **D5 — "a sequence of game CHOICES" (plural) and "may even cross multiple turns" are LEGAL.** Multi-action
  bodies are **confirmed in three drivers** (`drive_offline_devoted_vizier`, `drive_offline_grim_power`,
  `drive_offline_kilo_freed_relic` `corpus.rs:1537`). **Excluding turn-crossing loops is an ENGINEERING cut,
  not a rules one — waive it LOUDLY, with the CR quote.**

**CR 732.2a's own worked Example (`:6373`) is an object-growth loop** — **Presence of Gond + Intruder Alarm**,
*"I'll create a million tokens."* **The rulebook certifies the exact class we cannot detect, and RC-1 rejects
it.** It is the plan's **primary acceptance fixture**.

> **⭐ "for all players" is load-bearing, and it is also the constraint's LIMIT.** The proposer describes the
> sequence **for every player**. So an opponent's **`may`** trigger inside the loop (Suture Priest — *both*
> its clauses are `may`, verified) is a choice the proposer must **propose**, and **CR 732.2b** (`:6375`) is
> the opponent's lever to **shorten**. ⇒ The governing constraint is **not** *"opponents don't exist"*; it is
> ***"opponents respond afterward."***

### 4.2 Two rules that prune the design — and one that does NOT

- **CR 732.4** (`:6383`) + **CR 104.4b** (`:366`) — *"Loops that contain an optional action don't result in a
  draw."* Our loops contain the proposer's **optional** action ⇒ never a draw ⇒ the engine **offers**.
  **Already implemented**: `no_living_player_has_meaningful_priority_action` (**`engine.rs:2367`**).
  **Don't rebuild.**
- **CR 732.3** (`:6380`) — **fragmented loops.** If repetition needs an **opponent's** independent action, the
  active player must break it ⇒ **reject any sequence requiring an opponent's non-pass action.**

> ## ⛔ **CR 732.5 / 732.6 are NOT "out of scope" — they ARE the mandatory/optional classifier, and the engine does not implement them.**
> **CR 732.5** (`:6385`): *"No player can be forced to perform an action that would end a loop **other than
> actions called for by objects involved in the loop**."* **CR 732.6** (`:6388`) *manufactures*
> mandatory-ness (*"the loop will continue as though [A] were mandatory"*).
>
> **Measured:** `no_living_player_has_meaningful_priority_action` (`engine.rs:2367-2379`) probes **every
> living player's full flat priority action list**, and `has_meaningful_priority_action`
> (`ai_support/mod.rs:1025-28`) = `flat_actions_have_meaningful_priority(..) || has_activatable_sacrifice_for_mana(..)`.
> **There is no "objects involved in the loop" filter anywhere in that path.**
>
> ⇒ Any **out-of-loop** optional action (CR 732.5's own Seal-of-Cleansing example; any sac-for-mana outlet)
> makes the engine classify a genuinely **mandatory** loop as breakable ⇒ **it declines the CR 104.4b /
> 732.4 draw-or-auto-win it owes.** It errs **fail-closed** (not a false certificate), so it is **not a
> blocker** — but it **is rules-wrong**, it lands on exactly the **L-AUTOWIN** rows (17/18), and the old
> plan's *"out of scope"* sentence told the implementer not to look. **WAIVE IT LOUDLY. Do not delete the
> sentence — replace it with this one.**

### 4.3 The choice vector is enumerable — from CR 601.2 / 602.2

**CR 602.2b** (`:2531`): activating an ability follows **601.2b–i** identically. So what a fixed sequence must
pin is **closed and checkable**:

| CR | line | Choice to pin |
|---|---|---|
| 601.2b | `:2459` | mode · splice · **optional additional/alternative costs (buyback)** · **X** · hybrid · Phyrexian |
| 601.2c | `:2461` | **targets** (and the number) |
| 601.2d | `:2464` | division / distribution |
| 601.2f | `:2468` | order of applying cost **reductions** |
| 601.2g | `:2470` | **which mana abilities to activate** (CR 605.3a, `:2692`) |
| 601.2h | `:2472` | **payment choices — including convoke's tap-set** (CR 702.51b, `:4399`) |

**Measured gap — this is a BLOCKER, not an audit item.** `build_recast_template` emits `[ConvokeTaps]` or
`[]` (`engine.rs:1558`), and `drive_recast_iteration` **explicitly aborts** on every other `ConcreteDecision`
kind (`engine.rs:1501-1536`, `return Err(RecastAbort)`). Combo B's cycle opens
`WaitingFor::PayCost{TapCreatures}` — which lands on the `_ => Err(RecastAbort)` arm at **`engine.rs:1548`**.
**The driver cannot drive Combo B at all.** In **P3**'s scope.

### 4.4 Every failure mode collapses into one: *the fixed sequence becomes ILLEGAL*

All card text verified verbatim in `data/card-data.json`.

| Case | The place the sequence draws from | Δ(place) | Verdict |
|---|---|---|---|
| **Sprout Swarm** — convoke (CR 702.51b — a **payment**; no `{T}` on the creature) | untapped **green** creature | −1 + 1 (**token is green & untapped**) = **0** | **ACCEPT** ✅ |
| **Earthcraft** — *"Tap an untapped creature you control: Untap target **basic** land."* → cost is on **Earthcraft's own** ability, **no tap symbol** ⇒ **CR 302.6 does not apply** ⇒ a summoning-**SICK** Squirrel is legal fodder | untapped creature (sick or not) | −1 + 1 = **0** | **ACCEPT** ✅ |
| **Cryptolith Rite** — *"Creatures you control have '{T}: Add one mana of any color.'"* → the **creature's OWN `{T}`** ⇒ **CR 302.6 APPLIES** | **unsick** untapped creature | −1 + 0 (**new token is sick**) = **−1** | **REJECT** ✅ |
| **Presence of Gond + Intruder Alarm** (**CR 732.2a's example**) — `{T}` on the creature ⇒ CR 302.6 applies; Intruder Alarm untaps it | **unsick** untapped enchanted creature | −1 + 1 = **0** | **ACCEPT** ✅ |
| **Manaforge Cinder** — *"{1}: Add {B} or {R}. **Activate no more than three times each turn.**"* | activations remaining | −1 | **REJECT** |
| **Crucible of Worlds + Zuran Orb** | land plays remaining (**CR 305.2**, `:1692`) | −1 | **REJECT** |
| **Solemnity** + proliferate | — | **measured** Δ = 0 counters | **REJECT** |

> ### ⚠️ **Earthcraft says "target BASIC land."** The Squirrel Nest must sit on a **basic** land or the
> acceptance fixture **does not close**. Every prior revision of this plan dropped that word.

**CR 302.6** *(verbatim, `:1630`)*: *"A creature's activated ability **with the tap symbol or the untap symbol
in its activation cost** can't be activated unless the creature has been under its controller's control
continuously since their most recent turn began."*

**The engine CAN see this split** (verified): `AbilityCost::Tap` (the `{T}` symbol) vs
`AbilityCost::TapCreatures { requirement, filter }` (`types/ability.rs:7841`); CR 302.6 is enforced **only on
the former, against the ability's own `source`**, via `check_summoning_sickness_for_cost` →
`cost_contains_tap_or_untap` (`game/restrictions.rs:618, 675`). **C2's place-split is implementable.**

> ## ⛔ TWO FIXTURES FROM PRIOR REVISIONS ARE DEAD. Do not resurrect them.
> - **Cryptolith Rite + Squirrel Nest is NOT A LOOP AT ALL.** **Neither card untaps anything.** The
>   rejection is **over-determined** — the detector rejects it whether or not CR 302.6 is implemented, so
>   **deleting the 302.6 check would not flip the test.** It is a **vacuous** discriminator.
>   **The non-vacuous fixture is `Cryptolith Rite + Presence of Gond + Intruder Alarm`**, where the loop's
>   mana **must** come from the freshly-created token's own `{T}`.
> - **Damping Sphere is UNSATISFIABLE on the Witherbloom board — it is a POSITIVE fixture in disguise.**
>   Verbatim: *"Each spell a player casts costs **{1} more** … for each **other spell that player has cast
>   this turn**."* Witherbloom grants **affinity for creatures** = **{1} less per creature you control**
>   (CR 702.41a). The loop adds **exactly one creature** *and* casts **exactly one spell** per iteration.
>   At iteration *k*: generic = `base + k − (C₀ + k)` = **`base − C₀` — constant in k. THE DELTAS CANCEL
>   EXACTLY and the loop still closes.** A C1 reject-test built on it **passes for the wrong reason.**
>   *(This is the Hum-of-the-Radix failure — Appendix B #10 — recurring one card downstream.)*
>   **C1 needs a scaler whose growth dimension the loop does NOT feed. See §7.**

### 4.5 Legality gates ARE consumables

*"3 activations left"*, *"1 land drop left"* (**CR 305.2**), *"unsick creatures"* (**CR 302.6**), *"loyalty
activations"* (**CR 606.3**, `:2715`) are **resources the fixed sequence spends**. `project_out_resources`
(**`resource.rs:2501`**) already **deliberately preserves** them — its own comment: *"blanket-clearing them
would erase the gate that makes a once-per-turn … ability NON-repeatable, **falsely certifying it as
infinite**."* Single authority: `ability_has_per_turn_activation_gate` (**`resource.rs:2848`**).

> **⛔ CR 704.5b does NOT apply to a mill loop the way prior revisions claimed.** CR 704.5b (`:5494`)
> requires a player to have ***"attempted to draw a card** from a library with no cards in it"*. **Milling to
> zero is not a loss, and there is no empty-library SBA.** Mesmeric Orb milling an empty library mills **0**
> and **the loop continues, harmlessly.** ⇒ Basalt Monolith + Mesmeric Orb is **not a C2 depletion fixture**
> — the resource **floors at 0 and the loop survives**. Its loss only lands at the next **draw step**.

### 4.6 ⭐ The governing constraint — correctly scoped

> ## **The player presents the loop FIXED. Other players respond AFTERWARD. Only the ACTIVE PLAYER'S CONTEXT matters.**
>
> This is **true and load-bearing.** It gives you:
> - **no search** — CR 732.2a forbids conditional actions ⇒ the choice vector is **pinned**;
> - **no opponent modelling** — CR 732.2b: they **accept or shorten** in the response window; interaction is
>   **not the cover's problem**;
> - **current board only** — no hidden zones, no hypotheticals.
>
> ### ⛔ What it does NOT give you — and this is where the last revision of this plan died.
> It does **NOT** give you *"therefore everything the loop does lands in the driven Δ."*
> **THE ERROR WAS TEMPORAL, NOT INFORMATIONAL.**
>
> A fixed sequence, from the current board, in the active player's context, **can still SCHEDULE an effect
> that executes at a PHASE BOUNDARY.** By **D4**, the loop's ending point is a **priority** beat ⇒ **the loop
> never advances the phase** ⇒ **it never executes what it scheduled.**
>
> > ## **The drive measures what RESOLVES inside the window. It is structurally blind to what the window SCHEDULES.**

### 4.7 The blind-spot taxonomy — **THREE categories**, and the five checks

**So ask the only question that matters: what can the CURRENT BOARD do that the DRIVE CANNOT SEE?**

| # | Blind spot | Why the drive misses it | Check |
|---|---|---|---|
| **1** | **Monotone depletion outside the drive window** | Δ is *constant* for the driven iterations; the sequence dies at iteration 4 (Manaforge's 3/turn, land drops, sickness) | **C2** |
| **2** | **A discontinuity — a threshold tripping at a future iteration COUNT** | Δ is *constant* until it trips | **C3** |
| **3** | ⭐ **DEFERRED EXECUTION (CR 603.7) — a first-class citizen** | The loop **SCHEDULES** an effect whose execution lands **OUTSIDE** the certifiable window | **C5 (new)** |

**Category 3, enumerated structurally over all 224 `Effect` variants** (`types/ability.rs:9305-12399`; count
verified): `CreateDelayedTrigger` (`:10995`) · `SkipNextTurn` (`:12080`) · `SkipNextStep` (`:12090`) ·
`ControlNextTurn` (`:9864`) · `AddPendingETBCounters` (`:11058`) · `ReduceNextSpellCost` (`:11036`) ·
`GrantNextSpellAbility` (`:11043`) · **the entire replacement family** (**CR 614.1**, `:3054` — *"watch for a
particular event that **would happen**"*; mutates nothing now).

**The named keyword instances — every 702.x number GREPPED** *(these are the single most hallucination-prone
citations in this codebase)*:

| Keyword | CR | line | The delayed-trigger clause |
|---|---|---|---|
| **Epic** | **702.50** | `:4389` / `:4391` | *"…creates a delayed triggered ability … at the beginning of each of your upkeeps for the rest of the game"* — **crosses turns** |
| **Suspend** | **702.62** | `:4470` | *"two … are triggered abilities that **function in the exile zone**"* — also a live **CR 113.6b/c** case |
| **Rebound** | **702.88** | `:4638` / `:4640` | *"may create a delayed triggered ability … at the beginning of [your next upkeep]"* |
| **Dash** | **702.109** | `:4802` / `:4804` | *"return the permanent … **at the beginning of the next end step**"* |
| **Myriad** | **702.116** | `:4844` / `:4846` | tokens exiled **at end of combat** — crosses a step |
| **Encore** | **702.141** | `:5038` / `:5040` | *"…gain haste. **Sacrifice them at the beginning of the next end step**"* |
| **Foretell** | **702.143** | `:5048` / `:5050` | *not* a delayed trigger — a **later-turn cast permission** (CR 113.6b/e). **A distinct class.** |
| **Blitz** | **702.152** | `:5116` / `:5118` | *"**sacrifice the permanent … at the beginning of the next end step**"* |

> ### ⚠️ **Kiki-Jiki defeats C1, C2, C3 AND C4 simultaneously. This is why C5 must exist.**
> **Kiki-Jiki, Mirror Breaker** *(verbatim)*: *"**{T}**: Create a token that's a copy of target nonlegendary
> creature you control, except it has haste. **Sacrifice it at the beginning of the next end step.**"*
>
> - Nothing depletes ⇒ **C2 blind.**
> - No threshold trips at any iteration **count** — **it fires on a CLOCK, not a count** ⇒ **C3 blind.**
> - Δ is perfectly constant at `tokens_created: +1` ⇒ **C1 blind.**
> - C4's shipped triple sees nothing wrong.
>
> **All four checks pass on a loop whose entire growth axis is destroyed at the next end step.**

### 4.8 The five checks

| # | Check | Catches | Status |
|---|---|---|---|
| **C1** | **Δ-constancy** across two **post-transient** pairs | anything that **scales** with the growth | drive exists; must skip the transient (**P5**) |
| **C2** | **Place non-depletion** | monotone depletion outside the window | ⚠️ **NEW LOGIC** (**P6**) |
| **C3** | **Threshold scan** — a fire-time `Comparator` / modification against the **growing axis** | **discontinuities** | ⚠️ **TWO narrowings — gates (1) AND (4)** (**P7**) |
| **C4** | **The shipped triple** — `net_progress_for(caster)` + `has_no_loss_axis` + `driving_resources_non_decreasing` | self-deck, self-damage, adverse scaling | **exists, unchanged** (`engine.rs:1756-1758`) |
| **C5** | ⭐ **Deferred-execution classification (CR 603.7)** | **an ω-axis destroyed outside the window** | ⚠️ **NEW CHECK** (**P8**) |
| **C6** | ⭐ **∞-composition fixpoint** — treat an already-proven-unbounded axis as **non-depleting** | **infinites on top of infinites** — today a second loop is rejected for "depleting" a resource the engine **already proved infinite** | ⚠️ **NEW CHECK** (**P9**); the **store already exists and has no reader** (§4.11) |

> **Why measurement, not derivation.** Δ cannot be derived from the AST — **replacements rewrite it at
> resolution** (Solemnity turns proliferate's AST-Δ of +1 into a true Δ of **0**), and **CR 704.3** (`:5485`)
> / **CR 603.3b** (`:2586`) put a full SBA + trigger settle between iterations. **The drive is the authority;
> the firewall's only job is what the drive is structurally blind to.**

### 4.9 ⭐ C5 — it bounds the ω-axis's **LIFETIME**. It does NOT blanket-reject.

> ## ⛔ **DO NOT WRITE "CR 732.2a FORBIDS KIKI." IT DOES NOT.**
> This was settled from the rule text, and it matters, because an implementer who greps it and finds we
> overclaimed will stop trusting the whole document.
>
> **Reasoned from CR 732.2a (`:6372`), in three steps:**
> 1. **The end-step sacrifice is NOT IN THE SEQUENCE.** The sequence's ending point is a **priority** beat
>    (D4); the next end step is a **phase later**.
> 2. ***"Predictable results"* is a LEGALITY condition on the PROPOSAL** — the proposer must be able to state
>    what happens, and they can: *"a million hasty tokens, all sacrificed at the next end step."* **It is not
>    a persistence requirement.**
> 3. ⇒ **Kiki-Jiki + Zealous Conscripts IS a legal CR 732.2a shortcut, and the engine is entitled to offer
>    it.** You genuinely CAN make a million tokens.
>
> **The rules do not mandate rejecting Kiki. What must be true is that the CERTIFICATE IS NOT A LIE** — and
> the CR says nothing about certificates. **That is an internal soundness obligation, not a rules one. Say so.**

**What is false is not the loop — it is that the tokens PERSIST.** Each is sacrificed at the next end step.
But **Kiki's tokens have HASTE** (**CR 702.10**, `:3969`), so the proposer **swings for lethal BEFORE that end
step.**

⇒ **A scheduled-outside-the-window effect does not invalidate the LOOP. It bounds the LIFETIME of the ω-axis,
and therefore what the certificate may CLAIM.**

**Ship C5 in TWO STAGES:**

- ### **C5 v1 — BUILD IT. IT *REPLACES* R6's `delayed_triggers` TERM. IT DOES NOT SIT BEHIND IT.**
  > ## ⛔ **"KEEP R6" + "C5 v1 fails closed" ARE MUTUALLY ANNIHILATING. Getting this wrong makes C5 v1 DEAD CODE.**
  > - **R6** (`resource.rs:1582-1586`) rejects on **any** non-empty deferred store — inside- *or*
  >   outside-window.
  > - **C5 v1** rejects a **strict subset** of that (outside-window only).
  > - ⇒ **C5 v1 ⊆ R6.** Every state C5 v1 rejects, R6 **already** rejects. **No fixture can distinguish
  >   them ⇒ no revert-probe on C5 v1 can flip any row ⇒ it is UNVERIFIABLE BY CONSTRUCTION.**
  >
  > **⇒ C5 v1 must REPLACE R6's `delayed_triggers` conjunct** (leaving the other four: `deferred_triggers`,
  > `pending_trigger`, `pending_trigger_order`, `epic_effects`). **Its value is the loops it ADMITS** — an
  > *inside*-window delayed trigger (e.g. one that re-arms identically every cycle) which R6 wrongly rejects
  > today. That admission is what a revert-probe can flip.

- ### **C5 v2 — NAME IT. DO NOT BUILD IT.**
  The ω-axis **lifetime** refinement: a short-lived axis may certify **`Win(LethalDamage)`** while being
  forbidden **`Advantage(Resource)`** ⇒ **Kiki becomes reachable.** **This requires the P1 type split — which
  is exactly why P1 is sequenced first, and is the second of three places §4c turns out to be load-bearing.**

> ## ⚠️⚠️ THE TRAP — and the v2 note is its ANTIDOTE. Neither is optional.
> **Gate (6) / "R6" is a FIVE-way OR** (`resource.rs:1582-1589`), not three:
> ```rust
> !state.delayed_triggers.is_empty()      // :1582
>     || !state.deferred_triggers.is_empty()   // :1583
>     || state.pending_trigger.is_some()       // :1584
>     || state.pending_trigger_order.is_some() // :1585   <-- prior revisions missed
>     || !state.epic_effects.is_empty()        // :1586   <-- prior revisions missed
> ```
>
> **Deleting R6 is unsound AND worth ZERO rows. Both halves are measured:**
>
> 1. ***"The delayed trigger fires in the drive"* — FALSE.** `DelayedTriggerCondition::AtNextPhase{phase}`
>    fires **only** on `GameEvent::PhaseChanged` (**`game/triggers.rs:6212`**), and `GameState::PartialEq`
>    pins **`turn_number`** (`game_state.rs:10823`) and **`phase`** (`:10825`) ⇒ **no certifiable cycle can
>    contain the phase change that fires it.**
>    **Measured** (real Kiki Oracle, real `{T}`, driven to a settled empty-stack Priority beat):
>    `delayed_triggers.len() == 1` — **still armed** — and the **token is still on the battlefield.**
>    **Non-vacuity: pass into the End step ⇒ the token leaves.** The negative passes because **the phase never
>    changed**, not because the harness cannot fire it.
> 2. ***"Worth 2 corpus rows on its own"* — FALSE. It is worth ZERO.** `eq_except_growable`
>    (`resource.rs:1409`) **begins** with `a == b` (`:1441`) ⇒ reuses `GameState::PartialEq` ⇒ which compares
>    **`delayed_triggers` (`game_state.rs:10875`)** ⇒ **Kiki is ALREADY rejected, independently of R6.**
>    *(Precision: `a == b` is the **first of three** conjuncts, not the last — `:1441-1444` also compare
>    `post_replacement_token_substitution_count` and `last_recast_context`. The conclusion is unaffected.)*
>
> ### **THE TRAP, spelled out so nobody walks into it:**
> > An implementer deletes R6 → sees Kiki **still** rejected → follows the trail to `eq_except_growable` →
> > **relaxes the `delayed_triggers` conjunct at `game_state.rs:10875`** to collect the promised rows →
> > **the detector now certifies a loop whose entire growth axis is destroyed at the next end step.**
> > ## **FALSE CERTIFICATE ON THE ONLY GAME-ENDING PATH.**
>
> **The `delayed_triggers` conjunct at `game_state.rs:10875` MUST NOT be relaxed.** Authority: the
> soundness-asymmetry table (§5b.1) — **a coarse relation may REJECT, never ACCEPT.**
>
> **⇒ Tell the implementer plainly: KIKI IS DEFERRED BEHIND A NAMED REFINEMENT (C5 v2), NOT PERMANENTLY OUT
> OF REACH.** That sentence is what stops them from chasing the rows.

> ## ⛔ **AND KIKI IS THE *TRAP* FIXTURE, NOT THE C5 *DISCRIMINATOR*. Do not build C5's test on it.**
> **Kiki's `delayed_triggers` store GROWS by one every cycle** ⇒ `a == b` fails ⇒ it is **already rejected by
> `PartialEq` with no classifier at all.** **A C5 revert-probe on Kiki cannot flip — the row is dominated by
> state inequality.** *(Exactly the Cryptolith-Rite vacuity, one layer down.)*
>
> **C5's ONLY real discriminator is a STABLE, PRE-ARMED, OUTSIDE-WINDOW delayed trigger:** one whose store
> does **not** grow (so `PartialEq` passes), whose execution lands **outside** the window (an `AtNextPhase`
> — which fires *only* on `GameEvent::PhaseChanged`, `triggers.rs:6212`, and the window never changes phase),
> **and whose effect would destroy the growing class.** **That fixture, and only that fixture, can flip a C5
> probe.** See §7 rows 24–25.

### 4.10 ⭐ Opponent interaction is the RESPONSE WINDOW's job — not the cover's. **(And the engine already gets this right.)**

This retires an entire class of objection (*"but what if they have an answer?"*) in one paragraph, and it is
**why the cover is entitled to read the present board only.**

> **CR 732.2b** *(verbatim, `docs/MagicCompRules.txt:6375`)*: *"Each other player, in turn order starting
> after the player who suggested the shortcut, may either **accept** the proposed sequence, or **shorten it
> by naming a place where they will make a game choice that's different** than what's been proposed."*
> **CR 732.2c**: *"…If the shortcut was shortened from the original proposal, **the player who now has
> priority must make a different game choice** than what was originally proposed for that player."*

### ✅ **MEASURED: the window EXISTS and is correctly implemented. This is a POSITIVE architectural finding.**

| Seam | Where |
|---|---|
| `WaitingFor::RespondToShortcut { player, remaining_players, proposal }` | **`types/game_state.rs:4347`** — its own doc: *"**CR 732.2b/c: the APNAP accept-or-shorten window.** After the proposer declares the shortcut, **each other living player is prompted in turn order**"* |
| `GameAction::DeclareShortcut` | **`types/actions.rs:803`** |
| `GameAction::RespondToShortcut { response: ShortcutResponse }` | **`types/actions.rs:810`** — documented as naming *"an earlier stopping point"* |

**The full interaction works today:** proposer declares → each opponent gets the CR 732.2b window in **APNAP
order** → an opponent **shortens to iteration k** → the game advances there → **that opponent has priority
(CR 732.2c)** → they cast **Borne Upon a Wind** (*verbatim*: *"You may cast spells this turn as though they
had flash."*) and flash in **Damping Sphere** → **the loop breaks.** **That is the game working exactly as the
rules intend.**

> ## ⇒ **THE DESIGN RULE — write it once, apply it everywhere:**
> **The cover certifies only that, FROM THE PRESENT BOARD, the loop is infinite FOR THE PROPOSER.**
> It is entitled to ignore hidden zones **not because opponents' answers are imaginary — they are
> real and live — but because handling them is A DIFFERENT PHASE'S JOB.**
>
> **An opponent holding Borne Upon a Wind + Damping Sphere does not falsify the certificate. They SHORTEN the
> shortcut (CR 732.2b) and break the loop.**
>
> A cover that tried to anticipate them would be **unsound** (it cannot see hidden zones), **rules-illegal**
> (**CR 400.2** — and that read *is* RC-1's bug), **and redundant** (the response window already covers it).
>
> **⚠️ Do NOT write "a card in an opponent's hand is dead/irrelevant." It is a LIVE ANSWER.** The point is
> *whose job it is*, not whether it matters.

### 4.11 ⭐ C6 — infinites on top of infinites. **The ∞ marks are WRITE-ONLY.** *(New; composes the checks, does not replace them.)*

> **USER:** *"There is the possibility for infinites-on-top-of-infinites… you just apply infinite statuses to
> things and resolve the stack."*

**MEASURED: the engine proves an axis infinite, then forgets it.**

| | Where |
|---|---|
| **Store** | `unbounded_resources: BTreeMap<PlayerId, BTreeSet<ResourceAxis>>` (**`types/game_state.rs:7276`**) — **already per-player** |
| **Writer** | `mark_unbounded_loop` (**`:10377`**) — only `entry.extend(axes)`; **it can never end a game** |
| **Reader 1** | `game/derived_views.rs:498` — **HUD projection only** |
| **Reader 2** | `game/mana_payment.rs:97` — its own doc: ***"Debug-only:*** top every player whose `unbounded_resources` contains any `ResourceAxis::Mana(_)`…"* |
| **Reader 3** | `game/turns.rs:354` — ***"Debug-only:*** CR 500.5 end-of-step empty is suppressed for a player with the **infinite-mana toggle** active"* |
| **Readers in `analysis/` or the detector path** | ⛔ **NONE.** |

**Both functional consumers are wired to the DEBUG `SetInfiniteMana` toggle, not to real play.**
⇒ **A second loop that SPENDS a resource the engine has ALREADY PROVEN INFINITE is rejected by C2 /
`net_progress_for` for "depleting" it.**

**This is live on the USER'S OWN BOARD:** Kilo + Freed + Relic + Pentad Prism ⇒ unbounded **counters** ⇒
unbounded **mana**. Witherbloom + Sprout Swarm ⇒ unbounded **creatures**. **Two loops. Zero composition.**

### It is a MONOTONE FIXPOINT — **not** linear programming. That is *better* news.

**With ∞ statuses, quantities collapse to booleans.** You never solve for *how much* (which is what would make
it an LP) — only for **reachability**. The rule is **monotone** (adding an ∞ axis can only *enable* more
loops, never fewer), and **`ResourceAxis` (`analysis/resource.rs:552`) is a finite enum** ⇒ the fixpoint
**terminates**. **A least-fixed-point closure — Datalog-shaped, decidable, ~20 lines, no solver, no dependency.**

```text
∞ := {}                                    // per player
repeat until fixpoint (bounded by |ResourceAxis|; add a round cap as a backstop):
    for each candidate loop L on the present board:
        if L certifies with the axes in ∞ treated as NON-DEPLETING:
            ∞ ∪= L.produced_axes
```

**The code change is ONE DISJUNCT:**
- **C2** → *"every place the sequence draws from is **non-decreasing OR already marked unbounded for this
  player**."*
- **`net_progress_for`**'s *"no net-negative mana"* → **exempt axes already in ∞.**
- **The store already exists — it just has no reader.**

### ⚠️ FOUR SOUNDNESS CONSTRAINTS — do not get these wrong

1. **∞ is PER-PLAYER.** The type already enforces it (`BTreeMap<PlayerId, …>`). **An opponent's ∞ mana does
   not make YOUR loop sustainable.** **Key every exemption on the PROPOSER.**
2. ⭐ **The fixpoint is a MONOTONE AMPLIFIER FOR FALSE CERTIFICATES.** One unsound mark poisons everything
   downstream. ⇒ **It composes with P1: only a CERTIFIED `Advantage(_)` — the revocable, SAFE side of the
   split — may seed ∞. Never a speculative mark, and never the `Win` side.**
   **This is the THIRD time the `WinKind` split turns out to be load-bearing.**
3. **CR 106.4** (`:416`) **/ CR 500.5** (`:2119`): **mana empties at end of step.** "Unbounded mana" is usable
   **only within the step** — the *debug* consumers cheat this deliberately (`UnitDisposition::Keep`,
   `turns.rs:354`). **A real composition must stay inside the step, or use a DURABLE axis.** *(This is exactly
   why **counters, not mana**, are the durable ω-axis, and why Pentad Prism's **charge counters** — not the
   mana they make — are what `loop_states_cover_modulo_counter_growth` certifies.)*
4. **Termination** is guaranteed by the finite axis set. **State it, and add a round cap as a backstop.**

> ### ⭐ **BONUS — the existing design already makes this sound, and the reason is subtle.**
> **`unbounded_resources` is deliberately EXCLUDED from loop equality** — `GameState::PartialEq` skips it
> (`game_state.rs:10606`), guarded by the revert-probe test `unbounded_resources_excluded_from_loop_equality`
> (**`:11434`**: *"manual PartialEq must exclude unbounded_resources (display state)"*).
> **That exclusion is precisely what MAKES the fixpoint work:** the closure *adds marks between rounds*, and
> **if marks were part of `PartialEq`, seeding one would itself break the board equality the next round
> depends on.** **Do not "fix" that exclusion. It is load-bearing for C6.**

> **A note for the next maintainer.** This is the **third** time a monotone-fixpoint / Datalog shape has
> surfaced in this workstream — after the `egg` AST spike (§5b.2, **rejected**) and the scalarset closure
> (§5b.3, **accepted as a prescription**). **This is the layer where the instinct is actually right — and even
> here it needs no library.** Worth knowing where that instinct pays and where it does not.

---

## 5. RC-4 / object identity — the honest picture

> **Appendix B #7 — *"generalizing `normalize_recast_frame` lifts all 13 `ObjectReentry` rows and is worth
> more than Phases 1–5 combined"* is FALSE.** It lifts **ZERO** of them directly, and the real fix is the
> **riskiest change in the program**. **Not a quick win. Do not sequence it as one.**

`DeferralBucket::ObjectReentry` (`corpus.rs:93`) is a **coarse bucket over two structurally different failures:**

**Group A — token ACCUMULATION; id churn is NOT the blocker (6 rows).**
Kiki-Jiki + Zealous Conscripts · Splinter Twin + Deceiver Exarch · Midnight Guard + Presence of Gond ·
Scurry Oak + Ivy Lane Denizen · Felidar Guardian + Saheeli · Earthcraft + Squirrel Nest.
These are **pure object growth**, and `loop_states_cover_modulo_{object,fodder}_growth` **already exclude the
add-set from id-keyed equality**. What actually blocks them: **Kiki/Twin** — every token carries a *"sacrifice
/ exile it at the beginning of the next end step"* delayed trigger ⇒ **gate (6)** (and, independently,
`GameState::PartialEq`) rejects. **The rest** — **RC-1** and **RC-3**.
⇒ **P3/P4/P7/P8 lift Group A. Object identity is irrelevant to it.**

**Group B — TRUE re-entry; id churn IS the blocker, and `normalize_recast_frame` is the WRONG fix (7 rows).**
Palinchron + Deadeye · Dockside + Sabertooth · Mikaeus + Triskelion · Food Chain + Eternal Scourge ·
Gravecrawler + Altar + Blood Artist · Karmic Guide + Reveillark + Viscera Seer · Reassembling Skeleton +
Ashnod's + Nim Deathmantle.

`normalize_recast_frame` (`engine.rs:1599`) handles churn by **deleting the object from both frames** — sound
**only** because the recast card is `ctx`-identified **and off the battlefield** (a card in hand, carrying no
board state). **Neither holds for Group B:**

1. **The churning object IS the engine piece.** Deleting Palinchron erases its own board state — including
   `summoning_sick`, **the exact CR 302.6 field C2's place-split depends on.** You would project out the
   thing you are checking.
2. **Id churn contaminates STABLE objects through id-valued fields.** `object_content_eq`
   (`game_state.rs:10453`) compares **`attached_to`, `attachments`, `paired_with`** — all `ObjectId`-valued.
   Palinchron is soulbonded to **Deadeye Navigator**: after the blink, **Deadeye's `paired_with` points at a
   NEW id**, so **Deadeye — a stable, never-moved object — fails content equality.** Stripping Palinchron
   does not fix Deadeye. Same for Nim Deathmantle's `attached_to`.

**The real fix is id-canonicalization of the whole frame** — remap `ObjectId`s to a canonical order **and**
canonicalize every id-valued field. **Content-multiset equality is EXACTLY where a false certificate enters:**
two boards can be content-equal per-object yet differ in **which object the stack, a delayed trigger, or an
aura POINTS AT.**

> **Verdict: object identity across a loop cycle is a real, general, unsolved problem that deserves its OWN
> PR with its OWN soundness proof. It is **P10 — LAST.**

## 5b. Can we buy the equality core off the shelf?

### 5b.1 ⭐ The soundness asymmetry — the single most important table in this document

Our equality relation sits on the **only game-ending path.** Its error direction is **not symmetric**:

| Relation errs… | Meaning | Consequence |
|---|---|---|
| **TOO COARSE** (says *equal* when they are not) | certifies a recurrence that did not happen | ⛔ **FALSE CERTIFICATE — ends a real game wrongly. CATASTROPHIC.** |
| **TOO FINE** (says *different* when they are equivalent) | misses a real loop | ✅ **false negative — a missed offer. SAFE / fail-closed.** |

> ## **⇒ A COARSE RELATION IS ONLY EVER ADMISSIBLE AS A *REJECT* FILTER, NEVER AS AN *ACCEPT* DECISION.**

**Everything in P8 and P9 hangs on that sentence.** It is also the authority for *"do not relax
`delayed_triggers`"* (§4.9) and for the P1 type split (§6, P1).

### 5b.2 ⛔ `egg` / e-graphs — **REJECTED. Do not spike it.**

*(A prior revision proposed adopting [`egg`](https://docs.rs/egg) to re-express `Axes` as an `egg::Analysis`.
It is rejected on measurement, and the reasoning is recorded so it is not re-proposed.)*

- **`Axes` IS a join — and that is exactly what kills it.** With **zero rewrite rules** (the spike's own
  scoping — every rewrite rule is a CR claim, so rewrites were correctly excluded), **no e-class unions ever
  occur**, so `Analysis::merge` is **never called**. **egg-minus-rewrites = hashconsing + a memoized
  catamorphism — and `ability_scan.rs` ALREADY IS the catamorphism.**
  **The bug is a WRONG ARM (`ability_scan.rs:2456`), not drift. No formalism fixes a wrong arm.**
- **Hosting the AST would need a ~645-variant mirror IR** (`Effect` lacks `Hash`/`Ord`; 9 `HashMap`
  payloads) ≈ **2× `ability_scan.rs`, 4–8 engineer-weeks, 12 new deps, and a WASM bundle cost** — **to reach
  the verdicts a 10-line fix already produces.** *(The engine ships to WASM with `opt-level='z'` + LTO, and
  the detector is on the live in-game path — it cannot be feature-gated out.)*
- **egg is ALSO unsound for STATE equality**, independently: congruence **collapses multiplicity** (3 vs 4
  identical Saprolings hashcons to the same e-class) — and **multiplicity IS the growth axis.** Accepting on
  congruence certifies iteration N ≡ N+1 **exactly when the tokens grew.** By 5b.1 that is a **false
  certificate on the game-ending path.**

### 5b.3 ✅ The prescription that SURVIVES — Murφ scalarset symmetry reduction (for P9)

**`ObjectId` is a *scalarset*.** Two boards are the same board **up to a permutation of object identities**;
permuting them induces automorphisms of the state graph; the fix is a **canonical representative per orbit.**
This is [Murφ](http://www.cfdvs.iitb.ac.in/download/Docs/verification/tools/murphi/html/murphiinfo.html)'s
symmetry reduction — decades old, with **published correctness proofs**, which is exactly what P9's soundness
obligation is asking for. ([survey](https://www.doc.ic.ac.uk/~afd/papers/2006/ACMSurvey.pdf))

**It hands us the safe engineering split:**

| Murφ strategy | Property | For us |
|---|---|---|
| **Normalization** (lightweight) | may yield **several** representatives per orbit ⇒ errs **TOO FINE** | ✅ **misses some loops; never certifies a false one. SHIP THIS FIRST.** |
| **Canonicalization** (heavyweight) | **unique** representative per orbit ⇒ **exact** | graph-iso-hard, but **board sizes are tens of objects ⇒ nauty-class tools are effectively free here.** The upgrade path. |

Rust bindings exist: [`graph-canon`](https://github.com/noamteyssier/graph-canon) (nauty),
[`nauty-pet`](https://docs.rs/nauty-pet), [`canonical-form`](https://github.com/avangogo/canonical-form).

⚠️ **Do NOT reach for 1-WL / colour refinement as the ACCEPT test.** It errs **coarse** ⇒ wrong direction ⇒
false certificate. Admissible **only** as a reject filter (5b.1).

⚠️ **The old §5b.3's *proof* was wrong** (hashconsing collapses *subterms*, not containers; different arity is
never congruent). **Its PRESCRIPTION — normalization first — is right.** Keep the prescription; discard the proof.

---

> ### 🛠 **THE PLAN (§6–§8).** This is the executable half. **Start here.**
> Each phase carries its own warnings inline — you should not need to page back into §3–§5b to execute it.
> **Every phase gates on a CLASS property (§0), never on a canary going green.**

## 6. Implementation plan

> **Phase renumbering.** Two user directives added prerequisite phases. Mapping to the planner brief's
> numbering: **P0**/**P1** are new; brief-P0→**P2**, brief-P1→**P3**, brief-P2→**P4**, brief-P3→**P5**,
> brief-P4→**P6**, brief-P5→**P7**, brief-P6→**P8**, brief-P7→**P10**. **P9 (C6, §4d) is new.**

### P0 — Collapse `LoopDetectionMode` to a binary ⭐ **USER DIRECTIVE**

> **USER:** *"`On` is a relatively useless state for the combo detector now — not useful for users, only
> confusing. Assume everything is 'interactive'. Record players' states and trap them into the detector."*

**The code agrees, in its own words.** `interactive_loop_bridge`'s Path A comment (**`engine.rs:499`**,
verbatim): *"FIRM #1 — mandatory winning drain: **identical to the `On` auto-win**."*
⇒ **`Interactive` STRICTLY SUBSUMES `On`** — same auto-win + `mark_unbounded_loop` when the loop is mandatory,
**plus** the CR 732.2a offer when it is optional. **`On` adds nothing.**

**Target shape** (`types/game_state.rs:5785`):
```rust
pub enum LoopDetectionMode { #[default] Off, On }   // `On` CARRIES TODAY'S `Interactive` SEMANTICS
```
**Keep the name `On`, delete the name `Interactive`** — *"interactive"* is jargon on a user-facing toggle, and
`On` touches fewer frontend strings. *(The behaviour that survives is `Interactive`'s.)*

**`Off` STAYS — non-negotiable** (#4603: game-changing features ship behind a user-controllable toggle whose
OFF state restores pre-feature behavior).

**Why this is a real win, not tidying — the CORRECT rationale:**
**The shipped default is `Off`** (`types/match_config.rs:27`, verbatim: *"Default `Off` = exact pre-detector
behavior (opt-in invariant, issue #4603)"*, enforced at the wire by
`#[serde(default, skip_serializing_if = "LoopDetectionMode::is_off")]` at `:36`). But a user who **opts in**
today faces a confusing **three-way** choice: `Off` / **`On` (auto-win only — NO OFFERS: a crippled
half-detector)** / `Interactive` (auto-win **+** offers). **`On` is strictly dominated.**
⇒ *"Trap them into the detector"* = **once you opt in, you get the FULL detector, not a half one.**

> ⛔ **Do NOT write "every real match gets the detector after the collapse."** That claim was **false** — it
> rested on `match_config.rs:89`, which is inside a `#[cfg(test)]` block (`:60`). See **Appendix B #13**.

**Blast radius — MEASURED. `grep -rn "LoopDetectionMode::On" crates/` returns 18 sites, not 16:**
`game/engine.rs:357` (the dispatch arm to delete) · `game/match_flow.rs:669,672,744,747` ·
`analysis/corpus.rs:1871` · `game/triggers.rs:23170,23251,23434` · `analysis/corpus_tests.rs:1404,1437` ·
`types/match_config.rs:89,97` · `server-core/src/session.rs:1613` ·
`tests/integration/loop_shortcut.rs:338` · `tests/integration/pr65_growing_cascade_win.rs:111` ·
**`types/game_state.rs:5804` and `:5819`** *(the two in the definition file itself — every prior enumeration
missed them)*.

**There are TWO predicates, not one:**
| predicate | line | production callers |
|---|---|---|
| `is_on()` | `:5803` | **ZERO** — all six call sites are `#[test]` fns (`corpus_tests.rs:1244,1348,1408`; `session.rs:1625,1636`, both inside `fn loop_detection_config_persists_across_bo3_rebuild`) ⇒ **consider deleting it outright** |
| `is_off()` | `:5809` | ⚠️ **PRODUCTION-LOAD-BEARING** — `match_config.rs:36`'s `skip_serializing_if`. **Keep.** |
| `samples()` | `:5818` | 4 production gates: `casting_costs.rs:6785`, `engine.rs:336`, `engine.rs:448`, `engine.rs:2307`. **Collapses to `!matches!(self, Off)`.** |

**⚠️ SERDE MIGRATION HAZARD — do not miss this.** `LoopDetectionMode` is `Serialize`/`Deserialize` with
`#[serde(tag = "type")]` (`game_state.rs:5783-84`). **Persisted states and debug exports carry
`{"type":"Interactive"}` — including the repro fixture — and will FAIL to deserialize once the variant is
gone.** Add **`#[serde(alias = "Interactive")]` on `On`** so both spellings load, and add a **round-trip test
proving an old export still deserializes.**

**Frontend:** `client/src/game/loopDetectionMode.ts` (the `?loop=` query parser/serializer) and
`client/src/components/lobby/HostSetup.tsx:226` (the lobby toggle). **Collapse the UI to a binary.**

**Removing a variant is an engine-surface change ⇒ run the `/add-engine-variant` checklist; grep
`data/engine-inventory.json`.**

### P1 — Split `WinKind` → `LoopOutcome` ⭐ **USER DIRECTIVE · PREREQUISITE FOR C5 v2**

> **USER:** *"There's a semantic problem in `WinKind` — `WinKind`s can be non-winning advantages in the
> current setup. It's probably a very useful distinction to separate these two classes."*

**This is not a naming smell. The engine already hand-simulates the missing type — and says so in a comment.**
All three measured:

1. **`Advantage`'s own doc comment states it is NOT a win** (`analysis/loop_check.rs:103-107`): *"…without, by
   itself, being a direct loss condition for an opponent… **the payoff that converts the advantage to a win is
   a separate card**."* **An enum called `WinKind` contains a variant documented as not-a-win.**
2. **`ExtraTurns` is ALSO misclassified.** It cites **CR 500.7**, which is **purely mechanical**
   (`docs/MagicCompRules.txt:2127` — *"Some effects can give a player extra turns. They do this by adding the
   turns directly after the specified turn…"*) and **says nothing about winning.** Win conditions are
   **CR 104.2** (`:328`). **Infinite turns is a CAPABILITY, not a game-end.**
3. **`engine.rs:637` does `classify_win_kind(controller, &delta) == WinKind::Advantage`** — an **equality
   check against a single variant** to gate the non-terminal path — and the comment immediately above it
   (`:630-632`) reads: *"…which **NEVER produces a GameOver**; an over-claim is a **revocable capability, not
   a wrongful game-end**."* **That comment IS the missing type.** Confirmed: `mark_unbounded_loop`
   (`game_state.rs:10377`) only does `entry.extend(axes)` — **it can never end a game.**

**The split is §5b.1's soundness asymmetry, lifted into the type system:**

| Outcome | Over-claim consequence |
|---|---|
| **`Win`** — terminal, **ENDS THE GAME** | ⛔ **WRONGFUL GAME-END. CATASTROPHIC.** |
| **`Advantage`** — non-terminal, no game-end by itself | ✅ **revocable capability mark. SAFE.** |

```rust
pub enum LoopOutcome   { Win(WinKind), Advantage(AdvantageKind) }
pub enum WinKind       { LethalDamage /*CR 704.5a*/, PoisonLoss /*CR 704.5c*/,
                         Decking /*CR 104.3c*/, ImmediateWin /*CR 104.2b*/ }
pub enum AdvantageKind { Resource /*CR 732.2a*/, ExtraTurns /*CR 500.7 — NOT a win condition*/ }
```
**`WinKind` has SIX variants today, not five** (`loop_check.rs:83-107`): `LethalDamage` `:86` · `PoisonLoss`
`:89` · `Decking` `:93` · **`ImmediateWin` `:98`** · `ExtraTurns` `:101` · `Advantage` `:107`. **All six must
be mapped.**

**⭐ Why it is REQUIRED, not tidy — it is the type C5 v2 needs.**
Kiki's tokens die at the next end step, so:
- ✅ they **CAN** certify **`Win(LethalDamage)`** — the tokens have **haste** (CR 702.10, `:3969`); swing
  **before** the end step;
- ❌ they **CANNOT** certify **`Advantage(Resource)`** — they **evaporate**; no durable resource.

⇒ **A short-lived ω-axis supports a terminal `Win` inside the window but NOT a durable `Advantage`.** Today
that is a comment. With the split it is a **compiler-enforced invariant** — and **C5 v2's lifetime→claim
mapping is only expressible once this lands.**

**⚠️ `shortcut_iteration_count` (`engine.rs:730-741`) is an ORTHOGONAL axis — do NOT conflate it.** It maps
`LethalDamage | PoisonLoss => UntilLethal`, everything else ⇒ `Fixed(1)`: that is *"asymptotic vs one-cycle"*,
**not** terminal-vs-non-terminal. **Keep both axes.** `iteration_count_maps_every_win_kind` (`engine.rs:9216`)
must stay **exhaustive**.

**Blast radius — measured, and larger than first scoped:** `analysis/loop_check.rs` (def + `classify_win_kind`) ·
`engine.rs:517,521,637,719,731,749,803,1773,1915,9216` · **`ai_support/candidates.rs:4936`** ·
**`bin/combo_verify.rs:64-99`** (a binary) · **`analysis/ability_graph.rs:39`** · `corpus.rs`'s
`ComboRow.win_kind` · `corpus_tests.rs:81`.

**⚠️ Wire shape changes.** `WinKind` crosses the serde boundary into `LoopCertificate` / `ShortcutProposal`
(externally tagged; unit variants as bare strings) and reaches the frontend
(`client/src/components/modal/LoopShortcutModal.tsx`, `client/src/adapter/types.ts:1437`). **A nested enum
changes the JSON ⇒ update the TS types and add a round-trip test.**

**Run `/add-engine-variant`; grep `data/engine-inventory.json`.**

### P2 — `run_combo_live`: the DUAL of the corpus harness (tests only; no fix)

`run_combo(board, step)` (**`corpus.rs:1175`**), where **"`step` drives exactly ONE loop iteration's actions"**
— **`step` IS the CR 732.2a fixed sequence.** A human writes it; `detect_loop` merely *judges* it. The live
path must **DISCOVER** the same cycle. Build the dual, sharing `ComboRow` / `ComboBoard` / `step`.

> ### ✅ **RESOLVED: `LoopProbe` IS drivable through the real reducer. P2 is SMALLER than prior revisions feared.**
> Chain verified, all four hops: `LoopProbe::act` (**`analysis/sim.rs:191`**) → `GameRunner::act`
> (**`game/scenario.rs:1172`**) → `apply_as_current` (**`engine.rs:2108`**) → **`apply_action_boundary`**
> (**`engine.rs:2154`**; defined at `:201` — **the same fn `apply()` calls at `:181`**).
> ⇒ **`run_combo` ALREADY drives every action through the real reducer.** The offline/live split was never
> reducer-vs-not — **it is only WHO JUDGES** (the harness calling `detect_loop`, versus the in-reducer hook
> setting `WaitingFor::LoopShortcut`). **The old "UNVERIFIED; check before building" caveat is VERIFIED-SAFE.**

- `ComboDriver::Offline(f)` → route-agnostic `Cycle(f)`; `DRIVERS` (`corpus.rs:673`) stays the single source
  of truth so its meta/partition tests extend for free.
- **`run_combo_live(board, step)`** drives `step` and asserts on the **live** terminal.

> ## ⛔ **P2 IS VACUOUS UNLESS THE DETECTOR IS TURNED ON. This killed the previous matrix.**
> `build_board` (**`corpus.rs:845-866`**) — the builder for **all 10 `Offline` rows** — **NEVER sets
> `loop_detection`**, and **`LoopDetectionMode::Off` is `#[default]`** (`game_state.rs:5787`). **`corpus.rs`
> assigns `loop_detection` at exactly ONE line in the entire file** (`:1871`, inside `build_drain_board_n`).
> ⇒ **The live hook is structurally unreachable on every offline corpus board**, independent of RC-1/2/3.
> **P2 MUST opt every live board in.** *(After **P0** there is only one on-state to get wrong — which is
> precisely why P0 comes first.)*

**The partition has FOUR cells** (CR 104.4b makes the first split a *rules* distinction):

| Partition | Rows | Live terminal |
|---|---|---|
| **L-OFFER** — cycle contains ≥1 **optional** player action | the 10 `Offline` drivers + the `ObjectReentry` + `Other` + `ColorConverting` deferrals | **must** reach `WaitingFor::LoopShortcut` |
| **L-AUTOWIN** — **mandatory** cascade | **17**, **18** (the 2 `LiveDrain` drivers) | **must** reach `WaitingFor::GameOver`; **must NOT offer** (CR 104.4b) |
| **GATED** | the **4** `gated_on`-nonempty rows (cards with `Unimplemented` parts) | excluded, **loudly** |
| **WAIVED — by ENGINEERING, not by rules** | the `ExtraTurnOrCombat` deferrals | none today — ⚠️ **CR 732.2a explicitly permits these** (*"may even cross multiple turns"*, `:6372`). **Waive LOUDLY, with the CR quote in the exclusion comment.** Silently bucketing them as "offline-only" is exactly the dressing-a-cut-as-a-rule that **D5** forbids. |

**The invariant:**
```
certifies_offline  ==  (offers_live XOR auto_wins_live)      // for every non-WAIVED, non-GATED row
```
- **⇒ failing = RC-3** (a false negative in real play). **Today all 10 `Offline` rows fail it** — including
  **row 1, Kilo + Freed + Relic**: *the corpus already certifies Combo B offline and has never once offered
  it live.*
- **⇐ failing = UNSOUNDNESS** — the live path certifying what the analyzer rejects. **Must never go red.**

> ## ⚠️ **FIX THE ASYMMETRY UPWARD, or this invariant will make things WORSE.**
> `run_combo` requires **one** covering pair after `WARMUP`; the live path requires **two, from iteration 0**
> (§3.2). The bi-implication therefore applies pressure to **relax the live path to one pair** to go green —
> **degrading the only game-ending path, and it will look like progress.**
> ## **⇒ Make `run_combo` ALSO require two consecutive covering pairs with equal Δ. Do this IN P2, before the invariant exists.**

Also: real cards, real libraries, real mana bases; add **Presence of Gond + Intruder Alarm** as a first-class
row; port `object_growth_51st_sprout_swarm_covers_and_offers` onto them (**it must FAIL today**).

### P3 — RC-3: ONE generalized arming context + driver ⚠️ **A REWRITE**

> ## ✅ **MEASURED: P3 is NOT on Combo A's path — and it STAYS IN THE PLAN ANYWAY.**
> The instrumented run (§3.1) shows `armed=true` and `drives OK`: **the live path already arms on Combo A**
> (a buyback-paid, token-creating recast is exactly the one bespoke shape `casting_costs.rs:6785` handles),
> and `drive_recast_iteration` drives it three times cleanly.
>
> **P3 is RESEQUENCED (Tier 2), NOT DELETED.** **RC-3 is a CLASS defect** — *zero of 53 corpus rows exercise
> the live path at all* — **and it is a class defect whether or not the canary needs it.** **Combo B** (a
> two-activation cycle) and the **P2 dual** both require it.
> ## **A phase does not leave the plan because the canary does not need it.** (§0.)
>
> **P3's class gate:** an **activation** loop, a **land-play** loop **and** a **cast** loop **all arm** — not
> just Combo B.

**Do NOT add `last_activation_context` as a sibling** (sibling-cluster smell). **But do not let the naming fix
disguise the cost.** Measured, `drive_recast_iteration` (**`engine.rs:1451`**) has **eight structural
cast-shaped elements**:

| # | line | hardcoded element |
|---|---|---|
| 1 | `:1460-1468` | card re-find by `(card_id, zone, controller)`, `min_by_key(id)` |
| 2 | `:1472` | `GameAction::CastSpell` — **the action kind itself** |
| 3 | `:1475` | `targets: vec![]` |
| 4 | `:1476` | `payment_mode: CastPaymentMode::Auto` |
| 5 | `:1487-1496` | `DecideOptionalCost { pay: ctx.uses_buyback.pays() }` — **buyback by name** |
| 6 | `:1501-1536` | `ManaPayment` resolves **`ConcreteDecision::ConvokeTaps` pins ONLY**; Order/Targets/Mode/MayChoice/UnlessBreak ⇒ `Err(RecastAbort)` |
| 7 | `:1539-1545` | `Priority` → empty stack = settle boundary, else `PassPriority` |
| 8 | **`:1548`** | `_ => Err(RecastAbort)` — **where Combo B's `WaitingFor::PayCost{TapCreatures}` lands** |

*(Its signature at `:1451-1456` takes **four** parameters — `clone`, `template`, `ctx`, `iteration`. Only
`iteration` varies per cycle.)*

Plus: `build_recast_template` emits `[ConvokeTaps]` (`:1558`) · `normalize_recast_frame` (`:1599`) ·
**`derived_fodder_class` fails closed unless EXACTLY ONE new battlefield object appears** (`:1633`, the check
at `:1638-1647`) — **a hard shape constraint no prior revision surfaced: a recast that makes a token AND
anything else ⇒ `None` ⇒ no offer.**

- Build `LoopProbeContext { actions: Vec<GameAction>, controller, decisions }` — **`actions` is a SEQUENCE**
  (CR 732.2a *"choices"*, plural; three drivers are multi-action; **Combo B is two**).
- Build `drive_loop_iteration(&[GameAction])`. **New context + new driver + likely a new
  `ConcreteDecision`/`PinnedDecision` variant ⇒ the `/add-engine-variant` gate is MANDATORY and is a hard
  prerequisite, not a conditional.** Grep `data/engine-inventory.json` first.
- ⚠️ **A NEW CHEAP NECESSARY-CONDITION PRE-GATE IS REQUIRED.** Commit **`57b0e537d`** (*"bound loop-shortcut
  iteration count (remote DoS in #5672)"*) bounds shortcut **EXECUTION** — `MAX_SHORTCUT_CYCLES = 1_000`
  (`engine.rs:2411`) caps the post-acceptance replay — **not DETECTION.** The pre-offer clone-drive is bounded
  today by exactly one thing: **it almost never runs** (`last_recast_context.is_some()`, `engine.rs:450`).
  Remove that and the drive runs on **every player action at every empty-stack priority beat**: 3× full
  `GameState::clone()` + 2× a cascade whose beat cap is `auto_pass_loop_max_iterations` (`engine.rs:2413`),
  each beat re-running `flush_layers`. **Without a new pre-gate, the #5672 remote DoS is the deliverable.**
- **Leave `engine.rs:3081` and the ring alone** — arming, not the ring, is the fix (§3.3).

### P4 — RC-1(c): a real CR 113.6 zone-of-function predicate ⚠️ **IT DOES NOT EXIST — NEW CODE**

- **`active_trigger_definitions` (`game/functioning_abilities.rs:391`) implements NO CR 113.6 logic** — it
  gates only phased-out (CR 702.26b, `:4172`) and non-emblem command zone. **`battlefield_active_triggers`
  (`:416`) is literally `state.battlefield × active_trigger_definitions`.** ⇒ **"just use
  `battlefield_active_triggers`" IS "hard-code battlefield-only"** — the thing CR 113.6 forbids.
  **The predicate must be written.**
> ## ⛔ **APPLY IT TO FOUR GATES — (1), (3), (4) AND (5b). A prior revision listed only three and left out gate (3), which is the one that actually blocks the user's board.** *(MEASURED, §3.1.)*
> **Gate (2) is ALREADY correctly battlefield-scoped — leave it alone.**
>
> ## ⛔⛔ **GATE (3): DO NOT HAND-MIRROR THE ZONE PREDICATE. *CALL* THE AUTHORITY. (Appendix B #18.)**
> **A prior revision of this section said the authority *"restricts to `[Battlefield, Command]`"*. That is the
> FIRST OF FIVE CLAUSES, and shipping it would have been UNSOUND.**
>
> **The real authority — `object_replacement_candidate_applies` (`game/replacement.rs:4829`), measured at
> `:4891-4897`:**
> ```rust
> if !in_scanned_zone && !is_entering && !is_being_discarded
>    && !is_applicable_dredge && !is_stack_self_move { return false; }
> ```
> …and **clause 1 is itself compound** (`:4873`):
> `in_scanned_zone = !is_liminal_source && [Battlefield, Command].contains(&obj.zone)`.
>
> **The four carve-outs are CR-carved exceptions to CR 113.6 — they are REAL RUNTIME FUNCTION:**
>
> | Carve-out | Functions from | CR |
> |---|---|---|
> | `is_entering` | anywhere → battlefield | **CR 614.12** *(self-replacement only)* |
> | `is_being_discarded` | **hand** | **CR 614.12** *(self only)* |
> | `is_stack_self_move` | **stack** | **CR 608.2n + 614.1a** *(self only)* |
> | ⚠️ `is_applicable_dredge` | **graveyard** | **CR 702.52a/b** — **and it is NOT SelfRef-gated** |
>
> ### ⛔ **AND MIND THE DIRECTION — THIS IS WHY IT IS THE MOST DANGEROUS LINE IN THE PLAN:**
> ## **Gate (3) is a *REJECT* gate. Narrowing what it scans ⇒ FEWER rejections ⇒ MORE ACCEPTS. An under-inclusive zone predicate is a FALSE-CERTIFICATE GENERATOR.**
> **A dredge card in a graveyard FUNCTIONS at runtime. A naive `[Battlefield, Command]` filter makes it
> INVISIBLE to the analysis** — and by §5b.1 that is the **catastrophic** direction, not the safe one.
>
> ### ✅ **THE FIX — one authority, two callers.**
> **Factor the functioning clause OUT of `object_replacement_candidate_applies` into a shared predicate**
> taking `event: Option<&ProposedEvent>`:
> - **`Some(ev)`** = **runtime** — evaluate all five clauses against the real event.
> - **`None`** = **ANALYSIS TIME** ⇒ **fail-CLOSED over all four event-keyed carve-outs** (we do not know which
>   event will occur, so we must assume any of them could).
>
> **Hand-mirroring drifts the moment someone adds the next CR carve-out — and drift in THIS direction is
> silent and unsound.** This is the repo's own **single-authority** principle, and here it is also the **only
> sound option.**

**Class gate (this is P4's acceptance criterion — NOT "the canary went green"):**
> **The verdict is invariant under ANY hidden-zone content.** Adding an arbitrary card to any library, hand
> or graveyard **must not change the answer**. *(`real_board_verdict_is_invariant_under_hidden_zone_contents`
> — and it must **assert the OFFER in every arm**, or it passes vacuously as `false == false`.)*
- **CR 113.6's exceptions are live and verified** (`docs/MagicCompRules.txt:771-801`). **The full letter set —
  prior revisions cited `b/c/d/e/f/j/k` and were INCOMPLETE:**

| exc. | line | what |
|---|---|---|
| a | `:773` | CDAs function everywhere |
| **b/c** | `:775` / `:777` | abilities that state their zones |
| **d/e/f** | `:779` / `:781` / `:783` | cost/play-modifying — function **on the stack and in the zone the object would be cast from, including the HAND** |
| g | `:785` | can't-be-countered/copied → stack |
| ⭐ **h** | **`:787`** | **modifies how the object ENTERS the battlefield → functions AS it enters.** **This is the actual authority for including the entering card in the replacement predicate — P4 relies on it and no prior revision cited it.** |
| i | `:789` | counters-can't-be-put → as it enters |
| **j** | `:791` | activated ability whose cost **can't be paid on the battlefield** (Reassembling Skeleton) |
| **k** | `:793` | trigger condition that **can't trigger from the battlefield** |
| ⭐ **m** | **`:796`** | **an ability whose cost/effect moves the object OUT OF a zone functions ONLY in that zone.** **The NARROWING counterpart to (j) — without it, a naive (j) implementation OVER-SCANS.** |
| n / p | `:799` / `:801` | deck construction / emblem-plane-scheme → command |

*(There is no 113.6l or 113.6o — the letters skip.)*

- **CR 400.2 is about HIDDEN zones; CR 113.6 is about FUNCTION. Do not conflate them.**
- **R4's fix is mis-aimed.** `active_replacements` (`functioning_abilities.rs:446`) is **deliberately
  all-zones**, and its own doc names the real runtime authority: **`find_applicable_replacements`
  (`game/replacement.rs`)**, which restricts to `[Battlefield, Command]` **plus the entering card (CR 113.6h /
  614.12) or the discarded card (CR 702.35a Madness)**. **Share THAT predicate.**
- **Permanent guard test:** the verdict **must not change** when an arbitrary card is added to any library or
  hand. **`Solemn Simulacrum` is the canonical fixture** — its ETB trigger **can** trigger from the
  battlefield, so **113.6k does NOT rescue it** ⇒ scanning it from the library is a **pure** CR 113.6 violation.

### P5 — RC-2: tolerate the bounded start-up transient (CR 732.2a D3)

- Drive until the cover holds on **two consecutive pairs with equal Δ**, rather than on the first two.
  **The SKIP is sound** — two consecutive covering pairs at offset *k* is exactly as strong as at offset 0
  (`board_covers_modulo_fodder` already demands exact content equality on the whole stable partition,
  `resource.rs:1033`).
- ⚠️ **DO NOT SHIP THE POPULATION BOUND.** *"Non-fodder population + 2"* is a **heuristic** dressed as a
  theorem. **Use the DoS cap:** drive to the cap, take the first *k* with two consecutive equal-Δ covering
  pairs, and **decline LOUDLY on overflow.** The population bound buys nothing (the cap already bounds it) and
  **is the only place in this phase an unsound argument can hide.**

### P6 — C2: place non-depletion ⚠️ **MUCH SMALLER THAN PRIOR REVISIONS CLAIMED — 3 of 5 axes ALREADY EXIST**

> ## ⭐ **MEASURED: three of the five C2 axes are ALREADY IMPLEMENTED. They become REGRESSION GUARDS, not new work.**

| Gate / place | Status | Where (measured) | CR |
|---|---|---|---|
| **activation gates** | ✅ **ALREADY WORKS** | `project_out_resources` **retains** gated keys (`resource.rs:2544-2558`) + `GameState::PartialEq` compares `activated_abilities_this_turn` (`game_state.rs:10918`) ⇒ a gated cycle compares **UNEQUAL** | — |
| **land plays** | ✅ **ALREADY WORKS** | `lands_played_this_turn` is **NOT** projected out (`resource.rs:2436`, exhaustive destructure, **no `..`**) **and is** compared (`game_state.rs:10840`) | **CR 305.2** (`:1692`) |
| **library size** | ✅ **ALREADY WORKS** | `library: _` is a strict-equality field (`resource.rs:2433`) | **CR 704.5b** (`:5494`) |
| **CR 732.3 fragmented loop** | ⚠️ **GENUINELY NEW** | — | **CR 732.3** (`:6380`) |
| **summoning sickness — the SICK-POOL axis** | ⚠️ **GENUINELY NEW, and narrower than it looks** | see below | **CR 302.6** (`:1630`) |

> ## ⭐ **SUMMONING SICKNESS CAN NEVER BREAK A LOOP AT A FUTURE ITERATION. Proved from code.**
> 1. `has_summoning_sickness` (`game/combat.rs:3402-3410`) reads the per-object bool `obj.summoning_sick`,
>    cleared at the **untap step**.
> 2. **A certifiable window never advances the turn or phase** — `GameState::PartialEq` pins `turn_number`
>    (`game_state.rs:10823`) and `phase` (`:10825`), and CR 732.2a's ending point is a **priority** beat (D4).
> 3. ⇒ **`summoning_sick` is CONSTANT across the window.** A token created inside the loop is sick for the
>    loop's **entire lifetime**; a creature unsick at iteration 0 is unsick at iteration N.
>
> ⇒ Sickness can only:
> - **(a)** break the **first** iteration that needs a sick creature's own `{T}` — **already caught inside the
>   drive** by `check_summoning_sickness_for_cost` → `cost_contains_tap_or_untap` (`game/restrictions.rs:676`),
>   which is **already rules-correct** (it matches `Tap|Untap` but **not** `TapCreatures`); or
> - **(b)** ⭐ **deplete a FINITE POOL of pre-existing unsick creatures**, consumed one-per-iteration via their
>   **own** `{T}`, replenished only by permanently-sick tokens.
>
> ## **(b) is the ONLY genuine C2 sickness axis, and it is only a false-certificate hazard when the pool size EXCEEDS the drive window.** Build exactly that. Nothing more.
>
> *(Corollary: **Kiki's tokens have HASTE** ⇒ `combat.rs:3406` exempts them ⇒ **never sick** ⇒ that is why Kiki
> defeats C2 as well as C1/C3/C4.)*

**The engine CAN see the cost split** (verified): `AbilityCost::Tap` (the `{T}` symbol) vs
`AbilityCost::TapCreatures { requirement, filter }` (`types/ability.rs:7841`). A blanket *"reject any `{T}`
cost"* would decline **CR 732.2a's own example** and most creature mana engines.
**Exhaustive typed enum + `_ => REJECT` + no-`..` totality guard.**

> **Library size is NOT a depletion axis** — see §4.5. **CR 704.5b requires an *attempted draw*; milling to
> zero is not a loss and there is no empty-library SBA.** The mill floors at 0 and **the loop survives**. The
> loss lands at the next **draw step**. It belongs to **C4** (`has_no_loss_axis`, `engine.rs:814-820`), and it
> is already a strict-equality field.

### P7 — C3: the firewall becomes a THRESHOLD scan — ⚠️ **TWO ARMS, NOT ONE**

**The drive measures every effect the current board produces.** The firewall's only remaining job is the
**discontinuity** the drive is structurally blind to. Everything else it currently does is duplicated work
that **gets the answer wrong**.

> ## ⛔ **C3 SPANS GATES (1) AND (4) — NOT gate (4) alone.**
> Gate (4) (`resource.rs:1524-1543`) iterates **`static_definitions` — statics only** (its own comment:
> *"Condition-gated statics (**CR 604.1** `:2663` / **CR 613.1** `:2958`)"*). But the canonical threshold
> example — *"when you control 10+ creatures, sacrifice…"* — is a **TRIGGERED** ability ⇒ **gate (1)**, and
> replacement thresholds are **gate (3)**. **An implementer who scopes C3 to gate (4) LOSES the trigger- and
> replacement-condition threshold scans — the exact class C3 exists for.**

> ## ⛔⛔ **P7 IS NOT "~10 LINES." A PRIOR REVISION SAID SO AND IT WAS MEASURABLY FALSE (Appendix B #17).**
> The *"one wrong arm at `ability_scan.rs:2456`, already probe-measured"* claim was based on a probe run on a
> **clean two-card board**. **Run on the REAL board, the 10-line fix changes nothing** — the next arm takes
> over, then the next. **See §3.1 for the instrumented run.**

### **ARM (a) — THE CLASS FIX: re-derive the `sibling` axis from a POSITIVE definition.** *(This is the phase.)*

**The defect (§3.1):** `sibling` is a **fail-closed default** over **~88 sites** in `game/ability_scan.rs`
(**57 `Axes::CONSERVATIVE` + 31 explicit `sibling: true`**), consumed by the detector as if it were a
**precise predicate**. `Axes::CONSERVATIVE` means *"I did not analyze this node"* — and on the `sibling` axis
that silently reads as *"this might observe the growing class."*

**The fix — define `sibling` POSITIVELY:**
> ## **A node reads `sibling` iff it reads a MUTABLE OBJECT SET whose cardinality the loop changes.**

Everything else is `sibling: false`. Worked, from the three arms the real board actually trips:

| Node | Verdict | Why |
|---|---|---|
| `QuantityRef::ObjectCount { filter }` (`:1593-1601`) | ✅ **`sibling: true` — KEEP** | it literally **counts a mutable object set**. **This is Gaea's Cradle, and it MUST stay fail-closed.** |
| `TargetFilter::Typed(tf)` (`:2456`) | ⛔ **`sibling: false`** | naming a *type* is not *counting* one. **Rejects Intruder Alarm — CR 732.2a's own example.** |
| `QuantityRef::ManaSpentToCast` (`:2072`) | ⛔ **`sibling: false`** | **stamped at CAST TIME, immutable thereafter.** Cannot observe a token count. **(Pentad Prism.)** |
| `Effect::RevealFromHand` (`:910`) | ⛔ **`sibling: false`** | reads the **hand**, not the battlefield. **(Choked Estuary — a land.)** |

> ## ✅ **P7 IS NOT A NEW PATTERN. THE TEMPLATE IS ALREADY IN THE FILE — AND IT SHIPS ITS OWN SAFETY NET.**
> **Read the comment above `ability_scan.rs:2454`:**
> ```rust
> // type/controller predicates read none. `event`/`sibling` stay CONSERVATIVE
> // (byte-preserved) — only the projected axis is refined.
> TargetFilter::Typed(tf) => Axes {
>     event: true,
>     sibling: true,                                 // <-- fail-closed default, DELIBERATELY un-refined
>     projected: typed_filter_reads_projected(tf),   // <-- REFINED, computed
> },
> ```
> **Someone already did to `projected` EXACTLY what P7 must now do to `sibling`, and consciously left
> `sibling` byte-preserved.** And `scan_filter_prop` (`:3124`) carries the guarantee — **verbatim**:
> > *"Classify a single `FilterProp` on the three read axes. **Exhaustive with NO `_` wildcard** — a NEW
> > `FilterProp` variant **fails to compile** here until it is classified (fail-closed to `CONSERVATIVE` when
> > its read surface is unproven)."*
>
> ## ⇒ **THAT ANSWERS "HOW DO YOU SAFELY FLIP 88 FAIL-CLOSED DEFAULTS": YOU DO NOT HAND-REVIEW 88 SITES.**
> **You make the COMPILER refuse to let a site go unclassified**, and you make **every arm carry the sentence
> that says why it cannot observe a loop-grown cardinality.** *(The `ManaSpentToCast` reasoning is the model:
> **"stamped at cast time, immutable thereafter, cannot observe a token count."** **All ~88 arms owe that
> sentence.** No wildcard. No default. No *"probably fine."*)*
>
> ⇒ **P7 = "extend the `projected` refinement to `sibling`, with the same no-wildcard completeness gate."**
> **This is `extend-don't-hack` + structural completeness, using a pattern that ALREADY EXISTS IN THIS FILE —
> and it converts an 88-site audit into a COMPILE-TIME OBLIGATION.** *(Size it honestly. Do not apologize for
> the size: this **is** the class fix.)*

**Method:**
1. **Refine `sibling` the way `projected` was refined** — a computed classifier per node, **exhaustive, no
   `_` wildcard**, so an unclassified variant **fails to compile**.
2. **Keep `CONSERVATIVE` as the default for genuinely unproven nodes** — fail-closed is correct (§5b.1).
   **The bug is not the default; it is that ~88 nodes were never revisited.**
3. **`typed_filter_reads_projected` (`:3113-3122`) already builds the full `Axes` and throws `event` +
   `sibling` away** (returns `acc.projected` at `:3121`) — **return `acc`.** The machinery exists; the
   **classification** is what is wrong.

> ## ⛔ **MIND THE DIRECTION.** Every arm you flip from `sibling: true` to `false` **REMOVES A REJECTION.**
> **These are not 88 chances to un-reject — they are 88 chances to WRONGLY CERTIFY** (§0 rule 3).
> **An arm you cannot justify in one sentence stays `CONSERVATIVE`.**

> ### ⭐⭐ **P7's CLASS GATE — and P7 MUST SHIP THE MISSING HALF OF THE TEST GATE (§0).**
>
> **The suite has a discriminating NEGATIVE guard and NO discriminating POSITIVE one. That asymmetry IS the
> root cause's institutional shadow, and P7 must close it.** Ship the positive guard **adjacent to its
> negative twin, in the same module, in the same style:**
>
> ```rust
> /// CR 732.2a Example (MagicCompRules.txt:6373) — the rulebook's OWN worked shortcut:
> /// Presence of Gond + INTRUDER ALARM, "I'll create a million tokens."
> ///
> /// `SetTapState { target: Typed[Creature], scope: All, state: Untap }` NAMES a type; it does
> /// not COUNT a mutable set. It cannot observe a loop-grown cardinality => it must NOT trip
> /// `sibling`, or the detector declines the Comprehensive Rules' own example of the rule it
> /// implements.
> ///
> /// DISCRIMINATING / revert-probe: delete the `sibling` refinement (restore the blanket
> /// `sibling: true` on TargetFilter::Typed) and this FLIPS TO FAIL.
> #[test]
> fn untap_all_creatures_does_not_fail_closed() {
>     assert!(!scan_effect(&intruder_alarm_untap_all()).sibling, "...");
> }
> ```
>
> ⚠️ **It MUST be revert-probed.** Delete the `sibling` refinement ⇒ **this test FLIPS TO FAIL.** *If it still
> passes with the refinement reverted, it is vacuous and it is not the gate.* **(The existing negative guard
> already carries exactly this discipline — copy it.)**
>
> ### **The class gate = the DISCRIMINATING PAIR + the dual:**
> | | Card | Shape | Must |
> |---|---|---|---|
> | **positive** *(NEW — does not exist today)* | **Intruder Alarm** | `Typed[Creature]` — **NAMES a type** | **NOT trip `sibling`** |
> | **negative** *(exists, `:4840`)* | **Gaea's Cradle** | `ObjectCount{Creature}` — **COUNTS a mutable set** | **STILL trip `sibling`** |
> | **corpus-wide** | — | — | **the P2 dual holds** |
>
> ## **The pair is the proof because the two cards STRADDLE the definition.**
> **A change that un-rejects BOTH is not a fix — it is the HOLE**, and by §0 rule 3 that is the **catastrophic**
> direction. **A change that un-rejects NEITHER has done nothing.** **Only the discriminating pair can tell
> those two apart — and until P7, the suite had only one of them.**
>
> **Combo A going green is a CONSEQUENCE, not the criterion.** *(A green canary with a broken Cradle is a
> false-certificate, not a win.)*

**⚠️ Arm (a) reaches FOUR gates at once:** (1), (2), (3) and (4) **all bottom out in the same
`scan_target_filter` / `scan_effect` walk.** One principled fix, four consumers.

### **ARM (b) — gate (4)'s UNCONDITIONAL `modifications` VETO (`resource.rs:1539`). A LATENT CLASS DEFECT.**
Replace `if !def.modifications.is_empty() { return true }` with the same treatment the condition arm gets:
*a modification whose **operand is the growing axis***, not *"any modification exists"*.

> **Measured: ZERO battlefield objects on the user's board carry a static modification**, so **P4 discharges
> gate (4) on THAT board.** **Arm (b) is not what blocks the user — it is what breaks the FIRST TIME AN ANTHEM
> RESOLVES.** **Empyrean Eagle, Favorable Winds and Door of Destinies are sitting in this very deck's
> library.** That is its hostile fixture (§7 row 16). *(Two authors asserted the opposite from memory —
> Appendix B #15, #16. Do not re-derive it; measure it.)*

**Also in scope:**
- **KEEP gate (4)'s CONDITION scan** (`:1532-1538`) — it is already the right place. **Narrow it** to a
  fire-time `Comparator` **against the growing axis**.
- **Retain the `projected` cost axis** and its firewall — it catches `ModifyCost{dynamic_count}`. *(A scaling
  cost also moves Δ, so C1 backstops it — but keep the axis; belt and braces on the only game-ending path.)*
- ⚠️ **`cost_surface_references_growing_class` is at `resource.rs:1629` — NOT `:1078`** *(a 551-line miss in
  every prior revision)* — **and it is NOT called from the fodder cover at all.** `loop_states_cover_modulo_fodder_growth`
  (`:1095-1153`) calls: `board_covers_modulo_fodder` → `grown_objects_are_inert` →
  `fire_time_conditions_read_growing_class` → `stack_entry_reads_growing_class` → `eq_except_growable` →
  `loyalty_activation_counts_match`. **No cost-surface gate.** It lives on the **object-growth** path (`:924`).
  **Do not assume otherwise.**

> ## ⛔ **IN P7, KEEP R6 INTACT. DO NOT DELETE IT HERE.**
> See §4.9 — deleting R6 **in this phase** is **unsound AND worth zero corpus rows**, and those two errors
> form a **trap** that ends in a **false certificate on the only game-ending path.**
>
> **R6's `delayed_triggers` conjunct is retired in P8 — and REPLACED by C5 v1, not merely removed.** Its other
> four conjuncts (`deferred_triggers`, `pending_trigger`, `pending_trigger_order`, `epic_effects`) **survive
> P8 as well.** ⇒ **P7 touches R6 not at all. P8 swaps exactly one of its five terms.**
>
> **DELETE R3 and R5 only if you can show, per §4.9's standard, that the drive measures what they scan.**

**Soundness note:** this phase is the one place the plan makes the detector **less** conservative. **Every
narrowing must be justified by *"the drive measures this"* — and the ⇐ direction of P2's duality invariant is
its runtime guard.** If a narrowing makes the live path certify something `detect_loop` rejects, **that is the
alarm.**

### P8 — C5: deferred execution (CR 603.7) ⚠️ **THE NEW CHECK**

**CR 603.7** *(verbatim, `docs/MagicCompRules.txt:2610`)*: *"An effect may create a **delayed triggered
ability** that can do something **at a later time**. A delayed triggered ability will contain 'when,'
'whenever,' or '**at**,' although that word won't usually begin the ability."*
**CR 603.7a** (`:2612`): *"Delayed triggered abilities are **created during the resolution of spells or
abilities**…"*

> ⭐ **CR 603.7a is the precise seam.** The delayed trigger is **armed INSIDE the drive window** even though
> it **fires OUTSIDE it.** That sentence is the whole check in one line.

**C5 v1 — BUILD. IT *REPLACES* R6's `delayed_triggers` TERM.**

**The classifier — measured over the full `DelayedTriggerCondition` enum (`types/ability.rs:2919`, 9 variants):**

| Variant | Fires on | Window |
|---|---|---|
| `AtNextPhase { phase }` | a **phase change** (`GameEvent::PhaseChanged`, `triggers.rs:6212`) | ⛔ **OUTSIDE** |
| `AtNextPhaseForPlayer { .. }` | a **phase change** | ⛔ **OUTSIDE** |
| `WhenEntersBattlefield { filter }` · `WhenDies { filter }` · `WhenLeavesPlay { .. }` · `WhenLeavesPlayFiltered { filter }` · `WhenDiesOrExiled { filter }` · `WheneverEvent { trigger }` · `WhenNextEvent { .. }` | an **event** | ✅ **INSIDE — can fire during the loop** |

**SEVEN of nine are EVENT-keyed** ⇒ **C5 v1 ⊄ R6. It has real ADMIT value**, and **R6 wrongly rejects an entire
class today** — event-keyed delayed triggers are the shape of **token / blink / persist / ETB engines**, i.e.
the whole `ObjectReentry` bucket.

```text
phase-keyed  (AtNextPhase | AtNextPhaseForPlayer)  ⇒ executes OUTSIDE the window ⇒ REJECT (fail closed)
event-keyed  (the other seven)                     ⇒ CAN fire inside            ⇒ ADMIT — the drive measures it
```
*(An armed event-keyed trigger the loop never fires is **harmless**: it sits armed, changes nothing in the
window, and does not affect repeatability. Admit it.)*

> ## ⛔⭐ **THE CLASSIFIER IS SCOPE-COUPLED — AND THE FIX IS TO MEASURE, NOT TO ASSUME. READ THIS BEFORE BUILDING C5.**
>
> **The justification *"the loop never advances the phase"* is NOT a rules fact.** **CR 732.2a explicitly says
> a shortcut *"may even cross multiple turns"*** (`:6372`) — it only has to **END** at a priority beat.
>
> **It is a property of the DRIVER. Measured — `drive_recast_iteration`'s Priority arm (`engine.rs:1539-45`):**
> ```rust
> WaitingFor::Priority { .. } => {
>     if clone.stack.is_empty() { return Ok(()); }        // HALTS at the first empty-stack priority beat
>     apply_action(clone, actor, GameAction::PassPriority, None)?;   // only ever passes with a NON-EMPTY stack
> }
> ```
> **Passing priority on an EMPTY stack is what advances the phase — and the drive never does it.**
> ⇒ **Phase-stability is a property of `drive_recast_iteration`, and P3 EXPLICITLY REWRITES THAT DRIVER.**
>
> **⚠️ And it is sharper than "turn-crossing":** endpoint equality does **NOT** imply the phase never moved.
> An **extra-combat** body (Aggravated Assault, Combat Celebrant, Time Sieve — the three `ExtraTurnOrCombat`
> rows) advances the phase and **RETURNS** to it, leaving `turn_number` **and** `phase` **equal at both
> endpoints** while having fired `PhaseChanged` in between. **`PartialEq` cannot see it.**
>
> ⇒ If anything ever admits a phase-advancing body, **the classifier SILENTLY INVERTS from conservative to
> WRONG** — it would **ADMIT** a phase-keyed trigger it must **REJECT**, **on the game-ending path.** And CR
> 732.2a blesses turn-crossing shortcuts, so **someone will eventually do this.**
>
> ### ✅ **THE FIX — do NOT encode the scope in a type, and do NOT write a prose invariant. OBSERVE IT.**
> **`ActionResult { events: Vec<GameEvent> }` (`game_state.rs:5824-25`) — the drive ALREADY receives every
> event from every `apply_action` call**, and **`GameEvent::PhaseChanged` (`types/events.rs:673`) is real and
> emitted.**
>
> ## ⇒ **C5 FAILS CLOSED IF THE DRIVEN WINDOW EMITTED A `GameEvent::PhaseChanged`.**
>
> **~3 lines. Sound under ANY loop-body scope, present or future.** It **observes the fact** instead of
> **encoding an assumption**; it is **self-maintaining** (widen the driver and it trips automatically, with no
> cross-file coupling to remember); and it fails in the **safe** direction (§5b.1 — a missed offer, never a
> false certificate).
>
> **A `WindowScope` type would make the compiler check an assumption. This makes the engine check reality —
> and it is this plan's own Principle 4 (*"Measure, don't derive"*) applied to C5 itself.** The scope-coupling
> exists **only** because we were deriving the window's extent instead of measuring it.
>
> *(Corollary: the phase-keyed/event-keyed split above is then sound **conditionally and checkedly**, rather
> than by assumption — and the WAIVED `ExtraTurnOrCombat` cell (§P2) stops being a silent landmine under C5.)*

> ## ⛔ **DO NOT LEAVE R6's `delayed_triggers` CONJUNCT IN PLACE "UNTIL C5 SUBSUMES IT." THAT MAKES C5 v1 DEAD CODE.**
> **C5 v1 ⊆ R6** (§4.9). If R6's `delayed_triggers` term still runs, **every state C5 v1 would reject is
> already rejected**, so **no fixture can distinguish them and no revert-probe can flip.** **C5 v1 must
> REPLACE that conjunct**, leaving R6's other four (`deferred_triggers`, `pending_trigger`,
> `pending_trigger_order`, `epic_effects`) intact.
>
> **C5 v1's VALUE is the loops it ADMITS** — an *inside*-window delayed trigger that re-arms identically each
> cycle, which R6 wrongly rejects today. **That admission is the testable claim.**
>
> **The `delayed_triggers` conjunct in `GameState::PartialEq` (`game_state.rs:10875`) is a DIFFERENT thing and
> MUST NOT be touched.** It is the trap-antidote (§4.9). **R6's gate ≠ `PartialEq`'s field comparison.
> Confusing the two is exactly how the false certificate gets shipped.**

**C5 v2 — NAME IT, DO NOT BUILD IT.** *(Requires **P1**.)*
The ω-axis **lifetime** refinement: map each axis's lifetime to the set of `LoopOutcome`s it may claim.
**Kiki:** `Win(LethalDamage)` ✅ (**haste — CR 702.10**, `:3969` — swing before the end step) /
`Advantage(Resource)` ❌ (the tokens evaporate).
**This is what makes Kiki reachable, and it is only expressible once P1's `LoopOutcome` split lands.** Write
it in **explicitly**, because it is **the antidote to the trap in §4.9** — an implementer who sees Kiki
rejected and does not know a refinement is coming **will go relax `eq_except_growable`.**

### P9 — C6: the ∞-composition fixpoint ⚠️ **A NEW CHECK — and the store already exists with NO reader**

**Per §4.11.** The engine proves an axis infinite (`mark_unbounded_loop`, `game_state.rs:10377`) and then
**forgets it**: every reader is HUD (`derived_views.rs:498`) or **debug-only** (`mana_payment.rs:97`,
`turns.rs:354`). **Nothing in `analysis/` or the detector path reads it.** ⇒ **a second loop that spends an
already-proven-infinite resource is rejected for "depleting" it — on the user's own board.**

- **The change is ONE DISJUNCT.** C2 → *"non-decreasing **OR already marked unbounded for this player**"*;
  `net_progress_for`'s no-net-negative-mana rule → **exempt axes already in ∞.**
- **Monotone least-fixed-point closure** — quantities collapse to **booleans** (reachability, not magnitude)
  ⇒ **not an LP.** `ResourceAxis` (`analysis/resource.rs:552`) is a **finite enum** ⇒ **terminates.**
  **~20 lines. No solver. No dependency.** *(Add a round cap as a backstop.)*
- **⚠️ Only a CERTIFIED `Advantage(_)` may seed ∞** — the revocable, safe side of P1's split. **The fixpoint
  is a monotone amplifier for false certificates:** one unsound mark poisons everything downstream.
- **⚠️ ∞ is PER-PLAYER** — the type already enforces it. **Key every exemption on the PROPOSER.**
- **⚠️ CR 106.4 / 500.5:** unbounded **mana** is usable **only within the step**. **Stay inside the step, or
  use a DURABLE axis (counters).**
- **⚠️ DO NOT "fix" the exclusion of `unbounded_resources` from `GameState::PartialEq`** (`:10606`, guarded by
  `unbounded_resources_excluded_from_loop_equality`, `:11434`). **That exclusion is what makes the fixpoint
  sound** — the closure adds marks *between rounds*, and if marks were compared, seeding one would break the
  board equality the next round depends on.

### P10 — RC-4: object identity across a loop cycle ⚠️ **ITS OWN PR, WITH ITS OWN SOUNDNESS PROOF**

Per §5: **not** a refactor, **not** a quick win. Requires **id-canonicalization of the whole frame** (remap
`ObjectId`s to a canonical order **and** canonicalize every id-valued field: `attached_to`, `attachments`,
`paired_with`, stack targets, delayed-trigger references).

- **Repoint to `objects_content_eq` (`types/game_state.rs:10428-10435`)** — the id-keyed seam is `b.get(id)`
  at `:10434`. **NOT `object_content_eq` (`:10453`), which contains no `ObjectId` at all.**
- **⭐ It already asserts `a.len() == b.len()` (`:10432`) ⇒ multiplicity is already preserved ⇒ scalarset
  normalization need only PERMUTE IDS.** That is a real scope reduction.
- **Take the formalism, do not invent one:** **Murφ scalarset symmetry reduction** (§5b.3). **Normalization
  FIRST** (errs **too fine** ⇒ misses loops ⇒ **fail-closed**); **canonicalization** (exact; nauty-class,
  effectively free at our board sizes) as the **proven upgrade**. **Never 1-WL / colour refinement as the
  ACCEPT relation — it errs coarse.**
- **Target: §5 Group B (7 rows).**

**The `Quotient` parameterization** — one `loop_states_cover(prior, current, &[Quotient])` replacing the four
`loop_states_cover_modulo_*` siblings (`resource.rs:924`, `:1095`, `:1326`, + `loop_states_cover_modulo_growth`)
— is still the right **shape** (the sibling-cluster smell is real), and it belongs **here**, **earned by the
canonicalization proof, not asserted as a refactor.**

> ⚠️ **`Quotient` is a PROPOSED type. It does not exist in the tree** (`grep` returns nothing). A prior
> revision used it in three places and defined it nowhere, which made its phase unexecutable. **Define it in
> the PR that introduces it, and run `/add-engine-variant`.**

---

## 7. Verification matrix

**Every row names its changed seam, its runtime test, the revert-probe that must FLIP to FAIL, and — for every
negative — its PAIRED POSITIVE REACH-GUARD.** Cast-pipeline tests follow the **`/card-test`** recipe
(`GameScenario` + `GameRunner::cast(..).resolve()` + `CastOutcome` deltas, verbatim Oracle text).

> ### ⚠️ The five traps that made the PREVIOUS matrix vacuous. All measured. Do not re-enter them.
> 1. **Arming-gate domination.** Six prior rows were dominated by `last_recast_context.is_some()`
>    (`engine.rs:450`), not by the axis under test. **Every activation/land-play loop is marked † — it
>    requires P3 to be reachable at all.**
> 2. **Detector-OFF domination.** The whole live half is vacuous unless the board opts in (§P2). **Marked ‡.**
> 3. **Cryptolith Rite + Squirrel Nest is NOT A LOOP** ⇒ the CR 302.6 split was never consulted. **Dead.**
> 4. **Damping Sphere's deltas CANCEL on the affinity board** ⇒ a positive fixture in disguise. **Dead.**
> 5. **Basalt Monolith + Mesmeric Orb has no cover and no empty-library SBA** ⇒ the axis is never consulted.

| # | Claim | Phase | Changed seam | Runtime test | Revert-probe (must FLIP to FAIL) | Paired positive reach-guard |
|---|---|---|---|---|---|---|
| 1 | **`Interactive` JSON still loads** | P0 | `LoopDetectionMode` serde (`game_state.rs:5783`) | round-trip `{"type":"Interactive"}` → `On` | drop `#[serde(alias)]` | **the repro fixture itself deserializes** (`repro_user_combo.rs:66`) |
| 2 | **`Off` still restores pre-feature behavior** | P0 | `samples()` (`:5818`) | `Off` ⇒ no ring, no arming, no offer | make `samples()` return `true` | `On` ⇒ ring populates |
| 3 | ⭐ **A `Win` can end the game; an `Advantage` cannot** | P1 | `LoopOutcome` | `Advantage(_)` ⇒ `mark_unbounded_loop` only, **never `GameOver`** | let `Advantage` reach the `GameOver` arm | a `Win(LethalDamage)` row **does** reach `GameOver` |
| 4 | **`shortcut_iteration_count` stays exhaustive** | P1 | `engine.rs:730-741` | `iteration_count_maps_every_win_kind` (`:9216`) | add a variant without a match arm ⇒ **compile error** | — *(compiler-enforced)* |
| 5 | ⭐⭐ **THE DUAL (⇒ coverage)** | P2 | `run_combo_live` vs `run_combo` | `certifies_offline ⇒ (offers_live XOR auto_wins)`. **Today 10 certify, 0 offer** | revert P3 ⇒ **every row but Combo A goes red** | **the reach-guard for the whole plan.** If only Combo A + B go green ⇒ **did not generalize; DO NOT SHIP** |
| 6 | ⭐⭐ **THE DUAL (⇐ SOUNDNESS)** | P2 | ″ | `offers_live ⇒ certifies_offline`. **MUST NEVER GO RED** | — | ⚠️ **first make `run_combo` require 2 covering pairs** — else the invariant pressures the live path *downward* (§3.2) |
| 7 | **L-AUTOWIN stays autowin** ‡ | P2 | `interactive_loop_bridge` (`engine.rs:491`) | rows 17/18 reach `GameOver`, **must NOT offer** (CR 104.4b) | — | proves the 4-cell partition. **Use Marauding Blight-Priest + Bloodthirsty Conqueror** — untargeted, no `may`. *(Sanguine Bond **targets** ⇒ row 17 is not choice-free.)* |
| 8 | **Combo A certifies on the real board** | P5+P7 | `try_offer_object_growth_shortcut` (`engine.rs:1656`) | `real_board_sprout_swarm_offers_loop_shortcut` (**FAILS today**) | — | the acceptance test |
| 9 | **RC-1 + RC-2 are BOTH required** | P5,P7 | — | the acceptance test **must STILL FAIL after P4 alone** | — | **a green-after-P4 result is a FALSE POSITIVE** |
| 10 | **CR 113.6 / 400.2 invariance** | P4 | zone predicate, gates (1)(4)(5b) | `real_board_verdict_is_invariant_under_hidden_zone_contents` (`:146`) | restore the all-zones scan | ⚠️ **must ASSERT THE OFFER in every arm** — a bare `assert_eq!` passes vacuously as `false == false` |
| 11 | **CR 113.6 exceptions preserved** | P4 | zone predicate | a **113.6j** (Reassembling Skeleton, graveyard) and a **113.6k** ability are **still scanned**; a **113.6m** object is **not over-scanned** | hard-code battlefield-only | catches the P4 trap in **both** directions |
| 12 | **Gate (2) is NOT broken by P4** | P4 | `resource.rs:1478-1499` | gate (2) stays battlefield-scoped | widen it to all-zones | it is **already correct** — the test guards against a well-meaning over-fix |
| 13 | **RC-2 bounded transient** | P5 | two-pair cover (`engine.rs:1732-38`) | verdict invariant under **which green creature the cast convokes** | restore the `(cs_n, cs_n1)` requirement | ⚠️ **assert the OFFER in every arm** |
| 14 | **P5 declines LOUDLY on overflow** | P5 | the DoS-cap bound | drive past the cap ⇒ explicit decline, **not** a silent false | swap in the population heuristic | a within-cap loop still offers |
| 15 | ⭐⭐ **CR 732.2a's OWN EXAMPLE** † ‡ | P7(a) | `ability_scan.rs:2456` | **Presence of Gond + Intruder Alarm OFFERS** | restore the unconditional `sibling: true` arm | **THE C3 DISCRIMINATOR — this is what proves P7(a).** Probe already measured ✅ |
| 16 | **P7(b): an ANTHEM ON THE BATTLEFIELD still offers** ‡ | P7(b) | `resource.rs:1539` | Combo A's board **plus Empyrean Eagle resolved onto the battlefield** ⇒ still OFFERS | restore `!def.modifications.is_empty() ⇒ reject` ⇒ FLIPS | **Row 8 (the unmodified real board) must ALREADY pass without this arm** — that is what proves arm (b) is a *latent* defect, not the fix. ⚠️ **The anthem must be ON THE BATTLEFIELD**: in the library it is masked by P4, and the row would pass vacuously |
| 17 | **Gaea's Cradle stays closed** | P7 | `scan_mana_production` (`ability_scan.rs:2117`) | `for_each_creature_production_still_fails_closed` (**exists, revert-probe verified**) | collapse the count-arms to `Axes::NONE` | `fixed_production_reads_nothing` still passes |
| 18 | **C2 sickness (the crux)** † ‡ | P6 | cost shape (CR 302.6); `cost_contains_tap_or_untap` (`restrictions.rs:676`) matches `Tap\|Untap` but **not** `TapCreatures` | **ACCEPT (real cards): Earthcraft + Squirrel Nest on a BASIC land CERTIFIES** — the cost is on the **enchantment**, has no `{T}` ⇒ CR 302.6 cannot apply ⇒ the **fresh sick** Squirrel is legal fodder. **DECLINE (⚠️ SYNTHETIC — declared as such): the same board with the untap granted as the TOKEN'S OWN `{T}`** | make `cost_contains_tap_or_untap` return `false` for `AbilityCost::Tap` ⇒ the **own-`{T}`** arm FLIPS to certify an illegal loop | **The ACCEPT arm IS the paired positive** — same loop, identical Δ, **only the cost shape differs.** ⚠️ **THE DECLINE ARM HAS NO REAL CARD:** Earthcraft is the *only* card with that text, and **every** real "own-`{T}` untaps a land" creature (Ley Druid, Voyaging Satyr, Krosan Restorer…) is a **single, non-recurring** untapper ⇒ the loop breaks by **EXHAUSTION, not sickness** ⇒ Cryptolith-Rite vacuity again. **Declare the synthetic fixture openly.** ⚠️ **C-Rite + Squirrel Nest is NOT A LOOP — dead** |
| 19 | **C2 activation gate** † ‡ | P6 | `ability_has_per_turn_activation_gate` (`resource.rs:2848`) | **Manaforge Cinder DECLINES** | remove the axis | **the same board with the 3/turn cap lifted OFFERS.** *(The old reach-guard — "remove the mana source ⇒ OFFER" — was incoherent.)* |
| 20 | **C2 land drops** † ‡ | P6 | `lands_played_this_turn` | **Crucible + Zuran Orb DECLINES** (CR 305.2) | remove the axis | the board **minus Crucible** must OFFER |
| 21 | **C2 fragmented loop** | P6 | transition set | a sequence needing an **opponent's non-pass action** DECLINES (CR 732.3) | drop the check | a sequence needing only priority **passes** still offers |
| 22 | **C1 scaled cost** ‡ | C1 | Δᵢ vs Δᵢ₊₁ | ⚠️ **NOT Damping Sphere, NOT Hum of the Radix.** The fixture must (i) sit **ON THE BATTLEFIELD** (a cover reads the battlefield — CR 113.6) and (ii) scale on a dimension **the loop does NOT feed**. **PROVE THE NON-CANCELLATION ARITHMETICALLY** — that is the exact check both dead fixtures failed. **UNVERIFIED: no replacement card confirmed. See §8 Q3 — a PROOF OBLIGATION, not yet a test.** | drop the `projected` axis | board minus the scaler must OFFER |
| 23 | **Δ measured, not derived** † ‡ | C1 | drive | **Solemnity + proliferate DECLINES** (true Δ = 0 counters) | derive Δ from the AST | board **minus Solemnity** must OFFER |
| 24 | ⭐⭐ **C5's REAL discriminator — a STABLE, OUTSIDE-window delayed trigger DECLINES** ‡ | P8 | the C5 classifier | a **pre-armed, CONSTANT** `AtNextPhase` delayed trigger (store does **not** grow ⇒ `PartialEq` passes) whose effect would destroy the growing class ⇒ **DECLINES** | collapse the classifier to *"always inside"* ⇒ FLIP | **Row 24b** (below) must still CERTIFY. ⚠️ **KIKI IS NOT THIS FIXTURE** — Kiki's store **GROWS**, so `PartialEq` already rejects it and a C5 probe on Kiki **cannot flip**. Building C5's test on Kiki is the Cryptolith-Rite vacuity one layer down |
| 24b | ⭐ **C5 EARNS ITS KEEP — an INSIDE-window delayed trigger CERTIFIES** ‡ | P8 | replace R6's `delayed_triggers` conjunct (`resource.rs:1582`) | a loop whose delayed trigger fires **inside** the window and **re-arms identically** each cycle ⇒ **CERTIFIES** (R6 wrongly rejects it today) | restore R6's `delayed_triggers` conjunct ⇒ FLIP | **This is the ONLY row that proves C5 v1 is not dead code.** Without it, C5 v1 ⊆ R6 and nothing can distinguish them |
| 24c | ⭐⭐ **C5's SCOPE GUARD: a phase-advancing window FAILS CLOSED** ‡ | P8 | the drive's `ActionResult.events` (`game_state.rs:5825`) | a driven window that emits **`GameEvent::PhaseChanged`** (`events.rs:673`) ⇒ **C5 DECLINES**, regardless of what the classifier would have said | delete the `PhaseChanged` check ⇒ a **phase-advancing** body **ADMITS a phase-keyed trigger it must REJECT** ⇒ **false certificate** ⇒ FLIP | **A normal single-priority-window loop (Combo A) must still OFFER** — else the guard is just an off-switch. ⭐ **THIS ROW IS WHY C5 SURVIVES P3's DRIVER REWRITE.** Today `drive_recast_iteration` (`engine.rs:1539-45`) halts at the first empty-stack priority beat and **cannot** advance the phase — but that is a **DRIVER** property, not a rules one, and **P3 rewrites the driver.** Hostile fixture: an **extra-combat** body (`ExtraTurnOrCombat`), which returns `turn_number` **and** `phase` to equal at both endpoints while firing `PhaseChanged` in between — **`PartialEq` cannot see it** |
| 25 | ⭐⭐ **THE TRAP-GUARD: `PartialEq`'s `delayed_triggers` is NOT relaxed** | P8 | **`game_state.rs:10875`** | **Kiki-Jiki + Zealous Conscripts stays DECLINED**, and the test FAILS if the `delayed_triggers` conjunct is removed from `GameState::PartialEq` | remove the conjunct ⇒ **Kiki falsely certifies** ⇒ RED | **THIS ROW EXISTS SOLELY TO CATCH THE §4.9 TRAP** — it is the plan's immune system in test form. **Paired positive: Presence of Gond (row 15), a structurally identical token loop with NO delayed trigger, must CERTIFY** — proving the decline is caused by the delayed trigger, not by generic token-loop rejection. ⚠️ **NON-VACUITY, measured: at a settled empty-stack Priority beat `delayed_triggers.len() == 1` (still armed) and the token is still on the battlefield; PASS INTO THE END STEP ⇒ THE TOKEN LEAVES.** The negative passes because **the phase never changed**, not because the harness cannot fire it. ⚠️ **R6's gate ≠ this field comparison — do not confuse them** |
| 26 | **C4 adverse scaling** ‡ | C4 | `has_no_loss_axis` (`engine.rs:814-820`) | opponent's **Suture Priest** ⇒ Combo A declines on the life axis | drop `life >= 0` | ⚠️ **VACUOUS BEFORE P7** — Suture Priest's typed filter trips gate (1) ⇒ the cover fails at `:1732` **before** the triple at `:1756` runs. **Only valid after P7.** ⚠️ Both its clauses are **`may`** ⇒ model it as an **opponent choice** (CR 732.2b), not a mandatory drain |
| 27 | **C4 self-deck** † ‡ | C4 | `has_no_loss_axis` | ⚠️ **NOT Basalt Monolith + Mesmeric Orb.** **CR 704.5b requires an ATTEMPTED DRAW; milling to zero is not a loss and there is no empty-library SBA** ⇒ the mill floors at 0 and **the loop survives** ⇒ **no fodder and no counter growth ⇒ NO COVER APPLIES ⇒ the axis is never consulted.** **UNREACHABLE AS SPECIFIED — see §8 Q4** | — | *(when a cover lands: an **opponent**-decking loop must certify `Win(Decking)` — that pairing makes the self-deck negative discriminating)* |
| 28 | ⭐⭐ **C6: an ∞-composed loop CERTIFIES** ‡ | P9 | `unbounded_resources` read in C2 / `net_progress_for` | **the user's OWN board: Kilo+Freed+Relic+Pentad Prism (⇒ ∞ counters ⇒ ∞ mana) makes Witherbloom+Sprout Swarm sustainable.** Both loops certify **together** | remove the ∞ disjunct ⇒ the second loop declines for "depleting" mana | **Each loop must ALSO certify STANDALONE** — else the fixpoint is masking a bug, not composing |
| 29 | ⭐ **C6 is PER-PLAYER** | P9 | `BTreeMap<PlayerId, …>` (`game_state.rs:7276`) | **an OPPONENT's ∞ mana does NOT make the proposer's loop sustainable** ⇒ DECLINES | key the exemption on any player ⇒ FLIP | the **proposer's own** ∞ mana **does** sustain it |
| 30 | ⭐ **C6 seeds only from `Advantage`, never `Win`** | P9 | P1's `LoopOutcome` | a `Win(_)`-side or speculative mark **cannot** seed ∞ | let `Win(_)` seed ∞ ⇒ a false certificate becomes reachable | a certified `Advantage(Resource)` **does** seed ∞. **The fixpoint is a monotone amplifier — one bad mark poisons everything** |
| 31 | **C6 terminates** | P9 | the closure | fixpoint converges in ≤\|`ResourceAxis`\| rounds; the round cap is never hit on any corpus board | remove the cap ⇒ a crafted cyclic board must still terminate | `ResourceAxis` (`resource.rs:552`) is a **finite enum** |
| 32 | ⚠️ **`unbounded_resources` STAYS out of loop equality** | P9 | `game_state.rs:10606` | `unbounded_resources_excluded_from_loop_equality` (**exists, `:11434`**) stays green | add `unbounded_resources` to `PartialEq` ⇒ **seeding a mark breaks the very board equality the next fixpoint round needs** | **the exclusion is LOAD-BEARING for C6 — do not "fix" it** |
| 33 | **CR 732.2b window reaches the opponents** | — | `WaitingFor::RespondToShortcut` (`game_state.rs:4347`) | **each other living player is prompted in APNAP order**; a `ShortcutResponse` naming an earlier stopping point **shortens** the loop | collapse the window ⇒ the proposer's acceptance immediately materializes N cycles | ✅ **ALREADY IMPLEMENTED — this is a regression guard, not new work** (§4.10) |
| 34 | **Combo B (a TWO-action cycle)** † ‡ | P3 | `LoopProbeContext{actions}` | **Kilo + Freed + Relic OFFERS** | — | assert `engine.rs:3081` + `MAX_SHORTCUT_CYCLES` are **untouched** |
| 35 | **DoS** | P3 | the new pre-gate | generalized arming does **not** regress #5672 | remove the pre-gate | **the drive must NOT run on every priority beat.** Assert a **drive-counter**, not wall-clock |
| 36 | **Multiplayer** | all | — | ≥1 criterion exercises **>2 players** (the fixture is **4-player**) | — | — |
| 37 | **Corpus regression** | all | `analysis/corpus.rs` | the **12 `DRIVERS`** rows still certify; the 4-cell partition holds | — | **corpus is 53 `CORPUS` rows / 12 `DRIVERS` (10 `Offline` + 2 `LiveDrain`)** — not "53 drivers", not "55 rows" |

**Legend:** † **requires P3** (nothing arms without it — the row is vacuous before then).
‡ **requires the board to opt into the detector** (P0/P2) — otherwise the live half is vacuous.

### 7.1 Rows I could NOT make non-vacuous — **stated, not hidden**

- **Row 22 (C1 scaled cost) — NO FIXTURE EXISTS.** Both candidates are dead: **Damping Sphere's deltas cancel
  exactly** against affinity (`base + k − (C₀ + k) = base − C₀`, constant in *k*), and **Hum of the Radix**
  is artifact-scoped and cannot touch a green instant. **No replacement card has been verified to exist.**
  ⇒ **§8 Q3. Until one is found, C1's revert-probe is UNBACKED and must be waived LOUDLY.**
- **Row 27 (C4 self-deck) — UNREACHABLE.** The loop has **no fodder growth and no counter growth ⇒ no cover
  applies at all** ⇒ `has_no_loss_axis` is **never consulted** — *and* the CR premise was wrong besides (CR
  704.5b needs an **attempted draw**). ⇒ **§8 Q4. It needs a cover before it is a test.**
- **Row 18's DECLINE arm is SYNTHETIC.** No real card provides a recurring own-`{T}` land-untapper. Declared
  in-row rather than dressed up as a real-card test.

> **These are recorded as PROOF OBLIGATIONS, not quietly dropped.** **A matrix that hides its unreachable rows
> is the same failure as a plan that hides its new surface** — and it is how the previous matrix shipped six
> rows that could never flip.

---

## 8. Open questions — do NOT hand-wave

0. ⭐ **UNVERIFIED: P7's TRUE SIZE.** This document proves the **shape** of RC-1 (a fail-closed `sibling`
   default over ~88 sites) and proves the old *"~10 lines"* sizing was **false** (Appendix B #17). **It does
   NOT measure the new sizing.** *"~88 sites"* is a **surface area, not a cost**: the refinement may touch 30
   arms or 300 — **most `Axes::CONSERVATIVE` sites may well be correct and stay.** **Do not quote "88" as an
   effort estimate; it is the audit's INPUT, not its output.**
   > **How to pin it, cheaply:** in a fresh worktree, keep instrumenting `fire_time_conditions_read_growing_class`
   > and refining arms **until `real_board_sprout_swarm_offers_loop_shortcut` actually goes GREEN**, then
   > **count the arms that had to change.** That converts the surface area into a real number — **and it would
   > be the first green run of that test in this workstream's history.** ⚠️ **Then re-verify the class gate:
   > Intruder Alarm un-rejects AND Gaea's Cradle STILL fails closed. A green canary with a broken Cradle is
   > the false-certificate direction (§0 rule 3), not a win.**

1. **Is C1 + C2 + C3 + C4 + C5 SUFFICIENT?** Manaforge Cinder has Δ₁=Δ₂=Δ₃ and is illegal at **4** (C2 catches
   it, not C1). Kiki has constant Δ and dies to a **clock** (C5 catches it, not C1/C2/C3/C4). **Each new check
   so far was found by someone constructing a counterexample to "the checks are complete."** ⇒ **Prove no
   FOURTH blind spot exists — or expect a C7.** **Not attempted. A real proof obligation, and the plan's
   history says it is not idle.**
2. **What blocks the 20 `Other` deferral rows?** **UNVERIFIED** — only `ObjectReentry`, `ExtraTurnOrCombat` and
   `ColorConverting` were classified (`corpus.rs:89-104`).
3. ⭐ **Find a REAL C1 scaled-cost fixture.** Damping Sphere and Hum of the Radix are both dead (§4.4). The
   fixture needs a scaler **whose growth dimension the loop does not feed**. **Search `card-data.json`; if none
   exists, C1's revert-probe is unbacked and must be waived LOUDLY.**
4. ⭐ **Find a REAL C4 self-deck fixture** — one where a cover actually applies (§7.1). Or prove the axis is
   unreachable and waive it.
5. **What happens AFTER an offer is ACCEPTED?** `materialize_fixed_shortcut` — does the replay correctly
   re-execute the **transient prefix** P5 introduces? **UNVERIFIED, and P5 creates this question.**
6. **Does `Effect::Proliferate` trip the firewall** — i.e. does Kilo's own trigger self-reject? **UNVERIFIED.**
7. **The C3 replacement predicate** (gate 3). *"Could this replacement apply?"* needs a real event-type ×
   filter match. A blanket *"any replacement exists ⇒ reject"* is useless on a Commander board.
8. **P10's canonicalization soundness proof.** Content-multiset equality is where a false certificate enters
   (§5). **This is the proof obligation that gates P9.**
9. **CR 732.5/732.6 are not implemented** (§4.2). Fail-closed, so not a blocker — but **rules-wrong**, and it
   lands on the L-AUTOWIN rows. **Fix or waive loudly. Do not leave it undiscussed.**

---

## Appendix A — Design principles

1. **Scope every conservatism to the present board and the sequence actually executed** — never to all boards
   reachable from all cards in all decks and hands. Reaching into a library is a **CR 113.6** error *and* a
   **CR 400.2** violation.
2. **The loop must be infinite from the PROPOSER's perspective** (CR 732.2a), then **passed around for
   response** (CR 732.2b). **Interaction is the response window's job, not the cover's.**
3. **Monotone reads are not hazards.** A firewall rejecting *"references a typed filter"* rejects the
   rulebook's own example.
4. **Measure, don't derive.** Replacements rewrite Δ at resolution; SBAs and triggers settle between
   iterations. Only the drive sees the truth.
5. ⭐ **But the drive is blind to what the window SCHEDULES.** *(The error that killed the last revision. It
   was TEMPORAL, not informational.)*
6. ⭐ **A coarse relation may REJECT, never ACCEPT.** Over-claiming a **`Win`** is a wrongful game-end;
   over-claiming an **`Advantage`** is a revocable mark. **Encode the asymmetry in the type system (P1).**
7. **Real cards, real libraries, real mana bases** in every combo-detector test.
8. **Read the rule, don't cite it.** Every architectural correction here came from the rule *text*.
9. **Don't claim rules cover you don't have.** **CR 732.2a does not forbid Kiki** — C5 v1 is honest
   engineering conservatism. Say so, or the first implementer who greps it stops trusting the document.
10. **The rules work has held; every failure was a CODE claim from memory.** Eighteen for eighteen.
    **Grep before you assert, and put the `file:line` in the sentence.**

## Appendix B — What we got wrong (eighteen times)

> **Every single one is a CODE claim asserted from memory. The RULES layer has never once failed** — 40/40 CR
> citations and 32/32 Oracle texts verified across six audits. **This appendix is the plan's immune system.**
>
> **#15, #16 and #17 were all committed WHILE WRITING THIS DOCUMENT.** #15/#16 are the SAME fabrication, by two authors, an hour apart. **#17 is the plan asserting its own root-cause fix was "necessary and sufficient" — refuted by RUNNING IT.** All three were caught only by re-measuring. **Read them before you assert anything.**

| # | Claim | Reality |
|---|---|---|
| 1 | *"No counter-growth cover exists"* | **FALSE.** `loop_states_cover_modulo_counter_growth` (`resource.rs:1326`) exists, names **Pentad Prism**, is wired into `detect_loop` + `interactive_loop_bridge`, has 4 tests. |
| 2 | *"`ResourceVector` already computes these deltas"* | **FALSE.** No tap-state axis; `mana` summed across all players; growth axes zero under `snapshot`. |
| 3 | *"The payment choice is inexpressible"* | **FALSE.** Witherbloom is **Legendary**; Relic filters on `Legendary`. |
| 4 | *"Convoking Witherbloom is illegal at iteration 2 ⇒ the proposer must SEARCH"* | **FALSE, and it inverted the fix.** `select_convoke_taps` re-runs each iteration; the *place* is non-depleting (Δ=0). The real defect is **RC-2**: a **bounded transient** the cover forbids — which **CR 732.2a explicitly permits**. |
| 5 | *"Gaea's Cradle fail-closes via `repeat_for`"* | **FALSE.** It parses as `AnyOneColor{count: Ref(ObjectCount{Creature,You})}` — caught **only** by `scan_mana_production`. **Do not "simplify" that walker.** |
| 6 | *"Combo B's cycle is ONE activation"* | **FALSE.** `drive_offline_kilo_freed_relic` (**`corpus.rs:1537`**) takes **TWO** `ActivateAbility` actions. Its comment: *"Relic has two mana abilities; the tap-self one would not fire Kilo's trigger."* **A single-action arming latch cannot capture it.** |
| 7 | *"Generalizing `normalize_recast_frame` lifts all 13 `ObjectReentry` rows — worth more than Phases 1–5 combined"* | **FALSE.** It lifts **ZERO** directly. 6 rows are blocked by R6/RC-1/RC-3, not id churn. The other 7 need **id-canonicalization** — and stripping the object **does not fix stable objects whose `paired_with`/`attached_to` point at the churned id** (Deadeye Navigator never moves and still fails). **The riskiest change in the program, not a quick win.** |
| 8 | *"C3 is the one arm three rounds never broke — keep its logic"* | **FALSE, and it contradicted §3.1.** `ability_scan.rs:2456` sets `sibling: true` for **any** typed filter ⇒ the predicate rejects **Intruder Alarm — CR 732.2a's own worked example.** |
| 9 | *"Measured trips, in order"* (RC-1) | **Wrong provenance.** `board_covers_modulo_fodder` runs first (`resource.rs:1120`) and returns false before the firewall (`:1131`). Both root causes are real; the trips were seen **under instrumentation**, not on the live path. |
| 10 | *"Hum of the Radix DECLINES"* | **UNSATISFIABLE.** *"Each **artifact spell** costs {1} more"* — Sprout Swarm is a green instant. **Both arms OFFER.** |
| 11 | **(team-lead)** *"§4.10 contains a planted calibration contradiction"* | **§4.10 DID NOT EXIST.** §4 ran 4.1–4.6; §§4.7–4.10 were deleted **two commits earlier**. He was asserting from memory of a **superseded revision of his own document**. *(Collateral: `Quotient`'s definition died with it, so a phase became unexecutable.)* |
| 12 | *"regeneration (CR 701.15)"* | **FALSE. CR 701.15 is GOAD** (`docs/MagicCompRules.txt:3392`). **Regeneration is CR 701.19** (`:3428`). |
| 13 | **(team-lead)** *"`On` is TODAY'S DEFAULT FOR REAL MATCHES (`match_config.rs:89`, `session.rs:1613`)"* | **FALSE.** Both are **`#[cfg(test)]` fixtures** (`match_config.rs:60` is `mod tests`; `session.rs:1601` is `fn loop_detection_config_persists_across_bo3_rebuild`). **The shipped default is `Off`** (`match_config.rs:27`). He grepped the symbol, read the line numbers, and **inferred the context without reading it.** *(The P0 directive survived; only its rationale was wrong — and the true one is stronger.)* |
| 14 | **(planner)** *"`is_on()` has live production callers at `session.rs:1625,1636`"* | **FALSE.** **Zero production callers** — all six sites are inside `#[test]` fns. *(Caught pre-flight by team-lead; never reached the document. **`is_off()` IS production-load-bearing**, at `match_config.rs:36`.)* |
| **15** | **(planner)** *"Gate (4)'s `modifications` veto blocks the user's board — **Freed from the Real is an aura, and auras carry modifications**"* | ⛔ **FALSE.** Measured on the fixture's real `GameObject`: **`Freed from the Real \| Battlefield \| static_definitions: []`** — **EMPTY.** An aura granting only *activated* abilities has no modifications. **Battlefield objects with a static modification on that board: ZERO.** Gate (4) fires there **only via the LIBRARY** ⇒ **P4 fully discharges it** ⇒ **arm (a) alone IS sufficient.** ⭐ **The lesson: I read `resource.rs:1539` CORRECTLY and then inferred REACHABILITY from memory. A correct reading of the code is not a correct claim about the board.** *(Worse: my first refutation used a jq path — `.parse_details.*` — that does not exist. The conclusion held, but for a moment the evidence didn't. **Re-measure against the authoritative source.**)* |
| **16** | **(team-lead)** *"Freed from the Real is an AURA — which is precisely why it appeared in the trip list. **Your inference is confirmed.**"* | ⛔ **FALSE — and it "confirmed" a claim the planner had ALREADY RETRACTED.** Two authors, independently, **within one hour**, invented the *same* aura→modifications link from memory — **while writing the document whose entire purpose is to prevent exactly that.** The **decision** (two-arm P7) is right; the **rationale** is not. **Arm (b) is a latent class defect, not a root cause.** |
| **18** | ⛔⛔ **(planner)** *"Gate (3)'s runtime authority restricts to `[Battlefield, Command]` (+ entering / discarded). Share that predicate."* | ⛔ **UNSOUND — AND IN THE *ACCEPT* DIRECTION, WHICH IS THE CATASTROPHIC ONE.** That is **three of FIVE clauses**, and it misses the **`liminal`** sub-clause entirely. Measured, `replacement.rs:4891-97`: `!in_scanned_zone && !is_entering && !is_being_discarded && !is_applicable_dredge && !is_stack_self_move`, with `in_scanned_zone = !is_liminal_source && [BF, Command].contains(zone)` (`:4873`). **A DREDGE card in a GRAVEYARD functions at runtime (CR 702.52a/b, and it is not SelfRef-gated) — my predicate made it INVISIBLE to the analysis.** ⭐ **Gate (3) is a REJECT gate ⇒ narrowing what it scans ⇒ FEWER rejections ⇒ MORE ACCEPTS ⇒ a FALSE-CERTIFICATE GENERATOR.** **I spent this entire session writing *"a coarse relation may REJECT, never ACCEPT"* — and then wrote the accept-direction bug into P4 myself.** ⇒ **The fix is to CALL the authority, not mirror it** (P4). ⇒ **And the lesson generalizes: P4 and P7 are the only two phases that move the detector toward ACCEPT. They need double the review of everything else — see §0 rule 3.** |
| **17** | ⭐⭐ **(planner)** *"RC-1 is one wrong arm. **Arm (a) + P4 is NECESSARY AND SUFFICIENT** for the user's bug — ~10 lines, already probe-measured."* | ⛔⛔ **FALSE, AND IT IS THE MOST IMPORTANT ENTRY IN THIS TABLE.** **I built the fix in an isolated worktree and RAN the failing test.** It still declined. Instrumented: arming is **fine** (⇒ **P3 is not on Combo A's path** — that half was right), the drive is **fine**, and the cover dies in `fire_time_conditions_read_growing_class` at **gate (3)** — on **Pentad Prism**, *a card in the user's own combo* (`QuantityRef::ManaSpentToCast => Axes::CONSERVATIVE`, `ability_scan.rs:2072`). Fixed that arm ⇒ next blocker **Choked Estuary**, *a land* (`Effect::RevealFromHand => Axes::CONSERVATIVE`, `:910`). **Fix an arm, the next card appears.** ⇒ **RC-1 is a CLASS: `sibling` is a fail-closed default over ~88 sites (57 `Axes::CONSERVATIVE` + 31 `sibling: true`) consumed as a precise predicate.** ⭐ **THE ROOT ERROR: the probe that "proved" the 10-line fix was run on a CLEAN TWO-CARD BOARD. It was a VACUOUS DISCRIMINATOR — the exact failure this document spends a whole section quarantining, committed in the document's own root-cause claim.** **A single-fixture probe cannot distinguish a CLASS fix from a CARD fix. Only the corpus dual (P2) can — which is why P2 is in Tier 1.** |
| — | *"The untap step is CR 502.2"* | **FALSE.** 502.2 is **day/night** (`:2150`). The untap step is **CR 502.3** (`:2154`). |
| — | *"An LP / Petri-VAS model would replace the drive"* | **Unsound.** Δ is not derivable (replacements); legality is not a resource. |
| — | *"Adopt `egg` for the equality core"* | **REJECTED on measurement.** With zero rewrite rules, `Analysis::merge` is **never called** ⇒ egg-minus-rewrites is a memoized catamorphism, **and `ability_scan.rs` already is one.** **The bug is a wrong arm, not drift.** §5b.2. |
