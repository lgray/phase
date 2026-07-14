# Loop shortcuts — the spec, the measured engine, and what is actually left

**Date:** 2026-07-14 · **Written fresh from the spec.** No content inherited from the prior planning docs.
**Every code citation was measured against `main` @ `efc76ca1b`** — *not* against `debug/combo-generator`, which
is **768 commits stale** and whose citations no longer resolve.

> # ⛔ THE HEADLINE
> ## **PR-7 BUILT THE SOLUTION. THE COVERS MAKE IT UNREACHABLE.**
>
> The engine **can** detect the loop — the clone-drive, the object-growth cover, the determinism gate, the
> iteration count, the win/draw/advantage classification, the multiplayer response window are **all shipped and
> tested on `main` today** (§3).
>
> **And on any real board it will structurally never fire.** The fire-time firewall vetoes if **any** object in
> scope has an ability that reads a *"sibling-mutable"* axis — and `sibling: true` is the **fail-closed DEFAULT**
> for any typed filter (`ability_scan.rs:2420`, whose own comment says it *"stays CONSERVATIVE"*). A 4-player
> Commander board has ~100 permanents. **The probability that not one of them trips a conservative arm is
> approximately zero.**
>
> ### ⇒ **Every cover is individually rules-correct. Composed, they wrote the combo detector OUT OF THE GAMEPLAY LOGIC BY CONSTRUCTION.**
> ### ⇒ **This is a REACHABILITY defect, not a capability defect. Do not build more detector. Make the detector reachable.**

---

## 1. The spec

**The user's, 2026-07-14, and it is correct:**

> **Given a set of card actions the player performed, determine whether that exact set, repeated, produces an
> unbounded resource. Classify the outcome. Present the shortcut to the player. If accepted, pass priority
> around the table for interaction.**

**It is right in the place that matters most: it says *repeat the ACTIONS*. It does NOT say the game state must
return to where it started.** Neither does the rulebook (§2).

**One correction — to the SPEC, not to the engine:**

| The spec says | The rules say | On `main` |
|---|---|---|
| *"advantage or win"* | **THREE terminals.** **CR 104.4b** (`docs/MagicCompRules.txt:366`): a loop of **mandatory** actions *"with no way to stop"* ⇒ **the game is a DRAW**; *"loops that contain an optional action don't result in a draw."* In 4-player cEDH that draws **the whole table.** | ✅ **already built** — `interactive_loop_bridge` **Path B**, `game/engine.rs:536` |

**Two further corrections were drafted and are RETRACTED on measurement. Recorded so nobody re-derives them:**

- ⛔ *"The offer carries no iteration count."* **FALSE on `main`** — `ShortcutDecisionSchema.iteration_count`
  (`analysis/decision_template.rs:203`; `Fixed(n)` / `UntilLethal`). True of the debug branch only.
- ⛔ *"There is no live object-growth path."* **FALSE on `main`** — `try_offer_object_growth_shortcut`
  (`game/engine.rs:1656`). True of the debug branch only.

> **Both were asserted from the stale branch and refuted by grepping `main` — in the act of summarizing a
> workstream whose defining failure mode is exactly that. A plan's citations are only as current as the tree they
> were taken from.**

---

## 2. The rules (verified verbatim)

| Rule | Where | What it says |
|---|---|---|
| **CR 732.1b** | `:6366` | A loop is *"a set of **actions** [that] could be repeated indefinitely."* |
| **CR 732.2a** | `:6372` | A shortcut is *"a sequence of game choices… that may be legally taken based on the current game state and the predictable results"* — explicitly *"a loop that **repeats a specified number of times**."* **No conditional actions.** |
| **CR 732.2a Example** | `:6373` | **Presence of Gond + Intruder Alarm** — *"I'll create a million tokens."* |
| **CR 732.1c** | `:6368` | In a tournament, **the MTR takes precedence** over CR 732. |
| **CR 104.4b** | `:366` | A **mandatory** loop with no way to stop ⇒ **DRAW**. An **optional** action ⇒ **no draw**. |
| **MTR 4.4** | [official PDF](https://media.wizards.com/ContentResources/WPN/MTG_MTR_2026_Feb27_EN.pdf) | *"detailing a sequence of **actions** to be repeated… **The loop actions must be identical in each iteration** and cannot include conditional actions."* The proposer names the iteration count; each opponent may *"announce a lower number after which they intend to intervene."* **Non-deterministic loops may not be shortcut.** |

**⇒ Nothing to gate on Rules Enforcement Level.** MTR §4.4 carries **zero** REL qualifiers (measured: 0 hits for
`Competitive|Professional|Regular|Enforcement` across its 49 lines). The regime is identical at Regular /
Competitive / Professional, and the no-conditional-actions core is in **CR 732.2a itself** ⇒ it holds in casual
play too. **No "tournament mode." No toggle. Nothing to build.**

### ⭐ The theorem

**Every one of those texts defines a loop by the repeatability of the ACTION SEQUENCE. Not one requires the GAME
STATE to recur.** State recurrence appears in exactly two, strictly narrower places: MTR 4.4's stopping rule for
*non-deterministic* loops, and its *multi-turn* condition.

**CR 732.2a's own worked example proves it: Presence of Gond + Intruder Alarm ADDS A TOKEN EVERY ITERATION.** Its
state provably never repeats — and the rulebook shortcuts it a million times.

> ## ⇒ **A DETECTOR THAT REQUIRES STATE RECURRENCE MUST REJECT THE RULEBOOK'S OWN WORKED EXAMPLE.**

---

## 3. What `main` already does — **DO NOT REBUILD ANY OF THIS**

**All measured on `main` @ `efc76ca1b`. This is PR-7's work, and it is substantially the spec.**

| Spec requirement | On `main` | Where |
|---|---|---|
| **Repeat the exact action set** | ✅ **DRIVES the captured recast on a CLONE** — a real replay, not a static re-derivation | `try_offer_object_growth_shortcut`, `game/engine.rs:1656` |
| **Unbounded resource that GROWS THE BOARD** | ✅ object-growth cover, ~40 assertions | `loop_states_cover_modulo_object_growth`, `analysis/resource.rs:924` |
| **…that grows COUNTERS** | ✅ | `loop_states_cover_modulo_growth`, `analysis/resource.rs:784` |
| **Classify: WIN** | ✅ Path A (CR 704.5a); multiplayer-safe — requires exactly one non-faller (CR 104.2a) | `game/engine.rs:498` |
| **Classify: DRAW** | ✅ Path B (CR 104.4b / 732.4), gated on mandatory (CR 732.5) | `game/engine.rs:536` |
| **Classify: ADVANTAGE** | ✅ `WinKind::Advantage` | `analysis/loop_check.rs:83` |
| **Present the shortcut** | ✅ `WaitingFor::LoopShortcut` | `types/game_state.rs:4458` |
| **…with an iteration count** | ✅ `IterationCount::{Fixed, UntilLethal}` | `analysis/decision_template.rs:203` |
| **Pass priority for interaction (MTR 4.4)** | ✅ `RespondToShortcut { player, remaining_players, proposal }` — multiplayer-shaped | `types/game_state.rs:4476` |
| **Refuse non-deterministic loops (MTR 4.4)** | ✅ static gate + runtime backstop, CR-annotated | `game/engine.rs:1684`; `ability_scan.rs:4407` |

> ## ⇒ **The machinery is ~90% shipped. The problem is that it cannot be REACHED.**

---

## 4. ⭐⭐ WHY IT NEVER FIRES — the covers, composed

**The fire-time firewall.** `fire_time_conditions_read_growing_class` (`analysis/resource.rs:1457`), consumed by
**two** gates (`:968`, `:1131`), walks the objects in scope and **vetoes the whole detection** if any one of them
has an ability for which `ability_reads_sibling_mutable` (`game/ability_scan.rs:3580`) returns true.

**And `sibling: true` is the FAIL-CLOSED DEFAULT.** `ability_scan.rs:2416-2420`, in its own words:

> *"type/controller predicates read none. `event`/`sibling` stay **CONSERVATIVE**"* → `sibling: true`

**Measured on `main`: 54 `Axes::CONSERVATIVE` + 30 `sibling: true` = 84 fail-closed sites in one file.**

### The composition failure — this is the whole bug

- **Individually, every one of those defaults is RULES-CORRECT.** Fail-closed can only ever produce a **false
  negative** (a missed offer). It can never falsely certify a loop and wrongly end a game. **Each was the right
  call in isolation, and each was FREE — `sibling: true` costs a contributor nothing and trips no test.**
- **Composed over a real board, they are a near-certain veto.** The firewall is a **disjunction over every object
  in scope**. A 4-player Commander board carries ~100 permanents. **One conservative arm anywhere on the table
  kills detection for the entire table.** Measured on the real fixture, the trips were, in order: a
  `Solemn Simulacrum` **in the library**, a basic **Forest**, and **Freed from the Real** — *none of which is part
  of either combo.*

> ## ⇒ **THE FEATURE IS UNREACHABLE BY CONSTRUCTION. Not "buggy on some boards" — structurally unreachable on ANY board that looks like a real game.**
> ## ⇒ **The test suite could not see it, because the acceptance fixtures build boards that cannot exist** (no lands, empty library, stub oracle). **The detector was only ever exercised on boards with nothing on them.**

**And the suite is a one-sided ratchet.** There is a discriminating **negative** guard (*"Gaea's Cradle MUST fail
closed"* — it counts a mutable creature set, `ability_scan.rs:4840`), and **no discriminating positive guard at
all**: the sole *"must not trip"* assertion uses `fixed_drain` = `GainLife { Fixed(1), Controller }`
(`ability_scan.rs:5215`), **which references no object filter whatsoever.** ⇒ **Over-acceptance is structurally
detectable; over-rejection is not.** The conservative arm always won, 84 times.

---

## 5. ⭐ THE NEXT ACTION — one experiment, and it decides the shape of everything after it

**In a throwaway worktree cut from `main`, stub the fire-time firewall to ALWAYS ACCEPT, port the acceptance
fixture, and run it.**

The fixture and the two `#[ignore]`d acceptance tests live **only** on `debug/combo-generator` and have **never
been run against `main`**:

- `crates/engine/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json` (11 MB, real 4-player export)
- `crates/engine/tests/integration/repro_user_combo.rs`

| Result | Meaning | What it costs |
|---|---|---|
| 🟢 **GREEN** | **The covers are the ONLY thing in the way.** The machinery works end to end on a real board. | The work is **re-deriving the `sibling` axis from a POSITIVE definition** — *does this ability COUNT a mutable set (⇒ conservative), or merely NAME a type (⇒ not)?* — and installing the missing positive guard so the ratchet becomes symmetric. |
| 🔴 **RED** | The covers are **not** the only blocker; the certificate's **SHAPE** is also wrong (§6). | Re-derive the root cause **against `main`**. ⛔ **Do NOT inherit RC-1…RC-4 from the stale plan — two of their premises are already refuted above.** |

**Cheap, and it strictly dominates auditing the 84 sites: the audit ASSUMES the firewall is the blocker; this
TESTS that assumption.** A worktree gets its own `target/`, so it will **not** contend with Tilt's cargo lock on
`main` — it just pays one cold build.

> **⚠️ UNVERIFIED — it has not been run. Every claim in this document about *why the real board fails* is
> inherited from the stale tree, including the trip list in §4. Run this before believing any of it, including
> this document.**

---

## 6. The second gap (secondary — only bites if §5 comes back RED)

**`LoopCertificate.unbounded` — *"a non-empty vector is an invariant of a returned certificate"***
(`analysis/loop_check.rs:122-123`) **⇒ the engine is an UNBOUNDEDNESS PROVER, not a shortcut engine.**

Per **CR 732.2a**, unboundedness is **not** required to *offer* a shortcut — one may be *"a loop that repeats a
specified number of times."* Unboundedness is what the **DRAW** (CR 104.4b) and the **AUTO-WIN** need. The two are
conflated.

**The clean form is the spec's own:** drive the fixed action sequence on a clone until an action becomes illegal
or an outcome diverges from iteration 1 — **that iteration is K.** A certificate carrying a bound **K** subsumes
the unbounded case (`K = ∞`) and turns depletion and thresholds into things the drive **measures** rather than
static gates that must prove a negative.

> ### ⛔ SOUNDNESS — the rule that outranks everything in this document
> **A coarse relation may REJECT, never ACCEPT.** Too coarse ⇒ a **false certificate** ⇒ **a real game ends
> wrongly.** Too fine ⇒ a missed offer ⇒ **safe.**
> **The DRAW and the AUTO-WIN are game-ending: they KEEP the `unbounded` requirement, unchanged.** Any bound-K
> relaxation is confined to the **OFFER**, which ends no game and which every opponent may interrupt
> (`RespondToShortcut`). **Relax nothing on a terminal that can end a game.**
>
> **This cuts both ways, and §4 is why:** the covers exist because someone took this rule seriously. **The fix is
> not to delete them — it is to make them PRECISE.** A fail-closed default consumed as if it were a precise
> predicate is the actual defect.

---

## 7. Already correct — do not "fix" these

- ⛔ **Do NOT add a non-determinism gate.** It exists twice (static + runtime backstop) and is CR-annotated.
  `game/engine.rs:1684`, `ability_scan.rs:4407`.
- ⛔ **Do NOT add an REL / "tournament mode" toggle.** MTR 4.4 is not REL-gated (§2).
- ⛔ **Do NOT relax `GameState::PartialEq`'s `delayed_triggers` conjunct** — it is what stops certifying a loop
  whose growth axis dies at the next end step.
- ⛔ **Do NOT weaken `unbounded` on the DRAW or AUTO-WIN paths** (§6).

---

## 8. Provenance

**Superseded, retained for history only — all marked STALE:** `REAL-BOARD-RCA-AND-PLAN.md`, `SESSION-HANDOFF.md`,
`PLANNER-BRIEF.md`, `ADVERSARY-MANDATE.md`, `REVIEWER-MANDATE.md`, `DEBUG-BRANCH-README.md`. They were measured
against a tree **768 commits behind `main`**; their `file:line` citations do not resolve there, and several
central claims are refuted above. **Read them for the RULES reasoning (which has held throughout) and the
SOUNDNESS rules — never for a code fact.**

**Rules sources:** `docs/MagicCompRules.txt` (CR 104.4b `:366`, 732.1b `:6366`, 732.1c `:6368`, 732.2a + Example
`:6372`/`:6373`) · [MTR, effective 2026-02-27](https://media.wizards.com/ContentResources/WPN/MTG_MTR_2026_Feb27_EN.pdf)
§4.2/§4.4 · [MTR 4.4 judge annotations](https://blogs.magicjudges.org/rules/mtr4-4/).
