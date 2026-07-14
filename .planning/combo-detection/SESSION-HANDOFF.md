> # ⛔⛔ STALE — HISTORICAL ONLY. DO NOT ACT ON THIS DOCUMENT.
> **Superseded 2026-07-14 by [`LOOP-SHORTCUT-SPEC-AND-STATE.md`](./LOOP-SHORTCUT-SPEC-AND-STATE.md).**
>
> Every `file:line` below was measured against a tree **768 commits behind `main`** and **no longer resolves
> there.** Several central claims are **refuted by measurement on `main`** — including *"there is no live
> object-growth path"* and *"the offer carries no iteration count"*, **both FALSE on `main` today**
> (`game/engine.rs:1656`, `analysis/decision_template.rs:203`).
>
> **The engine described here no longer exists.** PR-7 shipped most of the machinery; the live blocker is
> REACHABILITY — the fail-closed covers veto on any real board. See the successor doc, §4.
>
> ## ⇒ Read this for the **RULES** reasoning (which has held — 40/40 CR citations) and the **SOUNDNESS** rules.
> ## ⇒ **NEVER for a code fact.**

---

# Combo-detector session handoff — RESUME HERE

**Last updated:** 2026-07-14 · **Branch:** `debug/combo-generator` (fork-only; **NEVER merge toward
`main`** — `.planning/` is gitignored upstream, force-add to commit here).

---

## 0. TL;DR — where we are

Diagnosing why two live infinite combos on the user's real 4-player Commander board are **not detected**
by the CR 732.2a loop-shortcut detector.

> # ✅ **THE PLAN IS FINAL** — phases **P0–P10** · 18-row Appendix B · **+ the MTR 4.4 ADDENDUM (2026-07-14).**
> # ⛔ **THE ADVERSARIAL REVIEW HAS *NOT* HAPPENED.** It is the next action.

> ## 🆕 **THE PLAN WAS RE-OPENED AFTER THE `e677fefb1` FREEZE — use the CURRENT tip, not `e677fefb1`.**
> A **tournament-rules layer** was added as an **ADDENDUM (§A.1–A.7)** in the plan's preface. **The real board is
> 4-player cEDH ⇒ CR 732.1c makes MTR 4.4, not CR 732.2a, the governing text.** The finding that matters:
>
> ### **CR 732.1b and MTR 4.4 both define a loop by the repeatability of the ACTION SEQUENCE. NEITHER EVER REQUIRES THE GAME STATE TO RECUR.** The engine's detector requires **board equality + a provably unbounded axis** — so it **must** reject CR 732.2a's own worked example (Presence of Gond + Intruder Alarm **adds a token every iteration**). **That is not a bad predicate; it is the WRONG predicate.**
>
> ⇒ **This may put P7 — the largest phase and the ONLY unsized open question — on the WRONG SIDE OF THE BUG.**
> **Addendum §A.3 names a cheap experiment that decides it. RUN THAT BEFORE THE REVIEW** (see §1 below).

**Nothing is being implemented.** Plan-only by explicit user instruction. **Zero files modified under
`crates/`** by the planning work.

**The bug, in one line:** *the engine's loop detector **rejects the Comprehensive Rules' own worked example
of the rule it implements*** — CR 732.2a's Example is **Presence of Gond + Intruder Alarm**
(`docs/MagicCompRules.txt:6373`).

| Artifact | Path | State |
|---|---|---|
| **THE PLAN** (the deliverable) | `.planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md` | **FINAL — committed + pushed `e677fefb1`** |
| **Adversary mandate** (the review that still owes to be run) | `.planning/combo-detection/ADVERSARY-MANDATE.md` | **on disk, UNEXECUTED** |
| **Planner brief** (the doctrine behind the plan) | `.planning/combo-detection/PLANNER-BRIEF.md` | committed |
| **Reviewer mandate** (superseded, prior round) | `.planning/combo-detection/REVIEWER-MANDATE.md` | on disk |
| **This handoff** | `.planning/combo-detection/SESSION-HANDOFF.md` | you are here |
| Real-board acceptance tests | `crates/engine/tests/integration/repro_user_combo.rs` | 1 passing guard + **2 `#[ignore]`d, FAILING** (the bug) |
| Real-board fixture (11MB export) | `crates/engine/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json` | committed |
| **The one code fix so far** | `crates/engine/src/game/ability_scan.rs` — `scan_mana_production` | committed (fixes "R2": a basic `Forest` vetoed all detection) |

```bash
cargo test -p engine --test integration real_board_fixture_is_intact   # PASSES (guards the fixture)
cargo test -p engine --test integration -- --ignored real_board        # FAILS  (the bug)
```

---

## 1. IMMEDIATE NEXT ACTION — THE ADVERSARIAL REVIEW (NOT YET RUN)

**The plan is final. The review that is supposed to gate it has NOT been executed.** One attempt was made
and **failed for a mechanical reason worth not repeating: the reviewer was handed the 1,865-line plan +
the `review-engine-plan` skill + CLAUDE.md + eight ranked attack targets AT ONCE, and blew its context.**
It created no worktree, ran no test, and produced no report. **A second attempt (fanning out four narrow
reviewers) was stopped by the user on token budget.**

### ⇒ Run it DECOMPOSED and CHEAP. Do not hand one agent the whole document.

`ADVERSARY-MANDATE.md` holds the full framework (the repo's own `.claude/skills/review-engine-plan/`
gate — 11 required checks) and ranks the targets. **But feed each reviewer ONE target and only the plan
sections it needs** (`grep -n "^### P" <plan>` to find phase headers; never read all 1,865 lines).

### ⇒ ⛔ BUT RUN CHECK 0 FIRST. IT IS CHEAPER THAN THE REVIEW AND CAN INVALIDATE ITS BIGGEST TARGET.

0. **⭐⭐ IS P7 EVEN ON THE CRITICAL PATH? (plan Addendum §A.3 — the falsifiable prediction.)**
   `LoopCertificate.unbounded` carries the invariant *"a non-empty vector is an invariant of a returned
   certificate"* (`analysis/loop_check.rs:123`) and `residual_board_delta` is *"**EMPTY for every certificate this
   phase produces** (both detection paths require an identical battlefield)"* (`:132`). **Witherbloom + Sprout
   Swarm's growth axis IS the battlefield** (tapped tokens). **If board equality gates certification, P7 can be
   PERFECT and the acceptance test STILL FAILS.**
   **⇒ In a throwaway worktree, stub the fire-time firewall to ALWAYS ACCEPT and run the ignored acceptance test.**
   **GREEN** ⇒ P7 is real; proceed to §8 Q0 sizing. **RED** ⇒ **no number of arms fixes it** — the root cause is the
   **certificate's SHAPE**, and **P7 leaves the critical path.** *(§8 Q0's instrumentation ASSUMES the firewall is
   the blocker. This TESTS that assumption.)*

**Then the three highest-value review checks, in order — any one alone is worth more than a full read:**

1. **⭐ C1's revert-probe has NO BACKING FIXTURE** *(nominated by the plan's own author, against its own
   work — start here).* **Both candidate fixtures are dead:** Damping Sphere's deltas **cancel exactly**
   against affinity (`base + k − (C₀ + k) = base − C₀`, constant in k), and Hum of the Radix reads *"each
   **artifact** spell"* — **Sprout Swarm is a green instant**, so it is unsatisfiable. **No replacement card
   has been verified to exist** (§7.1, §8 Q3). **A phase whose revert-probe has no backing fixture can pass
   for the wrong reason.**
2. **⭐ Is the "P4 and P7 are the ONLY ACCEPT-ward phases" claim TRUE?** Classify **all eleven** phases by
   direction independently. **P5 relaxes the cover; P10 coarsens state equality — how are those not
   ACCEPT-ward?** **If even one other phase removes a rejection, the central risk table is WRONG.**
3. **⭐ Is P7's class gate DISCRIMINATING?** In a **throwaway worktree**, flip `ability_scan.rs:2456`
   (`TargetFilter::Typed` `sibling: true → false`) and measure: **Intruder Alarm must un-reject AND Gaea's
   Cradle must STILL fail closed** (`for_each_creature_production_still_fails_closed`, `:4840`). **If the
   flip un-rejects BOTH, it is a HOLE in the catastrophic direction, not a fix.** Then **revert-probe**:
   with the flip reverted the positive guard must **FLIP TO FAIL**, or the gate is vacuous.
   *(This plan already shipped one vacuous discriminator in its own root-cause claim — Appendix B #17.)*

### Known defect to fix in the consolidation pass (do NOT hot-patch the frozen doc mid-review)

**Appendix B numbering:** the plan says *"eighteen"* and carries **18 rows**, but a **19th** error was
caught after the freeze — team-lead's claim that *"no test anywhere asserts `!sibling`"*, which is
**measurably FALSE** (`ability_scan.rs:5215` asserts exactly that, inside `sibling_mutable_axis_discriminates`).
**It earns row #19** — a false claim inside the very section arguing for rigour. The *precise* version is
also **stronger**: the positive guard that DOES exist (`fixed_drain` = `GainLife{Fixed(1), Controller}`,
`:4879`) **references no object filter at all**, so it is **NON-discriminating** — it gave *false comfort*
while the boundary went undefended.

---

## 2. Session constraints (carry these forward)

- **Tilt is OFF** (the user killed it — its churn was excessive). **Running `cargo` directly is sanctioned
  and the reviewer SHOULD run tests.** Prior reviews read code but executed nothing — that is now the
  single biggest evidence gap.
- **`.planning/` is gitignored** — planning docs live on this fork branch via `git add -f`. **They must
  never reach `main` or an upstream PR.**
- **Plan-only.** The user is conserving tokens; do not start implementing.
- Never revert other agents' work; never `git stash`.

---

## 3. The bug, in one paragraph

The engine **arms correctly** (`last_recast_context` is captured with the right card/zone/buyback/convoke)
and then **declines silently**. There are **four independent root causes**, and **no single one is
sufficient to fix**:

| | Root cause | Where |
|---|---|---|
| **RC-1** | The fire-time observer predicate is **wrong** — it rejects on *"references any typed object filter"*, which every Commander permanent does — **and** it scans **hidden zones** (a `Solemn Simulacrum` **in the library** vetoes detection; illegal under **CR 113.6** *and* **CR 400.2**). | `resource.rs:1451` |
| **RC-2** | The cover **forbids a bounded start-up transient** — it demands recurrence from iteration 0. **CR 732.2a explicitly permits** a non-repetitive prefix followed by a loop. **VERIFIED TRUE — a reviewer attacked it directly and could not break it.** | `engine.rs:1725` |
| **RC-3** | The live path **arms on ONE bespoke card shape**, and **zero of 53 corpus rows test the live path at all** (`grep -c "WaitingFor::LoopShortcut" corpus.rs` == **0**). | `casting_costs.rs:6785` |
| **RC-4** | Loop equality is **keyed on `ObjectId`**, which **CR 400.7** makes rules-wrong (an object that changes zones **is a new object**). | `game_state.rs:10456` |

**CI is green because the acceptance fixture builds a board that cannot exist in a real game** (no lands,
empty library, no auras, stub oracle) — `sprout_swarm_scenario`, `loop_shortcut.rs:2536`.

---

## 4. ⚠️ THE THING THAT KEEPS BITING US — read before asserting anything

**This plan has been wrong NINETEEN times.** 18 are catalogued in the plan's **Appendix B**; **#19 is
recorded in §1 above and still owes a row.**

> ## **Every single failure was a CODE claim asserted from memory. The RULES layer has NEVER failed — 40/40 CR citations, 32/32 Oracle texts, across six audits.**

**Three of the nineteen were committed *while writing the document*, by two different agents, and every one
was caught the same way: SOMEONE RE-MEASURED.** Grep before you assert, and **put the `file:line` in the
sentence.** The worst ones:
- *"Combo B is ONE activation"* — **FALSE.** `drive_offline_kilo_freed_relic` (`corpus.rs:1556`) takes
  **TWO** `ActivateAbility` actions. Killed an entire phase.
- *"Freed from the Real is an aura, and auras carry modifications"* — **FALSE.** Measured:
  `static_definitions: null`. **Two agents independently invented the same false mechanism from memory and
  a third ratified it.**
- *"Hum of the Radix DECLINES"* — **UNSATISFIABLE.** *"Each **artifact** spell"*; Sprout Swarm is a green
  instant. *(Damping Sphere is dead too: its deltas **cancel exactly** with affinity.)*
- *"Arm (a) is NECESSARY AND SUFFICIENT"* (#17) — **FALSE.** The probe that "proved" it ran on a **clean
  two-card board and never on the real one**. ***A vacuous discriminator, inside the plan's own root-cause
  claim.***
- *"No test anywhere asserts `!sibling`"* (#19) — **FALSE.** `ability_scan.rs:5215` asserts exactly that.

---

## 5. Document state — the palimpsest is GONE (do not go looking for it)

**The plan was FULLY REWRITTEN.** Earlier handoffs told the reviewer to hunt a *"deliberately planted
calibration contradiction at §4.10."* **⛔ THAT INSTRUCTION IS DEAD: §4.10 DOES NOT EXIST.** §§4.7–4.10 were
deleted at `7469f7904`; the "planted contradiction" was **itself an assertion from memory of a superseded
revision** — it is **Appendix B #11**. **Ignore any instruction that references it.**

Likewise **dead**: the old `REVIEWER-MANDATE.md` §3 seam list (turn-crossing §4.7-vs-§4.10, the `Quotient`
enum, the stale exec-summary surface box). Those targeted the pre-rewrite document. **`REVIEWER-MANDATE.md`
is SUPERSEDED — use `ADVERSARY-MANDATE.md`.**

**What IS current:** the plan @ `e677fefb1` — **§0 spine, phases P0–P10, 18-row Appendix B**, one
architecture, no struck-through claims.

---

## 6. The technical questions — RESOLVED, and the one that ISN'T

**All three of the old open questions were settled by measurement. Do not re-litigate them:**

1. **`egg` / e-graphs → REJECTED.** A **~10-line fix meets egg's own acceptance criterion**, `Axes` **IS**
   already a join, and **with zero rewrite rules `merge` is never called** — so egg-minus-rewrites is a
   memoized catamorphism that `ability_scan.rs` **already is**. *(And egg is outright **unsound for STATE
   equality**: congruence **collapses multiplicity**, and multiplicity **IS** the growth axis ⇒ it would
   certify N ≡ N+1 **exactly when the tokens grew**.)* For RC-4 the literature match is **Murφ scalarset
   symmetry reduction** — **normalization first** (errs fine ⇒ fail-closed), canonicalization as the exact
   upgrade.
2. **Deleting R6 → UNSOUND *AND* WORTHLESS. It is a TRAP.** `PartialEq` (`game_state.rs:10875`) **already
   compares `delayed_triggers`**, so **Kiki is already rejected and deleting R6 buys ZERO rows.** The trap:
   an implementer deletes R6, sees Kiki *still* rejected, relaxes the `delayed_triggers` conjunct to chase
   the promised rows, and **certifies a loop whose entire growth axis dies at the next end step.**
   ⇒ The real answer is **C5 (deferred execution, CR 603.7)**: **7 of 9 `DelayedTriggerCondition` variants
   are EVENT-keyed, not phase-keyed** (`ability.rs:2919`) ⇒ C5 has genuine **ADMIT** value.
3. **`LoopProbe` / the `run_combo_live` dual → RESOLVED, buildable.** It is **P2**, and it is **Tier 1 and
   non-negotiable** — *the only instrument that can tell a CLASS fix from a CARD fix.*

### ⚠️ THE ONE GENUINELY OPEN QUESTION — filed **UNVERIFIED**, not guessed (plan §8 Q0)

> **P7's TRUE SIZE.** *"~88 sites"* (**57 `Axes::CONSERVATIVE` + 31 `sibling: true`** in `ability_scan.rs`)
> is **the audit's INPUT, not its COST.** **Most `CONSERVATIVE` sites may well be correct and stay.** The old
> *"~10 lines"* sizing was **measured FALSE** (#17), but **nobody has measured the new one** — it could be 30
> arms or 300.
>
> **To pin it:** instrument in a throwaway worktree **until the canary actually goes green**, then **count
> the arms that had to change** — **and re-verify the class gate in the same breath** (Intruder Alarm
> un-rejects **AND** Gaea's Cradle still fails closed). **A green canary with a broken Cradle is a false
> certificate, not a win.** That run would also produce **the first green
> `real_board_sprout_swarm_offers_loop_shortcut` in this workstream's history.**

---

## 7. The governing insight (the user's, and it is the spine of the plan)

> **The player proposes a FIXED loop** (CR 732.2a: *"a sequence of game choices… it can't include
> conditional actions"*), **impactable only by what is CURRENTLY on the board** — and **we DRIVE that fixed
> sequence on a clone through the real reducer.**

⇒ **Every board ability that fires during the loop ALREADY FIRES IN THE DRIVE and ALREADY LANDS IN Δ.**
A firewall that re-derives them statically isn't conservatism — it's duplicated work that **gets the answer
wrong** (it rejects the rulebook's own worked example).

⇒ **So ask only: what can the current board do that the DRIVE cannot see?** Exhaustively two:
1. **monotone depletion outside the drive window** → **C2** (the only genuinely new logic)
2. **a discontinuity — a threshold tripping at a future iteration count** → **C3** (a *condition* scan,
   which **already exists** at gate (4))

Everything else is **measured**: an effect that **scales** moves Δ (**C1**); an effect that merely **reads**
the growing axis yields constant Δ and is **HARMLESS**.

⇒ **C3 collapses from a rewrite into a DELETION.** *(Whose soundness is open question #2 above.)*

**Other load-bearing rules deductions (all verified against `docs/MagicCompRules.txt`):** CR 732.2a permits
a **non-repetitive prefix + loop** (⇒ RC-2) and a **sequence of choices, plural** (⇒ multi-action loop
bodies, confirmed in 3 corpus drivers); **CR 104.4b** *"loops that contain an optional action don't result
in a draw"* ⇒ the 3-terminal corpus partition (L-OFFER / L-AUTOWIN / WAIVED); **CR 302.6** gives a clean
class discriminator (**Earthcraft** — cost on the enchantment, no `{T}` ⇒ sick fodder legal ⇒ **ACCEPT**;
**Cryptolith Rite** — the creature's own `{T}` ⇒ **REJECT**); **CR 106.4/500.5** (mana empties each step)
⇒ **counters, not mana**, are the durable ω-axis.

---

## 8. Agent / messaging failure modes hit this session (don't repeat them)

- **Reviewer reports did not reach team-lead — twice.** One reviewer later confirmed its report *had* been
  "delivered successfully." **A send receipt is NOT evidence of receipt.** → Use a **file relay** for
  mandates, demand a **challenge-response ACK**, and require the report in the **final response message**.
- **Reviewers spawned without a tmux pane idle-ping instead of delivering.** → Spawn drivers/reviewers as
  **tmux teammates** (named + `run_in_background`, **no `isolation`**).
- **I rewrote the review target three times mid-flight**, which stalled the first reviewer. → **Freeze the
  document before spawning a reviewer.**
- Shut idle agents down with a `shutdown_request` (TaskStop does not work on them).
