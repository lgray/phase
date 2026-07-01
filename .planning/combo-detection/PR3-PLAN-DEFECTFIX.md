# PR-3 Option C — Defect re-plan (Defect-1 seam frame + Defect-2 re-entrancy)

> Worktree `/home/lgray/vibe-coding/wt-combo-pr3`, branch `feat/combo-detect-pr3` off `5eca83b8c` (v0.7.0). Planning only — no engine source edited. Every anchor re-opened in this worktree (uncommitted Option C scaffold) with file:line inline. Every CR grep-resolves in `phase-rs-workdir/docs/MagicCompRules.txt` (line numbers inline).
>
> **Inputs:** `PR3-PLAN.md` (Option C, user-chosen), `PR3-IMPL-REPORT.md` (the executor STOP-AND-RETURN — the two measured defects), `PR3-IMPL-LOG.md`. This doc resolves the two defects and nothing else. The §7/§8/§9 building blocks, the strict CR 104.4b DRAW block, and the r3 MEDIUM eq-precedent comment stay **as-is** (re-verified untouched below).

---

## 0. The two defects in one sentence each (measured)

- **Defect-1 (inert seam frame).** §2 maintenance sits inside `priority::handle_priority_pass_with_limit` (`priority.rs:108-133`), but the trigger that REFILLS a self-refilling mandatory cascade is placed by `engine_priority::run_post_action_pipeline`, which runs **after** the seam returns (`engine.rs:476`). At the seam the stack has shrunk, so the gate never fires → ring stays empty (measured `len==0` over 200 beats, P1 200→150).
- **Defect-2 (re-entrant SIGABRT).** §3 detection lives in `reconcile_terminal_result` (`engine.rs:219`), which runs inside **every** `apply_action_boundary_with_stack_limit` (`:196`, `:200`) — INCLUDING the nested `apply_as_current` that `SimulationFilter` (`ai_support/filter.rs:112-113`) runs for legality. The §3 → §9 gate (`engine.rs:510`) calls `legal_actions` → `SimulationFilter` → nested apply → `reconcile_terminal_result` → §3 → … ∞ once the ring is populated (measured SIGABRT).

---

## 1. Defect-1 fix — relocate §2 maintenance to the post-pipeline frame, with a precise resolution-occurred gate

### 1.1 The exact relocated seam (file:line)

`crates/engine/src/game/engine.rs::pass_priority_once_with_pipeline` (`:438`). This is the **single wrapper every production PassPriority driver traverses** — the only non-test caller of `handle_priority_pass_with_limit` is `:455`, inside this function (verified: the other `handle_priority_pass(_with_limit)` call sites in `priority.rs:28,225-529` are the test-only wrapper / `#[cfg(test)]` module). The three production callers of `pass_priority_once_with_pipeline` are:

- per-beat `apply(PassPriority)` → `apply_action` `(Priority, PassPriority)` arm `engine.rs:1736` → `:1742`. ✓
- `run_auto_pass_loop` → `engine.rs:1252`. ✓
- `SetAutoPass` immediate pass → `engine.rs:4630`. ✓
- `resolve_all_fast_forward` → `apply_action` → `:1742`. ✓ (routes through the per-beat arm)

The current body (`:443-484`):

```
449   let stack_was_empty = state.stack.is_empty();
455   let wf = priority::handle_priority_pass_with_limit(current_seat, state, events, limit);  // resolves here
461   sync_waiting_for(state, &wf);
469   if matches!(state.waiting_for, Priority { .. }) { effects::drain_pending_continuation(...) }
476   let wf = engine_priority::run_post_action_pipeline(state, events, &state.waiting_for.clone(), skip_triggers)?;  // REFILL placed here (CR 603.3)
482   sync_waiting_for(state, &wf);
483   Ok(wf)
```

The refill trigger is on the stack only **after `:476`**. Sample at **`:482`/`:483`** (post-`sync_waiting_for`, pre-`Ok`).

### 1.2 The precise resolution-occurred gate predicate (answers the team-lead caveat AND the accumulation trap)

Capture at function entry (alongside `:449`):

```rust
let stack_len_before = state.stack.len();
let stack_top_before = state.stack.last().map(|e| e.id);   // StackEntry.id: ObjectId (resource.rs §7 canonicalizes it)
```

Insert AFTER `:482` (`sync_waiting_for(state, &wf)`), BEFORE `:483` (`Ok(wf)`):

```rust
// PR-3 (Option C) CR 732.2a loop-shortcut window accumulation — relocated here
// (PR3 Defect-1 fix). The refilling trigger is placed by run_post_action_pipeline
// (CR 603.3 / CR 704.3: triggers waiting to go on the stack are put there as a
// player would get priority), which runs at :476 — AFTER the resolution seam at
// :455. Sampling here is the only frame where a self-refilling cascade is already
// non-shrinking.
//
// RESOLUTION-OCCURRED GATE. `resolved_this_beat` is true iff there WAS a top entry
// at function entry and it is no longer the top — i.e. a stack entry was actually
// consumed/resolved this beat. A bare priority handoff (P0 passes, priority moves
// to P1, stack untouched) leaves the top unchanged ⇒ `resolved_this_beat == false`
// ⇒ the ring is LEFT INTACT so it keeps accumulating across the handoff beats that
// separate resolutions under the per-beat drive. (A naive `len >= before` gate would
// FALSE-POSITIVE on those handoffs; a strict clear-on-handoff would DESTROY the
// accumulation — both are wrong. This gate samples only on a real resolution and
// touches the ring only then.)
let resolved_this_beat =
    stack_top_before.is_some() && state.stack.last().map(|e| e.id) != stack_top_before;
if resolved_this_beat && !in_simulation_probe() {
    // REFILL gate: a self-refilling MANDATORY cascade holds the stack non-empty and
    // non-shrinking across the resolution, settling at a non-interactive priority
    // window that has reset to the active player (the canonical modulo-comparison
    // point — project_out_resources compares phase/priority EXACTLY). A normal
    // multi-spell stack SHRINKS; an interactive effect opens a non-Priority window;
    // a finite chain drains to empty — all three fall to the clear arm.
    if !state.stack.is_empty()
        && state.stack.len() >= stack_len_before
        && matches!(wf, WaitingFor::Priority { player } if player == state.active_player)
    {
        state.record_loop_detect_sample();   // push Arc::new(self.normalize_for_loop()); pop_front at cap
    } else {
        state.loop_detect_ring.clear();       // resolution ended the cascade (drain / shrink / interactive)
    }
}
// No else-branch: a bare handoff or an empty-stack pass-to-advance-phase does NOT
// touch the ring (leave-intact), so accumulation survives the inter-resolution beats.
```

**Why `resolved_this_beat = stack_top_before.is_some() && top_id changed` is exactly "a stack entry was resolved/consumed this beat":**

| Beat shape | `stack_top_before` | top after | `resolved_this_beat` | ring action |
|---|---|---|---|---|
| bare priority handoff (stack untouched) | `Some(A)` | `Some(A)` | **false** | **leave intact** ✓ (team-lead caveat resolved) |
| self-refilling cascade resolution (A resolves, B refills) | `Some(A)` | `Some(B)` (fresh ObjectId) | true | **sample** ✓ |
| finite chain drains to empty | `Some(A)` | `None` | true | clear (stack empty) ✓ |
| normal multi-spell shrink (A resolves, B beneath) | `Some(A)` | `Some(B)`, `len<before` | true | clear (shrink) ✓ |
| interactive stop (Scry/Search/target) | `Some(A)` | any, `wf != Priority` | true | clear (non-Priority `wf`) ✓ |
| empty-stack pass → advance phase | `None` | `None`/`Some(upkeep-trig)` | **false** | leave intact ✓ (no spurious sample) |

The fresh-`ObjectId` property of every refilled trigger (`next_object_id` is monotonic) guarantees a real resolution always changes the top id, and a bare handoff never does — so the predicate is exact and player-count-independent. The `wf == Priority{active_player}` clause double-excludes a non-active handoff for free and pins every retained snapshot to the same priority window the modulo comparator demands.

`!in_simulation_probe()` is the Defect-2 guard (§2 below): probes neither accumulate nor scan.

### 1.3 The old §2 block must be REMOVED (else it caps the ring at 1)

Delete the existing maintenance from `priority::handle_priority_pass_with_limit`:

- `priority.rs:81-82` — the `// PR-3 … capture pre-resolution depth` comment + `let stack_len_before = state.stack.len();` (now unused there; `consumed` at `:83` and the auto-pass-baseline loop `:89-93` and the `wf` compute `:99-106` STAY — they are pre-PR-3 logic).
- `priority.rs:108-133` — the entire PR-3 gate block (the `if matches!(wf, Priority) && !stack.is_empty() && stack.len() >= stack_len_before { record } else { clear }` and its KNOWN-INERT comment).

**Why removal is mandatory, not optional:** if the old block stays, every beat does `handle_priority_pass_with_limit` (old block runs at the *shrunk* frame: gate fails → `else` → **`clear()` the whole ring**) **then** the relocated block runs at the refilled frame (`record` one). Net: the ring can never exceed length 1 (cleared then re-seeded each beat) → no two cycle points → §3 never matches. The maintenance is a MOVE, not an addition.

### 1.4 Doc updates the relocation forces

- `game_state.rs:5687-5688` field doc — change "captured at the single resolution seam (`game::priority::handle_priority_pass_with_limit`)" to "captured at the post-pipeline frame of `game::engine::pass_priority_once_with_pipeline` (after `run_post_action_pipeline` places refilling triggers)". (Field treatment unchanged — see §3.)
- `loop_check.rs:23-25` module doc — same seam-name correction.
- `game_state.rs:7798` `record_loop_detect_sample` doc reference to the seam — same correction.

---

## 2. Defect-2 decision — **Option (a): thread-local re-entrancy guard** (top-level-only detection)

### 2.1 The re-entrancy path, traced in the real code (evidence)

```
apply(PassPriority)                                   engine.rs:163 → apply_action_boundary_with_stack_limit:180
  └ apply_action :195                                 (Priority,PassPriority) arm :1736 → pass_priority_once_with_pipeline:1742
  └ reconcile_terminal_result(state) :196             engine.rs:219
       └ §3 block :247  (ring populated + stack≠∅ + Priority)
            └ no_living_player_has_meaningful_priority_action(state) :267 / :510
                 └ probe = state.clone()  :512        ← CLONE COPIES loop_detect_ring (only serde+eq exclude it, NOT Clone)
                 └ legal_actions(&probe)  :516        → legal_actions_full:879 → validated_candidate_actions:41
                      └ FilterPipeline::default_pipeline (BasicLegalityFilter → SimulationFilter)  filter.rs:133
                           └ SimulationFilter::accept  filter.rs:107
                                └ let mut sim = state.clone()  :112   ← sim.loop_detect_ring populated
                                └ apply_as_current(&mut sim, PassPriority)  :113
                                     └ … → reconcile_terminal_result(sim)  → §3 block (sim.ring populated) → … ∞
```

Two measured facts make this exact:
1. **`GameState::clone` copies `loop_detect_ring`.** The field (`game_state.rs:5705-5706`) is a plain `pub` field with only `#[serde(skip, default)]`; it is excluded from serialization and from `impl PartialEq` (eq), but **not** from `Clone`. So `state.clone()` at `engine.rs:512` and `filter.rs:112` both carry the populated ring into the nested apply.
2. **`SimulationFilter` is the sole production clone-and-apply choke point.** `grep` of `ai_support/` for `apply_as_current`/`apply(` shows the only non-`#[cfg(test)]` clone-and-apply is `filter.rs:113`; `has_meaningful_priority_action` (`mod.rs:723`) inspects the action slice only (no apply), and `legal_actions` reaches an apply ONLY via `SimulationFilter`. So **every** nested legality apply — from the §9 gate, from `run_auto_pass_loop`'s `priority_player_has_meaningful_action` (`engine.rs:492,1217,1240`), and from top-level AI search — funnels through `filter.rs:113`.

### 2.2 Why (b) and (c) are rejected, and why (d) is insufficient — all from the trace

- **(b) "move detection to an outermost-only site" — REJECTED, measured.** There is **no** site that runs on the per-beat `apply(PassPriority)` path but not on the nested `SimulationFilter` apply: the nested apply goes through the **identical** `apply_as_current → apply → apply_action_boundary → apply_action_boundary_with_stack_limit → reconcile_terminal_result` chain (`filter.rs:113` → `engine.rs:362,168,177,180,196/200`). Moving §3 off `reconcile_terminal_result` to any earlier/later point on that chain either (i) is still on the nested path (no help) or (ii) leaves the per-beat reconcile seam, reintroducing the per-beat wall Option C was chosen to remove (PR3-PLAN §"Changes from round 3"). Option (b) violates the HARD CONSTRAINT.
- **(c) "non-simulating §9 predicate" — REJECTED.** `has_meaningful_priority_action` keys off `legal_actions`, whose precision depends on `SimulationFilter` (the authoritative legality oracle; the `cheap ⊆ sim` invariant in `filter.rs:193-217`). A `BasicLegalityFilter`-only predicate accepts a **superset** of legal actions → the gate would see phantom "meaningful actions" and refuse legitimate top-level shortcuts (a feature regression), and it **rewrites the reused §9 building block** the plan freezes. Wrong layer, degraded precision.
- **(d) "clear the ring in the `SimulationFilter` clone" — INSUFFICIENT (subtle, fragile).** Clearing `sim.loop_detect_ring` after `filter.rs:112` stops the *measured* §9-gate SIGABRT (there `priority_passes` is empty post-resolution, so a probe `PassPriority` is a 1-of-2 handoff that never re-resolves). **But** the relocated §2 site lives **inside** `pass_priority_once_with_pipeline`, which the probe's own `apply_as_current(sim, PassPriority)` executes — so a probe whose `priority_passes` is one-short-of-full (reachable from `run_auto_pass_loop`'s exit-check `priority_player_has_meaningful_action` at `:1217/:1240`, where `priority_passes == {P0}`) RE-RESOLVES and RE-ACCUMULATES a sample into the just-cleared `sim.ring`, then `reconcile_terminal_result(sim)` fires §3 again. Clearing only downgrades infinite → bounded-depth spurious detection inside legality probes, and its soundness rests on the non-obvious "`priority_passes` is empty whenever §3 runs" invariant rather than an explicit barrier. A legality oracle silently running game-ending shortcut logic on a throwaway clone is a category error (CLAUDE.md "separate abstraction layers"). Rejected in favor of (a), which forbids it unconditionally.

### 2.3 The decision: a thread-local probe guard, checked at the §3 detection site (soundness) and the §2 sample site (perf)

A `SimulationFilter` probe is a **hypothetical single-action legality test**, not part of the real CR 732.2a play sequence. Loop detection (the §3 scan) and ring accumulation (the §2 sample) are **TOP-LEVEL-ONLY**. Model "are we inside a legality/search probe?" as transient *execution context*, not game state — so a **thread-local**, not a `GameState` field.

**Definition (co-located with §2/§3 in `game/engine.rs`):**

```rust
thread_local! {
    /// PR-3 (Option C): set while inside a legality/search simulation probe
    /// (`ai_support::SimulationFilter`'s clone-and-apply). Loop-shortcut detection
    /// (`reconcile_terminal_result` §3) and ring accumulation
    /// (`pass_priority_once_with_pipeline` §2) are TOP-LEVEL-ONLY — a hypothetical
    /// single-action probe is NOT a real CR 732.2a play sequence, so it must neither
    /// shortcut nor accumulate. Engine game logic is single-threaded (no rayon /
    /// par_iter / std::thread::spawn in the apply or legal_actions path — verified),
    /// `apply()` is fully synchronous (no `.await` between set and restore), and the
    /// tokio server runs each apply synchronously within one task on one thread, so
    /// the RAII set/restore is balanced on a single thread within one call.
    static IN_SIMULATION_PROBE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// True while inside a `SimulationFilter` legality probe. Read by §2 and §3.
pub(crate) fn in_simulation_probe() -> bool {
    IN_SIMULATION_PROBE.with(|f| f.get())
}

/// RAII guard: sets the probe flag, restores the PREVIOUS value on drop (panic-safe,
/// nesting-correct — a probe that itself enumerates legal actions keeps the flag set).
#[must_use]
pub(crate) struct SimulationProbeGuard(bool);
impl SimulationProbeGuard {
    pub(crate) fn enter() -> Self {
        SimulationProbeGuard(IN_SIMULATION_PROBE.with(|f| f.replace(true)))
    }
}
impl Drop for SimulationProbeGuard {
    fn drop(&mut self) {
        IN_SIMULATION_PROBE.with(|f| f.set(self.0));
    }
}
```

**Set point (the ONLY one) — `ai_support/filter.rs:107-114`, around the nested apply:**

```rust
fn accept(&self, state: &GameState, candidate: &CandidateAction) -> bool {
    if super::structurally_valid_tap_for_convoke_payment(state, &candidate.action) {
        return true;
    }
    crate::game::perf_counters::record_state_clone_for_legality();
    let mut sim = state.clone();
    let _probe = crate::game::engine::SimulationProbeGuard::enter();   // ← PR-3 Defect-2 guard
    apply_as_current(&mut sim, candidate.action.clone()).is_ok()
    // _probe drops here → flag restored to prior value (panic-safe)
}
```

`filter.rs` already imports `crate::game::engine::apply_as_current` (`:32`) — add `SimulationProbeGuard` to that use.

**Check points (negated):**
1. **§3 detection guard — SOUNDNESS-CRITICAL.** `engine.rs:247-251`, add a conjunct:
   ```rust
   if !matches!(state.waiting_for, WaitingFor::GameOver { .. })
       && matches!(state.waiting_for, WaitingFor::Priority { .. })
       && !state.stack.is_empty()
       && !state.loop_detect_ring.is_empty()
       && !in_simulation_probe()                       // ← PR-3 Defect-2: never shortcut inside a probe
   { … }
   ```
   This alone stops the recursion: inside any probe the flag is set → §3 is skipped → no §9 gate → no nested `legal_actions` → no further `SimulationFilter` → the probe's `apply_as_current(...).is_ok()` returns normally.
2. **§2 sample gate — PERF.** Already shown in §1.2 (`resolved_this_beat && !in_simulation_probe()`): a probe does not waste a `normalize_for_loop` + `Arc` per resolution. Not soundness-load-bearing (its clone-ring is never scanned because §3 is guarded), but keeps probes free.

**Termination proof.** §3 fires only when `in_simulation_probe() == false`. The flag is `false` exactly at the outermost apply and `true` for the entire duration of every `SimulationFilter`-driven apply (set before `apply_as_current`, restored on drop, prev-saved so nesting stays `true`). Therefore §3 → §9 gate → `legal_actions` → `SimulationFilter` → nested apply executes its reconcile with the flag `true` → §3 skipped → the recursion has depth exactly 1 (one top-level detection that internally probes legality but those probes never re-detect). Top-level detection on the REAL per-beat `apply(PassPriority)` reconcile is unaffected (flag `false`). ∎

### 2.4 Why a thread-local, not a `state` flag (the trade-off the team-lead asked for)

| | thread-local `IN_SIMULATION_PROBE` (chosen) | `#[serde(skip, default)] in_probe: bool` on `GameState` |
|---|---|---|
| Serialized surface | **none** — not a `GameState` field; nothing to serde-skip, nothing to eq-exclude | adds a SECOND serde-skip+eq-excluded field; must be hand-omitted from `eq` per the `static_source_index`/`static_gate_truth` precedent (`game_state.rs:5684`) — the exact r3-MEDIUM footgun (a future hand who adds it to `eq` silently breaks AI-search dedup) |
| Nesting / propagation | RAII prev-save handles nesting; one set-site | propagates by clone (convenient) but every clone path must be reasoned about |
| Abstraction layer | correct — it is *execution context* ("are we in a probe?"), not game data | category error — execution context masquerading as game state on the serialized struct |
| Concurrency | safe: engine game logic single-threaded (no `rayon`/`par_iter`/`thread::spawn` in apply or `legal_actions` — verified), `apply()` synchronous, server applies per-task on one thread; idiom already in-engine (`speed.rs:140`, `perf_counters.rs:19`, `layers.rs:1369`, `quantity.rs:1029`) | safe but heavier surface |

The thread-local keeps the **serialized-surface delta at exactly the one field `loop_detect_ring`** (already correct) and avoids minting a second eq-excluded invariant. Decision: **thread-local.**

---

## 3. Serialized-surface analysis (any new flag)

- **No new serialized state.** Defect-2's guard is a thread-local → absent from save JSON, MP broadcast, WASM→JS, `engine-inventory.json`, and `impl PartialEq`. Defect-1 only **moves** the existing `loop_detect_ring` maintenance to another in-engine function and removes a now-unused local.
- **`loop_detect_ring` unchanged** (`game_state.rs:5705-5706`): `#[serde(skip, default)] VecDeque<Arc<GameState>>`, eq-excluded by the manual-`eq` convention, field doc already cites `static_source_index`/`static_gate_truth` (and explicitly NOT `public_state_dirty`/`state_revision`/`layers_dirty`, which are serde-skip but ARE compared at `game_state.rs:7848/7857/7858` — the r3 MEDIUM correction, re-verified present at `:5700-5704`). **Leave byte-for-byte.**
- **Net serialized-surface delta: ZERO** (unchanged from PR3-PLAN §1.3/§7). No new `GameEvent` (§3 reuses `GameOver { winner }` / `WaitingFor::GameOver`). No inventory regen (no enum variant). No WASM/TS change.

---

## 4. On-paper Step-0 win-path re-verification (idx 18) + idx 17 disposition

**Cards (idx 18, `corpus_tests.rs` row "Marauding Blight-Priest + Bloodthirsty Conqueror"):** Blight-Priest "Whenever you gain life, each opponent loses 1 life" (non-targeted, `LoseLife{Fixed 1, scope Opponent}`); Bloodthirsty Conqueror "Whenever an opponent loses life, you gain that much life" (non-targeted, `GainLife{EventContextAmount, Controller}`). Period-2 cascade: P0 +1 / P1 −1 per two resolutions (matches IMPL-REPORT §2 measurement). Setup: P1 life **200** (so a natural CR 704.5a death cannot be the cause), both hands empty / nothing castable / nothing activatable (so §9 passes), seed one external "P0 gains 1 life" trigger on the stack, then drive **repeated `apply(GameAction::PassPriority)`** (the real per-beat driver).

Trace **with BOTH fixes** (priority resets to active P0 after each refill; APNAP):

| beat | action | seam outcome | `resolved_this_beat` | ring | §3 (top-level) |
|---|---|---|---|---|---|
| 1 | PassPriority(P0) | handoff → `Priority{P1}` | false (top unchanged) | `[]` (intact) | skipped (empty) |
| 2 | PassPriority(P1) | all pass → resolve Blight-Priest (P1 200→199) → `run_post_action_pipeline` refills Conqueror trig → `Priority{P0}` | true (Aᵇᵖ→Bᶜᵒⁿq) | `[s1]` | runs; 1 prior == cur ⇒ delta 0 ⇒ `is_progress` false ⇒ `None` |
| 3 | PassPriority(P0) | handoff → `Priority{P1}` | false | `[s1]` (intact) | skipped |
| 4 | PassPriority(P1) | resolve Conqueror (P0 +1) → refill Blight-Priest → `Priority{P0}` | true | `[s1,s2]` | runs; no modulo match yet (s2 is the Blight-Priest window, s1 the Conqueror window) |
| 5 | PassPriority(P0) | handoff | false | `[s1,s2]` | skipped |
| 6 | PassPriority(P1) | resolve Blight-Priest (P1 199→198) → refill Conqueror → `Priority{P0}` | true | `[s1,s2,s3]` | **s3 modulo-matches s1** (both: Conqueror trig on stack, canon stack id, same phase/priority, board equal modulo life P1 199 vs 198 / P0 +k). `delta` over the s1→s3 cycle = P1 −1, P0 +1 ⇒ single faller P1 ⇒ §8 `live_mandatory_loop_winner ⇒ Some(P0)`. §9 gate: both hands empty, 2 living, no meaningful action for either ⇒ `true`. **⇒ `GameOver{Some(P0)}`** |

Each numbered claim the team-lead asked to confirm:
1. **Accumulates at the relocated resolution-gated seam** — yes: handoff beats `leave intact` (1.2), resolution beats `record` → ring reaches `[s1,s2,s3]` by beat 6 (vs the ~400 beats the high-life 704.5a death would need).
2. **Reaches §3 at the TOP-level reconcile** — yes: the per-beat `apply(PassPriority)` reconcile at `engine.rs:196` runs with `in_simulation_probe()==false`.
3. **Passes §9 WITHOUT recursing** — yes: §9 clones probes and calls `legal_actions` → `SimulationFilter::accept` sets the probe flag (`filter.rs`), so each probe's nested reconcile finds the flag set → §3 skipped → returns. Depth-1. No SIGABRT.
4. **Emits `GameOver{winner=P0}`** — yes (beat 6).

**Answer: YES — the on-paper Step-0 trace yields a LIVE `GameOver{Some(P0)}` for idx 18 with both fixes.**

**idx 17 (Sanguine Bond + Exquisite Blood — TARGETED `LoseLife`) disposition (per original §5.1.3).** Sanguine Bond's "target player loses that much life" carries a target; the per-beat drive reaches a `WaitingFor::TriggerTargetSelection`/`SelectTargets` window when that trigger needs its target UNLESS the engine auto-selects the sole legal target. If it **stops** on target selection: `run_post_action_pipeline` returns a non-`Priority` `wf` → the §1.2 gate's `else` arm clears the ring → no accumulation → idx 17 is **not** drivable through pure `PassPriority` and stays **targeting-deferred** (NOT promoted). If Step-0 measures **auto-resolution** of the sole legal target (no `SelectTargets` stop): idx 17 accumulates exactly like idx 18 and is promoted. **Disposition: leave idx 17 targeting-deferred; promote to `DRIVEN_ROW_INDICES` only if the re-measured Step-0 shows the targeted trigger auto-resolves under the per-beat drive.** The defect fixes unblock the measurement that the report could not run; the targeting question itself is unchanged.

---

## 5. Verification matrix — every C-test now NON-VACUOUS (each names its revert-fail line)

Building-block unit tests **U1–U10 / U-draw / U-gate / U-stack preserved unchanged** (they prove §7/§8/§9 against EMPTY-ring hand-built states — they never trip Defect-2 and stay green). New **live** tests drive the real per-beat path (`GameRunner` dispatching `PassPriority` repeatedly). With Defect-1+2 fixed, each row's named revert flips the asserted outcome (discriminating) and the win path actually fires (non-vacuous — the precondition the r1/IMPL STOPs flagged).

| # | Claim | Setup (real cards, repeated `apply(PassPriority)`) | Assert | Revert that breaks it (file:line) |
|---|---|---|---|---|
| **C-L1** | persisted ring wins idx 18 under the default per-beat drive | idx 18 board; hands empty; P1 life 200; seed one P0 life-gain; loop `apply(PassPriority)` | terminal `waiting_for == GameOver{Some(P0)}`, emitted ≪ 2×200 beats | (a) remove §3 block `engine.rs:247-278` ⇒ grinds, no early `GameOver` — FAILS; (b) remove the relocated §2 sample (the `record_loop_detect_sample` block in `pass_priority_once_with_pipeline`, §1.2) ⇒ ring never persists ⇒ no `GameOver` — FAILS (proves the **persisted ring** is load-bearing) |
| **C-L1-probe** | the Defect-2 guard makes detection top-level-only (the recursion is gone) | idx 18 board pre-driven to a populated ring (stack≠∅, `Priority`), then call `legal_actions(state)` directly | returns a bounded action list; process does **not** stack-overflow; no `GameOver` mutates the live state | remove `&& !in_simulation_probe()` from the §3 guard (`engine.rs:251`) ⇒ `legal_actions` re-enters `reconcile→§3→§9→legal_actions` ⇒ **stack overflow / SIGABRT** (the measured Defect-2) — test process aborts, cannot pass |
| **C-L2** | §7 stack-id canon is load-bearing at the live site | idx 18 board as C-L1 | `GameOver{Some(P0)}` | revert the `entry.id = ObjectId(pos)` canon loop in `project_out_resources` (`resource.rs` §7, ~`:742`) ⇒ fresh stack ids never modulo-match ⇒ no `GameOver` — FAILS |
| **C-neg-A** | a victim with a meaningful response is NOT declared a loser | idx 18 board; **P1 holds a castable instant** (mana up) | **no** `GameOver{winner}`; drive stops to let P1 act | remove the §9 gate call (`engine.rs:267`) ⇒ emits `GameOver{Some(P0)}` while P1 had an out — FAILS |
| **C-neg-B** | §9 probes the NON-current holder | idx 18 board; **controller P0 holds a meaningful activated ability**, priority reset to P0; P1 empty | gate `false` ⇒ **no** `GameOver` | swap §9 for `priority_player_has_meaningful_action` (`engine.rs:492`, current-holder-only) ⇒ misses the masked holder ⇒ wrong `GameOver` — FAILS |
| **C-neg-C** | net-zero / mutual drain not hijacked into a win | mutual-drain board (both lose 1/cycle) under the per-beat drive | **no** `GameOver{winner}` (single-faller guard ⇒ `None`) | drop `life_fallers.len()==1` in §8 (`loop_check.rs`) ⇒ wrong `Some` — FAILS (existing `live_winner_mutual_drain_is_none` U-test covers the unit level) |
| **C-neg-D** | non-loop multi-spell stack stays out of the ring | finite multi-spell stack that drains to empty | terminal normal `Priority`/empty stack, **no** `GameOver`; `loop_detect_ring.is_empty()` after each shrinking resolution | remove `state.stack.len() >= stack_len_before` from the §1.2 refill gate ⇒ a normal drain records snapshots (ring non-empty) — FAILS the `is_empty()` assertion (ring-hygiene discriminator) |
| **C-neg-E** | faller immune ⇒ genuine non-termination, not a wrong win | idx 18 board; P1 has "you can't lose the game" (Platinum Angel) | **no** `GameOver` | remove `player_has_cant_lose` firewall in §8 (`loop_check.rs` / `sba.rs:308`) ⇒ ends a game P1 can't lose — FAILS |

Non-vacuity anchors: **C-L1(b)** proves the persisted ring is what fires the win (its revert removes the new surface ⇒ the cascade does not end early); **C-L1-probe** proves the Defect-2 guard is load-bearing (its revert reinstates the measured SIGABRT). Both were impossible to write while the feature was inert — they are exactly the discriminating tests the STOPs were blocked on.

**Promotion:** add a `drive_*` live test (C-L1 shape) for **idx 18** and promote idx 18 into `DRIVEN_ROW_INDICES` (`corpus_tests.rs:1927`). `confirmed_drivers_match_expected` (`:1942`) passes (idx 18 `gated_on == None`). idx 17 promoted only on the §4 measurement. Update the DRAIN FEEDBACK bucket doc (`:1908`) to "live-shortcut-driven via the persisted ring under the default per-beat drive (CR 732.2a → CR 704.5a)" for the promoted row(s). Update `loop_check.rs` module doc (§1.4).

---

## 6. CR annotations (grep-verified in `docs/MagicCompRules.txt` this round)

| CR | line | use in this change |
|---|---|---|
| **117.4** | 958 | all players pass in succession ⇒ top of stack resolves (the resolution the §2 gate keys off) |
| **603.3** | 2582 | a triggered ability is put on the stack "the next time a player would receive priority" — WHY the refill lands after `run_post_action_pipeline` (Defect-1 root cause + relocation rationale) |
| **603.3b** | 2586 | multi-trigger placement during the priority-grant process (the refill step) |
| **608.1** | 2783 | each all-pass resolves the top object (resolution semantics) |
| **608.2** | 2785 | resolution steps |
| **704.3** | 5485 | SBAs + "triggered abilities waiting to be put on the stack are put on the stack, then … the appropriate player gets priority" — the §2 post-pipeline sample frame IS this point; §3 detection timing |
| **704.5a** | 5492 | 0-or-less life loses — the determinate end the shortcut anticipates (fires FIRST at `engine.rs:228-229`) |
| **732.2 / 732.2a** | 6370 / 6372 | the shortcut procedure (predictable, non-conditional sequence ending at a priority point) — the live-win authority |
| **732.4** | 6383 | mandatory loop ⇒ DRAW (net-zero; the strict CR 104.4b path, untouched) |
| **732.5** | 6385 | no player forced past an available loop-ender — the §9 gate |
| **104.4b** | 366 | mandatory-loop draw (the byte-for-byte-kept strict block in `run_auto_pass_loop`) |
| **104.2a** | 330 | sole survivor wins (the named `winner`) |

The thread-local guard and the `stack_top_before` capture are engine plumbing (re-entrancy control / resolution detection), **not** rule logic → no CR annotation (per CLAUDE.md "do not annotate plumbing"); they carry plain-English comments only. No NEW CR numbers are introduced beyond those already present in the scaffold (IMPL-REPORT §7) plus 117.4/603.3/603.3b/608.1, all grep-verified above.

---

## 7. Risk-scaled verification cadence

Non-trivial engine plumbing (a `GameState`-adjacent transient field's maintenance moves seams; a new shared `ai_support`↔`engine` re-entrancy guard; the SBA-reconciliation detection seam; §7 reuse of shared PR-2 `analysis::` code) → **full Tilt-first verification**:

1. `cargo fmt --all` (always direct; Tilt does not auto-format).
2. Parser/AST gate **N/A** (no parser change).
3. **Full `analysis::` suite as no-regression** (§7/§8/§9 are shared PR-2 code; U1–U10/U-draw/U-gate/U-stack must stay green): `tilt logs test-engine` filtered to `analysis::` once the build settles.
4. **`clippy` + full `test-engine`** via `./scripts/tilt-wait.sh clippy test-engine`: the relocated §2, the §3 guard, the thread-local guard, the live `C-*` tests, and the idx-18 promotion. **C-L1-probe is the load-bearing recursion regression test** — confirm it terminates (a revert that drops the §3 guard must abort it).
5. **`wasm`** (`tilt logs wasm`): the thread-local + `#[serde(skip)]` field compile to WASM (no `Send` regression; the guard is a `Cell<bool>` thread-local, WASM-single-threaded-safe).
6. **`check-frontend`**: stays green; expect NO change (zero wire/TS surface).
7. Confirm `data/engine-inventory.json` **unchanged** (no enum variant; struct field + thread-local invisible to it).
8. AI no-regression spot check is **not** required (no policy/scoring touch), but an AI-vs-AI smoke (no spurious early `GameOver` on non-loop games) is prudent given the reconcile-site addition — gated on the team-lead's call, not a blocker.

Trap reminders (CLAUDE.md): do NOT diagnose `buildHistory[0].error` while `currentBuild.spanID` is present (queued behind a cargo lock); only act on `updateStatus == "error"` AND `currentBuild.spanID == "none"`. Do NOT run `cargo build/clippy/test` directly while Tilt is up. This worktree is currently Tilt-unwatched (per IMPL-REPORT) — if it stays unwatched, `cargo`-direct is the documented fallback (`tilt get uiresource clippy >/dev/null 2>&1` to detect).

---

## 8. What stays untouched (re-verified)

- **§7** `resource.rs::project_out_resources` stack-id canon — reused verbatim (the §1.2 gate's `resolved_this_beat` reads live `StackEntry.id`; canon is a comparison-layer concern, independent).
- **§8** `loop_check.rs::live_mandatory_loop_winner` (`:207`, `pub(crate)`) + `sba.rs::player_has_cant_lose` (`:308`) — verbatim.
- **§9** `engine.rs::no_living_player_has_meaningful_priority_action` (`:510`) — verbatim (the guard is added at the §3 CALL site, not inside §9).
- **Strict CR 104.4b DRAW block** in `run_auto_pass_loop` (`engine.rs:1289-1298`) + window push — byte-for-byte (§6 already removed only the §10 WIN block; the comment at `:1300-1309` stays).
- **The r3 MEDIUM eq-precedent comment** on the field (`game_state.rs:5700-5704`) — byte-for-byte.
- `apply_action`-entry ring clear on non-pass actions (`engine.rs:1696-1698`) — unchanged (the §2.3 condition-2 invalidation; still correct).

---

## 9. Summary for the orchestrator

- **Defect-1 fix:** MOVE §2 maintenance out of `priority::handle_priority_pass_with_limit` (delete `priority.rs:81-82` + `:108-133`) INTO `engine::pass_priority_once_with_pipeline` at the post-`run_post_action_pipeline` frame (`engine.rs:482→483`), capturing `stack_len_before`/`stack_top_before` at entry (`~:449`). **Gate predicate:** sample iff `resolved_this_beat && !in_simulation_probe() && !stack.is_empty() && stack.len() >= stack_len_before && wf == Priority{active_player}`, where `resolved_this_beat = stack_top_before.is_some() && stack.last().id != stack_top_before`; a bare priority handoff (`resolved_this_beat == false`) **leaves the ring intact** (accumulation survives the inter-resolution beats), and the ring is cleared only when a real resolution ended the cascade (drain/shrink/interactive).
- **Defect-2 fix:** **Option (a)** — a thread-local `IN_SIMULATION_PROBE` guard, RAII-set around `SimulationFilter::accept`'s nested `apply_as_current` (`filter.rs:112-113`), checked (negated) at the §3 detection guard (`engine.rs:251`, soundness-critical) and the §2 sample gate (perf). One-line soundness rationale: §3 fires only when the flag is `false`, which holds exactly at the outermost apply and is `true` throughout every legality probe, so the `reconcile→§3→§9→legal_actions→SimulationFilter→reconcile` cycle terminates at depth 1 while the real per-beat reconcile still detects. Rejected (b) [no outermost-only site on the per-beat path — nested probe shares the identical reconcile seam], (c) [rewrites/weakens reused §9], (d) [the relocated §2 re-accumulates inside the probe ⇒ insufficient/fragile].
- **Serialized surface:** ZERO new — the guard is a thread-local (no GameState field, no eq/serde concern); `loop_detect_ring` unchanged.
- **idx 18 on-paper Step-0:** **YES** — live `GameOver{Some(P0)}` by ~beat 6 (≪ the ~400 a high-life 704.5a death needs). **idx 17:** targeting-deferred; promote only if the re-measured Step-0 shows the targeted Sanguine trigger auto-resolves.
- **Tests:** C-L1 (+ the new C-L1-probe recursion regression test), C-L2, C-neg-A..E — all non-vacuous, each names its revert-fail line. Promote idx 18 to `DRIVEN_ROW_INDICES`; refresh DRAIN FEEDBACK + `loop_check.rs`/field docs.
- **Gate:** full Tilt-first (`fmt` → `analysis::` no-regression → `clippy`+`test-engine` → `wasm` → `check-frontend` → inventory-unchanged), with C-L1-probe as the recursion backstop.

---

## defect-fix review resolution (2026-06-27) — APPROVED with corrections (authoritative for re-implementer)

Adversarial review (pr3-defectfix-reviewer): **0 BLOCKER · 0 HIGH · 2 MED · 1 LOW**. BOTH make-or-break decisions SOUND and measured: (1) `SimulationFilter::accept` (`filter.rs:113`) is the ONLY nested clone-and-apply path that re-enters `reconcile_terminal_result` (callers 196/200; exhaustive scan) → the RAII guard there is sufficient; (2) `resolved_this_beat` correctly samples resolutions and leaves handoffs intact — every placed trigger gets a fresh monotonic ObjectId (`triggers.rs:3894`), so resolution+refill always changes the top id while a handoff never does (all 6 case rows verified). Corrections:

- **[MED-1 — CI gate, LEAD-VERIFIED via `git diff HEAD`] The priority.rs revert must restore the ORIGINAL TRAILING `if/else` EXPRESSION, not just delete the sample lines.** The scaffold converted the original trailing expression (`if matches!(waiting_for, Priority) { reset; Priority{active} } else { waiting_for.clone() }`) into `let wf = if … ; <sample/clear block>; wf`. Deleting only the sample block leaves `let wf = …; wf` → `clippy::let_and_return` → fails `clippy -D warnings`. FIX: revert the ENTIRE §2 scaffold region in `priority.rs` (the added `stack_len_before` at :81-82, the `let wf =` wrapper, the sample/clear block :108-133, and the trailing `wf` :134) back to its HEAD state — the trailing `if/else` expression. §2 maintenance lives ENTIRELY in `engine::pass_priority_once_with_pipeline` after relocation; priority.rs ends up byte-identical to HEAD.
- **[MED-2 — scope the claim honestly] §3 detection accelerates the PER-BEAT drive ONLY (the tested target C-L1), NOT `run_auto_pass_loop`.** `reconcile_terminal_result` is NOT called inside `run_auto_pass_loop`, so its internal net-progress grind still runs to the natural CR 704.5a high-life death (the strict CR 104.4b draw block is life-sensitive and doesn't match a net-progress drain). This is acceptable: the production frontend default IS the per-beat repeated `apply(PassPriority)` drive, which §3 (at reconcile, called after each apply) catches within a few beats (idx-18 → `GameOver{P0}` by ~beat 6). Re-word any plan text that claims Option C accelerates ALL drivers; the deliverable is the per-beat win. (Optional: add an auto-pass measurement noting it still terminates via 704.5a.)
- **[LOW-1] §7 canon is at `resource.rs:751`** (not ~742). Correct the cite.

Re-verified untouched/correct: §7 canon mutates only a clone (live ids safe), §8/§9 intact, thread-local idiom precedent exists + `apply` is async-free + the ring is serde-skip/eq-excluded (manual eq at game_state.rs:7869), CR 104.4b draw block is complementary to the §3 win (strict-equality vs modulo) so no draw/win conflict, idx-18 traces to `GameOver{P0}` by beat 6, C-L1-probe (revert §3 guard ⇒ SIGABRT) is a valid non-vacuous recursion discriminator, all CR numbers grep-verify.
