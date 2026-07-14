# Planner brief — rewrite the combo-detector plan from measured facts

**Date:** 2026-07-14 · **Branch:** `debug/combo-generator` (fork-only, **never toward `main`**).
**Your job:** produce a NEW plan at `.planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md`.

> ## ⛔ THE OLD PLAN IS POISONED. DO NOT EDIT IT — REPLACE IT.
> `REAL-BOARD-RCA-AND-PLAN.md` @ `69fe6c3ea` was revised **six times in place**. A fifth adversarial
> review (which **ran tests**, unlike the first four) found its **spine is false**, its only
> less-conservative phase is **unsound**, its blind-spot taxonomy is **incomplete**, its `egg` section
> should be **deleted**, its verification harness is **vacuous**, and it **cites 20 wrong line numbers**
> plus an enum (`Quotient`) that is **used 3× and defined nowhere.** Patching it in place reproduces
> exactly the failure mode that generated its first eleven errors.
>
> **This brief is the source of truth.** The old document is a **reference for the RCA narrative and the
> CR deductions ONLY** (§1–§4.5, which survived). **Everything from §4.6 onward is superseded by this
> brief.**

**Everything below is MEASURED** — from the code, from `docs/MagicCompRules.txt`, or from
`data/card-data.json`. Line numbers here are verified. **Re-verify anything you restate.**

---

## 0. The failure mode that has cost us eleven errors — internalize this

> **Eleven errors. Every single one was a CODE claim asserted from memory. The RULES work has held
> through five audits (20/20 CR citations verified, 15/15 Oracle texts verified).**

**Error #11 was the team-lead's**, and it is instructive: he "planted a calibration contradiction" at
**§4.10** for the reviewer to find — but **§4.10 does not exist.** §4 runs **4.1–4.6 only**; §§4.7–4.10
were deleted at `7469f7904`, **two commits before he froze the doc.** He was asserting from memory of a
**superseded revision of his own document.** *(Collateral: `Quotient`'s definition died with §4.10, so P6
became unexecutable.)*

**Rule: grep before you assert, and put the `file:line` in the sentence.** If you cannot verify it, write
**UNVERIFIED**. An honest "did not reach this" beats a plausible claim that costs a cycle to refute.

---

## 1. The bug (all four root causes CONFIRMED — keep these)

The engine **arms correctly** and then **declines silently** on the user's real 4-player Commander board.
Repro: `crates/engine/tests/integration/repro_user_combo.rs` (1 passing guard + 2 `#[ignore]`d FAILING).
Fixture: `crates/engine/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json`.

| | Root cause | Status |
|---|---|---|
| **RC-1** | The fire-time observer predicate is **wrong** — and it scans **hidden zones** | **CONFIRMED BY MEASUREMENT** (see §3) |
| **RC-2** | The cover **forbids a bounded start-up transient** (demands recurrence from iteration 0) | **CONFIRMED — a reviewer attacked it directly and could not break it** |
| **RC-3** | The live path arms on **one bespoke card shape**, and **`grep -c "WaitingFor::LoopShortcut" corpus.rs` == 0** | **CONFIRMED** |
| **RC-4** | Loop equality is **id-keyed** (CR 400.7: an object that changes zones **is a new object**) | conclusion CONFIRMED; **the plan cited the WRONG SEAM** (see §7) |

**RC-2's asymmetry table is exactly right and is the strongest evidence in the document:**

| | transient tolerated | covering pairs required |
|---|---|---|
| **Offline** `run_combo` (`corpus.rs:1179`) | **≥4 cycles** (`WARMUP:2` + failed `STEADY` retries) | **1** |
| **Live** `try_offer_object_growth_shortcut` (`engine.rs:1656`) | **0** | **2, from iteration 0** |

⇒ the old plan's *"make `run_combo` ALSO require two consecutive covering pairs"* is **correct and
necessary** — otherwise the duality invariant pressures us to **relax the live path**, degrading the only
game-ending path while looking like progress. **Carry it.**

---

## 2. ⛔ THE SPINE WAS FALSE — re-derive from a THREE-category taxonomy

**The old §4.6 "governing constraint" said:**
> *"Every ability on the battlefield that fires during the loop ALREADY FIRES IN THE DRIVE and ALREADY
> LANDS IN Δ."*

**FALSE. The drive measures what RESOLVES inside the window. It is structurally blind to what the window
SCHEDULES.** **Every deletion in the old P5 was derived from that sentence**, so they must all be
re-derived.

### The corrected taxonomy — what the current board can do that the DRIVE cannot see

| # | Blind spot | Why the drive misses it | Check |
|---|---|---|---|
| **1** | **Monotone depletion outside the drive window** | Δ constant for the driven iterations; the sequence dies at iteration 4 (Manaforge's 3/turn, land drops, library, sickness) | **C2** |
| **2** | **A discontinuity — a threshold tripping at a future iteration COUNT** | Δ constant until it trips | **C3** |
| **3** | ⭐ **DEFERRED EXECUTION (CR 603.7) — NEW, and it is a first-class citizen** | The loop **SCHEDULES** an effect whose execution lands **OUTSIDE** the certifiable window | **C5 (new)** |

**Category 3, enumerated structurally over all 224 `Effect` variants:** `CreateDelayedTrigger` (Kiki-Jiki,
Splinter Twin, blitz, dash, myriad, encore, rebound, epic) · `SkipNextTurn` / `SkipNextStep` ·
`ControlNextTurn` · `AddPendingETBCounters` · **the entire replacement family** (CR 614.1 — alters a
*future* event, mutates nothing now) · `ReduceNextSpellCost` · `GrantNextSpellAbility`.

### ⚠️ Kiki-Jiki defeats C1, C2, C3 AND C4 simultaneously — this is why C5 must exist

Nothing depletes (**C2 blind**). No threshold trips at any iteration count — **it fires on a CLOCK, not a
count** (**C3 blind**). Δ is perfectly constant at `tokens_created: +1` (**C1 blind**). And C4's triple
sees nothing wrong. **All four checks pass on a loop whose entire growth axis is destroyed at the next end
step.**

**C5's shape:** classify each armed `DelayedTrigger` / replacement by whether its execution falls **inside
or outside** the certifiable window, and **FAIL CLOSED on anything outside.** This is a **new check, not a
deletion.**

---

## 3. ⛔ RC-1's ROOT FIX IS ~10 LINES — and it kills the `egg` proposal

**Measured on a pristine tree:** Intruder Alarm and Gaea's Cradle produce **byte-identical**
`Axes { event: true, sibling: true, projected: false }`. **That IS RC-1, confirmed by measurement.**

**Measured with a single-line revert-probe** (`ability_scan.rs:2456`, `sibling: true → false`, nothing
else changed):
- **Intruder Alarm — UN-REJECTED** ✅ *(it is **CR 732.2a's own worked example**, verified at
  `docs/MagicCompRules.txt:6373`: Presence of Gond + Intruder Alarm)*
- **Gaea's Cradle — STILL fails closed** ✅
- `ability_scan::mana_production_scan_tests::for_each_creature_production_still_fails_closed` — **still
  green** ✅

**Why they separate:** `QuantityRef::ObjectCount` **hard-sets `sibling: true` in its OWN arm**
(`ability_scan.rs:1593-97`), whereas `SetTapState` takes its bit **solely from the shared `Typed` child**
(`:2454`). The asymmetry is real ⇒ **separable in a context-free fold.**

**The fix (~10 lines):** `sibling: typed_filter_reads_sibling(tf)` — and **`typed_filter_reads_projected`
(`:3113-21`) ALREADY builds the full `Axes` and throws `event` + `sibling` away.** Return `acc` instead of
`acc.projected`. Ship it with a `FilterProp` audit and revert-probe tests.

> ### ⛔ VERDICT: **REJECT `egg`. DELETE the old §5b.2 and §5b.4 entirely.**
> - **`Axes` IS a join — and that is exactly what kills it.** With **zero rewrite rules** (the spike's own
>   scoping), **no e-class unions ever occur**, so `Analysis::merge` is **never called**. egg-minus-rewrites
>   = hashconsing + a memoized catamorphism, and **`ability_scan.rs` already IS the catamorphism.**
>   **The bug is a WRONG ARM, not drift. No formalism fixes a wrong arm.**
> - Hosting the AST would need a **~645-variant mirror IR** (`Effect` lacks `Hash`/`Ord`; 9 `HashMap`
>   payloads) ≈ **2× `ability_scan.rs`, 4–8 engineer-weeks, 12 new deps, a WASM bundle cost** — **to reach
>   the verdicts a 10-line fix already produces.**
> - **KEEP** the old §5b.1 (the soundness-asymmetry table) and §5b.3's **prescription**. See §7.

---

## 4. ⛔ DELETING R6 IS UNSOUND *AND* WORTHLESS — and the two errors form a TRAP

The old plan ordered R6 deleted (`state.delayed_triggers` non-empty ⇒ reject) on two claims. **Both are
measured FALSE:**

1. ***"The delayed trigger fires in the drive"* — FALSE.** `AtNextPhase{phase}` fires **only** on
   `GameEvent::PhaseChanged` (`triggers.rs:6212`), and `GameState::PartialEq` pins **`turn_number`
   (`game_state.rs:10823`)** and **`phase` (`:10825`)** ⇒ **no certifiable cycle can contain the phase
   change that fires it.**
   **Test (real Kiki Oracle, real `{T}`, driven to a settled empty-stack Priority beat):**
   `delayed_triggers.len() == 1` **still armed**, **token still on the battlefield.**
   **Non-vacuity:** pass into the End step ⇒ **the token leaves.** The negative passes because the phase
   never changed, **not** because the harness cannot fire it.
2. ***"Worth 2 corpus rows on its own"* — FALSE. It is worth ZERO.** `eq_except_growable`
   (`resource.rs:1409`) ends in **`a == b`** ⇒ reuses `GameState::PartialEq` ⇒ which compares
   **`delayed_triggers` (`game_state.rs:10875`)** ⇒ **Kiki is ALREADY rejected, independently of R6.**

> ### ⚠️ THE TRAP — write this into the plan so nobody walks into it
> An implementer deletes R6 → sees Kiki **still** rejected → follows the trail to `eq_except_growable` →
> **relaxes the `delayed_triggers` comparison** to collect the promised 2 rows → **the detector now
> certifies a loop whose entire growth axis is destroyed at the next end step. FALSE CERTIFICATE on the
> only game-ending path.**

**⇒ P5 becomes: KEEP R6. Fix gate (1)'s wrong arm (§3). That ~10-line fix IS the RC-1 root fix.**

*(Also: R6 is a **3-way** gate — `delayed_triggers || deferred_triggers || pending_trigger`,
`resource.rs:1582-84`. The old plan described one, and cited `:1577`.)*

---

## 5. ⛔ P0's LIVE DUAL IS VACUOUS — the default-OFF toggle (never mentioned; 0 grep hits)

- The live hook requires `state.loop_detection.samples()` (`engine.rs:448`).
- `samples()` = `On | Interactive` (`game_state.rs:5819`).
- **`LoopDetectionMode::Off` is `#[default]`** (`game_state.rs:5787`).
- **`corpus.rs` sets `loop_detection` at exactly ONE line in the entire file** (`:1871`, inside
  `build_drain_board_n`, and to `On`). **`build_board` (`corpus.rs:845-66`) — the builder for ALL 10
  `Offline` rows — NEVER sets it.**

⇒ **the live hook is structurally unreachable on every offline corpus board**, independent of RC-1/2/3 and
of arming. **P0's dual as specified would hand `run_combo_live` detector-OFF boards ⇒ every live row
declines VACUOUSLY ⇒ and P1 (arming) takes the blame.**
Second-order: rows 17/18 run under `On`, but the offer path only runs under `Interactive` ⇒ P0's
*"must NOT offer"* assertion **also passes vacuously.**

**⇒ P0 MUST set `loop_detection = Interactive` on every live board**, add a **GATED** partition cell
(the 4 `gated_on` rows), and resolve the shared-`step`/hook collision (once arming generalizes, the hook
flips `waiting_for` **mid-sequence**).

**This is NOT a 5th root cause of the user's bug** — the real fixture is `Interactive`
(`repro_user_combo.rs:66`), so *"every cheap gate at `engine.rs:450` is green"* still holds.

---

## 6. ✅ RESOLVED: `LoopProbe` IS drivable through `apply()` — P0 is SMALLER than the old plan claims

`LoopProbe::act` (`sim.rs:191`) → `GameRunner::act` (`scenario.rs:1172`) → `apply_as_current`
(`engine.rs:2108`) → `apply_action_boundary` (`:2154`).

⇒ **`run_combo` ALREADY drives every action through the real reducer.** The offline/live split was never
reducer-vs-not — **it is only WHO JUDGES** (the harness calling `detect_loop`, versus the in-reducer hook
setting `WaitingFor::LoopShortcut`). Mark the old *"UNVERIFIED; check before building"* caveat
**VERIFIED-SAFE.**

---

## 7. Other measured corrections the new plan must carry

- **⛔ *"New surface drops to essentially C2 alone"* is arithmetically FALSE.** It silently deletes **P1**,
  which the plan's own §1 calls a **PREREQUISITE**. egg is an *AST analysis*; **P1 is an *action driver*.**
  **Honest surface: P1 (driver) + P2 (CR 113.6 predicate) + C2 + C5. State it ONCE, in the exec box, and
  never contradict it.**
- **⛔ C3 names the WRONG GATE.** Gate (4) (`resource.rs:1524-43`) iterates **`static_definitions` —
  statics only** (its own comment: *"Condition-gated statics (CR 604.1 / CR 613.1)"*). But §4.6's own
  example (*"when you control 10+ creatures, sacrifice…"*) is a **TRIGGERED** ability ⇒ **gate (1)**, and
  the replacement thresholds are **gate (3)**. **An implementer executing the old P5 literally LOSES the
  trigger- and replacement-condition threshold scans — the exact class C3 exists for.**
- **⛔ RC-4 cites a function with no `ObjectId` in it.** `object_content_eq` (`game_state.rs:10453`) takes
  `(&GameObject, &GameObject)`. **The id-keyed seam is `objects_content_eq` (`:10428-34`)**: `b.get(id)`.
  **BONUS — P6 SHRINKS:** that function already asserts **`a.len() == b.len()`**, so **multiplicity is
  already preserved** ⇒ **scalarset normalization need only permute ids.**
- **KEEP** the soundness-asymmetry table (coarse ⇒ **catastrophic false certificate**; fine ⇒ **fail-closed
  false negative**; ⇒ *a coarse relation may REJECT, never ACCEPT*) and the **Murφ scalarset,
  normalization-first** prescription for RC-4. *(The old §5b.3's **proof** was wrong — hashconsing collapses
  *subterms*, not containers; different arity is never congruent — but its **prescription** is right.)*
- **P2 CONFIRMED:** `active_trigger_definitions` (`functioning_abilities.rs:391`) implements **NO CR 113.6
  logic**. `battlefield_active_triggers` (`:416`) is literally `battlefield × active_trigger_definitions`,
  so *"use it"* **IS** the battlefield-hard-coding CR 113.6 forbids. **The predicate must be written.**
- **CR ERROR (#12):** the old §5b cites *"regeneration (CR 701.15)"*. **CR 701.15 is GOAD.** **Regeneration
  is CR 701.19** (`docs/MagicCompRules.txt:3428`).
- **~20 drifted `file:line` citations.** Verified corrections include: `engine.rs:1725 → 1732`;
  `engine.rs:1690 → 1656`; the offer gate is **`engine.rs:450`** (the old doc cites it as **445**, **448**
  *and* **449** — three numbers, none right); `resource.rs:1451 → 1457`; `resource.rs:1577 → 1582`.
  **VERIFY EVERY LINE NUMBER YOU WRITE.**
- **`R1`–`R6` are never mapped to `gate (1)`–`gate (6)`, and R1/R3/R4 are never defined.** **R3 is
  mis-described** as "activated-ability bodies" — measured, it is **gate (2), the all-kinds body scan**.
  **Define the mapping in a table, or drop the R-labels entirely.**

---

## 8. What SURVIVES from the old plan — do not re-litigate, carry it forward

- **The whole RCA narrative (§1–§3)** and **the CR deductions (§4.1–§4.5)**. The rules layer has held
  through five audits.
- **CR 732.2a's own worked Example IS Presence of Gond + Intruder Alarm** (`MagicCompRules.txt:6373`) —
  the rulebook certifies the exact class we cannot detect, and **RC-1 rejects it**. It is the plan's
  primary acceptance fixture.
- **CR 732.2a fixes the player's choices** ⇒ a shortcut is a **fixed, straight-line action sequence**, and
  the test is *"is this fixed sequence legally repeatable forever, with constant Δ?"*
- **CR 732.2a permits a non-repetitive PREFIX + a loop** ⇒ **RC-2**. And *"a sequence of game **choices**"*
  (plural) ⇒ **multi-action loop bodies** (confirmed in 3 corpus drivers; **Combo B is TWO
  `ActivateAbility` actions**, `corpus.rs:1556`).
- **CR 104.4b** (*"loops that contain an optional action don't result in a draw"*) ⇒ the **three-terminal**
  corpus partition (L-OFFER / L-AUTOWIN / WAIVED) — **plus the new GATED cell (§5).**
- **CR 302.6** gives a clean class discriminator, **verified in `card-data.json`**: **Earthcraft**
  (*"Tap an untapped creature you control:"* — cost on the **enchantment's own** ability, **no `{T}`**) ⇒
  sick fodder is legal ⇒ **ACCEPT**; **Cryptolith Rite** (*"Creatures you control have '{T}: …'"* — the
  **creature's own** `{T}`) ⇒ **REJECT**. The engine **can** see the split:
  `AbilityCost::Tap` vs `AbilityCost::TapCreatures` (`ability.rs:7841`), enforced only on the former via
  `cost_contains_tap_or_untap` (`restrictions.rs:675`).
- **CR 106.4 / 500.5** (mana empties each step) ⇒ **counters, not mana**, are the durable ω-axis ⇒
  `loop_states_cover_modulo_counter_growth` (`resource.rs:1329`) **already exists and already covers Pentad
  Prism. Build nothing there.**
- **CR 104.4b optional-loop gate already exists**: `no_living_player_has_meaningful_priority_action`
  (`engine.rs:2367`). **Don't rebuild.**
- **`Appendix B`** — the eleven (now twelve) refuted claims. **Carry it forward and ADD #11 and #12.**

---

## 9. Deliverable

Rewrite `.planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md` as a **clean, single-architecture,
internally-consistent plan**, following `/engine-planner`'s checklist (including the mandatory
**Verification Matrix** with paired positive reach-guards and hostile fixtures, and the **Analogous Trace**).

**Recommended phase shape:**
1. **P0** — the `run_combo_live` **dual** (share `ComboRow`/`ComboBoard`/`step`; assert
   `certifies_offline == (offers_live XOR auto_wins_live)`), **setting `loop_detection = Interactive`**,
   with the **four-cell** partition (L-OFFER / L-AUTOWIN / WAIVED / **GATED**), and **`run_combo` upgraded
   to require two consecutive covering pairs** so the invariant cannot pressure the live path downward.
2. **P1** — ONE generalized arming context + `drive_loop_iteration(&[GameAction])` (**a REWRITE**, not a
   parameterization: `drive_recast_iteration` has **8 cast-shaped elements and 1 parameter**) + **a new
   cheap DoS pre-gate** (commit `57b0e537d` bounds shortcut **EXECUTION**, not **DETECTION**).
3. **P2** — the **CR 113.6 zone-of-function predicate** (new code; the exceptions **113.6b/c/d/e/f/j/k** are
   live, so battlefield-only is wrong).
4. **P3** — RC-2: tolerate the bounded transient (**bound it by the DoS cap, NOT by the population
   heuristic**; decline loudly on overflow).
5. **P4** — **C2** place non-depletion.
6. **P5** — **C3**: fix gate (1)'s **wrong arm** (~10 lines) — **KEEP R6** — and scope C3 across gates
   (1)/(3), **not** gate (4) alone.
7. **P6** — **C5**: deferred execution (CR 603.7) — **the new check.**
8. **P7** — RC-4 object identity: **Murφ scalarset normalization** (errs **fine** ⇒ fail-closed), its own
   PR with its own soundness proof. **Repoint to `objects_content_eq`.**

**Every phase must state its verification, its revert-probe, and its paired positive reach-guard.**
