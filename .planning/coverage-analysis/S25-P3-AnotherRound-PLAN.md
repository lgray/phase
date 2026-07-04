# S25 P3 Wave 1 — Another Round — PLAN

## ⚠️ REVIEW OUTCOME (/review-engine-plan, af432ff0ad737e5b8, opus/xhigh): REJECT (variant) → REUSE `repeat_for`. BUILD TO THIS.
The proposed `RepeatContinuation::FixedTimes` variant + frozen-file arms are UNNECESSARY. The plan's premise (repeat_for drops the return) was FACTUALLY WRONG — it saw only the gated driver `drive_repeat_for_outermost` (effects/mod.rs:4427) and missed the UNGATED whole-chain driver. Driver independently CONFIRMED by reading effects/mod.rs:6396-6518:
- `:6396` `repeated_full_chain = ability.repeat_for.is_some() && effective.sub_ability.is_some()` (ungated).
- `:6450` `resolve_ability_chain(state, &full_chain_iteration, …)` loops the FULL exile→return chain per iteration; `:6452` clears inner repeat_for; `:6516` `return Ok(())` self-contained.
- `:6472-6512` interactive pause/resume via `pending_repeat_iteration` (preserves sub_ability = the return; explicit Winds of Abandon citation).
Another Round's shape (exile root + sub_ability=return; member_driven/kind_driven false for an Offset count) reaches this exactly.

**REUSE DESIGN (engine: ZERO changes, ZERO frozen touches, parser-only):**
- Stamp on ROOT: `repeat_for = QuantityExpr::Offset { inner: Box::new(Ref{ QuantityRef::Variable{name:"X"} }), offset: 1 }` = X+1 total ("once + X more"). X=0 → 1 run.
- Correctness rides tested infra: X+1 via fold_compose Offset threads chosen_x (quantity.rs:1228/1231/935), total_iterations frozen once (6350/6509 → CR 107.3a/601.2b); pause at change_zone.rs:636 → pending_repeat_iteration (6472) → drain_pending_repeat_iteration (mod.rs:1023) re-runs full chain, fresh re-prompt each iter (targets NOT cached); blink correct with NO per-iteration reset — interactive exile resume always allocates fresh TrackedSetId + rebinds chain_tracked_set_id (engine_resolution_choices.rs:3408-3411, gated on pending_continuation.is_some() = the return sub) → CR 400.7/603.3d. Do NOT add a chain_tracked_set_id reset. Proven by Winds of Abandon / Doubling Chant.
- Parser: nom arm in `try_parse_repeat_process_directive` (mod.rs:20637) recognizing "<q> more time[s]" via `parse_quantity_expr_number` (oracle_nom/quantity.rs:544). Emit `RepeatProcessOutcome::FixedCount(QuantityExpr)` → `pending_repeat_for` → `ir.repeat_for` → root `repeat_for` (lower.rs:1662-1686 already stamps repeat_for). The `Offset(+1)` wrap goes at the recognizer. Replaces the plan's repeat_until stamping. NO RepeatContinuation change, NO dispatch/drain/scan arms.
- Architectural: RepeatContinuation's doc (types/ability.rs:14564) declares it the NON-COUNT companion to repeat_for; FixedTimes{count} would be a layer violation. Count belongs in repeat_for.

**CONFIRMED-SOUND (carry over):** CR annotations 608.2c/107.3a/601.2b/400.7/603.3d all grep-verified; parse_quantity_expr_number nom approach; exile→return already parses (only the repeat directive is the gap); pause point change_zone.rs:636; test = runtime cast (mirror claim_jumper_repeat.rs). **Cadence: back to parser-only (3-stage effective) — no variant, no frozen touch. /review-impl STILL mandatory.**

**REQUIRED TEST (repeat_for shape):** runtime cast, chosen_x=N, M creatures → assert N+1 exile→return cycles (new object ids per cycle prove blink) + X=0 → exactly 1 cycle. Revert-to-red: Offset(+1)→(+0) or remove the "<q> more times" arm → wrong count / Unimplemented returns. Parser-shape: root `repeat_for == Some(Offset{inner: Ref{Variable{"X"}}, offset:1})`, NO Unimplemented{name:"repeat"}.

---

## ORIGINAL PLAN (SUPERSEDED by the REVIEW OUTCOME above — variant path REJECTED, kept for provenance)

Planner: ab8fae5de119c3763 (opus/xhigh). Base HEAD 5db3047d9. ~~CADENCE: SELF-ESCALATE to 4-stage~~ — REJECTED: reuse `repeat_for` instead (see above).

## Card + measured current parse
Oracle: "Exile any number of creatures you control, then return them to the battlefield under their owner's control. Then repeat this process X more times."
Current AST (Spell): `ChangeZone{Exile, Typed(Creature,You)}` → sub `ChangeZone{Battlefield, TrackedSet(0)}` → sub **`Unimplemented{name:"repeat","repeat this process X more times"}`**. Exile→return-under-owner already parses; the ONLY gap is "repeat this process X more times".

## Why SELF-ESCALATE (measured, reuse rejected)
- **X is variable:** mana_cost shards ["X","X","White"] → "X more times" = mana-{X} = `QuantityExpr::Ref{ QuantityRef::Variable{name:"X"} }` (ability.rs:4506; resolved via chosen_x, quantity.rs:1773). NOT i32.
- **Whole-process repeat** = re-run the exile→return chain = `RepeatContinuation`/`repeat_until` job (drives `resolve_chain_body`, effects/mod.rs:5368). Plain `repeat_for` (QuantityExpr sibling field) does NOT fit: its whole-chain driver `drive_repeat_for_outermost` is gated behind `player_scope|unless_pay` (effects/mod.rs:4427); Another Round has neither → a bare repeat_for on the exile head loops only the exile, drops the return.
- **No existing RepeatContinuation variant fits:** ControllerChoice (interactive), UntilStopConditions (stop-predicate), WhileCondition{condition, max_iterations: Option<u32>} — u32 cap CANNOT hold variable X, and an always-true condition misrepresents a non-conditional card (CR 608.2c pure-count vs conditional). Widening max_iterations→QuantityExpr + faking a condition = 2-part shared-enum change, LARGER blast radius (every WhileCondition site + claim_jumper test) than a clean variant.

## add-engine-variant gate (engine-inventory.json present)
1. Existence — no variant/field expresses "unconditional whole-process repeat, count=QuantityExpr(variable)". repeat_for excluded (gate above).
2. Parameterization — reject reuse-by-hack; FixedTimes is the whole-process analog of repeat_for.
3. Categorical boundary — unconditional count is a distinct CR 608.2c category from predicate/interactive forms.

## Design
- **types/ability.rs:14578** — add `RepeatContinuation::FixedTimes { count: QuantityExpr }` (count = ADDITIONAL iterations).
- **Parser: oracle_effect/mod.rs:20637** `try_parse_repeat_process_directive` — nom arm for "repeat this process <q> more time[s]" via existing `parse_quantity_expr_number` (oracle_nom/quantity.rs:544 → "x"→Variable{X}, number→Fixed). Extend local return tuple to carry Option<QuantityExpr>; when set + no condition → `Continuation(FixedTimes{count})`. Outcome plumbing (mod.rs:21871/21889 → 24252 → lower.rs:2175 `result.repeat_until`) is variant-agnostic; stamps onto ROOT ability; consumes the chunk so today's `Unimplemented{name:"repeat"}` disappears.
- **Resolver dispatch, effects/mod.rs:5259 (FROZEN — new arm)** — mirror WhileCondition: resolve count ONCE (chosen_x), loop resolve_chain_body; reset `state.chain_tracked_set_id=None` each iteration (fresh TrackedSet(0) per blink); on inner pause (exile "any number" → WaitingFor::EffectZoneChoice, change_zone.rs:636) stash `FixedTimes{count: Fixed(remaining)}` into pending_repeat_until; else `should_repeat_fixed_times(&mut remaining)` (remaining==0→stop, else -1→repeat). First body run + X repeats = X+1 total (X=0 → once).
- **Drain resume, effects/mod.rs:690 (FROZEN — new arm)** — mirror WhileCondition:722: peel Fixed(remaining), should_repeat, re-enter resolve_ability_chain with Fixed(remaining-1). Snapshot to Fixed at stash REQUIRED so resume doesn't re-read full chosen_x.
- **ability_scan.rs:249** scan_repeat_continuation — new arm `FixedTimes{count} => scan_quantity_expr(count)` (not frozen).

## CR (grep-verified docs/MagicCompRules.txt)
608.2c (follow instructions / repeat process); 107.3a + 601.2b (X announced at cast → fixed once, chosen_x); 400.7 (returned creature is a NEW object — auras fall, counters cease, tokens vanish); 603.3d (blink ETB/LTB triggers on stack after Another Round resolves).

## Files
types/ability.rs:14578 (variant); parser/oracle_effect/mod.rs:20637 + oracle_nom/quantity.rs:544 (recognizer); **game/effects/mod.rs:5259 & :690 (FROZEN — 2 new arms, the escalation trigger)**; game/ability_scan.rs:249. Frozen filter.rs/delayed_trigger.rs UNTOUCHED.

## Tests (mirror tests/integration/claim_jumper_repeat.rs; card-test)
1. Runtime cast — Another Round, chosen_x=N, M creatures on battlefield; assert exile→return runs N+1 times (count ETB fires / leaves-zone deltas / per-run counter), plus X=0 → exactly 1 run. Revert-to-red: revert FixedTimes arm → collapses to 1 run.
2. Parser-shape — root `repeat_until == Some(FixedTimes{ count: Ref{Variable{"X"}} })`, NO `Unimplemented{name:"repeat"}` sub. Anti-hollow-win: (1) is a real cast.

## Risks / open for /review-engine-plan
(a) exile-selection pause resumes FixedTimes cleanly through drain_pending_repeat_until ordering (effects/mod.rs:663-677); (b) token exiled this way ceases (existing ChangeZone, not our code); (c) resolve_chain_body per-iteration re-selection of "any number of creatures you control" must re-prompt fresh each run — verify targets not cached across iterations; (d) AI: variable-X repeated-blink has no eval surface — flag.

## COMMIT + FOLLOW-UPS (2026-07-04, /review-impl APPROVE-WITH-NITS)
- /review-impl a9477b0ffcd9df35b = APPROVE-WITH-NITS, no must-fix. Corpus blast-radius independently reproduced: EXACTLY 3 cards moved (Another Round 1→0, Professor Onyx 1→0, Development 2→1), 0 LOST, +2 supported. Recognizer eof-guarded, no over-match. repeat_for on ROOT (9 transitions prove driver engaged). Tests discriminating (summoning_sick starts false → set true only on ETB, flips on X=0 revert). CR 608.2c/400.7 grep-verified.
- FOLLOW-UP 1 (MED-latent, DOCUMENTED per reviewer): cast-time exile → each repeat iteration re-blinks the SAME cast-time-chosen set, not re-choosing per process (CR 608.2c). Pre-existing any-number cast-time lowering, not introduced here. Memory: nontargeted-graveyard-exile-casttime-quirk (Another Round added). Count + X+1 cycles correct.
- FOLLOW-UP 2 (LOW, robustness): a hypothetical "if <cond>, repeat this process N more times" would drop the count (condition branch at mod.rs:20706 precedes the more_times arm → WhileCondition{cap:None} unbounded). ZERO corpus cards. Fix if such a card appears.
- FOLLOW-UP 3 (LOW): Professor Onyx / Development count-supported via the recognizer; per-iteration interactive semantics ride the pre-existing repeat_for driver, runtime-untested (only Another Round runtime-verified).
