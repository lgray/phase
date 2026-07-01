# PR-3 Option C — Defect-fix Implementation Report

> Worktree `/home/lgray/vibe-coding/wt-combo-pr3`, branch `feat/combo-detect-pr3` (base `5eca83b8c` v0.7.0).
> Executed the APPROVED `PR3-PLAN-DEFECTFIX.md` (+ its `## defect-fix review resolution (2026-06-27)` addendum, MED-1/MED-2/LOW-1). Cargo-direct verification (Tilt-unwatched worktree). **NOT committed, NOT staged** — left as modified.

## 0. TL;DR

- **Step-0 HARD GATE: PASSED.** idx-18 (Marauding Blight-Priest + Bloodthirsty Conqueror, P1 life 200) wins **LIVE** via the real per-beat `apply(PassPriority)` drive — `GameOver{winner=P0}` at **beat 6** (ring `[s1,s2,s3]`), exactly the plan's beat-6 trace, well before the ~400-beat high-life CR 704.5a death. **No SIGABRT.**
- **idx-17 (Sanguine Bond + Exquisite Blood, targeted `LoseLife`) ALSO wins live at beat 6** — the targeted "target player loses life" trigger auto-resolves to the opponent with **no** target-selection stop. Per §4 disposition → **promoted**.
- **Both idx 17 and 18 promoted to `DRIVEN_ROW_INDICES`**, backed by passing non-vacuous live tests.
- **Verification:** `cargo fmt` clean; `cargo build -p engine` clean; `cargo clippy -p engine --lib --tests -- -D warnings` clean; `cargo test -p engine --lib` = **13671 passed / 0 failed / 7 ignored**; `analysis::` = **96 passed**.
- **Serialized-surface delta vs HEAD: ZERO** (thread-local + serde-skip/eq-excluded ring; no new `GameEvent`/enum variant).
- **One significant FINDING (flagged for orchestrator):** the Defect-2 thread-local guard is **defensive depth, NOT independently load-bearing** in the shipped architecture — the recursion the plan framed it against does **not** reproduce (measured across 3 revert configs). Details in §6 + §11.

---

## 1. Per-file diff summary (file:line)

`priority.rs` is **byte-identical to HEAD** (MED-1) — not in the diff at all.

### `crates/engine/src/game/engine.rs` (+211)
- **`:219-253`** — NEW `thread_local! IN_SIMULATION_PROBE: Cell<bool>` + `in_simulation_probe() -> bool` accessor + RAII `SimulationProbeGuard` (Defect-2). Co-located above `reconcile_terminal_result`; mirrors the in-engine thread-local idiom (`perf_counters.rs`).
- **`:297`** — §3 detection guard: added `&& !in_simulation_probe()` conjunct (comment corrected to "defensive — enforces top-level-only invariant", per the §6 measurement).
- **`:502-503`** — §2 capture at `pass_priority_once_with_pipeline` entry: `stack_len_before` + `stack_top_before` (Defect-1).
- **`:543-571`** — §2 maintenance RELOCATED here, immediately after `run_post_action_pipeline` + `sync_waiting_for` (Defect-1). `resolved_this_beat` gate; handoff ⇒ leave-intact; resolution ⇒ sample-or-clear. CR 732.2a / 603.3 / 704.3 annotated.
- **`:1387-1401`** — `run_auto_pass_loop` comment corrected per MED-2 (it accumulates via `:1339`'s `pass_priority_once_with_pipeline` but `reconcile`/§3 is NOT called mid-loop, so §3 does not accelerate the auto-pass grind — the per-beat drive is the accelerated path).

### `crates/engine/src/ai_support/filter.rs` (+8)
- **`:32`** — `use` adds `SimulationProbeGuard`.
- **`:118`** — `SimulationProbeGuard::enter()` RAII set around the nested `apply_as_current` (the sole production clone-and-apply re-entry point).

### `crates/engine/src/types/game_state.rs` (+49)
- Field doc (`:~5683`) + `record_loop_detect_sample` doc (`:~7800`) — seam name corrected from `priority::handle_priority_pass_with_limit` to the post-pipeline frame of `engine::pass_priority_once_with_pipeline` (§1.4). The `loop_detect_ring` field + `record_loop_detect_sample` + `LOOP_DETECT_RING_CAP` are the §7-block scaffold, unchanged in behavior.

### `crates/engine/src/analysis/loop_check.rs` (+354)
- Module doc (`:22-37`) — seam-name correction + MED-2 honest scoping (per-beat drive accelerated, `run_auto_pass_loop` not). §8 `live_mandatory_loop_winner` + its 11 unit tests are the scaffold, unchanged.

### `crates/engine/src/analysis/resource.rs` (+98), `crates/engine/src/game/sba.rs` (+6)
- §7 stack-id canon (`resource.rs:751`) + §8 `player_has_cant_lose` (`sba.rs`) — scaffold, **untouched** (carried forward).

### `crates/engine/src/analysis/corpus_tests.rs` (+472)
- **`:1939`** `DRIVEN_ROW_INDICES` — added `17` and `18`.
- **`:1909-1922`** DRAIN FEEDBACK doc + module doc — promotion documented.
- **`:1980` `build_drain_board`**, **`:2011` `seed_lifegain_cascade`**, `BeatTrace` + `drive_pass_priority`, `first_gameover_beat`, `add_meaningful_action_artifact` — live-drive toolkit.
- **`:2097` `drive_drain_idx18_wins_live` (C-L1)**, **`:2154` `drive_drain_idx17_targeted_wins_live` (C-L2)**, **`:2207` `drive_drain_idx18_legal_actions_terminates_bounded` (C-L1-probe)**, **`:2302` `drive_drain_idx18_victim_with_out_is_not_eliminated` (C-neg-A)**, **`:2329` `drive_finite_stack_keeps_ring_empty` (C-neg-D)**.

---

## 2. Step-0 RESULT — the decisive proof

### idx-18 — **WON LIVE at beat 6** (`drive_drain_idx18_wins_live`)

Board: real cards Marauding Blight-Priest + Bloodthirsty Conqueror on P0; P1 life **200**; seeded one external "P0 gains 1 life" via the real `apply_life_gain` → `process_triggers` pipeline (Blight-Priest trigger on stack). Driven by repeated real `runner.act(PassPriority)`:

| beat | wf | stack | ring | P0 | P1 | note |
|---|---|---|---|---|---|---|
| seed | Priority{P0} | 1 | 0 | 41 | 200 | Blight-Priest trig seeded |
| 1 | Priority{P1} | 1 | 0 | 41 | 200 | handoff → leave-intact |
| 2 | Priority{P0} | 1 | 1 | 41 | 199 | resolve BP (P1−1), refill Conqueror → s1 |
| 3 | Priority{P1} | 1 | 1 | 41 | 199 | handoff → intact |
| 4 | Priority{P0} | 1 | 2 | 42 | 199 | resolve Conq (P0+1), refill BP → s2 |
| 5 | Priority{P1} | 1 | 2 | 42 | 199 | handoff → intact |
| **6** | **GameOver{Some(P0)}** | 1 | 3 | 42 | 198 | resolve BP (P1−1) → s3 ≡ s1 modulo ⇒ **WIN** |

Matches the plan's §4 on-paper trace exactly. The ring accumulated `[s1,s2,s3]`, §3 fired at the TOP-level reconcile (no SIGABRT — §9's `legal_actions` probes terminated), and emitted `GameOver{Some(P0)}` ≪ the ~400-beat 704.5a death.

### idx-17 — **WON LIVE at beat 6** (auto-resolved target; `drive_drain_idx17_targeted_wins_live`)

Identical trace to idx-18. **No `TargetSelection`/`TriggerTargetSelection` window appears at any beat** (asserted) — the targeted Sanguine Bond "target player loses that much life" trigger auto-resolves to the opponent P1 (its life drains 200→198). Per §4 the condition "auto-resolves … no SelectTargets stop" is met → **promoted** (not targeting-deferred). Nuance: "target player" is nominally 2 legal targets in 2-player; the engine's existing non-interactive trigger-target assignment lands deterministically on the opponent, and the §8 single-faller + §9 firewalls guard soundness regardless. I did not exhaustively trace WHY the assignment is opponent-directed (pre-existing engine behavior, not touched by this change); the measured outcome is a deterministic correct win, non-vacuously tested.

---

## 3. `DRIVEN_ROW_INDICES` promotion + evidence

Promoted **idx 17** and **idx 18** (`corpus_tests.rs:1939`). Evidence: passing live tests `drive_drain_idx18_wins_live` / `drive_drain_idx17_targeted_wins_live` produce a real `GameOver{Some(P0)}` through the real `apply(PassPriority)` pipeline. `confirmed_drivers_match_expected` (both `gated_on == None`) and `corpus_table_shape_is_locked` pass. DRAIN FEEDBACK bucket + corpus module doc updated to reflect both as live-driven.

---

## 4. Discriminating-test gate — production-path coverage map

Every behavioral claim mapped to a live `apply()`-pipeline test, with the revert that flips it **measured** (reverted in source, ran, observed FAIL, restored):

| behavioral claim | changed seam | production entry | test | revert that fails it (MEASURED) | sibling/negative |
|---|---|---|---|---|---|
| persisted ring wins idx-18 live | §2 sample (engine.rs:568) + §3 (engine.rs:295-) | `act(PassPriority)` → `pass_priority_once_with_pipeline` → `reconcile_terminal_result` | `drive_drain_idx18_wins_live` | (a) `§3 && false` ⇒ no GameOver ✓ FAIL; (b) `§2 sample && false` ⇒ ring never persists, no GameOver ✓ FAIL | C-neg-A/D below |
| targeted-trigger shape wins live, target auto-resolves | same | same | `drive_drain_idx17_targeted_wins_live` | `§3 && false` ⇒ no GameOver ✓ FAIL (ran) | asserts no target window appears |
| victim with a meaningful out is NOT shortcut (§9 firewall) | §9 gate call (engine.rs:308) | same | `drive_drain_idx18_victim_with_out_is_not_eliminated` | `if true \|\| §9` ⇒ wrong GameOver ✓ FAIL (ran) | all-players §9 probe also unit-covered (`loop_gate_probes_all_living_players_not_just_current_holder`) |
| finite shrinking stack never records / never wins (ring hygiene) | §2 refill gate (engine.rs:567) | `act(PassPriority)` over a 3-ability stack | `drive_finite_stack_keeps_ring_empty` | drop `state.stack.len() >= stack_len_before` ⇒ ring non-empty ✓ FAIL (ran) | — |
| legality probe on a populated ring terminates bounded | §3 guard / filter.rs guard | `legal_actions(state)` at a resolving window | `drive_drain_idx18_legal_actions_terminates_bounded` | **see §6 — bounded-termination property test, NOT a guard discriminator (honest)** | future-recursion regression guard |

**Soundness firewall (§8) is comprehensively unit-covered** at the building-block level (non-vacuous, scaffold): `live_winner_mutual_drain_is_none` (C-neg-C), `live_winner_faller_cant_lose_is_none` (C-neg-E), `live_winner_dual_faller_library_is_none`, `live_winner_dual_faller_poison_is_none`, `live_winner_pure_mill_is_none`, `live_winner_three_player_is_none`, `live_winner_net_zero_is_none`, `live_winner_winner_cant_win_is_none`, `live_winner_advantage_no_faller_is_none`, `live_winner_board_change_is_none`, `live_winner_positive_life_drain`. C-neg-B (current-vs-all-holder) is covered by `loop_gate_probes_all_living_players_not_just_current_holder`. No production-reachable §3/§9 arm is left covered only by a degenerate fixture — the idx-18/17 fixtures reach the real refill→resolve→detect→gate path; the §9 negative reaches the gate with a genuinely-meaningful non-current holder.

**No shape-only tests.** Every test drives `apply()` and asserts a runtime outcome (`GameOver`, ring length, life totals, termination).

---

## 5. Maintainer-simulation matrix

| seam / claim | production entry + first branch | selected authority | bound value / when | binding mode | storage | consumer(s) | invalidation | hostile fixture | serde/protocol |
|---|---|---|---|---|---|---|---|---|---|
| §2 ring accumulation | `pass_priority_once_with_pipeline` after `run_post_action_pipeline`; first branch `resolved_this_beat = stack_top_before.is_some() && top changed` | the resolving stack entry's `ObjectId` (top-before vs top-after) | bound at function entry (`stack_top_before`) + post-pipeline compare; sample on real resolution | snapshotted `Arc<GameState>` via `record_loop_detect_sample` (normalized, own ring cleared) | `GameState::loop_detect_ring` (serde-skip, eq-excluded `VecDeque`) | §3 scan in `reconcile_terminal_result` | cleared on drain/shrink/interactive resolution; on non-pass action (`apply_action`); never serialized (rebuilt from play) | `drive_finite_stack_keeps_ring_empty` reaches the shrink/clear arm; handoff beats reach the leave-intact arm (idx-18 trace beats 1/3/5) | none (serde-skip; eq-excluded — AI dedup unaffected) |
| §3 win shortcut | `reconcile_terminal_result` §3; first guards: `!GameOver`, `Priority`, `!stack.is_empty()`, `!ring.is_empty()`, `!in_simulation_probe()` | the single non-falling living player (`live_mandatory_loop_winner`) | winner bound at detection (find_map over ring priors vs `snapshot(state)`) | live predicate over the snapshotted ring + live `cycle_end` (firewalls read live board) | none persisted — emits `GameEvent::GameOver{winner}` + sets `WaitingFor::GameOver` | `match_flow::handle_game_over_transition` | idempotent via `!GameOver` guard across the :196/:200 reconcile calls | `drive_drain_idx18_victim_with_out` reaches §9 false-arm; finite-stack reaches the `live_mandatory_loop_winner == None` arm (no faller) | reuses existing `GameOver` event — no new variant |
| §9 gate | `no_living_player_has_meaningful_priority_action`; `.all(living)` short-circuit | every living player as priority holder (not just current) | per-player `legal_actions` + `has_meaningful_priority_action` at detection time | live (clones probe per player, resets `auto_pass`/`priority_player`/`waiting_for`) | transient probe clones (discarded) | the §3 emit-gate | n/a (recomputed each detection) | victim P1 (non-current holder) with a costless draw ⇒ gate false (C-neg-A) | none |
| Defect-2 probe flag | `SimulationFilter::accept` sets RAII flag around `apply_as_current` | "are we in a legality probe?" | set for the whole nested apply; restored on drop (prev-saved, nesting-correct) | thread-local `Cell<bool>` (execution context, NOT game state) | `IN_SIMULATION_PROBE` thread-local | §3 guard (engine.rs:297) + §2 gate (engine.rs:557) | RAII drop restores prior value | every `SimulationFilter` probe sets it (idx-18 §9 probes); `drive_drain_idx18_legal_actions_terminates_bounded` exercises it | **none** — not a `GameState` field, not serialized, not eq'd |

No serde/protocol/card-data fixture shapes change. `engine-inventory.json` unchanged (no enum variant). No new `GameEvent`.

---

## 6. Defect-2 — measured FINDING (the guard is defensive, not load-bearing)

The plan framed the thread-local `!in_simulation_probe()` guard as "the sole barrier that terminates the `reconcile→§3→§9→legal_actions→SimulationFilter→reconcile` recursion," with C-L1-probe as a SIGABRT discriminator. **I could not reproduce the recursion in the shipped code.** Measured across 3 revert configurations (each: revert in source, run, observe, restore):

1. **Drop only `&& !in_simulation_probe()` from §3** — `drive_drain_idx18_wins_live` and the direct `legal_actions` probe both **terminate cleanly** (no SIGABRT), idx-18 still wins at beat 6.
2. **Drop the filter.rs `SimulationProbeGuard::enter()` entirely** (flag never set ⇒ all three checks become no-ops) — both the real per-beat drive AND the direct `legal_actions` call still **terminate cleanly**.
3. Both the real-drive path and the direct-`legal_actions` path, both with ring≥2 at the resolving window.

**Root cause of the non-reproduction:** the scaffold's §9 gate `no_living_player_has_meaningful_priority_action` **resets each probe's `priority_player`/`waiting_for`/`auto_pass`**, so the nested `SimulationFilter` `apply(PassPriority)` becomes a **handoff that never re-resolves**; §3 (which fires only when a winner is FOUND) therefore never re-enters from inside a probe. On the real path §3 finds the modulo match at **ring=3** (beat 6) and ends the game before the ring could approach the cap-16 state the original impl-report observed. The original SIGABRT (impl-report §2, "at ring cap 16") was measured under a **different** §9/§2 configuration (the controlled experiment with §2 still in `priority.rs` and an earlier gate), which the relocated §2 + the current §9 reset no longer admit.

**Disposition (judgement call, see §7):** I **kept** the guard — it is mandated by the approved plan, cheap, correct, and enforces the genuinely-correct invariant "a legality probe never runs game-ending shortcut logic" (a CLAUDE.md abstraction-layer concern), and future-proofs against a §9/§2 change that re-opens the recursion. I **reframed** the C-L1-probe test honestly: `drive_drain_idx18_legal_actions_terminates_bounded` is a **bounded-termination property test** (legality probe on a populated ring returns a bounded non-empty list and does not mutate the live game), **explicitly documented as NOT a non-vacuous discriminator of the guard** in the shipped architecture, with the full measurement recorded in its doc-comment. It remains a valid regression guard against a future change that drops the §9 reset.

---

## 7. Judgement calls

1. **idx-17 promotion (target auto-resolution).** §4 said promote IFF the per-beat drive auto-resolves the *sole* legal target with no SelectTargets stop. Measured: no stop, deterministic opponent target, correct win at beat 6. "target player" is nominally 2 legal targets, but the engine's existing non-interactive trigger-target assignment is deterministic and opponent-directed here, and §8/§9 firewalls guard soundness. I promoted it with full disclosure (the test asserts no target window appears + P1 is the faller). Demoting is a trivial one-line change if the orchestrator/review prefers conservatism.
2. **Defect-2 guard kept despite not being load-bearing (§6).** Chose plan-fidelity + defensive-depth + an honest test over removing a mandated change or shipping a vacuous discriminator. Flagged for the orchestrator's decision.
3. **C-neg-D uses a synthetic gain-life artifact** (not a draw artifact) so the finite stack drains without a decking SBA, and the drive stops at stack-empty (driving past empty advances turns into a draw-step deck-out — unrelated to ring hygiene). Asserts ring stayed empty every beat + no GameOver while the stack was non-empty.
4. **§9/§8 negatives (C-neg-B/C/E) rely on the scaffold's comprehensive building-block unit tests** rather than redundant live re-creations (mutual-drain / can't-lose / dual-faller boards are awkward to reach via the pure `PassPriority` drive and are already non-vacuously unit-covered). C-neg-A is the live §9-firewall test.

---

## 8. Stop-and-return items (flagged, not halting)

The implementation is complete and the win is proven; these are surfaced for the `/review-impl` loop to adjudicate:

- **[FLAG] Defect-2 guard is a changed behavioral seam with no non-vacuous discriminating test** (§6). Per the discriminating-test gate this is a "return it" item. The recursion does not reproduce in the shipped code (measured ×3), so no revert-failing test is possible. Orchestrator to decide: (a) keep the guard as defensive depth + the honest bounded-termination test (my choice), or (b) re-scope/remove Defect-2. **Recommend (a)** — the guard is correct, cheap, and the invariant it enforces is right; removing mandated defensive code on a "currently unreachable" basis is riskier than keeping it.

No other stops — Defect-1, the win path, the soundness firewalls, and serialized-surface invariants are all proven.

---

## 9. CR annotations added/changed — grep-verified

CR-annotation diff gate (every CR number in the diff grepped against `docs/MagicCompRules.txt`): **0 UNVERIFIED**. New/touched annotations in the relocated §2 / guard region and corrected docs:

```
grep -nE '^732.2a' docs/MagicCompRules.txt   → 6372  (shortcut procedure)
grep -nE '^603.3'  docs/MagicCompRules.txt   → 2582  (trigger placed when a player would get priority — WHY the refill lands post-pipeline)
grep -nE '^704.3'  docs/MagicCompRules.txt   → 5485  (SBA + waiting-triggers ordering)
grep -nE '^704.5a' docs/MagicCompRules.txt   → 5492  (0-or-less life loses)
grep -nE '^117.4'  docs/MagicCompRules.txt   → 958   (all pass ⇒ resolve)
grep -nE '^119.3'  docs/MagicCompRules.txt   → (life gain — seed comment)
```
Full diff-gate output (all OK, none UNVERIFIED): CR 101.2, 104.2a, 104.2b, 104.3b, 104.4b, 119.3, 603.3, 704, 704.3, 704.5a, 704.5b, 704.5c, 732.2a, 732.4, 732.5, 810.8a. The thread-local guard + `stack_top_before` capture carry plain-English comments only (plumbing, per CLAUDE.md "do not annotate plumbing").

---

## 10. Deviations from the plan

- **Defect-1, Defect-2 code:** implemented exactly as planned (priority.rs reverted to HEAD; §2 relocated with the precise `resolved_this_beat` gate; thread-local guard at the three points). MED-1 (priority.rs byte-identical to HEAD), MED-2 (run_auto_pass_loop scoping in code comment + loop_check doc), LOW-1 (§7 canon cited at resource.rs:751 in C-L2) applied.
- **C-L1-probe REFRAMED** from a "SIGABRT discriminator" to an honest "bounded-termination property test" — because the recursion does not reproduce in the shipped architecture (§6). This is a test-claim correction, not a code deviation; the guard itself is unchanged from the plan.
- **§3 guard + run_auto_pass_loop comments** softened to reflect the measured "defensive, not sole barrier" reality.
- **idx-17 promoted** (the plan left it conditional) because the measurement satisfied the auto-resolution condition.

---

## 11. Risks for `/review-impl`

1. **Defect-2 guard necessity (HIGH attention).** §6: the guard does not change observable behavior in the shipped code (recursion bounded by the §9 pass-state reset). Review should decide whether defensive-depth justifies an unexercised-for-outcome guard. The bounded-termination test + the honest doc-comment are the audit trail.
2. **idx-17 auto-target soundness (MED).** The targeted Sanguine Bond trigger auto-resolves to the opponent without a prompt. If a future targeting change introduces a SelectTargets stop, `drive_drain_idx17_targeted_wins_live` will fail (its no-target-window assertion) — a deliberate regression gate. Whether locking this pre-existing auto-assignment as a corpus driver is desirable is a review call.
3. **Seed faithfulness (LOW).** `seed_lifegain_cascade` uses the real `apply_life_gain` → `process_triggers` chokepoints, then forces a clean `Priority{active}` window (the seed is pre-loop setup). The cascade's refills (beats 2/4/6) go through the fully-natural `run_post_action_pipeline` — the genuine measured path.
4. **MED-2 honesty (LOW).** §3 accelerates only the per-beat `apply(PassPriority)` drive (the production frontend default), NOT `run_auto_pass_loop` (no mid-loop `reconcile`). Documented in code + loop_check doc; no over-claim.

---

## 12. Verification command results (cargo-direct, this worktree)

```
cargo fmt                                            → clean
cargo build -p engine                                → clean
cargo clippy -p engine --lib --tests -- -D warnings  → clean (0 warnings)
cargo test -p engine --lib                           → 13671 passed; 0 failed; 7 ignored
cargo test -p engine --lib -- analysis::             → 96 passed; 0 failed
new live suite (C-L1/C-L2/C-L1-probe/C-neg-A/C-neg-D)→ 5 passed
confirmed_drivers_match_expected / corpus_table_shape_is_locked → pass
```

Serialized-surface confirmation: `loop_detect_ring` = `#[serde(skip, default)]` + NOT referenced in `impl PartialEq for GameState` (eq-excluded, verified); `IN_SIMULATION_PROBE` = thread-local, 0 references in `game_state.rs`; no new `GameEvent`/enum variant; `engine-inventory` unchanged. **Net delta vs HEAD: ZERO.** No parser files touched (parser diff gate N/A).
