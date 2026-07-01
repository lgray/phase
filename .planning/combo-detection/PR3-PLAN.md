## Option C — GameState detection ring (chosen by user)

> Worktree `/home/lgray/vibe-coding/wt-combo-pr3`, branch `feat/combo-detect-pr3` off main `5eca83b8c` (v0.7.0). Planning only — no code written. Every anchor re-opened and re-verified in `wt-combo-pr3` this round (file:line inline). Every CR grep-resolves in `docs/MagicCompRules.txt`.
>
> The §7 (`resource.rs::project_out_resources` stack-id canon), §8 (`loop_check.rs::live_mandatory_loop_winner`), and §9 (`engine.rs::no_living_player_has_meaningful_priority_action`) building blocks are ALREADY implemented (uncommitted) in this worktree and are **reused verbatim**. The strict CR 104.4b comparator (`loop_states_equal` / `normalize_for_loop` / the run_auto_pass_loop draw block) stays **byte-for-byte untouched**.

---

### Changes from round 3 (D+B → C)

| Round 3 (D+B) | Round 3 → This round (Option C) |
|---|---|
| Hook the WIN detection in `resolve_all_fast_forward` (engine_resolve_batch.rs); idx 17/18 **descoped** as live discriminators (they die via CR 704.5a). | Hook a **persisted bounded ring on `GameState`**, maintained at the single resolution seam **every driver shares**, detected at the single SBA-reconciliation seam every driver shares. idx 17/18 **win live under the default per-beat drive.** |
| Detection state is a **call-local** `VecDeque` (rebuilt every `apply()`); cannot accumulate across the per-beat single-apply drive (the architectural wall). | Detection state **persists across `apply()` calls** on `GameState` (`#[serde(skip, default)]`), so the per-beat drive accumulates the cycle. The wall is removed. |
| Two candidate sites (run_auto_pass_loop §10; resolve_all_fast_forward §6) — neither reaches idx 17/18 under its real drive. | **One** maintenance site + **one** detection site, both on the path **every** driver traverses (per-beat, server, resolve_all, run_auto_pass, bench). Option C **subsumes B** (it also fixes the Resolve-All over-drain) and makes the round-2 §10 wiring redundant. |
| §10 wiring kept (flagged unreachable). | §10 **WIN** wiring **removed** (redundant + a second divergent site); strict CR 104.4b **DRAW** block kept untouched. |

**Soundness verdict (re-derived for the per-beat drive, §4): SOUND.** The round-2 "inherently mandatory by construction" argument does not transfer (the per-beat drive offers the victim priority every beat). At this site the **§9 all-living-players gate is the entire firewall** and it is non-vacuous: any living player holding a meaningful response blocks the shortcut, so no live game is ever ended early. The shortcut is a legitimate CR 732.2a shortcut of a predictable sequence to its CR 704.5a end, checked at a legal CR 704.3 point strictly **after** the existing state-based actions.

---

## 1. The ring: data structure, location, and serde treatment (serialized-surface delta = **zero**)

### 1.1 The field

Add **one** field to `GameState` (`crates/engine/src/types/game_state.rs`, struct at `:5488`):

```rust
/// CR 732.2a loop-shortcut detection ring (PR-3). A bounded FIFO of recent
/// post-resolution NORMALIZED board snapshots, captured at the single resolution
/// seam (`game::priority::handle_priority_pass_with_limit`) and scanned at the
/// SBA-reconciliation seam (`game::engine::reconcile_terminal_result`). A
/// self-refilling MANDATORY cascade drives the engine one resolution per `apply()`
/// with no call-local window (the per-beat single-apply drive), so the window that
/// detects the loop MUST persist across `apply()` calls — hence on `GameState`.
///
/// TRANSIENT DERIVED STATE — `#[serde(skip, default)]`. It is never serialized: it
/// is rebuilt deterministically from play and is a pure optimization over the
/// existing CR 704.5a SBA (which already ends every realistic-life drain), so
/// losing it across a save/load/MP-snapshot boundary only defers the shortcut by a
/// few resolutions — never changes a winner. Snapshots are `Arc`-shared so the
/// frequent `GameState::clone` (AI search, §9 probes) pays O(ring.len()) refcount
/// bumps, not deep copies. INTENTIONALLY omitted from `impl PartialEq for GameState`
/// (derived state, like `static_source_index`/`static_gate_truth` — both
/// `#[serde(skip)]` AND eq-excluded) so AI-search dedup on
/// semantically-identical positions is unaffected.
#[serde(skip, default)]
pub loop_detect_ring: std::collections::VecDeque<std::sync::Arc<GameState>>,
```

Plus two associated `const`s (module-private, next to the field or in `engine.rs`):

```rust
/// Max retained snapshots. A determinate drain has period ≤ 2; 16 covers ≥ 8 cycles
/// and any loop whose repeating phase begins within a 16-resolution preamble. A
/// longer-period/longer-preamble loop simply falls back to the natural CR 704.5a SBA
/// death (fail-safe — never a wrong win). Kept small because the live `GameState`
/// carries it through every clone.
const LOOP_DETECT_RING_CAP: usize = 16;
```

**Why `Arc<GameState>` and not `GameState`:** `GameState` already uses `im::HashMap`/`im::Vector` (O(1) structural-sharing clone), so a bare `VecDeque<GameState>` would also clone cheaply — but `Arc<GameState>` makes the cost *explicitly* O(1)-per-element and self-documents "these are shared read-only snapshots, never mutated in place." Verified viable: `GameState` is `Send + Sync` (derived; RNG is `ChaCha20Rng`, itself `Send + Sync`), AI search is single-threaded (no `rayon`/threads), and the codebase has no `Rc` to preserve `!Send`. `Arc<GameState>` keeps `GameState: Send + Sync` (the server moves it into tokio tasks), which `Rc` would break.

### 1.2 Three edits the field forces (lockstep)

1. **`normalize_for_loop` (`:7754`) must clear the ring in its output:** add `clone.loop_detect_ring.clear();` alongside the existing field zeroing. Snapshots stored in the ring are produced by `normalize_for_loop`, so without this each snapshot would carry a clone of the live ring → recursive/quadratic growth. With it, every stored snapshot has an empty ring (clone depth = 1). This does **not** affect any comparison (the ring is excluded from `eq`).
2. **`impl PartialEq for GameState` (`:7822`):** do **nothing** — the ring is a `#[serde(skip)]` derived field and is omitted from `eq` by the same convention as `static_source_index`/`static_gate_truth` (both `#[serde(skip)]` AND eq-excluded; the comment MUST cite these, NOT `public_state_dirty`/`state_revision`/`layers_dirty`, which are `serde(skip)` but ARE compared in `eq` at `:7848/:7857/:7858` — measured). (Adding the ring to `eq` would be a bug: two semantically-identical positions reached with different recent-cascade histories must still dedup in AI search.)
3. **`apply_action` entry (`:1465`, after ALL preference early-returns — `CancelAutoPass`, `SetPhaseStops`, `ReorderHand`, and any Debug action):** clear the ring on any action that could break a mandatory cascade — see §2.3. (Clearing is fail-safe even if placed before a no-op preference return, but it must not sit ahead of a real cascade-continuing path; place it after the full preference early-return block.)

### 1.3 Serde / MP / replay / WASM analysis (the deliberate decision)

| Boundary | What crosses it | `#[serde(skip)]` effect | Determinism / correctness |
|---|---|---|---|
| **Save / load** | Full `GameState` via `serde_json` (engine-wasm `lib.rs:1113/1132/1187`; phase-ai `saved_state.rs`) | Field absent from JSON; deserializes to `default()` (empty `VecDeque`). | A game saved mid-cascade re-accumulates the ring as resolutions continue on load. Harmless — the shortcut is a pure optimization over CR 704.5a; at worst it fires a few resolutions later. |
| **MP server → client** | `filter_state_for_player` returns a **full** (filtered) `GameState` (`server-core/filter.rs:7`), broadcast directly (`session.rs`); no separate view type. | Field never serialized to the guest. | **Server is authoritative** (`session.rs` runs plain `apply()`); it builds its own transient ring deterministically from the action stream it processes. The guest renders engine output and does not run detection. No host/guest divergence. |
| **Replay** | Re-applies actions from an initial state. | N/A (rebuilt). | Identical action stream → identical ring → identical detection. Deterministic. |
| **WASM → JS** | `ClientGameState { state: GameState, derived }` (`derived_views.rs:228`) | Field absent from the JS payload. | Frontend is a display layer; never needs the ring. No TS/WASM-bridge change. |
| **`engine-inventory.json`** | Enum variants only (`engine-inventory-gen/main.rs:106`, `Item::Enum`). | A **struct field** is not catalogued. | **No inventory change.** |

**Serialized-surface delta: ZERO.** No wire-format change, no `engine-inventory.json` change, no WASM/TS boundary change, no new `GameEvent` (the shortcut reuses `GameOver { winner }` / `WaitingFor::GameOver`). This is the load-bearing reason `#[serde(skip, default)]` is correct rather than a serialized field: nothing downstream needs it, and it is reconstructable from play, so paying for MP/replay determinism via serialization would be dead weight that could only *introduce* divergence.

---

## 2. Maintenance site — one normalized sample per resolution at the shared seam

### 2.1 The seam (verified shared by every driver)

`crates/engine/src/game/priority.rs::handle_priority_pass_with_limit` (`:31`). When all living players have passed (`:46`) with a **non-empty** stack (`:77`), it calls `super::stack::resolve_next_with_limit` (`:82`; `stack.rs:1625`, returns `consumed`), updates auto-pass baselines (`:87-91`), and on a non-interactive result `reset_priority`s and returns `Priority{active}` (`:97-101`). **Every** driver reaches this exact code for a resolution:

- per-beat `apply(PassPriority)`: `engine.rs:1719` arm → `pass_priority_once_with_pipeline` (`:399`) → seam (`:416`). ✓
- `run_auto_pass_loop`: `engine.rs:1213` → `pass_priority_once_with_pipeline` → seam. ✓
- `resolve_all_fast_forward`: `engine_resolve_batch.rs:116` → `apply_action_boundary_with_stack_limit` → `apply_action` → seam. ✓
- server MP / bench: plain `apply()` → seam. ✓

Maintaining the ring **here** means it accumulates regardless of driver — the core architectural win of Option C.

### 2.2 The push (and the cheap refill gate)

In the non-empty-stack branch (`priority.rs:77-105`), capture the pre-resolution depth, and **after** `reset_priority` records one sample iff the resolution **refilled** the stack and left a non-interactive priority window:

```rust
let stack_len_before = state.stack.len();                       // before :82
let consumed = super::stack::resolve_next_with_limit(state, events, stack_resolution_limit);
// ... existing auto-pass baseline update (:87-91) ...
let wf = if matches!(state.waiting_for, WaitingFor::Priority { .. }) {
    reset_priority(state);
    WaitingFor::Priority { player: state.active_player }
} else {
    state.waiting_for.clone()
};
// PR-3 (Option C) ring maintenance — CR 732.2a window accumulation:
// REFILL GATE: a self-refilling mandatory cascade holds the stack non-empty across
// the resolution (len_after >= len_before). A NORMAL multi-spell stack SHRINKS
// (len_after < len_before) and so never snapshots — the ring stays EMPTY in normal
// play (near-zero per-resolution cost). Captured AFTER reset_priority so the
// post-resolution priority window (priority_player == active_player) is consistent
// across every cycle point (the modulo comparator compares priority exactly).
if matches!(wf, WaitingFor::Priority { .. })
    && !state.stack.is_empty()
    && state.stack.len() >= stack_len_before
{
    state.record_loop_detect_sample();   // push Arc::new(self.normalize_for_loop()); pop_front at cap
} else {
    state.loop_detect_ring.clear();      // cascade ended (stack drained) or hit an interactive choice
}
wf
```

`record_loop_detect_sample` is a small `GameState` method:

```rust
pub(crate) fn record_loop_detect_sample(&mut self) {
    if self.loop_detect_ring.len() == LOOP_DETECT_RING_CAP {
        self.loop_detect_ring.pop_front();
    }
    let snapshot = std::sync::Arc::new(self.normalize_for_loop()); // empty-ringed (see §1.2.1)
    self.loop_detect_ring.push_back(snapshot);
}
```

### 2.3 Reset/invalidation conditions

The ring may hold samples **only** from an uninterrupted run of mandatory resolutions (CR 732.2a: the shortcut sequence is pure predictable resolutions; a voluntary action breaks the loop). Three clears enforce that:

1. **Cascade end / interactive choice** (the `else` above): stack drained to empty, or the resolution opened a non-`Priority` `WaitingFor` (Scry/Search/target prompt). The cascade is no longer a self-refilling mandatory loop.
2. **Any non-pass action** — at `apply_action` entry (`:1465`), after ALL preference early-returns (`CancelAutoPass`, `SetPhaseStops`, `ReorderHand`, and any Debug action — not just the first two): `if !matches!(action, GameAction::PassPriority) { state.loop_detect_ring.clear(); }`. A cast/activate/play-land breaks the cascade. (Note: `run_auto_pass_loop` and `resolve_all_fast_forward` call `pass_priority_once_with_pipeline` *directly*, not via `apply_action`, so this clear does not fire during their internal iterations — the ring accumulates correctly there.)
3. **Refill gate miss** (§2.2): a shrinking resolution does not push and clears, so finite trigger chains and normal stack draining never leave stale samples.

**Soundness note (documented in code):** none of these clears is *required* for soundness — the board-equality gate (§3) only matches structurally-identical boards, and the §9 gate re-derives mandatory-ness at the detection instant — so a stale sample can at worst fail to match (fail-safe). The clears are for **bounded size + near-zero normal-play cost** (the ring is empty whenever a cascade is not actively self-refilling).

### 2.4 Per-resolution cost

- **Normal play:** the refill gate fails on every shrinking resolution → no snapshot, ring stays empty → the detection scan (§3) short-circuits on `ring.is_empty()`. Cost ≈ two `usize` compares + one `is_empty()` per resolution. Negligible.
- **During a self-refilling cascade:** one `normalize_for_loop` (cheap — `im::` structural sharing) + `Arc::new` per resolution, capped at 16 retained. The detection scan is O(ring × board) per `apply()` but the shortcut fires within a few resolutions, so total work is bounded and small.

---

## 3. Detection + shortcut — at the SBA-reconciliation seam, **after** CR 704 SBAs

### 3.1 The site and CR ordering

`crates/engine/src/game/engine.rs::reconcile_terminal_result` (`:219`), called at `apply_action_boundary_with_stack_limit:196` (after `apply_action`) and `:200` (after `run_auto_pass_loop`). Its existing body: the pending-player-loss SBA (`:228-229`, `has_pending_player_loss_sba` → `check_state_based_actions` → CR 704.5a), `ensure_game_over_if_terminal` (`:235`), and the GameOver transition (`:236-239`). **Append the loop-shortcut after `:239`:**

```rust
// CR 732.2a + CR 704.5a: shortcut a NET-PROGRESS mandatory cascade to its
// determinate single-opponent loss. Runs AFTER the CR 704 state-based actions
// above (CR 704.3 ordering), so a player ALREADY at 0 life loses via the real
// 704.5a SBA first and this never preempts or double-fires a legitimate win — it
// only fires when the game would otherwise grind on (high victim life, or mid-drain
// before 0). The `!GameOver` guard makes it idempotent across the :196/:200 calls.
if !matches!(state.waiting_for, WaitingFor::GameOver { .. })
    && matches!(state.waiting_for, WaitingFor::Priority { .. })   // a player would get priority (CR 704.3)
    && !state.stack.is_empty()
    && !state.loop_detect_ring.is_empty()
{
    // Clone the Arc handles (cheap refcount bumps) to release the borrow on `state`
    // before the mutation below.
    let priors: Vec<std::sync::Arc<GameState>> = state.loop_detect_ring.iter().cloned().collect();
    let cur = crate::analysis::resource::ResourceVector::snapshot(state);
    if let Some(winner) = priors.iter().find_map(|prior| {
        let delta = crate::analysis::resource::ResourceVector::delta(
            &crate::analysis::resource::ResourceVector::snapshot(prior),
            &cur,
        );
        crate::analysis::loop_check::live_mandatory_loop_winner(prior, state, &delta)
    }) {
        // CR 732.5: shortcut ONLY a loop NO living player can break. The gate runs
        // ONCE after find_map (not per prior). At the per-beat drive this is the
        // entire soundness firewall (see §4).
        if no_living_player_has_meaningful_priority_action(state) {
            result.events.push(GameEvent::GameOver { winner: Some(winner) });
            state.waiting_for = WaitingFor::GameOver { winner: Some(winner) };
            result.waiting_for = state.waiting_for.clone();
            match_flow::handle_game_over_transition(state);
        }
    }
}
```

**Reuse verbatim (no edits, no visibility bumps):** `live_mandatory_loop_winner` is `pub(crate)` (reachable from `engine.rs`); `no_living_player_has_meaningful_priority_action` is a private `fn` in the same module; `ResourceVector::snapshot/delta` are `pub`; `match_flow::handle_game_over_transition` already used at `:237`. **No new `GameEvent`, no new `pub(super)`** — a strict improvement over Option B (which needed a §9 visibility bump).

### 3.2 Why this is the correct CR-704.3/704.4 point and cannot double-fire

- **CR 704.3** checks SBAs "whenever a player would get priority." `reconcile_terminal_result` runs exactly when an action has settled and a player is about to receive priority; the guard `matches!(state.waiting_for, Priority)` confirms it. The CR 732 shortcut is naturally applied at the same timing (the loop's repeating point is a priority window — CR 732.2a requires the shortcut's endpoint be "a place where a player has priority").
- **Ordering:** the existing CR 704.5a SBA runs **first** (`:228-229`). If the victim is already at ≤0, the real SBA ends the game with the correct winner and sets `WaitingFor::GameOver` → the loop-shortcut block's `!GameOver` guard skips it. The shortcut therefore **anticipates** a future 704.5a loss; it never preempts a present one. The two are mutually exclusive (victim at 0 ⇒ SBA; victim > 0 ⇒ shortcut), so no double-fire and the winner is identical either way.
- **CR 704.4** ("SBAs pay no attention to what happens during resolution"): the ring is sampled only at *settled* post-resolution priority windows, never mid-resolution — consistent with SBA timing.
- **Idempotency across `:196`/`:200`:** if `:196` fires `GameOver`, `run_auto_pass_loop` sees a non-`Priority` `waiting_for` and breaks immediately; `:200`'s guard skips. If a `run_auto_pass_loop`-driven cascade accumulates samples *between* `:196` and `:200`, `:200` catches it.

### 3.3 The detection chain (reused §7 + §8 + §9)

1. **§7 board-equality (modulo monotone resources + volatile stack id):** inside `live_mandatory_loop_winner` → `detect_loop` → `loop_states_equal_modulo_resources` → `project_out_resources` (`resource.rs:588`). Confirmed (`resource.rs:557-558`) that `turn_number`/`phase`/`priority`/object-content still compare **exactly**, so cross-turn/cross-phase/board-changed samples cannot falsely match.
2. **§8 single-determinate-loser firewall:** `living.len()==2`, exactly one life-faller, no library/poison second loss path, `player_has_cant_lose(faller)` / `player_has_cant_win(winner)` on the **raw** `cycle_end`, and `WinKind::LethalDamage` confirmation. Returns `None` on any ambiguity.
3. **§9 all-living-players gate (CR 732.5):** `no_living_player_has_meaningful_priority_action` — the firewall (§4).

The raw `loop_fingerprint` is **deliberately not used as a pre-filter for the win path**: it hashes `player.life` (`game_state.rs:7725`), which is the very axis monotonically changing each cycle, so it would never match across a drain cycle. The modulo comparator (`project_out_resources` zeros life) is what matches. This mirrors the existing §10 win `find_map`, which also omits the fingerprint pre-filter. (A future *modulo* fingerprint — `hash(project_out_resources(state))` — could pre-filter to skip the projection on non-matching priors; deferred as an optimization, not needed for correctness with a 16-cap ring.)

---

## 4. Soundness re-derivation for the per-beat single-apply drive

The round-2 argument ("every iteration in `run_auto_pass_loop` is mandatory by construction because a meaningful action would have broken the auto-pass session") is **specific to `run_auto_pass_loop`** and does **not** transfer: at the per-beat drive the opponent receives priority **every beat** and the frontend *chooses* to pass. So passing is a choice, not a forced step. The soundness basis at this site is therefore **entirely** the §9 gate + the §7/§8 structural firewalls. The re-derivation:

**Claim.** At the detection instant, emitting `GameOver { winner }` is a legitimate CR 732.2a shortcut iff (a) the board returns identical modulo monotone life across two cycle points, (b) exactly one living opponent's life strictly falls each cycle with the winner immune to neither winning nor the loser to losing, and (c) **no living player has any meaningful priority action.** All three are enforced before `GameOver` is pushed.

1. **The actions are non-conditional and mandatory at the loop point (CR 732.2a "predictable results", "no conditional actions").** The cascade is a chain of mandatory triggered abilities (idx 18 fully non-targeted: Marauding Blight-Priest "each opponent loses 1 life" + Bloodthirsty Conqueror "you gain that much life"). The §9 gate confirms that at the loop point **no player has a meaningful action** — so the only "choice" available to any player is to pass (or take a loop-irrelevant mana action), which does not alter the outcome. Hence the sequence is predictable for all players (CR 732.2a). This is the property the per-beat drive does **not** give us for free, and the §9 gate supplies it directly.
2. **The §9 gate genuinely fires when any living player has a meaningful response (no live false positive).** `no_living_player_has_meaningful_priority_action` probes **each** living player as the priority holder (`probe.auto_pass.clear(); probe.waiting_for = Priority{p}`) and asks `has_meaningful_priority_action`. That predicate (`ai_support/mod.rs:723`) returns `true` for any legal non-pass, non-trivial-mana action — including a castable instant (`CastSpell` → the `_ => true` arm at `:730`), a meaningful activated ability, or a sac-for-mana ability (`:737-746`). So a victim (or controller) holding **any** removal / lifegain / counter / Stifle response makes the gate return `false` → the block **does not** fire → the cascade falls through and the player gets their beat to respond. **No real game with a live response is ended early.** Probing *every* living player (not just the current holder) is load-bearing because at the detection instant priority has reset to the active player, so the victim is not the current holder yet still must be consulted (proven by `U-gate`, `engine.rs:1126`).
3. **CR 732.5 is satisfied.** "No player can be forced to perform an action that would end a loop other than actions called for by objects in the loop." The shortcut fires only when *no* loop-ending action **exists** for any player — so no player is forced past an available loop-ender.
4. **It is a WIN, not a CR 732.4 draw.** CR 732.4 makes a loop of only mandatory actions a *draw* — but only when the loop **truly repeats** (net-zero). The drain makes **net progress** (one opponent −1/cycle), so the board is *not* identical (life falls); it is identical only **modulo** the monotone life axis (§7). The strict CR 104.4b/732.4 path (`loop_states_equal` without projection) fires on net-zero and runs first in `run_auto_pass_loop`; the modulo win path fires only on net progress. The single-faller guard (§8) makes a mutual/net-zero drain return `None`, so the win path can never hijack a draw (proven by `live_winner_mutual_drain_is_none`, `live_winner_net_zero_is_none`).
5. **The predicted end is the same as the real one.** Each cycle removes 1 life deterministically; after finitely many cycles the faller reaches 0 and loses via CR 704.5a with `winner` as the sole survivor (CR 104.2a). The shortcut names that same `winner` earlier. The `player_has_cant_lose` firewall (§8) refuses to shortcut when the faller is immune (Platinum Angel / "you can't lose"), leaving it a genuine non-termination exactly as today.

**Conclusion.** At the per-beat drive the shortcut is a CR 732.2a shortcut of a predictable, all-players-mandatory (per the §9 gate) net-progress sequence to its determinate CR 704.5a end, checked at a legal CR 704.3 point after CR 704 SBAs. **Sound.** Relationship to CR 732: this is the *shortcut* rule (732.2a) governed by the *no-forced-loop-ender* rule (732.5, the §9 gate), distinct from the *mandatory-draw* rule (732.4, the untouched strict path).

---

## 5. Step-0 non-vacuity (mandatory) + discriminating tests through the REAL per-beat drive

### 5.1 Step-0 measurement (before asserting anything — the non-vacuity proof)

Drive **idx 18** (Blight-Priest + Conqueror, fully non-targeted) through **repeated `apply(GameAction::PassPriority)`** (the actual per-beat driver — *not* `run_auto_pass_loop`, *not* `resolve_all_fast_forward`, *not* a synthetic window), seeded by one external P0 life-gain to start the cascade, at **high** P1 life (e.g. 200) so the natural 704.5a death cannot be the cause of any `GameOver`:

1. **WITHOUT the §3 detection block (and/or with §2 maintenance removed):** capture that repeated `apply(PassPriority)` resolves one entry per two beats, the ring (if maintained) modulo-matches across the period-2 cycle, but **no** `GameOver` is emitted — the cascade grinds toward the high-life 704.5a death (or, if maintenance is also removed, stalls/loops indefinitely). Confirm the §7 fail-safe preconditions on two same-phase cycle points: `stack.len()==1`, `trigger_event` byte-stable (`LifeChanged`, no volatile id), `subject_match_count == None`, `die_result == None`, `stack_paid_facts`/`stack_trigger_event_batches`/`pending_trigger_entry` empty/None. **This is the revert that proves non-vacuity.**
2. **WITH §2 + §3:** capture `GameOver { Some(P0) }` emerging from the **persisted ring** after a few `apply(PassPriority)` calls (≪ the ~400 the high-life 704.5a death would need), proving the persisted accumulation is what fires it.
3. **idx 17** (Sanguine + Exquisite, **targeted** `LoseLife`): measure whether the per-beat drive auto-resolves the sole legal target without a `WaitingFor::SelectTargets` stop. If it **auto-resolves** → it is drivable here and is promoted (§5.3). If it **stops** on target selection → document as targeting-deferred (the per-beat drive halts at the prompt; the ring clears via §2.3 condition 1) and do **not** promote idx 17. Record the measured outcome.

### 5.2 Discriminating-test map (each names its revert-fail assertion)

Building-block unit tests **U1–U10 / U-draw / U-gate / U-stack are preserved unchanged** (they prove §7/§8/§9 in isolation; still non-vacuous). New **live** tests drive the real per-beat path (`GameRunner` repeatedly dispatching `PassPriority`):

| # | Claim | Setup (real cards, repeated `apply(PassPriority)`) | Assert | Revert that breaks it |
|---|---|---|---|---|
| **C-L1** | persisted ring wins idx 18 under the **default per-beat drive** | idx 18 board; both hands empty / no castable / no activated (so §9 passes); P1 life **high** (200) so natural death can't be the cause; seed one P0 life-gain; then loop `apply(PassPriority)` ~N times | terminal `waiting_for == GameOver{Some(P0)}`; emitted well before 2×200 resolutions | (a) remove the §3 detection block ⇒ grinds toward high-life death, no early `GameOver` — FAILS; (b) remove the §2 ring maintenance ⇒ no persisted window, no `GameOver` — FAILS (proves the **persisted** ring is load-bearing, i.e. the per-beat wall is the real driver) |
| **C-L2** | §7 stack-id canon is load-bearing at the live site | idx 18 board as C-L1 | `GameOver{Some(P0)}` | revert the `entry.id = ObjectId(pos)` loop in `project_out_resources` (`resource.rs:742`) ⇒ cascade never modulo-matches (fresh stack ids) ⇒ no `GameOver` — FAILS |
| **C-neg-A** | a victim with a meaningful response is **not** declared a loser (live false-positive guard) | idx 18 board; **P1 (victim) holds a castable instant** (e.g. a removal/lifegain spell with mana up) | **no** `GameOver{winner}`; the drive stops to let P1 act (engine `auto_pass`/§9 both decline) | remove the §9 gate call in §3 ⇒ emits `GameOver{Some(P0)}` while P1 had an out — FAILS (proves the all-players gate is the live firewall) |
| **C-neg-B** | §9 probes the **non-current** holder | idx 18 board; **controller P0 holds a meaningful activated ability** but priority has reset to P0/active; P1 empty | gate returns `false` ⇒ **no** `GameOver` | swap §9 for the current-holder-only `priority_player_has_meaningful_action` ⇒ misses the masked holder ⇒ wrong `GameOver` — FAILS |
| **C-neg-C** | net-zero/mutual drain not hijacked into a win | a mutual-drain board (both lose 1/cycle) under the per-beat drive | **no** `GameOver{winner}` (single-faller guard ⇒ `None`) | drop `life_fallers.len()==1` in §8 ⇒ wrong `Some` — FAILS |
| **C-neg-D** | non-loop multi-spell stack is unaffected (ring stays empty in normal play) | a finite multi-spell stack that drains to empty | terminal normal `Priority`/empty stack, **no** `GameOver`; assert `loop_detect_ring.is_empty()` after each shrinking resolution | — (proves the refill gate keeps normal play out of the ring and the modulo gate never fires on a non-loop) |
| **C-neg-E** | faller immune ⇒ genuine non-termination, not a wrong win | idx 18 board; P1 has "you can't lose the game" (Platinum Angel) | **no** `GameOver` (drive does not end the game) | remove the `player_has_cant_lose` firewall in §8 ⇒ ends a game P1 can't lose — FAILS |

All live tests are **discriminating** (each row's named revert flips the asserted outcome) and **non-vacuous** (C-L1's revert (b) proves the persisted ring — the new surface — is what fires the win; without it the cascade does not end early).

### 5.3 Corpus bookkeeping (`corpus_tests.rs`)

`DRIVEN_ROW_INDICES` (`:1927`, currently `[0,1,4,6,9,10,12,13,14,49]`) is the set of rows "confirmed end-to-end ... through the real `apply()` pipeline." Option C's live shortcut makes idx 18 (and conditionally idx 17 per §5.1.3) win through the **real `apply()` pipeline** — which is exactly that contract.

- Add a `drive_*` live test (the C-L1 shape: real board → real repeated `apply(PassPriority)` → assert `GameOver`) for **idx 18**, and **promote idx 18** to `DRIVEN_ROW_INDICES`. Promote **idx 17 only if** Step-0 (§5.1.3) shows its targeted trigger auto-resolves under the per-beat drive; otherwise leave it documented as targeting-deferred.
- `confirmed_drivers_match_expected` (`:1942`) only asserts `gated_on.is_none()` for driven rows — both 17/18 have `gated_on == None`, so it passes after promotion.
- Update the **DRAIN FEEDBACK bucket doc** (`:1908`) to record the measured finding: these cascades are now **live-shortcut-driven via the persisted ring under the default per-beat drive** (CR 732.2a → CR 704.5a), replacing the "bespoke driver follow-up" note for the promoted row(s).
- Update the `loop_check.rs` module doc: `live_mandatory_loop_winner` is now reached from the reducer via the **`reconcile_terminal_result` ring scan** (the round-2/§10 doc says `run_auto_pass_loop`), and via the shared seam, for **every** driver.

---

## 6. Disposition of the dead round-2 §10 wiring — **remove the WIN block, keep the DRAW block**

`run_auto_pass_loop`'s WIN block (`engine.rs:1261-1305`: the `find_map` over the local `loop_window` calling `live_mandatory_loop_winner` + the §9 gate + `GameOver`) is **redundant under Option C**: `run_auto_pass_loop` resolves via the same shared seam (`:1213` → seam), so the persisted ring accumulates during it, and `reconcile_terminal_result` (`:200`, after `run_auto_pass_loop` returns) runs the §3 detection on that ring. Keeping the §10 WIN block would create **two divergent win-detection sites** — exactly what the orchestration standard warns against. **Remove it.**

- **Remove** only the WIN block (`:1261-1305`) and its now-unused locals if any (`cur`/the win `find_map`).
- **Keep byte-for-byte:** the strict **CR 104.4b DRAW** block (`:1230-1259`) and its local `loop_window`/`mandatory_iters`/`FINGERPRINT_AFTER_ITERS`/`MAX_LOOP_WINDOW` — that path is pre-existing, out of PR-3's scope, and the team-lead mandate keeps the strict comparator untouched. (Consequence: during a *sustained* `run_auto_pass_loop` cascade the engine snapshots twice — the local `loop_window` for the draw path and the persisted ring for the win path — a minor, bounded duplication only on that rare path. Documented, not optimized, to keep the draw path untouched.)
- The persisted-ring win detection (§3) is the **single** win site for all drivers; the local draw detection (`run_auto_pass_loop`) remains the draw site (a separate concern; extending draw detection onto the ring is explicitly out of scope).

No `live_mandatory_loop_winner` / `no_living_player_has_meaningful_priority_action` becomes dead — both relocate to §3.

---

## 7. Maintainer-simulation matrix for `loop_detect_ring` (the serialized-surface gate)

| Axis | Resolution |
|---|---|
| **Authority (who writes it)** | The engine, at exactly two sites: `priority::handle_priority_pass_with_limit` (push/clear, §2) and `engine::apply_action` entry + the §2.3 cascade-end clear. No transport/frontend writes — it is `#[serde(skip)]`, so adapters cannot even observe it. |
| **Binding time (when set)** | Push: at a settled post-resolution priority window during a self-refilling mandatory cascade (refill gate). Clear: cascade end, interactive choice, or any non-`PassPriority` action. |
| **Storage (where/how)** | `#[serde(skip, default)] VecDeque<Arc<GameState>>` on `GameState`. Bounded `LOOP_DETECT_RING_CAP = 16` (FIFO eviction). Snapshots are `normalize_for_loop` outputs with their own ring cleared (§1.2.1) → clone depth 1, no recursion. `Arc` ⇒ O(ring.len()) refcount bumps per `GameState::clone`. |
| **Consumer (who reads)** | Only `engine::reconcile_terminal_result` (§3), via `live_mandatory_loop_winner` + the §9 gate. No serialized consumer (save/MP/WASM never see it). |
| **Invalidation** | §2.3: cascade end / interactive choice / non-pass action / refill-gate miss. On deserialize: `default()` (empty), rebuilt from play — harmless because the shortcut is a pure optimization over CR 704.5a. |
| **Serialized surface** | **Zero.** Not in save JSON, MP broadcast, or WASM→JS payload; `engine-inventory.json` catalogues enum variants only (struct field invisible); no new `GameEvent` (reuses `GameOver`); no WASM/TS change. Omitted from `impl PartialEq` (derived-state convention) so AI-search dedup is unchanged. |
| **Hostile fixtures (must not break it)** | (a) **Deserialized state mid-cascade** → empty ring, re-accumulates, shortcut fires a few resolutions later — no wrong winner. (b) **Huge board (~2,936 permanents) drain** → snapshots/scan O(board) but bounded by cap 16 and quick firing; ring empty outside the cascade. (c) **AI search cloning during a cascade** → cheap `Arc` clones, ring excluded from `eq` so dedup unaffected. (d) **Player casts an instant mid-cascade** → §2.3 clears the ring AND the board changes (no modulo match) AND the §9 gate would block — triple fail-safe. (e) **3+ players** → §8 `living.len()==2` guard returns `None`. (f) **Mutual/net-zero/decking/poison second-loss** → §8 single-faller + library/poison guards return `None`. (g) **Can't-lose faller / can't-win winner** → §8 firewall returns `None`. (h) **Cross-turn/phase samples** → modulo comparator compares turn/phase exactly → no match. |

---

## 8. Implementation gate (risk-scaled; §7 touches shared PR-2 code)

This change touches a serialized-surface `GameState` field, the shared resolution seam, the SBA-reconciliation seam, and (via §7 reuse) the shared `analysis::` modulo comparator. It is **non-trivial engine plumbing** → full verification, Tilt-first:

1. `cargo fmt --all` (always direct).
2. **Parser/AST gate** N/A (no parser change).
3. **Full `analysis::` suite as no-regression** (mandatory — §7/§8/§9 are shared PR-2 code; U1–U10/U-draw/U-gate/U-stack must stay green): `tilt logs test-engine` filtered to `analysis::` after the build settles.
4. **Engine build + clippy + full test-engine** via Tilt (`./scripts/tilt-wait.sh clippy test-engine`): the new field, the seam maintenance, the reconcile detection, the live `C-*` tests, and the `run_auto_pass_loop` §10 removal.
5. **WASM** (`tilt logs wasm`): confirms the `#[serde(skip)]` field and `Arc<GameState>` compile to WASM (no `Send` regression, no serde surface).
6. **`check-frontend`**: must stay green — but expect **no** change (no wire/TS surface).
7. Confirm `data/engine-inventory.json` is **unchanged** by this diff (struct field, not an enum variant).
8. AI no-regression spot check (`cargo ai-gate`) is **not** required by this change (no policy/scoring touch), but a quick AI-vs-AI smoke (no early-`GameOver` regressions on non-loop games) is prudent given the reconcile-site addition.

Do **not** run `cargo build/clippy/test` directly while Tilt is up (target-lock contention); use `tilt logs` / `tilt-wait.sh`.

---

## 9. CR greps (verified in `docs/MagicCompRules.txt` this round, line inline)

- **CR 104.2a @330** — a player wins if all opponents have left (2-player determinism; the named `winner`).
- **CR 104.4b @366** — mandatory-loop **draw** (the strict path — untouched).
- **CR 117.4 @958** — all pass in succession → top of stack resolves (the seam).
- **CR 608.2 @2785** — resolution steps.
- **CR 704.3 @5485** — SBAs checked **whenever a player would get priority** (the §3 detection timing).
- **CR 704.4 @5487** — SBAs ignore mid-resolution state (sample only at settled windows).
- **CR 704.5a @5492** — 0-or-less life loses (the SBA that fires first, and the determinate end the shortcut anticipates).
- **CR 704.5b @5494** — empty-library draw loss (§8 decking second-loss firewall axis).
- **CR 704.5c** — poison loss (§8 poison second-loss firewall axis).
- **CR 732.2 @6370 / 732.2a @6372** — the shortcut procedure: a priority holder may shortcut a predictable, non-conditional sequence (a loop) ending at a priority point (the live-win authority).
- **CR 732.4 @6383** — a loop of only mandatory actions is a **draw** (net-zero; the strict path, NOT the net-progress win).
- **CR 732.5 @6385** — no player forced past an available loop-ending action (the §9 gate).
- **CR 101.2 / 104.2b / 104.3b** — can't-win / can't-lose (the §8 firewall; CR 810.8a 2HG team rule correctly excluded — strict 2-player).

---

## 10. Summary for the orchestrator

- **Architecture:** a single bounded `#[serde(skip, default)] loop_detect_ring: VecDeque<Arc<GameState>>` on `GameState`, **maintained at the one resolution seam every driver shares** (`handle_priority_pass_with_limit`, behind a cheap refill gate so normal play stays empty) and **scanned at the one SBA-reconciliation seam every driver shares** (`reconcile_terminal_result`, after CR 704 SBAs). This removes the per-beat single-apply wall: the persisted window accumulates the cycle across `apply()` calls, so idx 17/18 win **live under the default drive**, and Option C **subsumes** the round-3 B (it also fixes Resolve-All over-drain) and makes the §10 wiring redundant.
- **Serialized surface: ZERO** — `#[serde(skip)]` is invisible to save/load, MP broadcast, and WASM→JS; no `engine-inventory.json` change; no new `GameEvent`; no WASM/TS change. Transient, rebuilt from play, server-authoritative determinism, harmless to lose across boundaries.
- **Soundness (per-beat drive, re-derived §4): SOUND** — the §9 all-living-players gate is the entire firewall and is non-vacuous (a castable response blocks the shortcut); the net-progress drain is a CR 732.2a shortcut to the determinate CR 704.5a loss (not a CR 732.4 draw), checked at a legal CR 704.3 point **after** the existing SBAs (no preempt/double-fire).
- **Reuse verbatim:** §7/§8/§9 unchanged; strict CR 104.4b comparator and the `run_auto_pass_loop` DRAW block byte-for-byte untouched. **No new visibility bumps** (detection lives in `engine.rs` alongside the private §9 fn and the `pub(crate)` §8 fn).
- **Disposition:** remove the §10 WIN block (redundant, second divergent site); keep the DRAW block.
- **Tests:** Step-0 non-vacuity through the **real repeated `apply(PassPriority)`** drive; `C-L1`/`C-L2` positives (revert removes the ring/§7 ⇒ no early `GameOver`), `C-neg-A..E` soundness negatives (victim-response / masked-holder / mutual-drain / non-loop / can't-lose). Promote idx 18 (and conditionally idx 17) to `DRIVEN_ROW_INDICES`; refresh the DRAIN FEEDBACK + `loop_check.rs` docs.
- **Gate:** full Tilt-first verification including the complete `analysis::` suite as no-regression (§7 touches shared PR-2 code).
- **No engine/transport boundary violation, no soundness gap** found — Option C is realizable as specified. Implementation-ready.

---

## r3 review resolution (2026-06-27) — APPROVED with corrections applied

Independent adversarial review (pr3-plan-reviewer-c) verdict: **0 BLOCKERS, 0 HIGH** — design soundness-CLEAN and approvable. The three load-bearing questions (serde-skip MP/replay determinism; refill-gate false-positive vectors; detection site + SBA ordering) all answered SOUND. Corrections folded in:

- **[MEDIUM — fixed]** §1.1 field doc + §1.2.2 eq-exclusion precedent was factually wrong. MEASURED in `crates/engine/src/types/game_state.rs`: `layers_dirty` (`:7848`), `public_state_dirty` (`:7857`), `state_revision` (`:7858`) are all `#[serde(skip)]` but **ARE compared** in `fn eq` (7823+). The correct `serde(skip)`+eq-excluded precedent is `static_source_index` (`:5685`, carries the explicit "INTENTIONALLY omitted from impl PartialEq" comment) and `static_gate_truth` (`#[serde(skip)]`, absent from eq body). `devour_eligible_snapshot` is eq-excluded but `skip_serializing_if` (not `serde(skip)`) → dropped from the cite. The planted code comment MUST cite `static_source_index`/`static_gate_truth` only.
- **[LOW-1 — fixed]** §1.2.3 / §2.3.2 ring-clear must sit after ALL preference early-returns (`CancelAutoPass`, `SetPhaseStops`, `ReorderHand`, Debug), not just the first two. (Fail-safe even if it clears on a no-op.)
- **[LOW-3 — hard gate, already mandated §5.1]** Must OBSERVE a real `GameOver` from the real per-beat `apply(PassPriority)` drive before promoting `DRIVEN_ROW_INDICES` — green-compile is insufficient. This is the first live exercise of the modulo-match win (the §10 path was never reached live per IMPL-STOP-r1; U1–U10 use `start = end.clone()`).
- **[LOW-4 — out of scope, recorded]** A pure net-ZERO mandatory DRAW remains undetected under the per-beat drive (`run_auto_pass_loop` breaks immediately with no auto-pass session ⇒ DRAW block never runs). Pre-existing, NOT a regression — Option C adds only the WIN path. Do not attempt to fix; do not claim draw coverage.
- **[LOW — no change]** CR 732.2a framing (engine automating a forced no-action loop) accurate.

Base note: this plan + review are measured against `wt-combo-pr3` @ `5eca83b8c` (v0.7.0). MEASURED that v0.8.0's engine.rs churn (+564) is in test modules / unrelated handlers; `reconcile_terminal_result` (`:219`, body unchanged), `run_auto_pass_loop` (1095→1135, body unchanged), and the `priority.rs` seam (31→32) are structurally intact, and game_state.rs/loop_check.rs/resource.rs/corpus_tests.rs are unchanged in v0.8.0. ⟹ implement on this base; ship-time cherry-pick onto upstream/main hits untouched regions (clean, like MSH-F #4471).
