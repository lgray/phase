# PR-3 plan review — round 1 (pr3-plan-reviewer-r1)

VERDICT: CHANGES REQUIRED — 2 blockers, 1 high.

## Validated (not rubber-stamped)
- Anchors accurate: `run_auto_pass_loop`@engine.rs:1095; strict-draw block + `return`@1193-1208; window push@1220-1223; `emit_resolution_halt`@1290; `auto_pass_loop_max_iterations`=`clamp(stack.len()*living*2+16,500,10_000)`@481-495; `normalize_for_loop`/`loop_fingerprint`/`loop_states_equal`@7754/7712/7773. normalize zeroes ONLY revision/timestamp/object_id/pip_id/dirty — life/library/poison/objects preserved (§0.2 correct). Strict fingerprint hashes per-player life@7725 (§0.4 correct).
- Insertion point (after strict `return`@1208, before window push@1220) leaves strict block byte-for-byte untouched; strict runs first & returns → a true draw can't be hijacked (§3/§7 correct).
- `detect_loop` sig `(start,end,&delta,controller:PlayerId,mandatory:bool)`@loop_check.rs:135 matches §6 call; `classify_win_kind` LethalDamage via negative-life-on-non-controller@258.
- Non-adjacent window scan IS sound for life axis: `loop_states_equal_modulo_resources` (guard #1) only matches same-phase states (integer periods apart) → negative life delta = k×per-period drain; oscillation/net-zero pre-empted by strict draw; period>128 falls through fail-safe.
- All CRs grep-resolve: 732.2a@6372, 732.2b@6375, 732.5@6385, 704.5a@5492, 704.5b@5494, 704.5c@5496, 104.4b@366, 104.2a@330, 732.4@6383, 732.6@6388.
- GameEvent surface: engine-inventory has GameOver×2+ResolutionHalted, no LoopDetected — reuse GameOver keeps it byte-identical (§8 correct). All 4 corpus cards present in card-data.json.

## [BLOCKER] 1 — Firewall bypasses CR 101.2 / 104.2b / 104.3b can't-lose / can't-win
§6 guard set reasons ONLY about resource deltas; omits the loss-PREVENTION layer the engine models:
- `sba.rs:308 fn player_has_cant_lose(state, player_id)` (CR 104.3b + 810.8a) — Platinum Angel / Everybody Lives! `StaticMode::CantLoseTheGame`; SBAs that would make that player lose are SKIPPED.
- `effects/win_lose.rs:91 resolve_win` + `StaticMode::CantWinTheGame` (CR 104.2b) — a player under CantWin can't win.
- engine.rs:7459 models "Platinum Angel ⇒ P0 cannot lose from 0-or-less life."
`handle_game_over_transition` does NOT re-check these (enforcement is in the SBA layer). PR-3 emits `GameOver{winner:Some}` + calls `handle_game_over_transition` directly → BYPASSES the SBA. Concrete false positive: faller controls Platinum Angel (or "can't lose this turn", or winner under Abyssal Persecutor "opponents can't win") → drain drives faller<0 each cycle → all §6 guards pass → PR-3 ends a real game with the WRONG winner. "Sound-by-construction" claim is false; CR 101.2/104.2b/104.3b missing.
**Required:** `live_mandatory_loop_winner` returns None if `player_has_cant_lose(end, faller)` OR winner is under `CantWinTheGame`. Reuse the SBA-layer predicate (expose `player_has_cant_lose` as `pub(crate)`) — don't re-derive. Contradicts plan's "no visibility changes" claim; the fix needs that export.

## [BLOCKER] 2 — DRIVEN_ROW_INDICES integers wrong (hallucinated)
§12 says add `2` (Blight-Priest+Conqueror) and `1` (Sanguine+Exquisite) to `DRIVEN_ROW_INDICES`@corpus_tests.rs:1927. Actual CORPUS: index 1 = Kilo+Freed+Relic (already driven), index 2 = Doc Aurlock (gated_on Some). Real indices: **17 = Sanguine+Exquisite, 18 = Marauding Blight-Priest+Conqueror, 19 = Niv-Mizzet+Curiosity**. Adding `1` = no-op dup; adding `2` = a card-gated row → `confirmed_drivers_match_expected`@1943 asserts `CORPUS[idx].gated_on.is_none()` → FAILS. (Bucket-doc@1908-1911 and :1927/:1943 anchors correct; only the integers wrong.)
**Required:** promote `17` and `18` (Niv `19` stays undriven — dual library-faller correctly rejected by `any_library_loss`); re-verify against live CORPUS ordering.

## [HIGH] — "No meaningful response skipped" linchpin (§0.1) only partly true
The `priority_player_has_meaningful_action` break is at engine.rs:1151 inside the `AutoPassDecision::Exit` arm ONLY (player has NO auto_pass session). `priority_auto_pass_decision`@354 returns `Pass` for an active `UntilEndOfTurn` session WITHOUT a meaningful-action check. So a victim who set "pass until end of turn" while holding a loop-breaking instant is auto-passed into a designated LOSS. Unlike the symmetric CR 104.4b draw, a net-progress WIN is asymmetric & state is changing — the draw's soundness doesn't transfer. `reset_priority` doesn't clear auto_pass; today's `emit_resolution_halt` returns priority (preserving intervention); PR-3 ends the game. Narrow (typical combo victim is sessionless ⇒ Exit arm) but real; task says soundness paramount.
**Required:** gate win emission on an explicit "no living player has a meaningful priority action" check (the cap-path@1126 already uses this predicate — cheap defense-in-depth), OR prove the only reachable session case is an explicit auto-pass decline AND add a negative test ("victim has a meaningful instant ⇒ loop breaks, no GameOver").

## MEDIUM / LOW
- [MEDIUM] Panic risk: `map_delta`@resource.rs drops zero entries, so `delta.life`/`library_delta`/`counters` omit unchanged keys. §6's `delta.life[faller]`, `delta.counters[(Poison,Player)]`, `delta.library_delta[p]` are BTreeMap `[]` indexes → panic on absent key in the live reducer. Use `.get(..).copied().unwrap_or(0)`.
- [LOW] Test-harness gap: L1/L2 require an active `UntilStackEmpty` auto-pass session for `run_auto_pass_loop` to drive the cascade — unspecified. Step 0 should capture exact session setup. L4 (net-zero DRAW fixture) needs a concrete buildable loop cited.
- [LOW] Redundant: §6 calls `loop_states_equal_modulo_resources` (step 1) and again inside `detect_loop` (step 6). Harmless; can drop step 1.
- [LOW] Preamble-collision: a modulo-equal pair where `start` is a pre-cycle transient could mis-measure per-cycle drain; negligible post-32-iters within 128-window; worth one sentence.
