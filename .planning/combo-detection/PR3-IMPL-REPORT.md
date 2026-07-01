# PR-3 Option C implementation report (wt-combo-pr3)

Branch `feat/combo-detect-pr3` off main `5eca83b8c` (v0.7.0). Sole writer this session.
Executor mandate: implement the APPROVED Option C plan (§1 ring field, §2 maintenance,
§3 detection, §6 §10-removal) surgically; reuse §7/§8/§9 verbatim; run the LOW-3 HARD
GATE Step-0 measurement BEFORE promoting `DRIVEN_ROW_INDICES`; STOP if idx 18 does not
win live.

## TL;DR — STOP-AND-RETURN (two measured plan-vs-code contradictions)

**idx 18 does NOT produce a live `GameOver` under the plan as written.** The LOW-3 HARD
GATE Step-0 measurement (real per-beat `apply(PassPriority)` drive) surfaced **two**
blocking defects that green-compile hid:

- **Defect-1 (§2 seam frame — inertness).** The §2 refill gate samples inside
  `handle_priority_pass_with_limit`, but the trigger that REFILLS a self-refilling
  mandatory cascade is placed on the stack by `engine_priority::run_post_action_pipeline`
  **after** the seam returns. At the sample point the stack has SHRUNK (resolved entry
  gone, next trigger not yet placed), so the gate `stack.len() >= stack_len_before` never
  fires → **the ring stays permanently empty** for the idx 17/18 trigger-cascade class →
  §3 detection never runs → no live win. MEASURED: ring length stayed `0` across 200 beats
  while the cascade drained P1 200→150.

- **Defect-2 (§3 detection site + §9 gate — infinite recursion).** I proved Defect-1's fix
  (relocate maintenance to after `run_post_action_pipeline`) makes the ring accumulate and
  find_map find winner P0 — but it then **stack-overflows**. The §3 detection lives in
  `reconcile_terminal_result`, which runs inside **every** `apply_as_current`, INCLUDING
  the nested `apply_as_current` calls that `ai_support::SimulationFilter` makes to test
  candidate legality. The §9 gate (`no_living_player_has_meaningful_priority_action`) calls
  `legal_actions`, which runs that simulation. So:
  `apply_as_current → reconcile_terminal_result → §3 detection (ring populated) → §9 gate →
  legal_actions → SimulationFilter → apply_as_current → reconcile_terminal_result → §3 …`
  recurses without bound. MEASURED: stack overflow / SIGABRT once the ring populated.

Both are architectural and require **re-planning + re-review** (the soundness of a
detection-site relocation or a re-entrancy guard is not a surgical edit). Per the team-lead
LOW-3 HARD GATE ("do not fake it, do not promote") and the executor stop-and-return rules,
I **did not promote `DRIVEN_ROW_INDICES`** and left the plan implemented faithfully but
INERT (ring never accumulates ⇒ §3 body never executes ⇒ no recursion, no behavior change,
no regression). Full suite stays green (13666 pass / 0 fail).

---

## 1. Per-file diff summary (file:line of each edit)

All edits are the Option C NEW surface (§1/§2/§3/§6). §7 (`resource.rs`), §8
(`loop_check.rs` body + `sba.rs` `player_has_cant_lose`), §9
(`no_living_player_has_meaningful_priority_action`) reused VERBATIM (pre-existing
uncommitted building blocks; not rewritten).

### `crates/engine/src/types/game_state.rs` (§1)
- **:5705–5706** — new field `#[serde(skip, default)] pub loop_detect_ring:
  std::collections::VecDeque<std::sync::Arc<GameState>>`, placed immediately after the
  `static_source_index` precedent field. Doc comment cites `static_source_index` /
  `static_gate_truth` as the `serde(skip)`+eq-excluded precedent and EXPLICITLY says NOT
  `public_state_dirty`/`state_revision`/`layers_dirty` (those are serde-skip but ARE
  compared in `eq`) — the r3 MEDIUM correction.
- **:7407** — struct-literal initializer in `new_n_player` gets
  `loop_detect_ring: std::collections::VecDeque::new()` (forced by the non-`Default`
  constructor; compile error E0063 without it).
- **:7790** — `normalize_for_loop` adds `clone.loop_detect_ring.clear();` (§1.2.1 —
  prevents recursive ring growth; snapshots have clone-depth 1).
- **:7799–7805** — new `pub(crate) fn record_loop_detect_sample(&mut self)` (push
  `Arc::new(self.normalize_for_loop())`, pop_front at cap).
- **:7813** — `const LOOP_DETECT_RING_CAP: usize = 16;` (module-private).
- **impl PartialEq (~:7843)** — DONE NOTHING (ring auto-excluded by the manual `eq`
  convention; the r3-mandated precedent comment lives on the field doc, per plan §1.2.2).

### `crates/engine/src/game/priority.rs` (§2)
- **:82** — `let stack_len_before = state.stack.len();` before `resolve_next_with_limit`.
- **:97–134** — the non-empty-stack branch now binds `let wf = …` and, behind the refill
  gate (`wf` is Priority && stack non-empty && `stack.len() >= stack_len_before`), calls
  `state.record_loop_detect_sample()` else `state.loop_detect_ring.clear()`. Carries a
  KNOWN-INERT comment pointing at this report's Defect-1.

### `crates/engine/src/game/engine.rs` (§3 + §1.2.3 + §6)
- **:241–283** — §3 detection block appended to `reconcile_terminal_result` AFTER the
  existing CR 704.5a SBAs (CR 704.3 ordering), guarded `!GameOver && Priority &&
  !stack.is_empty() && !ring.is_empty()`: scan ring via §7 canon + §8 winner + §9 gate;
  emit `GameOver { winner }`. Reuses `live_mandatory_loop_winner` (pub(crate)) and the
  private §9 fn — no visibility bumps, no new `GameEvent`.
- **:1696–1698** — §1.2.3 ring clear at `apply_action` entry, placed AFTER all preference
  early-returns (CancelAutoPass / SetPhaseStops / ReorderHand / Debug / Grant- &
  RevokeDebugPermission — the r3 LOW-1 correction): `if !matches!(action,
  GameAction::PassPriority) { state.loop_detect_ring.clear(); }`.
- **:1300–1310** — §6: removed the dead round-2 §10 WIN block in `run_auto_pass_loop`
  (the `cur` local + `loop_window.find_map` over `live_mandatory_loop_winner` + §9 gate +
  GameOver), replaced with a comment pointing at the single §3 site. The strict CR 104.4b
  DRAW block and the window push are KEPT BYTE-FOR-BYTE.

### `crates/engine/src/analysis/loop_check.rs` (§5.3 doc only)
- **module doc (~:17)** — `live_mandatory_loop_winner` is now described as reached via
  `game::engine::reconcile_terminal_result` (the persisted-ring scan) for every driver
  under the per-beat drive — replacing the old `run_auto_pass_loop` framing.

### `crates/engine/src/analysis/resource.rs`, `crates/engine/src/game/sba.rs`
- UNCHANGED this session (pre-existing §7 / §8-helper building blocks). Shown in `git diff`
  because they were uncommitted before I started.

---

## 2. Step-0 measurement RESULT (the LOW-3 HARD GATE)

Method: `build_board(CORPUS[18].cards)` (Marauding Blight-Priest + Bloodthirsty
Conqueror), P1 life set to 200 (so the natural CR 704.5a death cannot be the cause of any
`GameOver`), seeded one synthetic "P0 gains 1 life" `TriggeredAbility` stack entry
(`Effect::GainLife{Fixed 1, Controller}`, controller P0) to start the cascade, then drove
**the real per-beat `GameRunner::act(GameAction::PassPriority)`** (which routes through
`apply_as_current`) repeatedly, observing `waiting_for`, both life totals, and
`loop_detect_ring.len()` per beat.

### idx 18 — DID NOT win live under the plan as written
- Cascade self-sustains correctly: P0 gains +1 / P1 loses −1 per ~2 beats (non-targeted
  Blight-Priest + Conqueror), exactly the period-2 drain.
- **`loop_detect_ring.len()` stayed `0` for all 200 beats.** No `GameOver`. P1 drained
  200 → 150 (grinding toward the high-life 704.5a death, as predicted for an inert ring).
- Root cause = **Defect-1** (above): the refilling trigger is placed by
  `run_post_action_pipeline` after the §2 seam returns; at the seam the stack has shrunk,
  so the refill gate never fires.

### Controlled fix experiment (to make the finding actionable, then reverted)
- Relocated the maintenance to `pass_priority_once_with_pipeline` immediately AFTER
  `run_post_action_pipeline` (capturing `stack_len_before` at function entry). Re-ran the
  same idx-18 drive:
  - **Ring NOW accumulates** (1,2,3,…) and `find_map` finds **winner = PlayerId(0)** once a
    same-stack-shape modulo pair is in the ring (≈ beat 5). Defect-1's fix is PROVEN.
  - But the drive then **stack-overflows** — root cause = **Defect-2** (§3-at-reconcile +
    §9-gate re-entrancy through `SimulationFilter`'s nested `apply_as_current`). Instrumented
    trace shows, at ring cap (16), the pattern "DETECT entry → winner found → call gate"
    repeating WITHOUT the per-beat counter advancing — i.e. recursion inside a single
    `act()` call, terminating in SIGABRT (stack overflow).
- The experiment was fully reverted; the worktree contains only the plan-as-written
  placement.

### idx 17 (Sanguine + Exquisite, targeted `LoseLife`) — NOT measured
Per §5.1.3 the idx-17 measurement is gated on whether the per-beat drive auto-resolves the
sole legal `LoseLife` target without a `SelectTargets` stop. I did **not** run it: Defect-1
and Defect-2 block the live win path for the WHOLE drain class (idx 17 shares the same
trigger-cascade refill timing and the same detection/gate path), so idx 17's targeting
question is moot until the architecture is re-planned. Recorded as targeting-deferred AND
architecture-blocked; not promoted.

---

## 3. `DRIVEN_ROW_INDICES` promotion — NONE

`DRIVEN_ROW_INDICES` (`corpus_tests.rs:1927`) is UNCHANGED (`[0,1,4,6,9,10,12,13,14,49]`).
Neither idx 18 nor idx 17 was promoted, because neither produces a live `GameOver` through
the real `apply(PassPriority)` drive under the plan as written (gate not satisfied). The
DRAIN FEEDBACK bucket doc and `confirmed_drivers_match_expected` are likewise unchanged.
`corpus_tests.rs` is NOT in the modified-file set (the temporary Step-0 measurement test
was removed as debug cruft).

---

## 4. Tests added — NONE shipped (discriminating-test gate cannot be satisfied)

The §5.2 live test map (C-L1/C-L2 positives, C-neg-A..E soundness negatives) **cannot be
made non-vacuous** while the live win path is inert: every C-test would assert "no
`GameOver`" and pass VACUOUSLY (PR-3 never fires regardless of the asserted revert), exactly
the discriminating-test FAILURE the r1 STOP flagged. Shipping vacuous tests would be worse
than none. So no C-* tests were added.

The pre-existing building-block unit tests (U1–U10, U-draw, U-gate `loop_gate_probes_all_
living_players_not_just_current_holder`, U-stack `modulo_equal_ignores_volatile_stack_entry_
id`) remain present and **non-vacuous in isolation** — they prove §7/§8/§9 against
hand-built EMPTY-ring states (so they don't trip Defect-2). All pass (see §5).

Discriminating-test gate status for the new live surface: **BLOCKED** by Defect-1/Defect-2;
returned as a stop-and-return item, not papered over.

---

## 5. Build / clippy / test counts (final post-revert state, run directly in worktree)

- `cargo fmt -p engine` — clean.
- `cargo build -p engine` — **OK** (compiles; required adding `loop_detect_ring` to the
  `new_n_player` struct literal, else E0063).
- `cargo clippy -p engine --lib -- -D warnings` — **OK, 0 warnings**.
- `cargo test -p engine --lib` — **13666 passed; 0 failed; 7 ignored**. (The inert ring
  causes no regression and trips no recursion across the whole suite — empirical
  confirmation of the inert+safe claim.)
- `cargo test -p engine --lib analysis::` — **91 passed; 0 failed** (includes U-stack,
  U-gate, and all `live_winner_*` soundness negatives).

(This worktree is unwatched by Tilt → cargo-direct is correct here, per the team-lead
instruction. No Tilt resources consulted.)

---

## 6. Serialized-surface delta — ZERO (verified)

- `loop_detect_ring` is `#[serde(skip, default)]` (game_state.rs:5705) → absent from save
  JSON / MP broadcast / WASM→JS payload.
- No `crates/engine/src/types/events.rs` change (the §3 shortcut reuses
  `GameEvent::GameOver { winner }` / `WaitingFor::GameOver` — no new variant).
- Struct field, not an enum variant ⇒ `data/engine-inventory.json` unchanged (catalogues
  enum variants only; not regenerated).
- Omitted from `impl PartialEq for GameState` (manual `eq`, ring auto-excluded) ⇒ AI-search
  dedup unaffected.

---

## 7. CR-annotation gate — CLEAN (0 UNVERIFIED)

`git diff | grep CR …` over the full diff, grepped against
`docs/MagicCompRules.txt`: every CR number resolves. Numbers present in the added lines:
`101.2, 104.2a, 104.2b, 104.3b, 104.4b, 704, 704.3, 704.5a, 704.5b, 704.5c, 732.2a, 732.4,
732.5, 810.8a`. Each grep-verified this session (e.g. `732.2a@6372`, `704.5a@5492`,
`704.3@5485`, `732.5@6385`, `104.4b@366`, `104.2a@330`, `704.5c@5496`; `810.8a@6733` is the
2HG team rule the §8 comment correctly EXCLUDES). The cited rules describe the annotated
code (shortcut procedure / 0-life loss / SBA timing / no-forced-loop-ender / mandatory
draw).

---

## 8. Judgement calls & deviations from the plan

1. **STOP vs. ship the proven fix.** I PROVED relocating §2 maintenance to after
   `run_post_action_pipeline` makes idx 18 win live — but it exposes Defect-2 (recursion),
   which needs a soundness-bearing redesign (detection-site move or re-entrancy guard).
   Shipping the seam-fix alone would ship an infinite-recursion landmine; shipping a
   re-entrancy guard I invented would be an unreviewed soundness change. Both violate the
   surgical-translation mandate and "bandaids that ship are worse than a clean handback." →
   STOP, implement plan-as-written (inert+safe), document the fix path. The team-lead LOW-3
   HARD GATE explicitly anticipated "idx 18 does NOT produce a live GameOver ⇒ STOP."
2. **Left §3 detection + §2 maintenance in place (plan-as-written) rather than reverting.**
   The team lead said "leave changes staged-as-modified for the lead's review." With §2 at
   the seam the ring is provably empty for the target class ⇒ §3's `!ring.is_empty()` guard
   is effectively always-false ⇒ the recursion is latent, never executed (validated: full
   suite green). Flagged loudly here so the lead does not "fix" §2 without simultaneously
   resolving Defect-2.
3. **Did not regenerate `engine-inventory.json`.** Struct field, not an enum variant; the
   inventory is unaffected and gitignored. Regenerating would be a no-op churn.
4. **Removed the Step-0 measurement test + seed helper** (debug cruft), mirroring the r1
   STOP cleanup. The measurement evidence lives in this report.

No deviation altered §7/§8/§9 (reused verbatim) or the strict CR 104.4b DRAW path
(byte-for-byte untouched).

---

## 9. What I could NOT complete (stop-and-return items for re-planning)

1. **Live win for idx 18/17** — blocked by Defect-1 (seam frame) + Defect-2 (recursion).
2. **`DRIVEN_ROW_INDICES` promotion** — not done (gate unsatisfied).
3. **C-L1/C-L2/C-neg-A..E live tests** — cannot be non-vacuous while inert.
4. **DRAIN FEEDBACK bucket doc + `confirmed_drivers_match_expected`** — left unchanged (no
   promotion to record).

### Recommended re-plan (measured, actionable)
- **Fix Defect-1:** move the §2 maintenance from `handle_priority_pass_with_limit` to
  `engine::pass_priority_once_with_pipeline`, immediately AFTER
  `run_post_action_pipeline(...)` (capture `stack_len_before` at function entry). This is
  the single shared post-trigger-refill point every PassPriority driver traverses
  (per-beat, `run_auto_pass_loop`, `resolve_all_fast_forward` via `apply_action`'s
  `(Priority, PassPriority)` arm at engine.rs:1736). PROVEN to make the ring accumulate and
  `find_map` find winner P0. NOTE: at that site a NON-resolving priority handoff (P0 passes,
  priority→P1, stack unchanged) also passes a naive `len >= before` gate — the re-plan must
  gate on "a resolution actually occurred" (e.g. only sample when the post-pipeline window
  has reset to the active player, or detect stack-top consumption) to avoid recording
  non-resolution passes.
- **Fix Defect-2 (the hard one — needs review):** the §3 detection at
  `reconcile_terminal_result` re-enters through `SimulationFilter`'s nested
  `apply_as_current` (invoked by the §9 gate's `legal_actions`). Options for the planner:
  (a) a re-entrancy guard (thread-local or a transient `state` flag) so loop detection runs
  only at the TOP-level apply, never inside a simulation probe; (b) move detection to a site
  that only runs at the outermost apply boundary (not inside nested `apply_as_current`);
  (c) make the §9 gate use a non-simulating meaningful-action predicate; or (d) suppress the
  ring inside the SimulationFilter's cloned probe. Each has soundness implications and MUST
  go back through `/review-engine-plan`.

---

## 10. Risks for the `/review-impl` loop

- **Latent recursion landmine.** §3 + §9 will infinite-recurse the instant the ring
  populates. With §2 at the plan seam the ring is empty in practice, so it is currently
  inert — but ANY future change that populates the ring (the Defect-1 fix, or a rare
  in-resolution stack push during normal play that trips the seam refill gate) re-arms it.
  Treat Defect-2 as a hard blocker that must land WITH any §2 relocation.
- **Inert feature.** As shipped, PR-3 adds ~635 lines that do nothing observable (the win
  path never fires). The building blocks (§7/§8/§9) and field/maintenance/detection scaffold
  are correct and reusable, but there is no behavioral coverage and no discriminating test —
  by design, pending the re-plan.
- **No serialized-surface / inventory / WASM / TS change** (verified) — low blast radius if
  the lead chooses to hold the scaffold for the re-plan.
