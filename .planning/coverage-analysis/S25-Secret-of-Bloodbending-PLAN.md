# S25 — Secret of Bloodbending — CR 723.2 phase-scoped player control (FULL BUILD)

Planner (xhigh). Worktree `/home/lgray/vibe-coding/s25-impl-wt` @ `330a1f18b` (NOT Tilt-watched).
Skills applied + NAMED per step: `add-engine-variant` (variant gate, §1), `add-engine-effect`
(lifecycle, §2/§5), `oracle-parser` (nom mandate, §5), `card-test` (discriminating revert-to-red, §7).

**User decision:** FUND the phase-boundary control (S25 stays 40/40). This plan starts from the banked
`S25-P2b-control-player-PLAN.md` §3.4 full-build design and HARDENS it to the turn-structure rigor bar.

**Card (verified via Scryfall API, TLA #69 — real mana cost is `{U}{U}{U}{U}`, NOT `{2}{B}{B}`; the
control logic is identical):**
> As an additional cost to cast this spell, you may waterbend {10}. You control target opponent during
> their next combat phase. If this spell's additional cost was paid, you control that player during
> their next turn instead. (You see all cards that player could see and make all decisions for them.)
> Exile Secret of Bloodbending.

---

## 0. Scope + the release-authority-extension principle

This is an **extension**, not a green-field. Full-turn player control (CR 723.1) already exists end to
end: `Effect::ControlNextTurn` (`types/ability.rs:8886`), resolver `control_next_turn.rs`, schedule
`ScheduledTurnControl` (`game_state.rs:7657`), turn-boundary activate (`turns.rs:539-549`) / release
(`turns.rs:461-476`), and submission routing `authorized_submitter_for_player` (`turn_control.rs:28`).
The ONE genuinely-new capability is **CR 723.2 limited-duration ("next combat phase") control**, which
`ControlNextTurn` cannot express and the runtime cannot release mid-turn.

**Release-authority-extension principle (team-lead rider 4 — binding constraint on this whole plan):**
There is exactly **one** player-control machinery — the pair (`state.scheduled_turn_controls` : `Vec<
ScheduledTurnControl>`, `state.turn_decision_controller` : `Option<PlayerId>`) plus the routing fn
`authorized_submitter_for_player`. This feature adds NO parallel Vec, NO second controller field, NO
bespoke routing. It adds a `window` discriminant to the ONE schedule and ONE new pair of activate/
release SITES keyed on that discriminant. The two release SITES (turn boundary in `start_next_turn`,
combat-phase boundary in `finish_enter_phase`) are inherent state-machine transitions — they are not a
fork. To make "single release authority" literal (not just structural), **both release sites — plus the
new leave-game cleanup (§Edge 4c) — route through one helper `turn_control::release_control_at(state,
idx)`** that clears `turn_decision_controller` and removes the consumed schedule entry. Window-specific
post-processing (CR 723.1 extra-turn grant; CR 723.2 no-op) stays at the calling site.

Design order below: (1) variant gate → `window`; (2) runtime activate/release (extend, don't fork);
(3) CR citations; (Edge) the four window edge cases DESIGNED; (5) parser dual-branch + anaphora +
self-exile; (6) frozen-file + files-touched (Bracelet seams flagged); (7) discriminating tests; (Risks).

---

## 1. `window` parameterization — `/add-engine-variant` gate (all three stages)

**Proposal:** express "control during next combat phase" (CR 723.2) vs "control during next turn"
(CR 723.1) as a parameter on the existing `Effect::ControlNextTurn`, not a new sibling.

```rust
// types/ability.rs — new leaf enum near ControlNextTurn:
/// CR 723.1 / CR 723.2: the duration window of a control-another-player effect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlWindow {
    /// CR 723.1: the affected player's entire next turn (Mindslaver, Worst Fears, Sorin −7).
    #[default]
    NextTurn,
    /// CR 723.2: limited duration — the affected player's next combat phase
    /// (Secret of Bloodbending). CR 723.2's enumerated-card list (Word of Command,
    /// Opposition Agent) predates this card and is non-exhaustive.
    NextCombatPhase,
}

// Effect::ControlNextTurn (ability.rs:8886) gains:
    #[serde(default)]           // = NextTurn — all existing card-data/fixtures deserialize unchanged
    window: ControlWindow,
```

- **Stage 1 — Existence (grep-verified).** `ControlNextTurn` carries only `{target,
  grant_extra_turn_after}` (`ability.rs:8886-8891`); `ScheduledTurnControl` carries no window
  (`game_state.rs:7657-7663`); no `ControlWindow`/`ControlDuration`/window enum exists in
  `types/ability.rs`. Verdict: **DOES_NOT_EXIST**.
- **Stage 2 — Parameterization.** A sibling `Effect::ControlNextCombatPhase` would differ from
  `ControlNextTurn` on exactly one axis — the control window/duration — the "differ only in a
  scope/duration dimension" smell. Verdict: **REFACTOR_FIRST → parameterize** with `window:
  ControlWindow` on the existing variant. Confirms the driver + team-lead pre-verdict.
- **Stage 3 — Categorical boundary.** The axis (control window) lies entirely within **CR 723**
  (723.1 vs 723.2 are subsections of the same "controlling another player" rule). Verdict:
  **WITHIN_SECTION.**
- **APPROVED as parameterization** (built with runtime in the same commit — no silent stub, per the
  skill's anti-pattern list).
- **Serialized-surface audit.** `ControlNextTurn` appears in `card-data.json`; `ScheduledTurnControl`
  appears in serialized `GameState`. Both new fields are `#[serde(default)]` = `NextTurn` so every
  existing fixture/card-data/saved-game loads unchanged. **Note (Rust literal churn, NOT serde):**
  `#[serde(default)]` only helps *deserialization*; every Rust *struct-literal* that constructs
  `ScheduledTurnControl { .. }` or `Effect::ControlNextTurn { .. }` must add the field explicitly. Sites
  enumerated in §6. Ship the converter/parser arm in the same commit (skill step 5).

---

## 2. Runtime — EXTEND, do NOT fork (the single release authority)

### 2.0 State-machine trace (verified file:line)

- Phase entry funnels through `enter_phase` (`turns.rs:147`) → seeds
  `pending_phase_transition_progress` → `drain_pending_phase_transition_progress` (`:194`) → when the
  APNAP mana-empty queue drains, `finish_enter_phase` (`:416`) fires **exactly once per phase entry**
  and sets `state.priority_player = turn_control::turn_decision_maker(state)` (`:426`).
  `turn_decision_maker` = `turn_decision_controller.unwrap_or(active_player)` (`turn_control.rs:8`).
- Combat begins at `Phase::BeginCombat` (CR 507); `enter_phase` increments
  `combat_phases_started_this_turn` there (`turns.rs:151-153`). Combat ends when the phase after
  `EndCombat` begins — normally `Phase::PostCombatMain` (CR 511.3), or `Phase::Cleanup` if the turn is
  ended in combat (`end_turn_to_cleanup`, `turns.rs:97`), or another `BeginCombat` for an extra combat
  phase (`advance_phase` extra-phase pop, `turns.rs:57-62`; `end_combat_phase_to_postcombat`, `:110`).
- `Phase::is_combat()` (`types/phase.rs:49`) already classifies the five combat steps.
- Submission routing `authorized_submitter_for_player` (`turn_control.rs:28-47`) reroutes ONLY the
  active player's seat to `turn_decision_controller`. **No change needed** — setting
  `turn_decision_controller` during combat is sufficient to pilot the controlled combat phase (CR 723.3:
  controlled player is still active player). Multiplayer filter (`server-core/src/filter.rs`) and
  `derived_views` key off the SAME `turn_decision_controller` → identical to the Mindslaver path, no new
  visibility work (CR 723.4; RNG-seed redaction precedent, commit 2851cea91).

### 2.1 The single release helper (turn_control.rs — NEW)

```rust
/// CR 723.1 / CR 723.2 / CR 800.4a: the ONLY function that ends a player-control
/// effect. Removes the consumed schedule entry (the resolver dedups to at most one
/// per target — CR 723.1a) and clears `turn_decision_controller` iff it pointed at
/// that entry's controller. Returns the removed entry so the caller applies
/// window-specific post-processing (CR 723.1 extra-turn grant). Turn-boundary,
/// combat-phase-boundary, and leave-game release all route here.
pub(super) fn release_control_at(state: &mut GameState, idx: usize) -> ScheduledTurnControl {
    let entry = state.scheduled_turn_controls.remove(idx);
    if state.turn_decision_controller == Some(entry.controller) {
        state.turn_decision_controller = None;
    }
    entry
}
```

### 2.2 Schedule the window (resolver — `control_next_turn.rs`)

- `:10` destructure `window` out of `Effect::ControlNextTurn` (thread it, don't drop it).
- `:34` set `window` on the pushed `ScheduledTurnControl`. The dedup `retain` (`:33`) stays — one entry
  per target regardless of window (CR 723.1a).

### 2.3 Turn-boundary paths (`turns.rs`) — surgical gate-edits, behavior preserved

- **Activation `:539-549`** — restrict to full-turn window so a NextCombatPhase entry is NOT activated
  at turn start: add `scheduled.window == ControlWindow::NextTurn` to the `rfind` predicate.
- **Release `:461-476`** — replace the `retain` with: locate the single `NextTurn` entry whose
  `target_player == completed_turn_key`, call `release_control_at`, and (guarded by `Some(entry
  .controller) == <the just-cleared controller>`) push the extra turn if `entry.grant_extra_turn_after`.
  **A `NextCombatPhase` entry for `completed_turn_key` must be LEFT IN PLACE (it carries — §Edge 4a).**
  ≤1 entry per target (resolver dedup) makes this find-one equivalent to the old retain; the four
  existing tests (`turns.rs:6887-6981`, all default `NextTurn`) stay green.

### 2.4 Combat-phase hook (`finish_enter_phase`, `turns.rs:416`) — the NEW site, at the TOP (before `:426`)

```rust
// CR 723.2 + CR 511.3 + CR 506.7d: phase-scoped ("next combat phase") player control.
// RELEASE runs BEFORE activation so a back-to-back extra combat phase (CR 500.8) releases
// the FIRST phase's control before we (correctly) decline to rebind it — "next combat
// phase" is the FIRST only (CR 506.7d). Bound combat phase is over on entry to any phase
// that is NOT a subsequent step of it: a fresh BeginCombat (new combat phase) or any
// non-combat phase (CR 511.3 → PostCombatMain, or CR 724.1d → Cleanup on an ended turn).
if state.turn_decision_controller.is_some()
    && (next == Phase::BeginCombat || !next.is_combat())
{
    if let Some(idx) = state.scheduled_turn_controls.iter().position(|s| {
        s.window == ControlWindow::NextCombatPhase
            && Some(s.controller) == state.turn_decision_controller
            && s.target_player
                == topology::normalize_shared_turn_recipient(state, state.active_player)
    }) {
        turn_control::release_control_at(state, idx);      // single authority
    }
}
// ACTIVATE: the affected player's next combat phase begins (CR 507).
if next == Phase::BeginCombat {
    if let Some(scheduled) = state.scheduled_turn_controls.iter().rfind(|s| {
        s.window == ControlWindow::NextCombatPhase
            && s.target_player
                == topology::normalize_shared_turn_recipient(state, state.active_player)
    }).copied() {
        state.turn_decision_controller = Some(scheduled.controller);
    }
}
```

Ordering proof (release-before-activate): entry PERSISTS while active (like the NextTurn path, which
keeps its entry through the controlled turn). On `BeginCombat(1)` release finds nothing (controller not
yet set) → activate binds. On a later `BeginCombat(2)` release fires (`next==BeginCombat` + active) →
removes the still-present entry and clears the controller; activate's `rfind` then finds nothing → NO
rebind (first-only latch). On `PostCombatMain`/`Cleanup` (`!is_combat()`) release fires → clears. A
carried-but-unactivated entry (combat skipped last turn) has `turn_decision_controller == None`, so
neither branch touches it until its real `BeginCombat`.

**Why `finish_enter_phase` and not a shared function with `start_next_turn`:** both are distinct
state-machine transitions; unifying them into one function would be a false abstraction. Rider 4 is
satisfied structurally (one schedule Vec, one controller field, one routing fn) and literally (both
release via `release_control_at`). This is EXTEND, not fork.

---

## 3. CR citations (every number grep-verified in `docs/MagicCompRules.txt` — team-lead rider 1)

| CR | line | what |
|---|---|---|
| 723.1 | 6159 | full-turn control; "applies to the next turn the affected player actually takes; doesn't end until the beginning of the next turn" → `NextTurn` |
| 723.1a | 6161 | multiple controlling effects overwrite; last wins → resolver dedup `retain` |
| 723.1b | 6163 | **skipped turn: pending effect WAITS until the player actually takes a turn** → the carry semantics, applied to the phase window (§Edge 4a) |
| 723.2 | 6165 | limited-duration control exists (enumerated list non-exhaustive; predates this card) → `NextCombatPhase` |
| 723.3 | 6167 | controlled player is still the active player; objects keep normal controllers |
| 723.4 | 6169 | controller sees what the controlled player sees → visibility (no new work) |
| 723.5/5a/5b | 6172/6176/6179 | controller makes all/only the controlled player's decisions, using only that player's resources |
| 500.8 | 2129 | extra phases inserted directly after their anchor → multiple combat phases (§Edge 4b) |
| 506.1 | 2196 | combat phase = 5 steps (begin, declare attackers, declare blockers, combat damage, end) |
| 506.7d | 2244 | "next combat phase" with multiple combats binds to the FIRST → first-only latch (§Edge 4b) |
| 507 (507.2) | 2252/2254 | beginning-of-combat step; active player gets priority → activation point |
| 511.3 | 2420 | "after the end of combat step ends, the combat phase is over and the postcombat main phase begins" → release boundary |
| 724.1d | (end-turn) | ending the turn skips to cleanup → release also on `Cleanup` |
| 800.4a | 6408 | player leaves game: effects giving them control of players END |
| 800.4b | 6414 | "If a player would be controlled by a player who has left the game, they aren't" |

**Scryfall official ruling (2025-10-02, TLA #69) — decisive for §Edge 4a:**
> "If the targeted player skips their next combat phase or turn, you'll control the next combat phase or
> turn the affected player actually takes."  → **CARRY, not lapse.**

---

## Window edge cases (designed — team-lead rider 2; NOT discovered)

### 4(a) — the target's next combat phase is SKIPPED → **CARRY (wait), do NOT lapse.**
Authority: the Scryfall ruling above, consistent with CR 723.1b (skipped-turn pending effects wait).
Design: the `ScheduledTurnControl { window: NextCombatPhase }` entry **persists** in
`scheduled_turn_controls` and is consumed ONLY by the phase-boundary release when it actually binds to a
combat phase. Two mechanics enforce carry: (1) §2.3 gates the turn-boundary release to `NextTurn` so the
entry is NOT dropped when the target's combat-less turn ends; (2) a fully-skipped combat phase never
calls `finish_enter_phase(BeginCombat)` (the CR 500.11 skip advances `state.phase` past it in
`advance_phase:76-86` without a phase-entry), so activation never fires until a real `BeginCombat`. The
entry carries across any number of combat-less turns until the target actually takes a combat phase.
Annotate the hook `// CR 723.1b + Scryfall ruling 2025-10-02: phase window carries to the next combat
phase actually taken`.

### 4(b) — MULTIPLE combat phases in one turn (extra-combat effects) → **bind FIRST only.**
Authority: CR 506.7d (multiple combat phases → "next" is the first) + CR 500.8 (extra phases). Design:
the first-only **latch** is the release-before-activate ordering in §2.4. On the FIRST `BeginCombat`,
activate binds and the entry persists. On the SECOND `BeginCombat` (extra combat), the release branch
(`next == Phase::BeginCombat` while active) removes the entry and clears the controller BEFORE the
activate branch runs; activate's `rfind` then finds no entry → no rebind. Result: control is live for the
first combat phase's five steps only, released the instant the second combat phase begins. (The entry is
also released normally at `PostCombatMain` when there is no extra combat.) Verified against
`advance_phase`'s extra-phase pop (`turns.rs:57-62`) and `end_combat_phase_to_postcombat` (`:110`, which
routes through `enter_phase(PostCombatMain)` → `finish_enter_phase` → the `!is_combat()` release).

### 4(c) — the CONTROLLING player leaves the game mid-window → **control ENDS immediately (CR 800.4a).**
Authority: CR 800.4a ("effects which give that player control of ... players end") + CR 800.4b ("if a
player would be controlled by a player who has left the game, they aren't"). **Finding:** `do_eliminate`
(`elimination.rs:320`) performs thorough CR 800.4a cleanup (stack spells, trigger-ordering, replacement
choices, owned objects) but **does NOT touch `scheduled_turn_controls`/`turn_decision_controller`** — so
this is a **pre-existing gap that also affects Mindslaver's full-turn control.** The class-level fix (one
site, both windows, both roles) belongs in `do_eliminate`:

```rust
// CR 800.4a + CR 800.4b: a control-another-player effect (CR 723) ends when either
// party leaves the game. Drop scheduled controls where `player` is the controller
// (800.4b) or the target (800.4a). If `player` was actively piloting (controller) OR
// was the controlled active player, end the live control via the single authority.
let leaving = topology::normalize_shared_turn_recipient(state, player);
while let Some(idx) = state.scheduled_turn_controls.iter().position(|s| {
    s.controller == player || s.target_player == leaving
}) {
    turn_control::release_control_at(state, idx);   // clears controller iff it matches
}
// controlled active player left: turn_decision_controller points at the (living)
// controller of a now-departed player — stale; clear it.
if state.turn_decision_controller.is_some()
    && topology::normalize_shared_turn_recipient(state, state.active_player) == leaving
{
    state.turn_decision_controller = None;
}
```

`release_control_at` clears `turn_decision_controller` only when it equals the removed entry's controller
(the controller-left case). The extra guard covers the controlled-player-left case (active player
departs mid-control). This closes 4c AND the latent NextTurn gap in one class-level fix. Discriminating
test in §7.4. *(Scope note: this is a small correct addition (~8 lines) that the rider explicitly
requires as DESIGNED; it may ship in the same commit or split — recommend same commit since it shares the
`release_control_at` authority.)*

### 4(d) — 3+ players → window open/close is rules-correct by construction.
The activation keys on `active_player == normalize(target)` at `BeginCombat`: the target opponent's next
combat phase occurs on the target's own next turn (they are the active player then), regardless of the
caster's turn — CR 500.1/506.2 turn order handles "which combat." During that combat the controller
pilots ONLY the active (controlled) player's seat — `authorized_submitter_for_player` reroutes only when
`semantic_player == state.active_player` (`turn_control.rs:39`); every other player (e.g. blockers)
decides for themselves, correct per CR 723.5. `normalize_shared_turn_recipient` (used in every hook
predicate) makes 2HG team turns correct (CR 805.8 — control the whole team; mirrors the existing
resolver at `control_next_turn.rs:28`). MEMORY (multiplayer correctness non-negotiable): satisfied — the
window uses the same seat/turn-order machinery as the shipped NextTurn path, only the boundary changed.

---

## 5. Parser — dual conditional branch + "that player" anaphora + self-exile (nom mandate)

Current parse (per banked plan, to be re-pinned by the implementer — Risk 1): outer ability + a
`sub_ability` gated `AdditionalCostPaidInstead`; BOTH control leaves are `Unimplemented`
(base → `Unimplemented{name:"phase"}`, paid → `Unimplemented{name:"you"}`). Self-exile and waterbend
already parse.

### 5.1 Combat-phase window axis (the base leaf)
`try_parse_control_next_turn_suffix` (`imperative.rs:295-328`) hard-codes the duration as
`terminated(alt(("that player's","their","its")), tag(" next turn"))` (`:301-304`). Extend the duration
to a single `alt()` axis returning a `ControlWindow` — compose, do NOT enumerate full strings:

```rust
// CR 723.1 / CR 723.2: the duration is one grammatical axis — possessive × window.
let (consumed, window) = preceded(
    tag::<_, _, OracleError<'_>>(" during "),
    preceded(
        alt((tag("that player's"), tag("their"), tag("its"))),
        alt((
            value(ControlWindow::NextTurn, tag(" next turn")),
            value(ControlWindow::NextCombatPhase, tag(" next combat phase")),
        )),
    ),
).parse(rem_lower.as_str()).map(...).ok()?;
```

Thread `window` through the return type `Option<(TargetFilter, bool, ControlWindow)>` → the AST field →
the Effect field. Sites: suffix `:295`; dispatch `:1855`/`:1867`; AST construct `:1861`/`:1875`; AST→
Effect lower `:2110-2116`. **`TargetedImperativeAst::ControlNextTurn`** gains a `window` field. The
second, inline suffix at `mod.rs:12280` (only the "gain control of ..." prefix, hard-codes " during that
player's next turn") is NOT on this card's path ("you control ..." → `imperative.rs:1855`); extend it for
the class OR (preferred) unify it to call the shared suffix (flag: pre-existing duplication).

### 5.2 The conditional dual branch (`AdditionalCostPaid` → window select)
Model as the existing override-instead pattern (verified: `apply_instead_swap`, `ability_utils.rs:149`,
does `overridden = parent.clone()` then replaces only `.effect`/effect-shape fields — the parent's
resolved `targets` vec is preserved; swap gate at `effects/mod.rs:5484-5523` reads
`ability.context.additional_cost_paid`):
- **Parent (base, unpaid):** `ControlNextTurn { target: Typed(Opponent), window: NextCombatPhase }`,
  target slot filled at cast.
- **Override sub (paid):** `ControlNextTurn { target: ParentTarget, window: NextTurn }`, condition
  `AdditionalCostPaidInstead`. On `additional_cost_paid`, the swap replaces the base effect with the
  paid effect and reuses the parent's `targets` → controls the SAME chosen opponent for the full turn.

### 5.3 "that player" anaphora — solved by the swap, not by re-targeting
Because `apply_instead_swap` clones the parent (incl. `targets`) and `control_next_turn::resolve` reads
`ability.targets.first()` (`control_next_turn.rs:20`), the paid sub's OWN target filter is never resolved
for targeting. Emit `target: ParentTarget` on the paid sub for correctness/other consumers; functionally
the parent's opponent is used. This is the CR 608.2c/608.2e "instead" mechanism — no new targeting, no
new `TriggeringPlayer`-vs-anaphora bug at runtime.

### 5.4 Self-exile — already parses to `ChangeZone { destination: Exile, target: SelfRef }` (no work).

**Nom compliance:** the only new parser code is the `alt((value(NextTurn, tag(" next turn")),
value(NextCombatPhase, tag(" next combat phase"))))` window axis composed into the existing suffix
combinator — one `alt()`, no `contains`/`split_once`/full-string enumeration.

---

## 6. Frozen-file check + files touched (Bracelet seams FLAGGED)

**Frozen (design edits forbidden):** `game/effects/mod.rs`, `game/filter.rs`,
`game/effects/delayed_trigger.rs`. **None touched for design.** `effects/mod.rs:2992` dispatch and
`:4931` target-match both use `{ .. }`/`{ target, .. }` — the new `window` field does not touch them.

**Files touched (all non-frozen):**

| File | Change | Bracelet seam? |
|---|---|---|
| `types/ability.rs` | + `ControlWindow` enum; + `window` field on `ControlNextTurn` (:8886); update full-construct sites `imperative`/tests | **SHARED w/ Bracelet (both add to `types/ability.rs`) — serialize impl; re-anchor line nums** |
| `game/ability_scan.rs` | `:514` full destructure `ControlNextTurn { target, grant_extra_turn_after: _ }` → add `window: _` (Axes::NONE; no choice/projection). Other arms (`:3315`) use `..` — unaffected | **SHARED w/ Bracelet (task note) — serialize impl** |
| `types/game_state.rs` | + `window: ControlWindow` on `ScheduledTurnControl` (:7657), `#[serde(default)]` | no |
| `game/effects/control_next_turn.rs` | resolver: destructure+thread `window` (:10, :34); update test literals (:58,:65,:80,:92,:113) | no |
| `game/turn_control.rs` | + `release_control_at` single-authority helper | no |
| `game/turns.rs` | phase hook in `finish_enter_phase` (:416); gate turn-boundary activate (:539) + release (:461) via helper; update test `ScheduledTurnControl` literals (:6894,:6930,:6957,:6964) | no |
| `game/elimination.rs` | 4c control-state cleanup in `do_eliminate` (:320) | no |
| `parser/oracle_effect/imperative.rs` | combat-phase window axis in suffix (:295); `TargetedImperativeAst::ControlNextTurn` + window (:1861,:1875,:2110); update parser-test Effect literals (:13132,:13160,:13185) | **parser seam — Bracelet also touches parser (`oracle_static/grammar.rs`, `oracle.rs`); DIFFERENT files, serialize to be safe** |
| `parser/oracle_effect/mod.rs` | (optional) unify/extend inline suffix (:12280) | parser seam |
| `game/casting_tests.rs` (:26067), `parser/oracle_trigger_tests.rs` (:17608) | add `window:` to Effect literals | no |

**Dead code (leave alone — YAGNI):** `parser/oracle_ir/ast.rs:969` `ControlNextTurn` is defined but
never constructed or lowered (grep: single hit). No `window` needed; adding one is harmless but
unnecessary. `mtgish-import/convert/action.rs:631` uses `{ ref mut target, .. }` — unaffected.

**Bracelet serialization ask:** the driver must serialize implementation on `types/ability.rs` and
`ability_scan.rs` (both increments append there) and re-anchor line numbers after whichever lands first.

---

## 7. Test plan (discriminating + revert-to-red — team-lead rider 3; `card-test` recipe)

Use `GameScenario` + `GameRunner::cast(...).resolve()`; assert on `CastOutcome`/state deltas. All below
FAIL if the specific hook/edit is reverted (revert-to-red evidence stated per test).

### 7.1 Both boundaries within one turn (the discriminating core — DESIGNED assertion set)
Schedule a `NextCombatPhase` control (target O, controller C) and drive the phase machine turn by turn.
Assert `turn_decision_maker(state)` (owner vs controller) at each phase of O's turn:
- Upkeep / Draw / **PreCombatMain** → `== O` (owner decides before combat).
- **BeginCombat / DeclareAttackers / DeclareBlockers / CombatDamage / EndCombat** → `== C` (controller
  pilots combat).
- **PostCombatMain / End** → `== O` (released; owner decides after).
Revert-to-red: removing the §2.4 activate → combat asserts fail (still O); removing the release →
PostCombatMain assert fails (stuck on C).

### 7.2 First-only latch (4b) — two combat phases
Give O an extra combat phase (`extra_phases`/Aurelia-style). Assert controller pilots combat phase 1,
and owner (not controller) decides during combat phase 2. Revert: removing the `next==BeginCombat`
release branch leaves C piloting combat 2 → fails.

### 7.3 Carry (4a) — skipped combat phase
Schedule the `NextCombatPhase` control, then advance O through a turn whose combat phase is skipped
(combat-skip). Assert the entry SURVIVES the turn boundary and turn_decision_controller stays None; then
give O a normal turn and assert control activates at its BeginCombat. Revert: NOT gating the
turn-boundary release to `NextTurn` (§2.3) drops the entry → activation never happens → fails.

### 7.4 Controller leaves the game (4c)
Schedule an ACTIVE control (turn_decision_controller = C) and eliminate C via the production
`elimination::eliminate_player`. Assert `turn_decision_controller == None` and no `scheduled_turn_controls`
entry with controller C remains. Revert: without the `do_eliminate` cleanup, the stale controller
persists → fails. (Non-vacuous: also assert a control for a DIFFERENT controller survives C's
elimination.)

### 7.5 3+ player (4d)
4-player game; C controls O's next combat phase. Assert during O's combat that O's seat routes to C
(`authorized_submitter_for_player(state, O) == C`) while a third player P routes to itself
(`... (state, P) == P`). Revert: any seat-scoping regression fails the P assertion.

### 7.6 Anti-hollow-win end-to-end cast-pilot (covers the parse→resolve→schedule→pilot chain)
Cast **Secret of Bloodbending** (no waterbend) targeting O; resolve. Assert `scheduled_turn_controls`
gained a `NextCombatPhase` entry (parse+resolver chain). Advance to O's next turn; assert owner O decides
in precombat main, controller (caster) decides in combat, released at PostCombatMain. This is the
non-vacuous full-pipeline proof. (The existing `turns.rs:6887-6981` tests hand-plant the schedule and
never exercise cast→resolve — this closes that gap.)

### 7.7 Paid branch (full-turn window) + self-exile
Cast Secret WITH waterbend {10} paid targeting O; resolve. Assert the swapped effect scheduled a
`NextTurn` entry bound to O (control the whole turn, released at the next turn boundary), and that Secret
is in exile. Revert: dropping the `AdditionalCostPaidInstead` sub or the window thread schedules the wrong
window → fails.

### 7.8 Parser AST tests (shape — NOT a substitute for 7.1/7.6)
`parse_effect("you control target opponent during their next combat phase")` → `ControlNextTurn { window:
NextCombatPhase, .. }`; `parse_effect("you control target opponent during their next turn")` → `window:
NextTurn` (regression guard for the default). Snapshot the full card in `oracle_parser.rs` (both
branches, self-exile).

---

## Risks / open questions for /review-engine-plan

1. **Paid-leaf current blocker (parser).** The banked plan attributes the paid `Unimplemented{name:"you"}`
   to "that player" → `TriggeringPlayer`, but §5.3 shows the runtime swap makes the sub's target filter
   irrelevant. The actual blocker may be the trailing " instead" / sentence-split, not the anaphora.
   **Implementer MUST first run `parse_oracle_text` on the full card and each isolated clause to pin the
   exact current AST before editing.** Fix is minimal once pinned; the design (parent NextCombatPhase +
   override-sub NextTurn) is blocker-agnostic.
2. **Waterbend → `additional_cost_paid`.** The `AdditionalCostPaidInstead` swap reads
   `ability.context.additional_cost_paid` (`effects/mod.rs:5489`). Confirm the OPTIONAL "you may
   waterbend {10}" payment path (`casting.rs:14077-14086` detours to ManaPayment Waterbend mode) sets
   that flag (cf. `ability_utils.rs:326`). If it doesn't, the paid branch silently never swaps → 7.7
   catches it.
3. **Two suffix parsers.** `imperative.rs:295` (shared, this card's path) vs `mod.rs:12280` (inline,
   "gain control of" only). Recommend unifying `mod.rs` to call the shared suffix; at minimum extend the
   card's path. Confirm which the card actually routes through (Risk 1 diagnostic covers it).
4. **4c scope.** The elimination cleanup fixes a pre-existing NextTurn gap too. Confirm the driver wants
   it in this commit (recommended — shares `release_control_at`) vs a separate follow-up.
5. **Turn-boundary release refactor safety.** §2.3 replaces the `retain` with find-one + helper, relying
   on the resolver's per-target dedup (≤1 entry). If any future path pushes two entries for one target
   the find-one drops only one — but that violates CR 723.1a and the resolver prevents it. The four
   existing tests (`turns.rs:6887-6981`) must stay green (behavior preserved).
6. **`combat_phases_started_this_turn` not used for the latch.** Deliberate: it resets at turn start
   (`turns.rs:625`) and can't survive a cross-turn carry (4a). The latch uses entry-presence + release-
   before-activate instead. Confirm no reviewer expectation that the counter gate it.

---

## §9. REVIEW OUTCOME (/review-engine-plan, opus/xhigh) — AUTHORITATIVE; hard gates

**VERDICT: APPROVE-WITH-REQUIRED-REVISIONS.** Design sound, rules-correct, single-authority release confirmed.
Two rulings CONFIRMED: edge-(a) CARRY-not-lapse (official ruling: "you'll control the next combat phase or turn the
affected player actually takes"; mtg.wtf/tla/69); elimination.rs 4c fix SHIPS IN THIS INCREMENT (real gap, latent
Mindslaver bug, class-level ~8 lines, routes through release_control_at — splitting violates extend-not-fork).

### R1 [BLOCKER — highest material risk] — pin/route the additional_cost_paid flag
The paid-branch swap keys on `ability.context.additional_cost_paid` (verified effects/mod.rs:5489
AdditionalCostPaidInstead arm). BUT `AbilityCost::Waterbend` (KeywordCost, types/ability.rs:7380) routes through
Convoke-mode ManaPayment (casting.rs:14078 find_waterbend_cost → enter_payment_step(ConvokeMode::Waterbend)) which
does NOT appear to set the flag. The flag IS set by generic AdditionalCost::Optional (handle_decide_additional_cost
casting_costs.rs:129, pay=true). → Implementer MUST pin via parse_oracle_text + runtime cast trace which representation
"you may waterbend {10}" currently produces. If it doesn't set the flag: model "you may" as AdditionalCost::Optional
(PREFERRED — single authority) OR choose a different swap key. GATE: test 7.6 (unpaid→NextCombatPhase) AND 7.7
(paid→NextTurn) BOTH pass at RUNTIME (flag-never-set breaks 7.7; flag-always-set breaks 7.6). Not done on parse-shape alone.

### R2 [REQUIRED] — §6 churn map incomplete (compile-forced sites)
- coverage.rs:3002 — Effect::ControlNextTurn{target, grant_extra_turn_after} full destructure (no ..), MISSING from §6
  → add window: _. Compile error if missed.
- parser/oracle_effect/mod.rs:12298 — constructs TargetedImperativeAst::ControlNextTurn{...}; §6 marked mod.rs
  "optional" but the AST field-add is MANDATORY (compile error) → reclassify REQUIRED.
- Minor: oracle_trigger_tests.rs:17608 is a match destructure (window: _), not a literal — right site, wrong edit-shape label.
- Confirmed-mapped: control_next_turn.rs:34/58/80, turns.rs:6894/6930/6957/6964, game_state.rs:7658 def, ability_scan.rs:514
  (full destructure, forces #4904 re-classify). effects/mod.rs:2992 {..} + :4931 {target,..} frozen matches UNAFFECTED.

### R3 [REQUIRED] — carry test 7.3 revert-to-red is inaccurate
7.3 claims reverting the §2.3 NextTurn-gate drops the carried entry, but turns.rs:461 release is guarded by
`if turn_decision_controller.is_some()` and a carried NextCombatPhase entry has None throughout O's turn → block never
runs regardless of §2.3; entry survives even with §2.3 reverted → 7.3 does NOT discriminate. The REAL carry mechanism:
a wholly-skipped combat never enters finish_enter_phase(BeginCombat) so activation never fires (advance_phase:84-85).
FIX: retarget 7.3's revert to the actual mechanism (assert no activation + entry persists; discriminating revert =
removing the activate window-gate). §2.3's window-gate is defensively-correct-but-REDUNDANT (is_some guard + resolver
dedup) — keep for explicit intent, don't present as sole carry mechanism.

### Precision note (annotate, not blocking)
CR 506.7d governs spell-casting timing restrictions, NOT control-window duration — the first-only ruling uses it BY
ANALOGY. Annotate as analogy; the real enforcement is entry-presence + release-before-activate.

### CONFIRMED-SOUND (blessed — do not re-litigate)
Single release authority (release_control_at all 3 sites); hook at finish_enter_phase turns.rs:416 before priority-set
:426; release cond next==BeginCombat || !next.is_combat() (Phase is phase+step enum); first-only latch (ExtraPhase
re-enters BeginCombat → release-before-activate → rfind finds nothing); authorized_submitter_for_player unchanged (4d
correct-by-construction, 2HG normalize handled); anaphora auto-solve (apply_instead_swap parent.clone preserves targets);
variant gate ControlWindow serde-default NextTurn; all CR (723.1-5, 500.8/11, 506.1, 507, 511.3, 724.x, 800.4a/b grep-
verified); parser imperative.rs:295 nom alt; turn-boundary refactor safe (retain dedup ≤1/target, 4 tests preserved).
