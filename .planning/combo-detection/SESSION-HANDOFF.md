# Combo-detector session handoff — RESUME HERE

**Last updated:** 2026-07-14 · **Branch:** `debug/combo-generator` (fork-only; **NEVER merge toward
`main`** — `.planning/` is gitignored upstream, force-add to commit here).

---

## 0. TL;DR — where we are

Diagnosing why two live infinite combos on the user's real 4-player Commander board are **not detected**
by the CR 732.2a loop-shortcut detector.

> # ✅ **THE PLAN IS FINAL — `e677fefb1`.** 1,865 lines · phases **P0–P10** · 18-row Appendix B.
> # ⛔ **THE ADVERSARIAL REVIEW HAS *NOT* HAPPENED.** It is the next action.

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

**The three highest-value checks, in order — any one of them alone is worth more than a full read:**

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

**This plan has been wrong TEN times.** All ten are catalogued in the plan's **Appendix B**.

> ## **Every single failure was a CODE claim asserted from memory. The rules work has held up under four reviews.**

**Grep before you assert, and put the `file:line` in the sentence.** The worst ones:
- *"Combo B is ONE activation"* — **FALSE.** `drive_offline_kilo_freed_relic` (`corpus.rs:1556`) takes
  **TWO** `ActivateAbility` actions. Killed an entire phase.
- *"Generalizing `normalize_recast_frame` lifts all 13 `ObjectReentry` rows"* — **FALSE.** It lifts **zero**.
- *"C3 is the arm no review broke"* — **FALSE.** Its predicate rejects **CR 732.2a's own worked example**.
- *"Hum of the Radix DECLINES"* — **UNSATISFIABLE.** *"Each **artifact** spell"*; Sprout Swarm is a green
  instant. The card is **Damping Sphere**.

---

## 5. ⭐ KNOWN-UNFIXED CONTRADICTION — deliberately left in the plan as reviewer calibration

**§4.10 and §5 directly contradict each other, and I left it that way on purpose.**

- **§4.10** (written earlier) claims `Quotient::ObjectIdentity` unlocks **all 13** `ObjectReentry` rows and
  is *"worth more than Phases 1–5 combined"* (payoff table: *"~17 of 37 deferrals from ONE
  parameterization"*).
- **§5** (written afterwards, from the round-4 review) says it lifts **ZERO** directly. Appendix B #7
  records the refutation.

**If the reviewer does not find this independently, its coherence audit is not trustworthy on the ones I
have NOT spotted.** Fix it in the consolidation pass regardless.

**The plan has been revised SIX times in place and nobody has read it whole since. Other seams to
reconcile** (all listed in `REVIEWER-MANDATE.md` §3): turn-crossing in-or-out (§4.7 vs §4.10 vs P0's WAIVED
partition); what the actual new surface is (exec-summary box vs §4.6 vs P5 vs §5b); whether `Quotient` or
scalarset canonicalization is the plan of record for P6; whether §3.1's catch-22 still argues from the
affinity evidence that Appendix B #8 records as **measured-false**; dangling phase numbers; **drifted line
numbers** (already caught: `no_living_player_has_meaningful_priority_action` = `engine.rs:2367` not 1765;
`ability_has_per_turn_activation_gate` = `resource.rs:2848` not 2842;
`fire_time_conditions_read_growing_class` = `resource.rs:1451` not 1468); corpus is **53** rows, not 55.

---

## 6. The three open technical questions the review must answer

1. **§5b VERDICT — adopt `egg`, or not?** The claim: `ability_scan.rs`'s `Axes` walk **already is** an
   abstract interpretation over the ability AST, it is hand-rolled and **measurably wrong**, and expressing
   it as an **`egg::Analysis`** would fix RC-1 **at the root** and collapse P2 + P5 from new subsystems into
   **queries**.
   - **Highest-value check: is `Axes` actually a *join*** (assoc/comm/idempotent)? If not, it cannot be an
     `egg::Analysis` and the option collapses.
   - **The category-error check:** an `egg::Analysis` is **bottom-up and context-free per e-class**, but the
     correct predicate depends on **which growth axis** — a property of the **LOOP**, not the AST. If the
     predicate is context-**sensitive**, e-class analysis is **structurally the wrong tool**.
   - **RULES HAZARD (primary consideration):** equality **saturation** rewrites terms per your rules, and
     **every rewrite rule is a CR claim** (destroy ≠ sacrifice ≠ put-into-graveyard under CR 701.15 /
     702.12 / 614). **Spike is scoped to congruence + `Analysis` with ZERO semantic rewrite rules.**
   - **`egg` is NOT a requirement — the user has pre-authorized "disregard it."**
   - **`egg` is NOT unsound for AST analysis, but IS unsound for STATE equality**: congruence **collapses
     multiplicity** (3 vs 4 identical Saprolings = same term = same e-class), and **multiplicity IS the
     growth axis** ⇒ accepting on congruence certifies iteration N ≡ N+1 **exactly when the tokens grew**.
     For RC-4 the literature match is **Murφ scalarset symmetry reduction** (`ObjectId` **is** a scalarset):
     **normalization first** (errs too fine ⇒ misses loops ⇒ **fail-closed**), **canonicalization** (exact;
     nauty-class, effectively free at our board sizes) as the upgrade. Rust: `graph-canon`, `nauty-pet`,
     `canonical-form`.

2. **⚠️ IS DELETING R6 SOUND? — THE MOST DANGEROUS LINE IN THE PLAN.** R6 rejects on any non-empty
   `state.delayed_triggers`. The plan claims a Kiki-Jiki token's *"sacrifice it at the beginning of the
   next end step"* delayed trigger **fires in the drive** and lands in Δ. **The drive settles at an
   empty-stack `Priority` beat — but that trigger fires at the beginning of the next END STEP, plausibly
   OUTSIDE the drive window entirely. If it never fires in the drive, deleting R6 certifies a loop whose
   tokens all die — a FALSE CERTIFICATE. MEASURE IT. RUN A TEST.**

3. **Is `LoopProbe` drivable through the real `apply()` reducer**, or is it offline-only? **If offline-only,
   the entire test strategy (P0's `run_combo_live` dual) is not buildable as specified — a BLOCKER.**

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
