# Secret of Bloodbending — PENDING USER RULING (do NOT treat as decided)

**Status (2026-07-03):** team-lead escalated to the user. "All 40 S25 cards ship" was an explicit user
directive, so 39/40 vs funding 1-card infra is the user's call. Both branches below are prepped so either
ruling is a small delta.

## The card
`{2}{B}{B}` Sorcery: "As an additional cost to cast this spell, you may waterbend {10}. You control target
opponent during their **next combat phase**. If this spell's additional cost was paid, you control that player
during their **next turn** instead. (You see all cards that player could see and make all decisions for them.)
Exile Secret of Bloodbending."

## Why it's gated (measured, corroborated by planner + /review-engine-plan)
- Base (default, no-cost) branch = control during **next combat phase** = CR 723.2 **limited-duration** player
  control. `Effect::ControlNextTurn { target, grant_extra_turn_after }` (types/ability.rs:8886) has NO
  duration/window field → full-turn only (CR 723.1). And `turn_control`/`turns.rs` activate (:539-549) + release
  (:461-476) BOTH inside `start_next_turn` → turn-boundary only; there is NO phase-boundary release path.
- **Class size = exactly 1** (35k-card sweep): the only player-control-during-combat-phase card. Word of Command
  (spell-resolution window) / Opposition Agent (while-searching) / Clocknapper (phase-steal) are different
  windows/mechanics. Building phase-boundary turn-control for 1 card = build-for-the-card.
- **All-or-nothing**: the card has TWO Unimplemented nodes (base `phase`, paid `next turn`); fixing only the paid
  branch leaves the base Unimplemented → 0 coverage.

## RULING A — DEFER (strict-failure tag) — the small delta to apply
Leave the base combat-phase branch a loud `Effect::unimplemented(...)` (its current state — no code change needed
to *keep* it unsupported). Prep tasks to finalize on an A-ruling:
1. Add a strict-failure/coverage test asserting Secret of Bloodbending parses to `Unimplemented` on the
   combat-phase branch (documents the known gap so it doesn't silently regress to a wrong support).
2. Convert this file's "full build" section to a standalone follow-up ticket; add a `project` memory pointer.
No engine change. Delta ≈ 1 test + doc.

## RULING B — FULL BUILD — banked design (small delta = execute the plan)
Full design is banked in `S25-P2b-control-player-PLAN.md` §3.4. Summary of the work:
1. Parameterize `ControlNextTurn { …, window: ControlWindow { NextTurn, NextCombatPhase } }`
   (parameterize-don't-proliferate; /add-engine-variant verdict = REFACTOR_FIRST/parameterize, same CR 723
   section; serde-default `window = NextTurn` keeps all fixtures loading). **Must ship the runtime in the same
   commit — no silent stub.**
2. Runtime: phase-boundary activate (at `BeginCombat` of the affected player's next turn) + release (at
   `PostCombatMain`) in `turns.rs` — the genuinely-new infra. `ability_scan.rs:514` fully destructures
   `ControlNextTurn` (no `..`) so the new field forces the #4904 fail-closed walker to re-classify (expected).
   `effects/mod.rs` uses `{ .. }` (frozen file untouched); thread `window → ScheduledTurnControl` manually
   (not compiler-enforced in the resolver).
3. Parser: combat-phase suffix in `try_parse_control_next_turn_suffix`; anaphoric "that player" → spell target;
   the `AdditionalCostPaid`-gated dual branch (waterbend {10} paid → NextTurn window, else NextCombatPhase);
   "Exile Secret of Bloodbending" self-exile.
4. Anti-hollow-win: end-to-end cast-pilot test (cast → advance → assert `turn_decision_controller == caster`,
   opponent still active per CR 723.3, control releases at the correct phase/turn boundary).

## Note
The full-turn control subsystem (`ControlNextTurn`, CR 723.1) already works for 6 cards (Mindslaver, Worst Fears,
Sorin, Construct a Cosmic Cube, The Dominion Bracelet's granted ability, …). Only the phase-scoped window is new.
