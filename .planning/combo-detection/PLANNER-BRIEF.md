# Planner brief — rewrite the combo-detector plan from measured facts

> # ⭐⭐ THE GOVERNING DESIGN RULE — USER DIRECTIVE. It outranks everything else in this brief.
>
> > *"**Build to the pattern, not to the use cases presented in the given board state.** Make sure that's
> > forefront in the design."*
>
> ## **Combo A and Combo B are ACCEPTANCE TESTS, not GOALS. Every phase fixes a CLASS. A change that turns a combo green without discharging a class property IS the purpose-built patch this plan exists to prevent.**
>
> **The team-lead violated this while framing the tiers** — writing *"Tier 1 = makes the user's Combo A
> fire."* **That sentence must not enter the document.** An implementer reading it lands exactly enough to
> turn one test green and stops — **shipping the very disease RC-3 diagnoses** (a detector that arms on ONE
> bespoke shape, with a corpus that never tests the live path).
>
> ### Every phase exists for its CLASS. The combo is merely one board that trips it.
> | Phase | The CLASS it discharges — **this is why it exists** |
> |---|---|
> | **P4** CR 113.6 zone-of-function predicate | **every** hidden-zone read — the whole rules violation, not one Solemn Simulacrum |
> | **P5** bounded start-up transient | **every** loop with a non-repetitive prefix — a shape **CR 732.2a explicitly permits** |
> | **P7(a)** the wrong `sibling` arm | **every** typed-filter false rejection — Intruder Alarm (**CR 732.2a's OWN worked example**), Freed, Suture Priest, *every Commander permanent* |
>
> ### ⛔ ACCEPTANCE CRITERIA ARE CLASS-LEVEL, NEVER CARD-LEVEL. This is the hard part.
> ***"`real_board_sprout_swarm_offers_loop_shortcut` goes GREEN" is NOT an acceptance criterion. It is a
> CANARY.*** A phase that only makes that test pass **has not discharged its class**.
> - **P4** ⇒ the verdict is invariant under **ANY** hidden-zone content — not *"we deleted Solemn Simulacrum and it worked."*
> - **P7(a)** ⇒ **Intruder Alarm un-rejects AND Gaea's Cradle still fails closed.** **That pair is the class proof. The combo is not.**
> - **P5** ⇒ the verdict is invariant under **WHICH** creature the real cast convokes — the transient, not the card.
>
> **If you find yourself doing anything to turn a combo green that is not derived from a class property —
> STOP. That is the purpose-built patch.**
>
> ### Tiers are SEQUENCING, not permission to ship less.
> They order **which classes unblock which others**. **No phase may ever ship with a card-level gate.**
> And **P2 (the corpus dual) is the ONLY thing that can tell us whether we fixed a CLASS or a CARD** —
> without it, *"we built to the pattern"* is an **unverified assertion**, which is Appendix B's failure mode
> one level up.


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

### 2.0 The GOVERNING CONSTRAINT, correctly scoped (user directive — this is the design's foundation)

> **The player presents the loop FIXED. It is responded to by other players afterward. Only the ACTIVE
> PLAYER'S CONTEXT matters.**

**This is true, it is load-bearing, and it simplifies enormously. Honour it. But scope it exactly:**

| ✅ What it GIVES you | ❌ What it does NOT give you |
|---|---|
| **No search.** CR 732.2a forbids conditional actions ⇒ the choice vector is **pinned**. No branching, no proposer-search. | ❌ *"Therefore everything the loop does lands in the driven Δ."* **This was the old spine and it is FALSE.** |
| **No opponent modelling.** CR 732.2b: other players **accept or shorten** in the response window. Their hands, libraries and responses are **not the cover's problem.** | |
| **Current board only.** No hidden zones (CR 400.2), no hypothetical boards. | |

**The error was TEMPORAL, not informational.** A fixed sequence, driven from the current board, in the
active player's context, **can still SCHEDULE an effect that executes at a PHASE boundary** — and
**CR 732.2a says the loop's ending point is a *priority* beat**, so **the loop never advances the phase**
and never executes what it scheduled.

> ⇒ **The drive measures what RESOLVES inside the window. It is structurally blind to what the window
> SCHEDULES.** **Every deletion in the old P5 derived from the false spine and must be re-derived.**

### 2.0.1 ⭐ The refinement this unlocks — C5 bounds the ω-axis's LIFETIME; it does NOT blanket-reject

**Kiki-Jiki + Zealous Conscripts IS a legal CR 732.2a shortcut.** You genuinely *can* make a million
tokens; the shortcut is real and offerable. What is *not* true is that the tokens **persist** — each is
sacrificed at the next end step. **But Kiki's tokens have haste, so the proposer swings for lethal
BEFORE that end step.**

⇒ **A scheduled-outside-the-window effect does not invalidate the LOOP. It bounds the LIFETIME of the
ω-axis, and therefore what the certificate may CLAIM.** The engine **already has the vocabulary**:
`WinKind` (`loop_check.rs:83`) distinguishes `LethalDamage` / `PoisonLoss` / `Decking` / `Advantage` /
`ExtraTurns`. **Kiki may certify `LethalDamage` (swing this turn). It must NOT certify `Advantage` (the
tokens evaporate).**

**Ship C5 in two stages, and SAY SO in the plan:**
- **C5 v1 (build now) — FAIL CLOSED.** Reject when a scheduled effect destroys the ω-axis outside the
  window. **This is precisely what R6 does today ⇒ KEEP R6 until C5 subsumes it.**
- **C5 v2 (NAME it, do not build it) — the ω-axis lifetime refinement.** Classify the axis's lifetime and
  let a short-lived axis certify `LethalDamage` while forbidding `Advantage`. **Kiki is then reachable.**

> ⚠️ **The v2 note is not optional — it is the trap's antidote.** Without it, an implementer who sees
> Kiki rejected will "fix" it by relaxing the `delayed_triggers` comparison (§4) and ship a **false
> certificate**. Tell them the honest route: **it is deferred behind a named refinement, not
> permanently out of reach.**

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

## 4b. ⭐ USER DIRECTIVE — **DELETE `LoopDetectionMode::On`. Collapse to a binary. This is a NEW PHASE.**

> *"`On` is a relatively useless state for the combo detector now — not useful for users, only confusing.
> Assume everything is 'interactive'. Record players' states and trap them into the detector."*

**The code agrees, in its own words.** `interactive_loop_bridge`'s Path A comment (`engine.rs:~499`):
> *"FIRM #1 — mandatory winning drain: **identical to the `On` auto-win**."*

⇒ **`Interactive` STRICTLY SUBSUMES `On`**: same auto-win + `mark_unbounded_loop` when the loop is
mandatory, **plus** the CR 732.2a offer when it is optional. **`On` adds nothing** — and it is precisely
what makes P0's L-AUTOWIN assertions vacuous (§5). **Deleting it fixes the vacuity at the ROOT instead of
working around it.**

**Target shape — a true binary:**
```rust
pub enum LoopDetectionMode { #[default] Off, On }   // `On` CARRIES TODAY'S `Interactive` SEMANTICS
```
*(Name it `On`, not `Interactive` — "interactive" is jargon on a user-facing toggle, and it touches fewer
frontend strings. Justify whichever you pick.)*

**`Off` STAYS — non-negotiable.** It is the repo's opt-in policy (#4603: game-changing features ship
behind a user-controllable toggle whose OFF state restores pre-feature behavior).

### ⚠️ CORRECTION — Appendix B **#13**, and it was the TEAM-LEAD's. `combo-plan-author` caught it.
**An earlier draft of this §4b claimed *"`On` is TODAY'S DEFAULT FOR REAL MATCHES (`match_config.rs:89`,
`session.rs:1613`)"*. That is FALSE.** Both citations are **`#[cfg(test)]` fixtures**
(`match_config.rs:60-61` is `#[cfg(test)] mod tests`; `session.rs:1601` is
`fn loop_detection_config_persists_across_bo3_rebuild()`). The claim came from grepping
`LoopDetectionMode::On`, reading the line numbers, and **inferring** production-default **without reading
the surrounding context.** *Textbook Appendix-B: a code claim from an under-verified grep.*

**THE SHIPPED DEFAULT IS `Off`.** `match_config.rs:27`, verbatim: *"Default `Off` = exact pre-detector
behavior (opt-in invariant, issue #4603)."* Enforced at the wire layer by
`#[serde(default, skip_serializing_if = "LoopDetectionMode::is_off")]` (`:36`).

**THE DIRECTIVE IS UNAFFECTED — only my rationale was wrong, and the true one is better:**
`Off` is the shipped default and **must stay** (#4603). But when a user **opts in** today they face a
confusing **three-way** choice: `Off` / **`On` (auto-win only — NO OFFERS: a crippled half-detector)** /
`Interactive` (auto-win **+** offers). **`On` is strictly dominated by `Interactive` and adds nothing.**
⇒ **"Trap them into the detector" = once you opt in, you get the FULL detector, not a half one.** The
toggle becomes an honest binary: **`Off` (pre-feature) / `On` (the full detector).**

### Blast radius — MEASURED (re-measured after the correction above).
- **18 `LoopDetectionMode::On` sites** across `crates/` (**not 16** — the earlier count omitted the two in
  the definition file itself): `engine.rs:357` (**the dispatch arm to delete**) ·
  `match_flow.rs:669,672,744,747` · `corpus.rs:1871` · `triggers.rs:23170,23251,23434` ·
  `corpus_tests.rs:1404,1437` · `match_config.rs:89,97` *(tests)* · `server-core/src/session.rs:1613`
  *(test)* · `tests/integration/loop_shortcut.rs:338` · `tests/integration/pr65_growing_cascade_win.rs:111`
  · plus the definition file. **VERIFY THE LIST YOURSELF.**
- **TWO predicates, not one:** `samples()` (`types/game_state.rs:5819`) **and `is_on()` (`:5804`)**.
  **`is_on()` has ZERO production callers** — all six sites are tests (`corpus_tests.rs:1244,1348,1408`;
  `server-core/src/session.rs:1625,1636`). **That SIMPLIFIES the collapse.** `samples()` reduces to
  `!matches!(self, Off)`; consider whether `is_on()` and `is_off()` survive at all.
- **Frontend:** `client/src/game/loopDetectionMode.ts` (the `?loop=` query parser/serializer) and
  `client/src/components/lobby/HostSetup.tsx:226` (the lobby toggle). **Collapse the UI to a binary.**
  The offer surface already exists: `client/src/components/modal/LoopShortcutModal.tsx`.
- **`WinKind` has SIX variants, not five** (`analysis/loop_check.rs:83-107`): `LethalDamage` ·
  `PoisonLoss` · `Decking` · **`ImmediateWin` (`:98`, CR 104.2)** · `ExtraTurns` · `Advantage`.
  **C5 v2's lifetime→claim mapping must classify all six.**
- **⚠️ SERDE MIGRATION HAZARD.** `LoopDetectionMode` is `Serialize/Deserialize` with
  `#[serde(tag = "type")]` (`game_state.rs:5783-84`). **Persisted states and debug exports carry
  `{"type":"On"}` and will FAIL to deserialize** once the variant is gone. **Add
  `#[serde(alias = "Interactive")]` (or `alias = "On"`, per your naming choice) so BOTH old spellings
  still load** — and add a round-trip test proving an old export still deserializes. *(The user's own
  fixture is `Interactive`, so the repro survives either way — but a saved `On` game would not.)*
- **`samples()`** (`game_state.rs:5818`) collapses to `!matches!(self, Off)`.

**Sequence this EARLY** — P0's dual and the whole corpus-live partition depend on it, and it deletes a
footgun rather than documenting one.

## 4c. ⭐ USER DIRECTIVE — **SPLIT `WinKind`. It conflates WINS with non-winning ADVANTAGES.** (NEW PHASE)

> *"There's a semantic problem in `WinKind` — `WinKind`s can be non-winning advantages in the current
> setup. It's probably a very useful distinction to separate these two classes."*

**Correct, and it is not a naming smell — the engine already hand-simulates the missing type, and SAYS SO.**

### The evidence (all measured)
1. **`Advantage`'s own doc comment states it is NOT a win** (`loop_check.rs:~104`): *"…without, by itself,
   being a direct loss condition for an opponent… **the payoff that converts the advantage to a win is a
   separate card.**"* **An enum named `WinKind` contains a variant documented as not-a-win.**
2. **`ExtraTurns` is ALSO misclassified.** It cites **CR 500.7** — which is **purely mechanical**
   (*"Some effects can give a player extra turns… added directly after the specified turn"*,
   `MagicCompRules.txt:2127`) and **says NOTHING about winning.** The real win conditions are **CR 104.2**.
   **Infinite turns is a CAPABILITY, not a game-end.**
3. **The engine already partitions it by hand — and admits why.** `engine.rs:637`:
   `classify_win_kind(controller, &delta) == WinKind::Advantage` — an **equality check against a single
   variant** to gate the non-terminal path. And the comment immediately above it (`~:630`):
   > *"…which **NEVER produces a GameOver**; an over-claim is a **revocable capability, not a wrongful
   > game-end**."*

   **That comment is describing the type distinction that does not exist.** Confirmed:
   `mark_unbounded_loop` (`game_state.rs:10377`) only **extends a set** — it can never end a game.

### ⇒ The split is a SOUNDNESS BOUNDARY, not a taxonomy
It is **the §5b.1 asymmetry, in the type system**:

| Outcome | Over-claim consequence |
|---|---|
| **`Win`** — terminal; the loop **ENDS THE GAME** | ⛔ **WRONGFUL GAME-END. CATASTROPHIC.** Fail closed. |
| **`Advantage`** — non-terminal; unbounded resource, **no game-end by itself** | ✅ **A revocable capability mark. SAFE.** |

```rust
/// What an infinite loop lets its controller ACHIEVE. The split is a SOUNDNESS boundary.
pub enum LoopOutcome {
    Win(WinKind),              // TERMINAL — ends the game
    Advantage(AdvantageKind),  // NON-TERMINAL — no game-end by itself
}
pub enum WinKind {        // terminal ONLY
    LethalDamage,  // CR 704.5a
    PoisonLoss,    // CR 704.5c
    Decking,       // CR 104.3c
    ImmediateWin,  // CR 104.2b — "an effect may state that a player wins the game"
}
pub enum AdvantageKind {  // NON-terminal
    Resource,    // CR 732.2a's canonical beneficial loop (mana/tokens/cards/counters/triggers)
    ExtraTurns,  // CR 500.7 — mechanical; NOT a CR win condition
}
```

### ⭐ Why this is REQUIRED, not merely tidy: **it is the type C5 v2 needs (§2.0.1)**
Kiki-Jiki's tokens are destroyed at the next end step, so:
- ✅ they **CAN** certify **`Win(LethalDamage)`** — the tokens have **haste**; swing **before** the end step;
- ❌ they **CANNOT** certify **`Advantage(Resource)`** — they **evaporate**, so there is no durable resource.

⇒ **A short-lived ω-axis can support a terminal `Win` inside the window but cannot support a durable
`Advantage`.** Today that is a *comment*. With the split it is a **compiler-enforced invariant.**
**C5 v2's lifetime→claim mapping is only expressible once this refactor lands. Sequence it BEFORE C5.**

### Scope (verify it yourself)
- `classify_win_kind` (`analysis/loop_check.rs`) → returns `LoopOutcome`.
- `engine.rs:637`'s `== WinKind::Advantage` equality gate → `matches!(.., LoopOutcome::Advantage(_))`
  (or becomes unnecessary — **prefer making the type carry the invariant**).
- **`shortcut_iteration_count` (`engine.rs:731-741`) is an ORTHOGONAL axis — do NOT conflate it.** It maps
  `LethalDamage | PoisonLoss => UntilLethal`, everything else ⇒ `Fixed(1)`. That is *"is the win reached
  asymptotically or in one cycle"*, **not** terminal-vs-non-terminal. Keep both axes; they are independent.
  `iteration_count_maps_every_win_kind` (`engine.rs:9216`) must stay exhaustive.
- **Serde/wire:** `WinKind` crosses the serialization boundary into `LoopCertificate` /
  `ShortcutProposal` (externally tagged, unit variants as bare strings) and reaches the frontend
  (`LoopShortcutModal.tsx`). **A nested enum changes the wire shape ⇒ update the TS types and add a
  round-trip test.**
- Run the **`/add-engine-variant`** checklist; grep `data/engine-inventory.json`.
- `corpus.rs`'s `ComboRow.win_kind` and `corpus_tests.rs:81` must be re-typed.

## 4d. ⭐ USER DIRECTIVE — **INFINITES ON TOP OF INFINITES. The ∞ marks are WRITE-ONLY.** (NEW PHASE)

> *"There is the possibility for infinites-on-top-of-infinites, which reduces to linear programming (you
> just apply infinite statuses to things and resolve the stack) for most cases."*

**The gap is real and MEASURED. The engine certifies an unbounded axis, paints an ∞ badge — and then
FORGETS IT when evaluating the next loop.**

### Measured: nothing in the detector consumes `unbounded_resources`
- **Writer:** `mark_unbounded_loop` (`types/game_state.rs:10377`) — only extends a set.
- **Readers — ALL of them:**
  - `game/derived_views.rs:498` — **HUD display only** (the `∞` rows).
  - `game/mana_payment.rs:76` — ***"Debug-only:*** top every player whose `unbounded_resources` contains
    any `ResourceAxis::Mana(_)` back up to `INFINITE_MANA_PER_TYPE`."
  - `game/turns.rs:348` — ***"Debug-only:*** CR 500.5 end-of-step empty is suppressed for a player with
    the **infinite-mana toggle** active." *(the partner of the above)*
- **`grep -rn unbounded_resources crates/engine/src/analysis/ crates/engine/src/game/engine.rs` ⇒ NOTHING
  but tests.** ⇒ **The ∞ marks are WRITE-ONLY for detection.** Both functional consumers are wired to the
  **debug `SetInfiniteMana` toggle**, not to real play.

⇒ **A second loop that SPENDS a resource the engine has ALREADY PROVEN INFINITE is rejected by C2 /
`net_progress_for` for "depleting" it.** **This is on the user's own board:** Kilo + Freed + Relic +
Pentad Prism ⇒ unbounded **counters** ⇒ unbounded **mana**; Witherbloom + Sprout Swarm ⇒ unbounded
**creatures**. **Two loops. Zero composition.**

### The shape: a MONOTONE FIXPOINT — **NOT** linear programming (and that is *better* news)
With **∞ statuses**, quantities **collapse to booleans**: you never need to solve for *how much* (which is
what would make it an LP). You need **reachability** — *given the current ∞ set, which further axes become
unbounded?*

- **Monotone:** adding an ∞ axis can only **enable** more loops, never fewer.
- **`ResourceAxis` is a FINITE enum** ⇒ the fixpoint **terminates in ≤ |ResourceAxis| rounds.**
- ⇒ **A least-fixed-point closure (Datalog-shaped). Decidable, terminating, trivially cheap.**
  **No LP solver. No dependency. ~20 lines.**

```text
∞ := {}                                    // per player
repeat until fixpoint (≤ |ResourceAxis| rounds):
    for each candidate loop L on the present board:
        if L certifies with the axes in ∞ treated as NON-DEPLETING:
            ∞ ∪= L.produced_axes
```

### The code change is ONE DISJUNCT
- **C2** (place non-depletion) becomes: *"every place the sequence draws from is **non-decreasing OR
  already marked unbounded for this player**."*
- **`net_progress_for`**'s *"no net-negative mana"* similarly **exempts axes already in ∞.**
- **Feed the fixpoint from `state.unbounded_resources`** — the store already exists; it just has no reader.

### ⚠️ Soundness constraints — do NOT get these wrong
1. **∞ is PER-PLAYER.** `unbounded_resources: HashMap<PlayerId, Set<ResourceAxis>>`. **An opponent's ∞
   mana does not make YOUR loop sustainable.** Key every exemption on the **proposer**.
2. **The fixpoint must be seeded ONLY from CERTIFIED loops.** An unsound mark poisons everything
   downstream — this is a **monotone amplifier for false certificates.** ⇒ **it composes with §4c: only a
   certified `Advantage(_)` (revocable, safe) may seed ∞; never a speculative one.**
3. **CR 106.4 / CR 500.5: mana empties at end of step.** "Unbounded mana" is only usable **within the
   step**. The **debug** consumers above cheat this deliberately (`UnitDisposition::Keep`). **A real
   composition must either stay inside the step or use a DURABLE axis.** *(This is exactly why counters,
   not mana, are the durable ω-axis — and it is why Pentad Prism's charge counters, not the mana they
   make, are what `loop_states_cover_modulo_counter_growth` certifies.)*
4. **Termination is guaranteed by the finite axis set — state it, and add a round cap as a backstop.**

**Sequence AFTER the four checks exist** (it composes them; it does not replace them). **And note it is
the third time a Datalog/monotone-fixpoint shape has appeared in this workstream — this time it is the
right layer, and it still needs no library.**

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

**⇒ P0 MUST set `loop_detection = Interactive` on every live board** — **`Interactive`, NOT `On`.**
**Verified:** `engine.rs:431` dispatches `LoopDetectionMode::Interactive => interactive_loop_bridge(..)`,
so under `On` **the offer path never runs** ⇒ P0's L-AUTOWIN *"must NOT offer"* assertions would pass
**vacuously — mode-gated off, not rules-gated off.** Also add a **GATED** partition cell (the 4
`gated_on` rows), and resolve the shared-`step`/hook collision (once arming generalizes, the hook flips
`waiting_for` **mid-sequence**).

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
