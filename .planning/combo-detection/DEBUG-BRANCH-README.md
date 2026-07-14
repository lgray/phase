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

# `debug/combo-generator` — fork-only debug branch

**DO NOT merge, rebase, or cherry-pick this branch toward `main` or an upstream PR.**
It carries planning docs, an 11 MB real-board fixture, and a deliberately-failing acceptance
suite. It exists so the remediation plan can be *reproduced*, not shipped.

## What's here

| Path | Purpose |
|---|---|
| `.planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md` | Root-cause analysis + phased remediation plan |
| `crates/engine/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json` | Real exported 4-player Commander board (debug panel → Export Game State) |
| `crates/engine/tests/integration/repro_user_combo.rs` | Real-board acceptance tests (2 are `#[ignore]`d and FAIL — that is the point) |
| `crates/engine/src/game/ability_scan.rs` | The one proven fix: `scan_mana_production` walker |

## Reproduce the bug

```sh
# Fixture sanity (must PASS):
cargo test -p engine --test integration real_board_fixture_is_intact

# The bug (must FAIL until the plan lands):
cargo test -p engine --test integration -- --ignored real_board
```

Two real infinite combos on this board are undetectable:
1. **Witherbloom, the Balancer + Sprout Swarm** (object growth) — engine arms correctly, then the
   cover declines. Tripped by, in order: `Solemn Simulacrum` **in the library** (also a CR 400.2
   hidden-zone violation), a basic **Forest**, and **Freed from the Real**.
2. **Kilo + Freed from the Real + Relic of Legends → Pentad Prism** (counter growth) — structurally
   undetectable: the ring is cleared by every deliberate action, and no counter-growth cover exists.

See the plan for the full RCA.
