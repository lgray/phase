# Implementation Plan — The Incredible Hulk (MSH)

**Card:** The Incredible Hulk (MSH), `{2}{R}{R}{G}{G}`, 8/8 Legendary Creature.
**Oracle:** "Reach, trample\nEnrage — Whenever The Incredible Hulk is dealt damage, put a +1/+1 counter on him. If he's attacking, untap him and there is an additional combat phase after this phase."

**Worktree:** `/home/lgray/vibe-coding/wt-msh-hulk` — **cargo-direct** (Tilt does NOT watch this worktree; run `cargo` here yourself).
**Base:** `7e6363fa8` (verified: worktree HEAD == `7e6363fa82a2bad7d75f7c63b3deb51212f1cf2b`, exactly the base).
**Cluster:** S01-reflexive-if-rider — a mid-effect `if <source-state>, <effects>` rider gating SUBSEQUENT sub-effects (CR 608.2c), NOT the trigger's intervening-if (CR 603.4).

All anchors below were re-verified against the worktree at base; the LEAD-VERIFIED grounding held with **two corrections** (see "Corrections to grounding").

---

## 1. Objective + Non-Goals

### Objective
Make the "If he's attacking," rider **gate** the untap + additional-combat-phase sub-effects, while leaving the `+1/+1` counter **ungated**. Today the card parses with no Unknown/Unimplemented but is **semantically wrong**: the rider is recognized then dropped, so Hulk untaps and gets an extra combat phase **unconditionally** on every damage event.

Build this for the **class** (source-anaphoric mid-effect combat-state riders), not just Hulk:
- `SourceIsAttacking`  → `SourceMatchesFilter { Typed([Attacking { defender: None }]) }`  (CR 508.1b)
- `SourceIsBlocking`   → `SourceMatchesFilter { Typed([Blocking]) }`                       (CR 509.1a)
- `SourceAttackingAlone` → `SourceMatchesFilter { Typed([AttackingAlone]) }`               (CR 506.5)

### Non-Goals (explicitly out of scope — leave in the `=> None` bucket, with their existing comments intact)
- **`SourceIsBlocked`** — there is **no** `FilterProp::Blocked`; the only combat-relation prop is `CombatRelation::BlockingOrBlockedBy` (ability.rs:2563-2565), which **conflates** "blocking" and "blocked". A bridge would be semantically wrong. Leave it None.
- **`SourceIsEquipped` / `SourceIsEnchanted`** — `FilterProp::EquippedBy`/`EnchantedBy` (ability.rs:2702-2703) mean "attached **by** something matching [filter]", not "has any Equipment/Aura attached" — different semantics, no clean 1:1. Whiplash / Winter Soldier ("if he's equipped") reach the recognizer too, **but on the TRIGGER intervening-if path** (`TriggerCondition::SourceIsEquipped`), which already works per the grounding — they do **not** depend on this sub-ability bridge. Defer to a separate change with its own verification.
- **`SourceIsMonstrous` / `SourceIsPaired` / `SourceIsHarnessed`** — designation markers with no present-tense runtime `FilterProp`; their existing comments say so. Leave None.
- **No new enum variant.** No new `Effect`, `AbilityCondition`, `StaticCondition`, `FilterProp`, or `TriggerCondition`. (add-engine-variant gate: see §6.)
- **No recognizer/combinator change.** `parse_inner_condition("he's attacking")` already yields `StaticCondition::SourceIsAttacking` (proven by an existing test).
- **No resolver change.** The resolver already gates sub-ability execution on `sub.condition` and already evaluates `SourceMatchesFilter`.
- **Reach / trample** already parse and resolve; untouched.
- Adjacent **target-anaphoric** riders ("if it's attacking" → `TargetMatchesFilter`: Sewers of Estark, Rampaging Geoderm) and **trigger intervening-if** gaps (Krond "it's enchanted") are **different code paths** — do NOT fold them in.

---

## 2. Root Cause (verified end-to-end at base)

1. Enrage is an **ability word** (CR 207.2c) — no rules meaning; the line is a plain `DamageReceived` trigger.
2. The rider "If he's attacking," follows a sentence boundary, so it is **not** hoisted to the trigger as an intervening-if (correct: CR 603.4 applies only to an `if` immediately after the trigger event). It is handled as a per-clause in-effect conditional.
3. The in-effect-if path runs `try_nom_condition_as_ability_condition`, whose final fallback (conditions.rs:**4224**) does `parse_inner_condition("he's attacking")` → `Ok(("", StaticCondition::SourceIsAttacking))`, then `static_condition_to_ability_condition(&SourceIsAttacking, ctx)`.
4. **DROP POINT:** `static_condition_to_ability_condition` (conditions.rs:**3066**) returns **`None`** for `SourceIsAttacking` — it sits in the catch-all `=> None` bucket at conditions.rs:**3298** (verified). With `None`, the in-effect-if returns `(None, text)`, the condition is discarded, both sub-effects parse with `condition: null`, and the `SwallowedClause{ detector: "Condition_If" }` warning fires.

The **only** missing link is the `StaticCondition → AbilityCondition` bridge. Recognition and resolution are already complete.

---

## 3. Building-Block Reuse Table (all verified present at base)

| Need | Building block | Location (verified) |
|------|----------------|---------------------|
| Recognize "he's/she's/they're attacking" → SourceIsAttacking | `parse_contraction_source_state_condition` (source-anaphoric; doc names Hulk) | `parser/oracle_nom/condition.rs:1325-1341` (wired via `parse_source_state_conditions:1349` → `parse_inner_condition`) |
| Single authority for inner conditions | `parse_inner_condition` | `parser/oracle_nom/condition.rs` (delegated to by the in-effect-if path; **do not** bespoke-match) |
| Bridge consumer (in-effect `if`) | `try_nom_condition_as_ability_condition` fallback | `parser/oracle_effect/conditions.rs:4224-4228` (requires `rest` empty — satisfied) |
| **Bridge to patch** | `static_condition_to_ability_condition` | `parser/oracle_effect/conditions.rs:3066` (None bucket at 3290-3332) |
| Bridge precedent (exact pattern to mirror) | `StaticCondition::SourceIsSaddled => SourceMatchesFilter { source_saddled_filter() }` | `conditions.rs:3219-3221`; helper `source_saddled_filter` at `conditions.rs:56-61` |
| Filter wrapper | `TargetFilter::Typed(TypedFilter { properties: vec![...], ..Default::default() })` | `types/ability.rs` (`TypedFilter`); shape proven by `source_saddled_filter` |
| Runtime combat props | `FilterProp::Attacking { defender: Option<ControllerRef> }` (2591, CR 508.1b), `Blocking` (2596, CR 509.1a), `AttackingAlone` (2612, CR 506.5) | `types/ability.rs` |
| Condition evaluator (no change) | `AbilityCondition::SourceMatchesFilter` arm → `matches_target_filter(state, ability.source_id, filter, FilterContext::from_ability)` | `game/effects/mod.rs:7412-7420` |
| Live attacker read | `FilterProp::Attacking` reads `state.combat.attackers` | `game/filter.rs` |
| Sub-ability condition gating (no change) | `sub.condition` checked at 6366, evaluated 6474, skip-on-false 6475; skip path 6507-6549 | `game/effects/mod.rs` |
| Within-sentence chain link (transitive gating) | `SubAbilityLink::ContinuationStep` is `#[default]` | `types/ability.rs:13662-13670` |
| Extra-combat observable (for test) | `state.extra_phases` holds `ExtraPhase { anchor: Phase::EndCombat, phase: Phase::BeginCombat }` | `game/turns.rs:3401-3414` |
| Test harness | `GameScenario`: `advance_to_combat`, `declare_attackers`, `combat_damage`, `is_tapped`, `assert_tapped`, `with_damage_marked`; `Effect::DealDamage` setup at scenario.rs:476 | `game/scenario.rs` |

---

## 4. The Change

### 4.1 Parser change — one bridge arm (no combinator change)

There is **no parser combinator change**: the recognizer is done. The change is a single bridge edit in `static_condition_to_ability_condition`.

**File:** `crates/engine/src/parser/oracle_effect/conditions.rs`

**Edit site:** the `=> None` group currently spanning lines 3277-3332. Remove `SourceAttackingAlone` (3297), `SourceIsAttacking` (3298), and `SourceIsBlocking` (3299) from that `|`-chain, and add three explicit arms **before** the None bucket, mirroring the `SourceIsSaddled` arm at 3219-3221:

```rust
// CR 508.1b + CR 608.2c: source-anaphoric mid-effect "if he's/she's/they're
// attacking" rider. SourceIsAttacking has no dedicated AbilityCondition variant,
// but "attacking" is a runtime FilterProp the resolver already evaluates against
// the ability source, so the gate composes as SourceMatchesFilter against the
// source — mirroring the SourceIsSaddled bridge above. Drives The Incredible
// Hulk's Enrage ("untap him and there is an additional combat phase") gate.
StaticCondition::SourceIsAttacking => Some(AbilityCondition::SourceMatchesFilter {
    filter: TargetFilter::Typed(TypedFilter {
        properties: vec![FilterProp::Attacking { defender: None }],
        ..Default::default()
    }),
}),
// CR 509.1a + CR 608.2c: same bridge for "he's blocking".
StaticCondition::SourceIsBlocking => Some(AbilityCondition::SourceMatchesFilter {
    filter: TargetFilter::Typed(TypedFilter {
        properties: vec![FilterProp::Blocking],
        ..Default::default()
    }),
}),
// CR 506.5 + CR 608.2c: same bridge for "it's attacking alone".
StaticCondition::SourceAttackingAlone => Some(AbilityCondition::SourceMatchesFilter {
    filter: TargetFilter::Typed(TypedFilter {
        properties: vec![FilterProp::AttackingAlone],
        ..Default::default()
    }),
}),
```

Confirm `TypedFilter` and `FilterProp` are already in scope in this module (they are — `source_saddled_filter` at conditions.rs:56 uses both). Use a bare `TypedFilter { properties, ..Default::default() }` — **do NOT** add a `creature()`/type constraint (the saddled precedent does not, and the source is already the creature). **[Correction to grounding: both agents wrote `TargetFilter::Typed(vec![...])` / `TypedFilter::creature().properties(...)`; the verified precedent shape is `TargetFilter::Typed(TypedFilter { properties: vec![...], ..Default::default() })`.]**

**Negation comes for free.** Do **not** add explicit `Not { SourceIsAttacking }` arms. The inner-`Not` sub-match's generic fallback at conditions.rs:3199 (`other => static_condition_to_ability_condition(other, ctx).map(|inner| Not { ... })`) automatically produces `Not { SourceMatchesFilter { Attacking } }` for "isn't attacking" once the affirmative arm exists. (The explicit `Not{SourceIsSaddled}` arm at 3177 is belt-and-suspenders; we rely on the generic path.)

### 4.2 How the condition reaches both sub-effects (resolver-gating finding — addressed)

The grounding's highest-priority open question is **distribution across the conjunction** ("untap him **and** there is an additional combat phase"). Resolved by reading the resolver + parser at base:

- **Resolver honors `sub.condition`** — verified. In the typed sub-ability chain (`game/effects/mod.rs:6336`), `sub.condition` is read at 6366, evaluated at 6474, and on **false** (6475) the sub is skipped. The skip path (6507-6549) then resolves **only** children whose `sub_link == SequentialSibling` (6524); **`ContinuationStep` children are NOT resolved**.
- **AdditionalPhase is a `ContinuationStep` child of the untap** — the two effects are joined by a mid-sentence "and" (not a sentence boundary), and `SubAbilityLink::ContinuationStep` is the `#[default]` (ability.rs:13669). So when the untap's `SourceMatchesFilter{Attacking}` condition is false, the untap is skipped **and** its ContinuationStep AdditionalPhase child is **transitively skipped**. The `+1/+1` counter is a separate first-sentence step (SequentialSibling from the trigger root) and is never under the condition → stays ungated.
- **Where the condition lands** depends on chunking at `mod.rs:20143`:
  - If `split_clause_sequence("untap him and there is an additional combat phase…")` yields **>1 chunk**, `apply_outer_condition_to_clause` (mod.rs:323) stamps the condition on **every** chunk → untap and AdditionalPhase each carry it explicitly. Best case.
  - If it yields **1 chunk** (grounding's claim: "there is" is not a clause-starter), the condition rides the head def via `lower.rs:1000 def.condition(cond)` (untap), and AdditionalPhase is gated **transitively** via the ContinuationStep skip above.

Either way the end behavior is identical and rules-correct. **The discriminating test (§5) is the ground-truth gate** — it asserts the end behavior regardless of which mechanism fires.

**Mandatory implementation step 0 (verify the chain shape before trusting transitive gating):** after the bridge edit, dump the parsed AST for the Hulk line and confirm:
1. `condition` is `Some(SourceMatchesFilter{Attacking})` on the untap (`SetTapState{Untap}`) def;
2. the `AdditionalPhase` def is reached as a `ContinuationStep` (its `sub_link` is `ContinuationStep`, i.e. not serialized / default) **OR** also carries the condition.

If — and only if — AdditionalPhase turns out to be a `SequentialSibling` of the untap with `condition: null`, the transitive skip does NOT cover it and the extra combat would still fire when not attacking. **Fallback design (apply only if step 0 shows this):** extend the 1-chunk path so the recovered condition is pushed onto the head's ContinuationStep child as well — narrowest touch is in lowering where `def.condition(cond)` is stamped (lower.rs:1000): also stamp the immediate ContinuationStep sub when the body is a single chunk carrying an outer condition. Do this **only** if step 0 proves it is needed; otherwise the bridge arm is the whole change.

Quickest way to run step 0 in this worktree: a throwaway unit test (or `dbg!`) that calls the oracle parser on the exact Hulk Enrage line and prints the resulting `AbilityDefinition` chain. Delete it before commit; the real coverage is §5.

---

## 5. Test Plan — non-vacuous, discriminating, with a revert probe

**Location:** inline `#[cfg(test)]` scenario test (e.g. in `game/effects/` near the sub-ability-condition tests) or a dedicated `crates/engine/tests/integration/incredible_hulk_enrage_attacking.rs`, modeled on existing attacking/combat integration tests (e.g. `tests/integration/issue_1124_ohran_frostfang_attacking_deathtouch.rs`). Use `GameScenario`.

**Isolate the single variable = attacking status.** Hold the damage source constant (direct `Effect::DealDamage` to Hulk to fire the `DamageReceived`/Enrage trigger) and vary only whether Hulk is in `state.combat.attackers`. This makes the two cases differ in exactly the bit the fix gates.

### Test A — Hulk IS attacking (proves the gated path FIRES; non-vacuous)
1. Hulk on battlefield (untapped), opponent with a creature to attack.
2. `advance_to_combat()`, `declare_attackers([hulk])` → Hulk is now tapped (CR 508.1f, no vigilance) and in `state.combat.attackers`.
3. Deal direct damage to Hulk (CR 506.4c: a declared attacker stays in combat) to fire Enrage; resolve the trigger.
4. **Assert:** `+1/+1` counter on Hulk (now 9/9) — counter is ungated.
5. **Assert:** `scenario.is_tapped(hulk) == false` — the gated untap fired.
6. **Assert:** `state.extra_phases` contains an `ExtraPhase { anchor: Phase::EndCombat, phase: Phase::BeginCombat }` — the gated extra combat was scheduled.

### Test B — Hulk is NOT attacking (proves the gate BLOCKS; this is the revert probe)
1. Hulk on battlefield, **artificially tapped** (so the untap, if it wrongly fired, would be observable), **not** in combat / not a declared attacker.
2. Deal direct damage to Hulk to fire Enrage; resolve the trigger.
3. **Assert:** `+1/+1` counter on Hulk — counter still applies (ungated). *(Non-vacuity: B is not "nothing happens"; the counter MUST still fire, proving we gated only the rider, not the whole chain.)*
4. **Assert:** `scenario.is_tapped(hulk) == true` — the untap did **not** fire (gated).
5. **Assert:** `state.extra_phases.is_empty()` — no extra combat scheduled (gated).

### Why this is discriminating (revert probe)
- **Revert the bridge arm** (put `SourceIsAttacking` back in the `=> None` bucket → condition dropped → today's bug): Test B step 4 **fails** (Hulk wrongly untaps) and step 5 **fails** (`extra_phases` non-empty). So Test B fails iff the rider is unconditional — it distinguishes the gated fix from the current bug.
- **A naive over-fix that gates the counter too** (e.g. stamping the condition on the parent PutCounter): Test B step 3 **fails** (counter missing when not attacking). So the test pins the counter as ungated.
- **A partial fix that gates only the untap but not the extra combat** (the distribution failure mode): Test B step 5 **fails** (`extra_phases` non-empty). So the test pins the AdditionalPhase gating specifically.
- **Test A is non-vacuous:** it asserts positive effects (counter + untap + scheduled extra combat), so the gate can't pass by simply never firing.

### Parser-level regression (cheap, fast, class-level)
Add a parser unit test asserting the Hulk Enrage line lowers to: ungated `PutCounter`, then an untap sub with `condition == Some(SourceMatchesFilter{ Typed([Attacking{defender:None}]) })`, then an AdditionalPhase reachable under that gate. This is the building-block-level test (covers the class shape, not one card name) and is the fastest revert detector. Pair it with a direct unit assertion that `static_condition_to_ability_condition(&StaticCondition::SourceIsAttacking, ctx) == Some(SourceMatchesFilter{...})` (and the same for `SourceIsBlocking`, `SourceAttackingAlone`) — three assertions that fail immediately if any arm regresses to None.

---

## 6. add-engine-variant gate (run mentally — NO new variant)

- **Existence:** `AbilityCondition::SourceMatchesFilter` exists (effects/mod.rs:7412 evaluator; conditions.rs:3205 affirmative bridge). `FilterProp::Attacking`/`Blocking`/`AttackingAlone` exist (ability.rs:2591/2596/2612). Nothing new to add.
- **Parameterization:** `SourceIsAttacking` is already a leaf `StaticCondition`; the bridge target `SourceMatchesFilter` is the parameterized runtime form. We are wiring an existing leaf to an existing parameterized variant — no sibling proliferation.
- **Precedent:** identical to the sanctioned `SourceIsSaddled` bridge (conditions.rs:3219) and `precise_source_condition_for_prop` policy (use a precise `AbilityCondition` only when one exists; none exists for attacking → `SourceMatchesFilter` is the sanctioned fallback).
- **Conclusion:** the gate is satisfied because **no variant is proposed**. If a future reviewer instead proposes a dedicated `AbilityCondition::SourceIsAttacking`, that path requires a new exhaustive `evaluate_condition` arm and a round-trip arm — strictly more work for no behavioral gain. Reject it in favor of this bridge.

---

## 7. CR Annotations (all grep-verified against `docs/MagicCompRules.txt`)

Add these to the new bridge arms / test (every number below was confirmed present at the line start):
- **CR 207.2c** — ability words appear in italics, have no rules meaning (Enrage). *Do not annotate a 702.x for Enrage.*
- **CR 508.1b** — attacking status / defending-player scope (the "he's attacking" predicate; already the annotation on `FilterProp::Attacking` at ability.rs:2589).
- **CR 506.4c** — a creature attacking a planeswalker/battle remains in combat (justifies that the source is still "attacking" when the damage trigger resolves; used in test setup).
- **CR 509.1a** — declare blockers (for the `SourceIsBlocking` sibling bridge).
- **CR 506.5** — sole attacker / "attacking alone" (for the `SourceAttackingAlone` sibling bridge; matches the `FilterProp::AttackingAlone` annotation at ability.rs:2608-2612).
- **CR 608.2c** — instructions resolved in the order written; later text modifies earlier — **the governing rule** for the sub-ability mid-effect rider gate.
- **CR 603.4** — intervening-if applies only to an `if` immediately following the trigger event; cited to **exclude** (Hulk's `if` follows a sentence boundary, so it is a normal-English mid-effect `if`, not an intervening-if).
- **CR 500.8** — effects can add phases (the AdditionalPhase being gated); **CR 500.10a** — extra phase added only to the affected player's own turn (existing AdditionalPhase infra).
- **CR 122.1** — counters (the ungated `PutCounter` head).

Run the `validate-cr-annotations` skill on the diff before commit.

---

## 8. Risks + Stop-and-Return Triggers

| Risk | Likelihood | Mitigation / Stop trigger |
|------|-----------|---------------------------|
| **Distribution failure:** AdditionalPhase is a `SequentialSibling` (not ContinuationStep) of the untap with `condition: null`, so the extra combat fires when not attacking even after the bridge fix. | Low (mid-sentence "and" defaults to ContinuationStep) but it is the #1 correctness risk. | Implementation **step 0** dumps the AST and checks the link. If it's a SequentialSibling, apply the §4.2 lowering fallback. **Stop-and-return** if neither transitive gating nor the minimal lowering push works without touching shared chain semantics — escalate rather than broaden the lowering change. Test B step 5 is the backstop. |
| Bridge accidentally covers a non-combat sibling we meant to leave None (e.g. typo moving `SourceIsBlocked` out of the bucket). | Low | Only move the three named arms; leave `SourceIsBlocked`/`SourceIsEquipped`/`SourceIsEnchanted`/`SourceIsMonstrous`/`SourceIsPaired` in the None bucket. Compiler keeps the match exhaustive. |
| `ability_condition_to_static_condition` round-trip (conditions.rs:3353, "exhaustive on purpose") mishandles the new `SourceMatchesFilter{Attacking}`. | Low | This inverse is only used for per-`StaticDefinition` keyword-grant gating (Odric-style); Hulk's is a sub-ability gate, and the function returns `Option` with a None default (safe). **Verify** it still returns the expected value (or None) for `SourceMatchesFilter{Attacking}` and does not panic; add an arm only if a real round-trip consumer needs it. |
| `FilterContext::from_ability` does not resolve `ability.source_id` to a battlefield object at trigger-resolution time, so `matches_target_filter` returns false even when attacking. | Low | The `SourceIsSaddled` bridge uses the identical evaluator path in production (Caustic Bronco), so the context is known-good. Test A is the proof. |
| `SwallowedClause{Condition_If}` warning persists after fix (coverage detector). | Low | After the fix the condition is represented, so the detector should clear for this card. **Verify** the warning is gone when card-data is regenerated; if it persists, the condition did not attach (re-check step 0). |
| Running `cargo` competes with another agent's locks. | N/A here | This worktree is **not** watched by Tilt — run `cargo build -p engine` / `cargo test -p engine <test>` directly here. Still run `cargo fmt --all`. |

**Hard stop-and-return triggers:**
1. Step 0 shows AdditionalPhase needs gating but the only way to gate it requires changing shared `split_clause_sequence` / chain-link semantics that affect unrelated cards — escalate.
2. The discriminating Test B cannot be made to fail on revert (would mean it is vacuous) — do not ship; redesign the test.
3. Any need to add a new enum variant appears — re-run the add-engine-variant skill before proceeding.

---

## 9. Verification Cadence (cargo-direct in this worktree)

1. `cargo fmt --all` (always direct).
2. Step 0 AST dump (throwaway), confirm chain shape; decide bridge-only vs +lowering fallback.
3. `cargo test -p engine` for the new parser unit test(s) + the two scenario tests + the three `static_condition_to_ability_condition` unit assertions.
4. `cargo clippy -p engine -- -D warnings`.
5. `validate-cr-annotations` on the diff.
6. Regenerate card-data for Hulk (or full) and confirm the `Condition_If` swallow warning clears and `supported == true` holds.

---

## Corrections to grounding (verified)
1. **Filter shape:** the precedent is `TargetFilter::Typed(TypedFilter { properties: vec![...], ..Default::default() })` (per `source_saddled_filter`, conditions.rs:56), **not** `TargetFilter::Typed(vec![...])` (agent 2) nor `TypedFilter::creature().properties(...)` (agent 1). No `creature()` constraint.
2. **`SourceIsBlocked` is NOT cleanly bridgeable** — there is no `FilterProp::Blocked`; only `CombatRelation::BlockingOrBlockedBy` (conflates blocking+blocked). The grounding listed `SourceIsBlocked` as a class member to bridge "in the same pass"; it must be **excluded** and left in the None bucket. The cleanly-bridgeable class is exactly {`SourceIsAttacking`, `SourceIsBlocking`, `SourceAttackingAlone`}.
