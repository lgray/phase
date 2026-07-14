# The combo detector — what it must do, and the one thing stopping it

**Date:** 2026-07-14 · Written fresh from the spec. **Every code citation measured against `main` @ `efc76ca1b`.**
*(The prior planning docs were written against a tree **768 commits behind `main`**; their citations do not resolve
there and several of their claims are refuted below. They are marked STALE.)*

---

## 1. The spec — the whole feature, in five stages

**Interactive is the only mode.** There is no auto-win mode and no off switch (§4).

| | Stage | Rule |
|---|---|---|
| **1** | **CAPTURE** the leading player's actions, **as FIXED choices** — a loop is a *sequence of actions*, not a decision tree. | CR 732.2a: *"a sequence of game choices… **can't include conditional actions**."* |
| **2** | **REPEAT** that exact sequence and see whether it yields an **unbounded resource**. | CR 732.1b: a loop is *"a set of **actions** [that] could be repeated indefinitely."* |
| **3** | **CLASSIFY** the unbounded resource as **ADVANTAGE**, **WIN**, or **DRAW**. | CR 704.5a (win) · **CR 104.4b (draw** — a *mandatory* loop with no way to stop; *"loops that contain an optional action don't result in a draw"*) |
| **4** | **PRESENT** it to the player. If they accept, **pass priority around the table** so every opponent may interact or shorten it. | CR 732.2b/c · MTR 4.4: each opponent may *"announce a lower number after which they intend to intervene."* |
| **5** | If accepted and un-interacted-with: **EMIT the certificate and APPLY the resulting game-state changes.** | CR 732.2a — the shortcut *is* the state change. |

> ### ⭐ The load-bearing omission — and it is correct
> **The spec says "repeat the ACTIONS." It does NOT say the game state must return to where it started.**
> **Neither does the rulebook.** CR 732.2a's own worked example — **Presence of Gond + Intruder Alarm**
> (`docs/MagicCompRules.txt:6373`) — **adds a token every iteration**, so its state provably never recurs, and the
> rules shortcut it a million times.
>
> ## ⇒ **A detector that requires STATE RECURRENCE must reject the rulebook's own worked example.**

**Not gated on Rules Enforcement Level.** MTR §4.4 carries **zero** REL qualifiers (measured: 0 hits for
`Competitive|Professional|Regular|Enforcement` across its 49 lines), and the no-conditional-actions core is in
**CR 732.2a** itself. Identical at Regular / Competitive / Professional and in casual play. **Nothing to gate,
nothing to toggle.**

---

## 2. All five stages ALREADY EXIST on `main` — **DO NOT REBUILD ANY OF THEM**

**This is PR-7's work. It is substantially the spec, and it is tested.**

| Stage | On `main` | Where |
|---|---|---|
| **1 · CAPTURE** | `last_recast_context` (the fixed recast frame) + `loop_detect_ring` (sampled prior states) | `game/engine.rs:450` · `:537` |
| **2 · REPEAT** | **Drives the captured sequence on a CLONE** — two iterations, three settle frames, under a re-entrancy guard. **A real replay, not a static re-derivation.** | `try_offer_object_growth_shortcut`, `game/engine.rs:1656`, `:1688` |
| **2 · unbounded, incl. board growth** | `loop_states_cover_modulo_object_growth` (~40 assertions) · `loop_states_cover_modulo_growth` (counters) | `analysis/resource.rs:924` · `:784` |
| **3 · WIN** | Path A — determinate single winner; multiplayer-safe (requires exactly one non-faller, CR 104.2a) | `interactive_loop_bridge`, `game/engine.rs:498` |
| **3 · DRAW** | Path B — CR 104.4b / 732.4, gated on mandatory (CR 732.5) | `game/engine.rs:536` |
| **3 · ADVANTAGE** | `WinKind::Advantage` | `analysis/loop_check.rs:83` |
| **4 · PRESENT** | `WaitingFor::LoopShortcut { proposer, predicted_winner, certificate, schema }`, carrying `IterationCount::{Fixed, UntilLethal}` | `types/game_state.rs:4458` · `analysis/decision_template.rs:203` |
| **4 · ACCEPT / DECLINE** | `GameAction::DeclareShortcut { count, template }` / `DeclineShortcut` | `types/actions.rs:834` · `game/engine.rs:4376` |
| **4 · INTERACT (APNAP)** | `WaitingFor::RespondToShortcut { player, remaining_players, proposal }` — multiplayer-shaped | `types/game_state.rs:4476` · `game/engine.rs:1992` |
| **5 · APPLY** | `apply_confirmed_shortcut` → `apply_until_lethal_shortcut` / `materialize_fixed_shortcut`; re-validates proposer + winner at consumption (CR 800.4a — a seat can concede during the window) | `game/engine.rs:855`, `:906`, `:1325` |
| **— · refuse non-deterministic loops** | static gate + runtime backstop, CR 705.1 / 706.1a / 701.9a-b | `game/engine.rs:1684` · `ability_scan.rs:4407` |

> ## ⇒ **The pipeline is complete end to end. The problem is not that it cannot be built. It is that it cannot be REACHED.**

---

## 3. ⛔⛔ THE DEFECT — the covers wrote the detector out of the game by construction

**The fire-time firewall.** `fire_time_conditions_read_growing_class` (`analysis/resource.rs:1457`), consumed by
**two** gates (`:968`, `:1131`), walks every object in scope and **vetoes the entire detection** if **any one** of
them has an ability for which `ability_reads_sibling_mutable` (`game/ability_scan.rs:3580`) returns true.

**And `sibling: true` is the FAIL-CLOSED DEFAULT.** `ability_scan.rs:2416-2420`, in its own words:

> *"type/controller predicates read none. `event`/`sibling` stay **CONSERVATIVE**"* → `sibling: true`

**Measured on `main`: 84 fail-closed sites in one file** — 54 `Axes::CONSERVATIVE` + 30 `sibling: true`.

### The composition failure

- **Individually, every default is RULES-CORRECT.** Fail-closed can only ever cause a **missed offer**. It can
  never falsely certify a loop and wrongly end a real game. Each was the right call in isolation — **and each was
  FREE: `sibling: true` costs a contributor nothing and trips no test.**
- **Composed, they are a near-certain veto.** The firewall is a **disjunction over every object in scope**, and a
  4-player Commander board carries ~100 permanents. **One conservative arm anywhere on the table kills detection
  for the whole table.** On the real fixture the trips were, in order: a `Solemn Simulacrum` **in the library**, a
  basic **Forest**, and **Freed from the Real** — **none of which belongs to either combo.** *(Inherited from the
  stale tree; **UNVERIFIED on `main`** — §5 re-measures it.)*

> ## ⇒ **UNREACHABLE BY CONSTRUCTION** — not "buggy on some boards." **Structurally dead on any board that looks like a real game.**

**The suite could not see it**, for two compounding reasons:
1. **Its fixtures build boards that cannot exist** — no lands, empty library, stub oracle. **The detector was only
   ever exercised on boards with nothing on them.**
2. **The guards are a one-sided ratchet.** There is a discriminating **negative** guard (*Gaea's Cradle MUST fail
   closed* — it **counts** a mutable creature set, `ability_scan.rs:4840`) and **no discriminating positive guard
   at all**: the only *"must NOT trip"* assertion uses `fixed_drain` = `GainLife { Fixed(1), Controller }`
   (`ability_scan.rs:5215`), **which references no object filter whatsoever.** ⇒ **Over-acceptance is structurally
   detectable; over-rejection is not.** The conservative arm always won — **84 times.**

### ⇒ The fix, stated as a class

**Re-derive `sibling` from a POSITIVE definition:** *does this ability **COUNT a mutable set** (⇒ conservative), or
does it merely **NAME a type** (⇒ not)?* — and **install the missing positive guard** so the ratchet becomes
symmetric. **Acceptance is class-level, and both halves are mandatory:**

> **Intruder Alarm must UN-REJECT** (CR 732.2a's own example) **AND Gaea's Cradle must STILL FAIL CLOSED.**
> **A flip that un-rejects both is a HOLE in the catastrophic direction, not a fix.**

---

## 4. Interactive becomes the only mode

**Measured:** `LoopDetectionMode { Off (default), On, Interactive }` (`types/game_state.rs:5942`).

- **`Off`** — pre-feature behavior; the shipped default. **Delete.**
- **`On`** — auto-wins a mandatory lethal drain **without offering it.** That is **rules-wrong**: CR 732.2a makes
  suggesting a shortcut *optional* and gives opponents a response window. **Delete.** *(Its only production-shaped
  call sites, `match_flow.rs:669`/`:744`, are inside `#[cfg(test)]` — mod tests opens at `:360`.)*
- **`Interactive`** — offers, runs the APNAP window, classifies win/draw/advantage. **This is the feature. Keep it,
  and make it unconditional.**

**Consequence:** deleting `Off` removes the opt-in gate — the detector becomes always-on, so **§3 must land first.**
An always-on detector that vetoes on every real board is merely always-on *and useless*; an always-on detector with
a **wrong** cover is a **false certificate that ends a real game.** **Order matters: fix reachability, prove the
class gate, then collapse the modes.**

---

## 5. ⭐ THE NEXT ACTION — one experiment, and it decides everything after it

**In a throwaway worktree cut from `main`: stub the fire-time firewall to ALWAYS ACCEPT, port the acceptance
fixture, and run it.**

The fixture and its two `#[ignore]`d acceptance tests live **only** on `debug/combo-generator` and have **never been
run against `main`**:
`tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json` (11 MB, real 4-player export) ·
`tests/integration/repro_user_combo.rs`

| Result | Meaning | Then |
|---|---|---|
| 🟢 **GREEN** | **The covers are the only thing in the way.** The five-stage pipeline works end to end on a real board. | Do **§3** — the positive re-derivation + the symmetric guard. Then **§4**. |
| 🔴 **RED** | Something else is also broken. | Instrument **that** failure **against `main`**. ⛔ **Do NOT inherit the stale plan's RC-1…RC-4 — two of their premises are already refuted.** |

**It strictly dominates auditing the 84 sites: the audit ASSUMES the firewall is the blocker; this TESTS that
assumption.** A worktree gets its own `target/`, so it will **not** contend with Tilt's cargo lock on `main` — it
just pays one cold build.

> **⚠️ UNVERIFIED — not yet run. Every claim in this document about *why the real board fails* (including §3's trip
> list) is inherited from the stale tree. Run this before believing any of it — including this document.**

---

## 6. Do not "fix" these — they are already right

- ⛔ **No non-determinism gate.** Exists twice (static + runtime), CR-annotated. `engine.rs:1684`, `ability_scan.rs:4407`.
- ⛔ **No REL / "tournament mode" toggle.** MTR 4.4 is not REL-gated (§1).
- ⛔ **Do not relax `GameState::PartialEq`'s `delayed_triggers` conjunct** — it is what stops certifying a loop whose
  growth axis dies at the next end step.
- ⛔ **Do not delete the covers. Make them PRECISE.** A fail-closed default consumed as if it were a **precise
  predicate** is the defect — not the fail-closed default itself.

> ### The rule that outranks everything here
> **A coarse relation may REJECT, never ACCEPT.** Too coarse ⇒ a **false certificate** ⇒ **a real game ends
> wrongly.** Too fine ⇒ a missed offer ⇒ **safe.** §3 moves the detector toward **ACCEPT**. It is the one direction
> that can lose someone a game. **Review it twice.**

---

## 7. Provenance

**STALE, historical only:** `REAL-BOARD-RCA-AND-PLAN.md`, `SESSION-HANDOFF.md`, `PLANNER-BRIEF.md`,
`ADVERSARY-MANDATE.md`, `REVIEWER-MANDATE.md`, `DEBUG-BRANCH-README.md`. Measured against a tree **768 commits
behind `main`**. **Refuted since:** *"there is no live object-growth path"* (**false** — `engine.rs:1656`) and *"the
offer carries no iteration count"* (**false** — `decision_template.rs:203`). **Read them for the RULES reasoning,
which has held throughout — never for a code fact.**

**Rules sources:** `docs/MagicCompRules.txt` — CR 104.4b `:366` · 732.1b `:6366` · 732.1c `:6368` · 732.2a +
Example `:6372`/`:6373`. [MTR, eff. 2026-02-27](https://media.wizards.com/ContentResources/WPN/MTG_MTR_2026_Feb27_EN.pdf)
§4.2/§4.4 · [judge annotations](https://blogs.magicjudges.org/rules/mtr4-4/).
