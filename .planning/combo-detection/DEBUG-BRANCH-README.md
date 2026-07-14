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
