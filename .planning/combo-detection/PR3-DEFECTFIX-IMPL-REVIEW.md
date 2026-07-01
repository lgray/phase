# PR-3 Option C — Defect-fix Implementation Review (independent / adversarial)

> Reviewer: `pr3-impl-reviewer`. Worktree `/home/lgray/vibe-coding/wt-combo-pr3`, branch `feat/combo-detect-pr3`, base `5eca83b8c` (v0.7.0). Full diff: `git diff 5eca83b8c` (7 files, +1181 / −17). All verification ran cargo-direct **in the isolated worktree** (not the Tilt-watched main tree). I did not write this code; every claim below is re-measured, not trusted from the report.

**Maintainer-Simulation Gate: PASS** — every changed seam (§2 ring accumulation, §3 win shortcut, §9 all-players gate, Defect-2 probe flag) has a concrete matrix row covering production entry, first branch, selected authority/bound value, binding time, live-vs-snapshot semantics, storage, consumer, invalidation, hostile fixture, and serialized-surface impact (report §5), and each row checks out against the diff.

---

## VERDICT: CLEAN

0 BLOCKER · 0 HIGH · 0 MED · 1 LOW. All 7 mandatory obligations independently **confirmed** (not merely accepted). **Defect-2 disposition: KEEP the guard (recommendation a)** — independently measured and adjudicated below.

---

## Obligation-by-obligation confirmation (all re-measured)

### (1) HARD GATE holds via discriminating runtime tests — **CONFIRMED**

Re-ran in the worktree:

- `drive_drain_idx18_wins_live` → **PASS** (asserts `GameOver{Some(P0)}`, `beat ≤ 12`, `max_ring ≥ 3`, stack non-empty every beat, P1 drained into (100,200), P0 gained). No SIGABRT.
- `drive_drain_idx17_targeted_wins_live` → **PASS** (asserts no `TargetSelection`/`TriggerTargetSelection` window at any beat, `GameOver{Some(P0)}`, `beat ≤ 12`, P1 < 200). No SIGABRT.
- `drive_drain_idx18_legal_actions_terminates_bounded`, `drive_drain_idx18_victim_with_out_is_not_eliminated`, `drive_finite_stack_keeps_ring_empty` → **PASS**.

Revert-discriminators independently reproduced (edit in source → run → observe → restore):

| Revert (in worktree) | Test | Result | Proves |
|---|---|---|---|
| **§3 win block disabled** (`… && false` on the §3 guard, engine.rs:286) | `drive_drain_idx18_wins_live` | **FAILED** at corpus_tests.rs:2104 (`expect` — no GameOver) | the §3 shortcut is the load-bearing win site |
| **§2 sample disabled** (`resolved_this_beat && !in_simulation_probe() && false`, engine.rs:557) | `drive_drain_idx18_wins_live` | **FAILED** at corpus_tests.rs:2104 | the **persisted ring** is the load-bearing new surface (ring never accumulates ⇒ §3 never matches) |
| **§8 `any_library_loss` dropped** (`… && false`, loop_check.rs:236) | `live_winner_dual_faller_library_is_none` | **FAILED** (`left: Some(..)`, `right: None`) | the §8 second-loss-path firewall is non-vacuous |

Both coverage-map revert-discriminators (`§3 && false`, `§2 sample && false`) are **real**, plus a §8 firewall spot-revert. All restored; `git diff 5eca83b8c --stat` returns to exactly +1181/−17.

### (2) Defect-2 FINDING — **independently reproduced; verdict: KEEP**

I dropped `&& !in_simulation_probe()` from the §3 guard (engine.rs:297) — the executor's "config 1" — recompiled, and ran `drive_drain_idx18*` (3 tests, incl. the direct `legal_actions` probe):

```
running 3 tests
test analysis::corpus_tests::drive_drain_idx18_wins_live ... ok
test analysis::corpus_tests::drive_drain_idx18_legal_actions_terminates_bounded ... ok
test analysis::corpus_tests::drive_drain_idx18_victim_with_out_is_not_eliminated ... ok
test result: ok. 3 passed; EXIT: 0
```

**The recursion does NOT reproduce.** With the §3 guard removed, the `reconcile→§3→§9→legal_actions→SimulationFilter→reconcile` path terminates cleanly (no SIGABRT), idx-18 still wins. The executor's measurement is accurate: the guard is **defensive depth, not the load-bearing barrier** the plan framed it as. Root cause (verified by reading the code): §9 `no_living_player_has_meaningful_priority_action` resets each probe clone's `priority_player`/`waiting_for`/`auto_pass` (engine.rs:599-606), so the nested `SimulationFilter` `apply(PassPriority)` is a handoff that never re-resolves; §3 only fires when a winner is *found*, so it never re-enters from inside a probe. On the real path §3 matches at ring=3 (beat 6) and ends the game before the ring could grow.

**Verdict: KEEP (recommendation a).** The abstraction-layer-invariant justification **legitimately overrides** the discriminating-test gate here, for five reasons:
1. The gate's *purpose* is to stop behavioral changes that ship silently wrong. This change is provably a **no-op** in the current architecture (measured ×3 — executor's two configs + my independent config-1), so it cannot ship silently wrong.
2. The honest alternative to a fabricated discriminator is exactly what was done: `drive_drain_idx18_legal_actions_terminates_bounded` reframed as a bounded-termination **property** test, with the full measurement recorded in its doc-comment and explicitly disclaimed as NOT a guard discriminator (corpus_tests.rs:2186-2206). Shipping a *fake* discriminator would itself **violate** the non-vacuous-test mandate — worse than honest defensive code.
3. The invariant the guard enforces — "a hypothetical single-action legality probe is not a real CR 732.2a play sequence and must never run game-ending shortcut logic" — is independently correct and is precisely the CLAUDE.md "separate abstraction layers" principle. A legality oracle silently ending the game on a throwaway clone is a category error regardless of current reachability.
4. The bound that makes the recursion currently-unreachable (§9's pass-state reset) is **incidental** — §9 resets pass-state to probe each player *as priority holder*, a concern orthogonal to re-entrancy. A future §9/§2 refactor could silently re-open the recursion; the thread-local is the explicit, local, documented barrier and the bounded-termination test is its regression guard.
5. Cost is ≈zero: one branch per reconcile + a `Cell<bool>` thread-local with **zero serialized surface**. Removing a mandated, correct, cheap defensive barrier on a "currently unreachable" basis trades real future-safety for nothing.

The RAII guard is also correct in isolation: set only in `filter.rs:118`, prev-saved/restored on drop, `apply()` is synchronous (no `.await` between set/restore), engine is single-threaded — so `in_simulation_probe()` is provably `false` at every top-level reconcile ⇒ no missed top-level wins.

### (3) idx-17 promotion soundness — **CONFIRMED SOUND** (mechanism differs from the report's description)

I traced *why* the targeted trigger auto-resolves. **The card is not "target player" — it is "target opponent".** The parsed AST in `data/card-data.json` for Sanguine Bond is:

```
oracle_text: "Whenever you gain life, target opponent loses that much life."
trigger.execute.effect = LoseLife { amount: Ref(EventContextAmount),
                                    target: Typed { controller: "Opponent", … } }
```

So the target filter is opponent-restricted. In the 2-player scope (the *only* scope where the win can fire — `live_mandatory_loop_winner` returns `None` unless `living.len()==2`, CR 104.2a), `add_players`-style legal-target enumeration yields **exactly one** legal target (the opponent), so `auto_select_targets_for_ability` (ability_utils.rs:980-1009, limit-2 search → `[only] ⇒ Ok(Some(..))`) auto-assigns it with no `WaitingFor` stop. This is the engine's **standard single-legal-target auto-assignment**, not a default-to-opponent heuristic and not a 2-target ambiguity that "happens to" resolve to the opponent.

**Conclusion:** locking idx-17 as a corpus driver is **sound** and *more robust* than the report claimed. The executor's flagged uncertainty (§7.1: "'target player' is nominally 2 legal targets … I did not exhaustively trace WHY the assignment is opponent-directed") is fully resolved: it is structurally 1 target because the card *says* "target opponent". The test's no-target-window assertion is therefore a stable regression gate, not fragile. No demotion to targeting-deferred is warranted.

### (4) Soundness firewalls §8/§9 — **CONFIRMED non-vacuous**

- **§9 `no_living_player_has_meaningful_priority_action` (engine.rs:602-608) probes ALL living players**, not just the current holder: `state.players.iter().filter(|p| !p.is_eliminated).all(|p| { clone; reset auto_pass/priority_player/waiting_for to p; legal_actions; !has_meaningful_priority_action })`. The unit test `loop_gate_probes_all_living_players_not_just_current_holder` pins the distinction (a non-holder P1 with a non-mana activated ability ⇒ gate `false`, while the current-holder-only `priority_player_has_meaningful_action` sees nothing for P0 — proving the all-players generalization is load-bearing). The live `drive_drain_idx18_victim_with_out_is_not_eliminated` confirms the firewall blocks the shortcut end-to-end. Mana-only actions are correctly excluded (a mana ability can't break a life-drain loop).
- **§8 `live_mandatory_loop_winner` returns `None` for every negative**: mutual drain (`life_fallers.len() != 1`), can't-lose victim (`player_has_cant_lose`), can't-win winner (`player_has_cant_win`), net-zero (no faller), pure mill / dual-faller-library (`any_library_loss`), poison (`any_poison_gain`), 3-player (`living.len() != 2`), board-change (`detect_loop` board-equality), pure advantage (no faller). All 11 `live_winner_*` + `live_winner_net_zero_is_none` unit tests pass; U9/U10 carry fixture-sanity asserts confirming the static is actually present; my §8 spot-revert (above) proves at least one firewall is genuinely discriminating. The win is scoped to the CR 704.5a life axis via `matches!(cert.win_kind, WinKind::LethalDamage)`.

### (5) Serialized-surface / eq invariants — **CONFIRMED**

- `loop_detect_ring: VecDeque<Arc<GameState>>` is `#[serde(skip, default)]` (game_state.rs:5706).
- **Excluded from `impl PartialEq for GameState`** (eq impl at game_state.rs:7871+): grep finds zero references to `loop_detect_ring` in the eq body — only the field decl, `new()`, the `normalize_for_loop` clear (7791), and `record_loop_detect_sample` (7802-7806). AI-search dedup and save/load are unaffected.
- `IN_SIMULATION_PROBE` is a **thread-local** (engine.rs:228), **not** a GameState field: zero references in game_state.rs, zero wire surface, nothing to serde-skip or eq-exclude.
- **No new `GameEvent`/enum variant**: §3 reuses existing `GameEvent::GameOver { winner }` + `WaitingFor::GameOver { winner }`. No `engine-inventory.json` regen needed (no variant added — verified by reading the full diff: only a struct field + thread-local + free functions + tests). `normalize_for_loop` clears the snapshot's own ring (game_state.rs:7791) so stored snapshots have clone-depth 1 (no quadratic/recursive growth).

### (6) CR annotations — **CONFIRMED** (all grep-resolve and describe the code)

All 18 cited rules resolve in `docs/MagicCompRules.txt` and match their use: 732.2a (6372, shortcut procedure ending at a priority point), 603.3 (2582, trigger placed when a player would get priority — the Defect-1 root cause), 603.3b (2586), 704.3 (5485, SBA + waiting-triggers ordering), 704.5a/b/c (5492/5494/5496, the determinate-loss axes), 117.4 (958), 608.1/608.2 (2783/2785), 104.2a (330, sole-survivor win), 104.2b (332, effect-stated win), 104.3b (342), 104.4b (366, mandatory-loop draw), 101.2 (228, rule/effect interaction — the can't-lose/can't-win firewall), 119.3 (1065), 732.4 (6383), 732.5 (6385, no player forced past a loop-ender — the §9 gate), 810.8a (6733, correctly cited as NOT applying outside Two-Headed Giant). Compound `+`/`/` forms used per the documented convention. Thread-local + `stack_top_before` capture carry plain-English comments only (plumbing) — correct per CLAUDE.md.

### (7) Building-block / architecture — **CONFIRMED**

- **Class, not card.** The detector keys off generic `ResourceVector::snapshot`/`delta` + the PR-2 `detect_loop` classifier + `live_mandatory_loop_winner`; it fires for *any* 2-player mandatory self-refilling life-drain cascade with a single faller and no second loss path. ≥3 class members beyond the two corpus rows: Sanguine Bond + Exquisite Blood (idx 17), Marauding Blight-Priest + Bloodthirsty Conqueror (idx 18), Vito/Defiant Bloodlord/Cliffhaven Vampire + Exquisite Blood, etc. Not a special case.
- **Engine owns the logic** (reconcile seam + `analysis::loop_check`/`resource`); transport/frontend untouched; reuses `player_has_cant_lose` (sba.rs, newly `pub(crate)`), `player_has_cant_win` (static_abilities), `legal_actions`/`has_meaningful_priority_action` (ai_support). No duplication.
- **Comparison symmetry is sound** (a key adversarial check): `loop_states_equal_modulo_resources` projects **both** inputs through `project_out_resources` (resource.rs:561-562), which itself begins with `normalize_for_loop()` (resource.rs:589). So the storage-time `normalize_for_loop` on ring snapshots is **idempotent** at comparison time and creates no asymmetry — the §7 stack-id canon (`entry.id = ObjectId(pos)`, resource.rs:751) is the only modulo-layer change and its documented invariant holds: any residual difference only *suppresses* a match (fail-safe), never manufactures a false win. `modulo_equal_ignores_volatile_stack_entry_id` confirms same-source/fresh-id ⇒ equal, different-source ⇒ unequal.
- **Idempotency / no double-fire**: §3's `!matches!(waiting_for, GameOver)` guard makes it safe across the two reconcile calls (engine.rs:196/200); §9 runs once after `find_map`, not per prior. `reconcile_terminal_result` is confirmed NOT called inside `run_auto_pass_loop` (1277-1488) — MED-2's honest scoping (per-beat drive accelerated, auto-pass grind not) is accurate.

---

## Findings

**[LOW]** Comments/docs describe Sanguine Bond as "target player loses that much life" when the real Oracle text and parsed AST are "target **opponent** loses that much life". Evidence: `corpus_tests.rs:2141` test doc, `PR3-PLAN-DEFECTFIX.md` §4, `PR3-DEFECTFIX-IMPL-REPORT.md` §2/§7.1; vs `data/card-data.json` (`controller: "Opponent"`). Why it matters: the inaccurate "nominally 2 legal targets … lands deterministically on the opponent" framing makes idx-17's auto-resolution look like a fragile heuristic, when it is actually the robust single-legal-target auto-assignment of an opponent-restricted effect (structurally 1 target in 2-player). Suggested fix: correct the comment to "target opponent" and note the auto-assignment is structural (one legal target via the opponent-controller filter), which removes the executor's flagged uncertainty and documents *why* the no-target-window assertion is stable. (Code is correct; this is documentation accuracy only.)

---

## Verification ledger (cargo-direct, this worktree)

- `cargo test -p engine --lib analysis::` → **96 passed; 0 failed**.
- Targeted: `drive_drain*` (4), `drive_finite_stack_keeps_ring_empty`, all `live_winner_*` (11), `modulo_equal_ignores_volatile_stack_entry_id`, `loop_gate_probes_all_living_players_not_just_current_holder` → all **pass**.
- 4 spot-reverts (§3, §2 sample, Defect-2 §3 guard, §8 library guard) each behaved exactly as claimed; all restored. `git diff 5eca83b8c --stat` = +1181/−17 (byte-identical to executor state).
- Report's `13671 passed / 7 ignored` is consistent with my filtered-count observations; I did not re-run the full 13.6k suite (worktree Tilt-unwatched; targeted + analysis:: evidence is sufficient and the report's full-suite run stands).

No commits made.
