# Combo-detector session handoff — RESUME HERE

**Last updated:** 2026-07-14 · **Branch:** `debug/combo-generator` (fork-only; **NEVER merge toward
`main`** — `.planning/` is gitignored upstream, force-add to commit here).

---

## 0. TL;DR — where we are

Diagnosing why two live infinite combos on the user's real 4-player Commander board are **not detected**
by the CR 732.2a loop-shortcut detector. **Investigation + planning is essentially COMPLETE.** What
remains is **one adversarial review pass**, then **one consolidation pass**, then the document goes to the
user / repo maintainer.

**Nothing is being implemented.** This is a plan-only workstream by explicit user instruction.

| Artifact | Path | State |
|---|---|---|
| **THE PLAN** (the deliverable) | `.planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md` | committed **`69fe6c3ea`** |
| **Reviewer mandate** (file relay) | `.planning/combo-detection/REVIEWER-MANDATE.md` | on disk |
| **This handoff** | `.planning/combo-detection/SESSION-HANDOFF.md` | you are here |
| Real-board acceptance tests | `crates/engine/tests/integration/repro_user_combo.rs` | 1 passing guard + **2 `#[ignore]`d, FAILING** (the bug) |
| Real-board fixture (11MB export) | `crates/engine/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json` | committed |
| **The one code fix so far** | `crates/engine/src/game/ability_scan.rs` — `scan_mana_production` | committed (fixes "R2": a basic `Forest` vetoed all detection) |

```bash
cargo test -p engine --test integration real_board_fixture_is_intact   # PASSES (guards the fixture)
cargo test -p engine --test integration -- --ignored real_board        # FAILS  (the bug)
```

---

## 1. IMMEDIATE NEXT ACTION

1. **Respawn the reviewer in tmux mode.** (Prior reviewers were spawned without a tmux pane; two of them
   produced full reports that **never reached team-lead**, and one later confirmed its report *had* been
   "delivered successfully." **The messaging transport is unreliable in this session — do not trust a
   send receipt.**)
   - Spawn as a **tmux teammate**: `Agent` with a **name** + `run_in_background: true` and **NO
     `isolation: "worktree"`** (isolation strips the tmux backend). Use a **fresh name** — a name
     collision corrupts message routing.
   - **Point it at `REVIEWER-MANDATE.md` on disk** — that file is authoritative and self-contained.
   - **Demand a challenge-response ACK** echoing specifics back before it starts, so an empty envelope is
     detectable immediately.
   - Require the **entire report in its final response message**, never in a file.
2. **Apply its findings in ONE consolidation pass.** The user has authorized exactly one iteration.
3. **Hand the document to the user.**

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
