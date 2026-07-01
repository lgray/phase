# PR-3 Option C Defect-Fix — Adversarial Plan Review

**VERDICT: CHANGES-REQUIRED** — BLOCKER 0 · HIGH 0 · MED 2 · LOW 1

The two make-or-break architectural **decisions are SOUND** (verified against the actual code, file:line below). The required changes are *spec corrections to the deletion instructions and one over-stated efficacy claim* — **not** a redesign of either fix. The thread-local re-entrancy guard (Defect-2) and the `resolved_this_beat` resolution gate (Defect-1) are both correct as designed.

---

## The two make-or-break answers (definitive, measured)

### (1) Is `SimulationFilter::accept` the ONLY nested-apply re-entry into `reconcile_terminal_result`? — **YES.**

The recursion cycle is `reconcile → §3 → §9 gate → legal_actions → SimulationFilter → apply_as_current → reconcile → §3 …`. For *infinite* recursion you must re-enter `§3`, and `§3` is reached only through `reconcile_terminal_result`, which has **exactly two production callers** — `engine.rs:196` and `engine.rs:200` (both inside `apply_action_boundary_with_stack_limit`; the only other hits are `loop_check.rs:18`/`game_state.rs:5689` doc comments and `engine.rs:7595/7622` which are in a `#[cfg(test)]` module). So a nested reconcile requires a nested `apply_*`. Exhaustively scanning every `apply_as_current` / `apply_action_boundary*` / `engine::apply` call site in `crates/engine/src` and classifying each by `#[cfg(test)]` brace-region:

- **`ai_support/filter.rs:113`** — `SimulationFilter::accept`'s `apply_as_current(&mut sim, …)`. The single production clone-and-apply. `FilterPipeline::default_pipeline()` (`filter.rs:133`) = `BasicLegalityFilter` (no apply — only `cheap_reject_candidate`, `filter.rs:88`) **+** `SimulationFilter` (the apply). `legal_actions → legal_actions_full → validated_candidate_actions (mod.rs:41) → FilterPipeline` — no other apply on the path. **This is the only apply the §9 gate's `legal_actions` can reach** ⇒ the plan's guard (RAII set around `filter.rs:112-113`) sits exactly on the cycle's choke point. **Sufficient.**
- `has_meaningful_priority_action` (`mod.rs:723-747`) — inspects the action slice + `activate_ability_is_meaningful_priority`/`activatable_object_mana_actions`; **no apply** (the only `#[cfg(test)]` marker in `ai_support/mod.rs` is at `:1144`; the apply sites at `:1724/:2788` are below it = tests).
- `engine_resolve_batch.rs:116` (`resolve_all_fast_forward`) — a **top-level** PassPriority driver (only called from its own `#[cfg(test)]` module + re-exported `pub use` at `engine.rs:56` for the transport layer); not nested inside a real apply. Routes through `pass_priority_once_with_pipeline` via the `(Priority,PassPriority)` arm at `:1742`, so it correctly traverses the relocated §2 site.
- `triggers.rs:3543` (`drain_order_triggers_with_identity`) — every caller is `#[cfg(test)]` / `tests/…` / the `scenario.rs` harness; not in the production apply path. And `OrderTriggers ≠ PassPriority`, so the `apply_action`-entry clear (`engine.rs:1696`) empties the ring before its reconcile ⇒ `§3` guard `!ring.is_empty()` is false anyway.
- `scenario.rs:*` (`GameRunner`/`GameScenario`) — test harness; top-level drivers, never nested.

**No unguarded nested-apply path exists.** The recursion provably cannot survive the guard.

### (2) Does `resolved_this_beat` sample resolutions while leaving handoffs intact across all cases? — **YES.**

Control flow re-measured in `priority::handle_priority_pass_with_limit`:
- **Resolution** happens **only** when all living players have passed (`priority.rs:46`) and the stack is non-empty (`:77`): `resolve_next_with_limit` (`:84`) consumes the top entry.
- **Bare handoff** (`priority.rs:136-147`, the `else`) moves priority to the next player and **does not touch the stack**.

Every triggered ability placed on the stack takes a **fresh monotonic id** `ObjectId(state.next_object_id); next_object_id += 1` (`triggers.rs:3894-3895`). Therefore:
- resolution+refill ⇒ top id changes (consumed A's id → refilled B's fresh id) ⇒ `resolved_this_beat = true` ⇒ **sample** (gate: stack non-empty, `len ≥ before`, `wf==Priority{active}`);
- bare handoff ⇒ top unchanged ⇒ `resolved_this_beat = false` ⇒ outer `if` skipped ⇒ **ring left intact** (no sample, no clear);
- drain-to-empty / normal-shrink / interactive-stop ⇒ `resolved_this_beat = true` but the inner gate's `!stack.is_empty()` / `len ≥ before` / `wf==Priority{active}` conjunct fails ⇒ **clear**.

All six rows of the plan's §1.2 case table check out. The team-lead's probe failure modes are covered: a same-id refill is impossible (monotonic `next_object_id`); multi-entry batch resolution still changes the top id; a `len`-equal-but-different-content resolution is precisely the intended sample case.

**Cross-cascade stale-sample concern (team-lead caveat c): not a new surface.** The original *inert* design's §2 lived in the resolve branch of `handle_priority_pass_with_limit` only — it never touched the ring on a handoff either. The relocation **reproduces** the identical "maintain-on-resolution, leave-on-handoff" behavior via `resolved_this_beat`; it does not introduce a new false-positive avenue. The firewall against a false modulo-match stitched across two different cascades remains §7 board-equality (`project_out_resources`, life-insensitive, ids canon'd) + §8 single-faller + §9 gate — all unchanged.

---

## Findings

### MED-1 — The priority.rs deletion as written breaks the `clippy -D warnings` CI gate; the plan's justification is factually wrong.

**Plan §1.3** says: delete `priority.rs:81-82` and `:108-133`, and keep "*the wf compute :99-106 STAY — they are pre-PR-3 logic*."

**This is false.** `git show HEAD:crates/engine/src/game/priority.rs` (base `5eca83b8c`) shows the original resolve branch ended in a **trailing `if/else` expression** — there was no `let wf =` and no trailing `wf`:
```rust
            if matches!(state.waiting_for, WaitingFor::Priority { .. }) {
                reset_priority(state);
                WaitingFor::Priority { player: state.active_player }
            } else {
                state.waiting_for.clone()
            }
```
The PR-3 scaffold **introduced** `let wf = if … {…} else {…};` (current `priority.rs:99-106`) plus the trailing `wf` (current `:134`) specifically to feed the §2 gate. If the executor follows the plan literally — delete `:108-133`, keep `:99-106` and the trailing `wf` — the resulting block is:
```rust
let wf = if … {…} else {…};
wf
```
which is **`clippy::let_and_return`**. CLAUDE.md: "*clippy -D warnings enforced in CI*" ⇒ this fails the gate.

**Fix:** the deletion must also revert `:99-106` + the trailing `wf` (`:134`) back to the original **trailing if/else expression** (drop the `let wf =` binding and the final `wf`). The block then reads exactly as HEAD. Evidence: `git diff` scaffold hunk on `priority.rs`; `git show HEAD:…/priority.rs` lines 97-104.

### MED-2 — "§3 at `:200` covers `run_auto_pass_loop`" is sound for *winner correctness* but the **shortcut does not accelerate the auto-pass path**; the claim is over-stated.

`reconcile_terminal_result` (and therefore §3) is **never called inside** `run_auto_pass_loop` — only at the boundary `:196`/`:200` (re-measured: production callers are exactly those two). Under a sustained `UntilStackEmpty`/`UntilEndOfTurn` auto-pass, `run_auto_pass_loop` (`engine.rs:1225-1267`) keeps calling `pass_priority_once_with_pipeline` (`:1252`) and resolving the cascade **internally**. For the net-progress drain, its strict `loop_states_equal` draw block (`:1282-1298`, life-sensitive) **never matches** (life decreases each cycle), so the loop grinds until either the natural CR 704.5a death fires inside a resolution's SBA pass (P0 wins — correct, but ~victim-life iterations later) or `MAX_EVENT_GROWTH`/`MAX_OBJECT_GROWTH` halts it (`:1261-1266`). In both cases `run_auto_pass_loop` returns at/near the natural end, so §3 at `:200` adds no early shortcut for that path.

**Severity rationale:** soundness is preserved (winner is P0 either way; the guard prevents recursion regardless), and the plan's *primary, tested* target — the **per-beat manual drive** (`C-L1`, corpus idx 18) — does fire §3 at `:196` after each `apply(PassPriority)` (in that drive `run_auto_pass_loop` is a no-op: no auto-pass flag). So this is a **claim-scoping** defect, not a behavioral break. **Fix:** scope the §3-at-`:200` claim to "covers the case where `run_auto_pass_loop` *yields mid-cascade* (Exit/Finish/meaningful-action/halt) with the ring populated," and either (a) add a `drive_*` test that exercises the auto-pass driver to measure whether the shortcut fires before the natural death, or (b) explicitly document that the auto-pass grind path relies on the pre-existing natural-death termination, not the §3 shortcut. Do not assert blanket acceleration of "every PassPriority driver."

### LOW-1 — §7 canon line drift.

Plan §8 / C-L2 cite the `entry.id = ObjectId(pos)` canon loop at "`resource.rs §7, ~:742`". Actual location is **`resource.rs:751`** (inside `project_out_resources`, which begins at `:719`). Cosmetic; update the citation. (Other cited anchors are exact: insertion `engine.rs:482→483`, capture `~:449`, set point `filter.rs:112-113`, §3 guard `:247-251`.)

---

## What I independently re-measured and found CORRECT

- **`pass_priority_once_with_pipeline` anchors are exact.** `:449` `stack_was_empty`, `:455` `handle_priority_pass_with_limit`, `:461`/`:482` `sync_waiting_for`, `:469` `drain_pending_continuation`, `:476` `run_post_action_pipeline`, `:483` `Ok(wf)`. The capture-at-entry / sample-after-`:482` placement is the only frame where a self-refilling cascade is non-shrinking. ✓
- **Single resolution choke point.** `handle_priority_pass_with_limit` has exactly one non-test caller: `engine.rs:455` (inside `pass_priority_once_with_pipeline`). All `priority.rs:225-529` callers are under the `#[cfg(test)]` at `:189`. Relocating §2 here reinstates no per-driver wall. ✓
- **Three drivers confirmed.** `:1252` (`run_auto_pass_loop`), `:1742` (per-beat `(Priority,PassPriority)` arm — also the route for `resolve_all_fast_forward`), `:4630` (`SetAutoPass` immediate pass). ✓
- **§7 canon operates on a CLONE.** `project_out_resources(&GameState) -> GameState` (`resource.rs:719`) mutates the returned copy `s` only; `resolved_this_beat`'s read of the live `state.stack.last().id` is independent. The U-stack test (`modulo_equal_ignores_volatile_stack_entry_id`) asserts the same-source/fresh-id equality **and** the different-source control inequality. ✓
- **§8 `live_mandatory_loop_winner` intact** (`loop_check.rs`): 2-living gate, single-`life_faller` firewall, `any_library_loss`/`any_poison_gain` rejection, `player_has_cant_lose`/`player_has_cant_win` reuse on the live `cycle_end`, and `WinKind::LethalDamage` scoping via `detect_loop` (which re-runs `loop_states_equal_modulo_resources`). ✓
- **§9 gate untouched** (`engine.rs:510`); the guard is added at the §3 *call site* (`:251`), not inside §9. The gate probes **every living player** as the holder — the U-gate test `loop_gate_probes_all_living_players_not_just_current_holder` discriminates it from the current-holder-only `priority_player_has_meaningful_action`. ✓
- **CR 104.4b DRAW block is complementary, not conflicting.** It uses the **strict** `loop_states_equal` (life-sensitive) at `engine.rs:1282-1288`, so it can only match a **net-zero** loop — a net-progress drain never strict-matches. Draw (net-zero, CR 732.4) and Win (net-progress single-faller, CR 704.5a) partition cleanly. The §6 removal correctly deleted only the duplicate WIN site and kept the DRAW block byte-for-byte (`:1289-1298`). ✓
- **Thread-local soundness.** `apply`/`apply_as_current` is synchronous — no `.await` in `engine.rs`; no `rayon`/`par_iter`/`std::thread::spawn` in the apply or `legal_actions` path. The idiom is already in-engine: `perf_counters.rs:19`, `speed.rs:140`, `quantity.rs:1029`, `layers.rs:1369`. The RAII `SimulationProbeGuard` prev-saves on `enter` and restores on `drop` (nesting-correct, panic-safe). The plan binds it as `let _probe = …` (a named binding that lives to end of `accept`) — **not** `let _ = …`, which would drop immediately; the plan got this right. ✓
- **Serialized-surface delta = ZERO.** `loop_detect_ring` is `#[serde(skip, default)]` (`game_state.rs:5706`) and **absent from the manual `impl PartialEq for GameState`** (`fn eq` at `:7869`; grep shows the ring is referenced only at the field/constructor/`normalize_for_loop`/`record_loop_detect_sample`, never in `eq`) — so it is eq-excluded by the manual convention exactly like `static_source_index`. The thread-local adds no field. No new `GameEvent`, no inventory regen, no WASM/TS surface. ✓
- **idx-18 on-paper trace is valid.** With both fixes, on the per-beat drive: handoff beats leave the ring intact, resolution beats sample; by beat 6 the ring holds `[s1,s2,s3]` where s1 and s3 both carry the Conqueror trigger on the stack (same source, fresh id ⇒ modulo-equal after §7 canon), `delta` over s1→s3 is single-faller P1 ⇒ §8 winner P0 ⇒ §9 passes (empty hands, probes guarded so no recursion) ⇒ `GameOver{Some(P0)}` at beat 6, ≪ the ~400 beats a high-life 704.5a death needs. The just-pushed s3 self-compares to `delta≈0` (is_progress false) so it doesn't self-match. ✓
- **`C-L1-probe` is a valid non-vacuous discriminator.** `legal_actions(state)` on a ring-populated state: with the guard, `SimulationFilter::accept` sets the flag before its nested apply ⇒ the probe's reconcile skips §3 ⇒ bounded return. Reverting `&& !in_simulation_probe()` from the §3 guard reinstates the measured `reconcile→§3→§9→legal_actions→SimulationFilter→apply→reconcile` recursion ⇒ SIGABRT ⇒ the test process aborts and cannot pass. Note §3's guard does **not** gate on `resolved_this_beat` (that's §2), so the probe re-enters §3 even on a handoff PassPriority — the recursion (and thus the discriminator) does not depend on a resolution occurring. ✓
- **C-test reverts name real lines:** C-L2 → the canon loop (`resource.rs:751`); C-neg-C → `life_fallers.len()==1` (§8); C-neg-E → `player_has_cant_lose` (§8 / `sba.rs:308`); C-neg-D → the `len ≥ before` conjunct in the §1.2 gate (removing it makes a normal shrink record ⇒ fails the `is_empty()` hygiene assertion). All discriminating. ✓

---

## Out-of-scope note (not a finding)

The architectural premise — that a *net-progress mandatory loop* shortcuts to a CR 704.5a **win** rather than a CR 732.4 **draw** — is the frozen Option C interpretation (§7/§8/§9 from PR-2), explicitly not re-opened by this defect-fix plan. I verified the firewall code is present and reused verbatim; I did not re-litigate the interpretation, which is the user-chosen architecture.

---

## Bottom line for the orchestrator

Approve the **two fixes as designed** — the guard and the resolution gate are both sound and the recursion provably cannot survive. Before implementation, require: **(MED-1)** correct the `priority.rs` deletion spec to also restore the trailing `if/else` expression at `:99-106`/`:134` (the plan's "pre-PR-3 logic" claim is false and the literal deletion fails `clippy -D warnings`); **(MED-2)** scope the §3-at-`:200` claim to "covers `run_auto_pass_loop` *when it yields mid-cascade*" and add/await a measurement of the auto-pass driver rather than asserting blanket acceleration; **(LOW-1)** fix the `resource.rs:751` line citation.
