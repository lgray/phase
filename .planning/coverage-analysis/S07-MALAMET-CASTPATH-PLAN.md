# S07 Increment B — Malamet Cast-Path Plan

**Worktree:** `/home/lgray/vibe-coding/s07-impl-wt` · branch `feat/std-s07-condition-if`.
**Verification:** Tilt does NOT watch this worktree — executor runs **direct cargo** (`cargo test -p engine`, `cargo clippy -p engine`, `cargo fmt --all`).
**Card-data:** read-only from `/home/lgray/vibe-coding/phase-rs-workdir/data/card-data.json`. NEVER edit under the main workdir.

> **PREMISE — increment A is COMMITTED at HEAD `e4128c9b4`.** Build on top of it:
> - `resolve_parent_slot_from_root(state, ability, index) -> Option<TargetRef>` (extracted from `targeting.rs:693-706`; flattens the chain from the root stack entry and indexes; returns `None` when the root has no slot at `index`).
> - `resolve_defined_or_targets` (`game/effects/counters.rs`) honors `ParentTargetSlot{index}` via that helper.
> - The 3 s25 no-op sites resolve `ParentTargetSlot{index}`.
> Rebase/verify HEAD before starting; do not re-implement increment A here.

Goal: make **Malamet Battle Glyph** cast correctly (currently RED/uncastable) and un-break the committed **Longstalk Brawl** cast path, as a set of **class** fixes (also correcting **Duel for Dominance**). **Tail Swipe** shares the fight clause but stays RED — its gap is an unparsed main-phase pump this increment does not address (§1). Four coupled sub-fixes: (a) a class detector for "if [filter] entered this turn", (b) the dual-fight spurious-slot class bug, (c) the counter-target rewrite (re-keyed), (d) the `subject_slot` field on `TargetMatchesFilter`.

---

## 0. Ground-truth AST (MEASURED at HEAD `49b62441a` — supersedes the stale `S07-TARGET-MODEL-PLAN.md` §1)

The executor's step-0 measurement invalidated three claims of the earlier trace. Trust these:

1. **Model verdict = B.** Casting Malamet lands the counter on the opponent only (`mine=0/theirs=1`). Chain descent REPLACES a child's targets with its immediate parent's (`effects/mod.rs:7382-7384`: `sub_with_targets.targets = ability.targets.clone()`); `apply_parent_chain_context` (`:1763-1812`) never merges `targets`. So every node below the two `TargetOnly` nodes sees **most-recent-only**.
2. **Malamet's condition is SWALLOWED, not parsed.** Lowered chain = `PutCounter{condition:null}` + a `SwallowedClause{detector:"Condition_If", "…If the creature you control entered this turn, put a +1/+1 counter"}`. The only "entered this turn" detector today is `oracle_condition.rs:1067` `ParsedCondition::SourceEnteredThisTurn` — **source-referential** ("if ~ entered"), cannot express the filter subject "the creature you control". A prior recognizer was reverted. **There is NO `TargetMatchesFilter` yet for `subject_slot` to attach to** — item (a) creates it.
3. **Counter target is already `ParentTarget` for BOTH cards** (NOT `SelfRef`/`Typed{You}` as the stale §1.3 claimed). The rewrite (c) re-keys on `PutCounter{target:ParentTarget}`.
4. **BOTH cards PANIC on cast — spurious required player slot.** "those creatures fight each other" lowers to `Effect::Fight { target: <unbound filter>, subject: SelfRef }`: `parse_target_with_ctx("each other")` is unrecognized → falls back to `TargetFilter::Any` / empty `Typed{}` (`oracle_target.rs:1428`), which at slot-gen (`targeting.rs:192-244`, `controller==None` → all players) yields `required target slot 2: legal_targets [Player(0),Player(1)]`. Class bug shared by Longstalk, Duel for Dominance, Tail Swipe. **PRE-EXISTING** (see §2.b).

---

## 1. Oracle text (verified in card-data.json)
- **Malamet Battle Glyph:** "Choose target creature you control and target creature you don't control. If the creature you control entered this turn, put a +1/+1 counter on it. Then those creatures fight each other."
- **Longstalk Brawl:** "...Put a +1/+1 counter on the creature you control if the gift was promised. Then those creatures fight each other." (condition = `AdditionalCostPaid`, target-independent — fully fixable by (b)+(c) alone, no (a)/(d) needed.)
- **Duel for Dominance:** "Coven — Choose target creature you control and target creature you don't control. If you control three or more creatures with different powers, put a +1/+1 counter on the chosen creature you control. Then the chosen creatures fight each other." (condition = coven count; needs (b)+(c); condition uses a count gate not "entered this turn".)
- **Tail Swipe:** "...the creature you control gets +1/+1... Then those creatures fight each other." **NON-TRANCHE bystander — stays RED.** Measured AST = `TargetOnly=1` + `Unimplemented=1`: the parser extracts only ONE target and leaves an `Unimplemented` node for the "if you cast during your main phase, …gets +1/+1" gate. (b) alone CANNOT make it castable-with-both-fighting (its gap is the unparsed main-phase pump, which this increment does not address). Not one of the S07 28. Scope: it is only a `(a)/(c)/(d)`-**exclusion** witness (its `Pump`/gate is NOT rewritten to `ParentTargetSlot` and gets no `subject_slot`); do NOT claim it becomes castable or both-fight here.

---

## 2. The four sub-fixes

### (a) NEW class detector — "if [filter] entered this turn"
**Root cause:** no per-target "entered this turn" condition exists; only the source-referential `SourceEnteredThisTurn` (`oracle_condition.rs:1067`).

**Build-block, not a special case:** this is a direct **sibling of `parse_target_attacked_this_turn_condition`** (`parser/oracle_effect/conditions.rs:1482-1500`), which already emits `TargetMatchesFilter { filter: Typed(creature).properties([AttackedThisTurn]), use_lki:false }` for "it attacked this turn". Clone that shape, swapping the verb + property:
- `FilterProp::EnteredThisTurn` already exists (`types/ability.rs:3247`) and is evaluated **per-object** at runtime: `filter.rs:3909` — `obj.entered_battlefield_turn == Some(state.turn_number)` (CR 400.7). No new prop, no new resolver.
- Add `fn parse_target_entered_this_turn_condition` + `_text` wrapper (mirror the attacked-this-turn sibling's `all_consuming` wrapper):
  - **subject parser** must accept BOTH: (i) the anaphor form ("it"/"that creature") — reuse `parse_target_anaphoric_subject:1415` (it returns `()`; the sibling hardcodes the filter as `TypedFilter::creature()` in the emit), and (ii) the **filter form** "the creature you control" — parse via `parse_target`/`parse_type_phrase` → `Typed{creature, controller:You}`. Case (ii) carries the controller into the filter — do NOT hardcode `creature()` for it.
  - verb: `alt((tag(" entered"), tag(" enters")))` then `tag(" this turn")`.
  - emit `TargetMatchesFilter { filter: <subject filter with EnteredThisTurn appended>, use_lki: false, subject_slot: None }`. **`add_filter_property` (`oracle_quantity.rs:3070`) is PRIVATE to its module** — do NOT call it from `conditions.rs`. Instead mirror the sibling: for the anaphor branch use `TypedFilter::creature().properties(vec![FilterProp::EnteredThisTurn])`; for the `parse_target` filter branch, append the prop with a small local helper (push `FilterProp::EnteredThisTurn` into the `TypedFilter.properties`).
- **Register** it in `try_nom_condition_as_ability_condition` (`conditions.rs:4266`) right beside the attacked-this-turn arm at **`:4302`**, so `strip_leading_general_conditional` (`:236→259`) picks it up and attaches it to the swallowed `PutCounter` node's `condition` — clearing the `Condition_If` swallow.
- NEVER verbatim-match Malamet's text. The detector covers the whole "if <filter> entered this turn, <effect>" class.

**Why not reuse `QuantityRef::EnteredThisTurn` (a count)?** It counts battlefield permanents matching a filter; `QuantityCheck{EnteredThisTurn{Creature∧You} >= 1}` would fire if ANY you-control creature entered this turn — not the specific chosen fighter. That is a CR-fidelity bug (the counter goes on THIS creature iff THIS creature entered). Per-target `TargetMatchesFilter` is required.

### (b) Dual-fight spurious-slot CLASS fix (pre-existing; also un-breaks committed Longstalk cast)
**PRE-EXISTING determination:** no `GameRunner::cast(...)` test ever exercised a chained "those creatures fight each other" card (subagent-verified; the class is called out as a known gap in `crates/engine/tests/s07_b3_target_identity.rs:43-51`). Committed Longstalk's only test drives `evaluate_condition` (`effects/mod.rs:17556-17592`), never a cast — so **committed Longstalk has never successfully cast; it panics**. Batch-2 did not introduce the structure; it shipped a card whose latent cast-path panic was never exercised. So (b) is a **pre-existing-bug fix** that also unbreaks Longstalk/Duel (Tail Swipe is NOT unbroken by (b) — its gap is the unparsed main-phase pump). NOT a batch-2 regression, but committed-Longstalk is currently broken-on-cast.

**Two coupled parts (both reuse increment-A's `resolve_parent_slot_from_root` / `flatten_targets_in_chain`):**

**b1 — Lowering: kill the spurious slot, via a SHARED helper covering BOTH fight dispatchers (B3).** "those creatures fight each other" is subject-stripped ("fight" ∈ `PREDICATE_VERBS`, `subject.rs:5026`) to "fight each other" and re-dispatched. There are **TWO** structurally-identical `tag("fight ")` arms that both call `parse_target_with_ctx` on the remainder:
- `parse_targeted_action_ast` — `parser/oracle_effect/imperative.rs:1835-1848`
- `try_parse_verb_and_target` (compound/sequence path) — `parser/oracle_effect/mod.rs:11625-11636`

Patching only one silently no-ops on the other and the cast still panics. **Fix (build-for-the-class): factor `fn parse_fight_target(target_text, ctx) -> (TargetFilter, remainder)`** that returns `TargetFilter::ParentTarget` when `target_text` is the reciprocal "each other" (optionally trailing punctuation) and otherwise delegates to `parse_target_with_ctx`. Call it from BOTH arms. At slot-gen (`ability_utils.rs:1929-1951`) a `ParentTarget` fight target generates NO slot (slots are pushed only for non-SelfRef/non-ParentTarget filters), so the 3rd player slot disappears. Subject stays as lowered — `fight_subject_needs_target_slot(SelfRef|ParentTarget)==false` (`ability_utils.rs:1761`) — so no subject slot either. Do NOT alter "~ fights target creature" / "it fights target creature" (those keep their explicit target slot — the helper delegates unchanged). *Executor: dump Malamet's + Longstalk's parsed `Effect::Fight` to confirm the live parse arm before AND after; both must reach the helper.*

**b2 — Resolver: make both creatures fight under model B WITHOUT regressing incumbent single-target `Fight{ParentTarget}` cards (B1).** With b1 done, the Fight node's local `ability.targets` is model-B most-recent-only (1 object), and `resolve_fight_subject(ParentTarget)` returns `ability.source_id` (the spell — `fight.rs:38-40`), so `resolve_fight_fighters` (`fight.rs:69-98`) would produce a spell-vs-b or self-fight. Fix: in `resolve_fight_fighters`, **before** the `object_targets.len() >= 2` check, attempt a dual-fight divert:
```
if object_targets.len() < 2 {
    if let (Some(TargetRef::Object(a)), Some(TargetRef::Object(b))) =
        (resolve_parent_slot_from_root(state, ability, 0),
         resolve_parent_slot_from_root(state, ability, 1)) {
        if a != b { return Ok(Some((a, b))); }
    }
    // else: fall through to the existing single-target path UNCHANGED
}
```
**Guard rationale (B1):** supported cards with a real `Fight{ParentTarget}` node but <2 chain slots (measured: **`time to feed`** `Fight{target:ParentTarget}`/1-slot, **`ezuri's predation`** `Fight{subject:ParentTarget}`/0-slot, `joust`, `rivals' duel`, `hog-monkey rampage`) run at 1-or-0 local objects + `ParentTarget`, which would satisfy a naive `len<2 && ParentTarget` guard and get their fight wrongly reinterpreted. (The round-1 "6 incumbents" list over-counted at the grep level — kraul harpooner / hans eriksson / skophos maze-warden have NO `Fight` node, so they never reach here.) The divert must fire ONLY when the root genuinely declares ≥2 distinct `TargetOnly` object slots — i.e. `resolve_parent_slot_from_root(0)` AND `(1)` BOTH return `Some(distinct Object)`. For a ≤1-slot root, slot 1 is `None` → fall through to the untouched single-target path (no regression, no panic). Global AST enumeration confirms **exactly 3** cards have `Fight`+≥2 `TargetOnly` slots (Malamet/Longstalk/Duel); every other fight card falls through. Model-independent; mirrors the counters.rs fix. Do NOT change the existing `len()>=2` (flat Ulvenwald-Tracker) path.

Annotate CR 701.14a ("two creatures fight each other"; line 3384, grep-verified).

### (c) Counter-target rewrite (re-keyed on `ParentTarget`)
Post-lowering rewrite in **`lower_effect_chain_ir`** (`parser/oracle_effect/lower.rs:1133`; per memory the chokepoint — triggers/activated abilities bypass the `parse_effect_chain` wrappers). For the class shape "chain declares ≥2 `TargetOnly` object slots AND a later node with **`matches!(node.effect, Effect::PutCounter { .. })`** whose `target` is **`TargetFilter::ParentTarget`**", rewrite that target to **`TargetFilter::ParentTargetSlot { index: 0 }`** (slot 0 = the you-control creature, declared first by `try_parse_two_targets`).
- **Hard `PutCounter`-only guard** — excludes Tail Swipe (`Pump`).
- Consumes increment-A's `resolve_defined_or_targets` `ParentTargetSlot` arm → the counter lands on slot 0 regardless of model-B propagation.
- **Executor: dump the lowered chain first and confirm the actual counter target.** The coordinator measured `ParentTarget` at HEAD; an earlier parser trace saw `SelfRef` for Malamet's "on it" (`resolve_it_pronoun`, `mod.rs:175`). Key the rewrite on "unbound anaphoric counter target" — match `TargetFilter::ParentTarget` and, if the dump shows it, `SelfRef` for the "on it" form — so both cards are covered. Verify slot ordering before hardcoding `index:0` (STOP flag §7).

### (d) `subject_slot` field on `TargetMatchesFilter` + emission
Model B ⇒ the condition subject must test slot 0, not the node-local most-recent target. `TargetMatchesFilter` has no slot field (`effects/mod.rs:7911` hardwires `ability.targets` first-object). Parameterize (don't proliferate):
- **Field:** add `subject_slot: Option<usize>` to `AbilityCondition::TargetMatchesFilter` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. `None` = current behavior (zero change for every existing consumer — measured: all current constructions omit it).
- **Emission:** the SAME `lower_effect_chain_ir` rewrite (c) also sets `subject_slot: Some(0)` on the node's condition, guarded on `matches!(node.condition, Some(AbilityCondition::TargetMatchesFilter { .. }))` AND the two-`TargetOnly`+`PutCounter` class shape. So Malamet's new (a) condition tests slot 0. (Longstalk's `AdditionalCostPaid` and Duel's count gate are not `TargetMatchesFilter` → untouched.)
- **Resolution:** in `TargetMatchesFilter` eval (`effects/mod.rs:7903-7928`), when `subject_slot == Some(n)`, resolve the tested object via `resolve_parent_slot_from_root(state, ability, n)` (increment-A helper) instead of `ability.targets.iter().find_map(first object)`. `None` keeps the existing `find_map`/`TriggeringSource` fallback.
- **Blast radius (measured):** ~67 total `TargetMatchesFilter { ... }` sites (~34 constructions + ~33 destructures), compile-enforced. Cross-crate: `crates/mtgish-import/src/convert/condition.rs` has **3** sites — constructions at `:152` and `:688`, destructure at `:3756` — enumerate all three. Constructions with `..` or that add `subject_slot: None` are trivial; destructures need `subject_slot` or `..`. Prefer `..` at sites that don't care; add `subject_slot: None` explicitly where the codebase favors exhaustive structs.
- **`add-engine-variant` gate (field addition):** run the skill checklist + `cargo engine-inventory` first. (1) Parameterization filter — leaf-level parameterization of the existing "which object does the filter test" axis, not a new concept → parameterize. ✅ (2) Categorical boundary — stays within CR 608.2c anaphora. ✅ (3) Existence/sibling smell — grep `data/engine-inventory.json`; confirm no slot-bearing condition already covers it. ✅

---

## 3. Field-expressiveness check
- **(a) detector →** `TargetMatchesFilter { filter, use_lki, subject_slot }` with `FilterProp::EnteredThisTurn` on the filter: EXPRESSIVE. The filter carries type+controller+entered-this-turn; per-object eval at `filter.rs:3909` tests exactly the slot-0 creature. ✅
- **(b) Fight target =** `ParentTarget` + `resolve_parent_slot_from_root` indices 0/1: EXPRESSIVE — the two declared slots ARE the two fighters in declared order. ✅
- **(c) counter target =** `ParentTargetSlot{0}`: EXPRESSIVE (increment-A verified). ✅
- **(d) `subject_slot: Option<usize>`:** EXPRESSIVE for "test chain slot N". `None` preserves legacy. ✅
No STOP-level field gaps. The only new type surface is the (d) field, gated by `add-engine-variant`.

---

## 4. CR annotations (grep-verified against `docs/MagicCompRules.txt`)
- **CR 400.7** (line 1950) — object entered the battlefield this turn / new-object identity. Annotate the (a) detector + `FilterProp::EnteredThisTurn` usage.
- **CR 608.2c** (line 2793) — later-text anaphora / "read the whole text". Annotate (a) condition subject, (c) counter slot rewrite, (d) `subject_slot` resolution.
- **CR 701.14a** (line 3384) — "two creatures to fight each other". Annotate (b1) lowering + (b2) `resolve_fight_fighters` dual-fight resolution.
Do NOT invent 701.x numbers from memory — each grep-verified above.

---

## 5. Tests — FULL cast, non-vacuous (NOT `evaluate_condition`)
Use `GameScenario` + `GameRunner::cast(...).resolve()` and assert resolved deltas (memory: byte-identical AST proves nothing; the committed-Longstalk blind spot was an `evaluate_condition`-only test that never cast). Each test needs a negative sibling that discriminates the fix from the pre-fix bug (uncastable panic / mislanded counter).

**T-cast-panic (b), the castability gate.** Cast **Malamet, Longstalk, Duel for Dominance** (NOT Tail Swipe — see below) end-to-end with two legal creatures; assert cast **does not panic / does not demand a 3rd (player) target**, and **BOTH creatures take fight damage** (each = other's power). Pre-fix sibling: assert the pre-fix AST produces a 3rd required slot / no dual damage. This is the (b) discriminator and proves b1+b2 together (these 3 are exactly the cards that fire both b1 and b2).
**T-malamet (a)+(c)+(d).** you-control creature entered THIS turn + opponent creature → `counters_on(you_control)==1`, `counters_on(opponent)==0`, both fight-damaged. Negative: you-control creature entered a PRIOR turn → `counters_on==0` on both, fight still happens. Second negative: swap which creature is declared 2nd (most-recent) → counter still on the you-control creature (guards model-B regression on both target AND condition subject).
**T-longstalk (b)+(c).** gift promised → `counters_on(you_control)==1`, `counters_on(opponent)==0`, both fight-damaged, castable. gift NOT promised → `counters_on==0` both, still fights. (Condition is `AdditionalCostPaid`, so no (a)/(d) — this proves the counter-target rewrite + fight fix independent of the entered-this-turn detector.)
**T-duel (b)+(c) regression.** coven satisfied → counter on you-control + both fight + castable; coven not satisfied → no counter, still fights.
**T-tailswipe — parse-level exclusion ONLY (stays RED).** Do NOT cast (measured AST = `TargetOnly=1` + `Unimplemented=1`; the unparsed main-phase pump keeps it gap>0, which (b) does not fix). Parse-only assertions: its `Pump` node was NOT rewritten to `ParentTargetSlot`, it carries no `subject_slot`, and it still has an `Unimplemented` node (remains unsupported). This witnesses the `PutCounter`-only guard (c) and the class-shape guard (d) without claiming castability.
**T-fight-guard-fallthrough (B1), the b2-guard discriminator — two levels, both non-vacuous.**
- **Synthetic unit test (primary, guaranteed non-vacuous, no card-data dependency):** build a `resolve_fight_fighters` input with ONE local object target + a 1-`TargetOnly` root (or a `Fight{subject:ParentTarget}` with a single chain slot). Assert `resolve_parent_slot_from_root(1) == None` → the dual-fight divert does NOT fire → the existing single-target path returns the expected `(subject, target)` pair. A sibling with a genuine 2-slot root fires the divert and returns `(slot0, slot1)`. This directly proves the narrowed guard on both branches without depending on any card's parse.
- **Real-card cast regression (corroboration):** pick cards that MEASURABLY carry a real `Effect::Fight` node reaching `resolve_fight_fighters` (the round-1 kraul harpooner / hans eriksson / skophos maze-warden are VACUOUS — measured `Fight=0`, no Fight node — do NOT use them). Use **`time to feed`** (`Fight{target:ParentTarget}`, 1 slot) and **`ezuri's predation`** (`Fight{subject:ParentTarget}`, 0 slots) — or `joust`/`rivals' duel` as substitutes. **Each test MUST first assert the card parses to a `Fight` node** (self-validating non-vacuity), then cast and assert fight damage UNCHANGED vs pre-fix. Slot 1 → `None` for their ≤1-slot roots, so the divert falls through — this FAILS under a naive `len<2 && ParentTarget` guard and PASSES with the both-slots-Some narrowing.
**T-detector (a) building-block.** Parse "If the creature you control entered this turn, put a +1/+1 counter on it." (and the "it entered this turn" anaphor variant) → assert `TargetMatchesFilter { filter: Typed(creature, controller:You, [EnteredThisTurn]), .. }`, no swallowed `Condition_If`. Negatives (A2 — over-fire guards; the detector registers at `conditions.rs:4302`, BEFORE the control-count gate, so `all_consuming` must reject these so they still reach the count-gate path): (i) "if you control a creature that entered this turn" (control-count → `QuantityCheck`/count gate, NOT `TargetMatchesFilter`); (ii) "if a creature entered the battlefield this turn" (existential count); (iii) "if ~ entered this turn" still maps to `SourceEnteredThisTurn` (don't cannibalize the source-referential form). Assert each does NOT parse as the new per-target detector.
**T-subject_slot (d) building-block.** `TargetMatchesFilter { subject_slot: Some(0) }` on a two-target chain tests slot 0; `Some(1)` tests slot 1; `None` tests node-local first object (legacy).

**Regression basis (B2 — corrected; measured at HEAD):**
- **`PutCounter{ParentTargetSlot}`: 0 existing cards** → the (c) rewrite has zero over-match.
- **`PutCounter{ParentTarget}` + ≥2 `TargetOnly` slots: exactly 3 cards = Malamet / Longstalk / Duel** → (c) hits precisely the intended class; Duel's counter target IS `ParentTarget` (correctly rewritten).
- **b2 guard fall-through:** cards with a real `Fight{ParentTarget}` node but <2 chain slots (`time to feed`, `ezuri's predation`, `joust`, `rivals' duel`, …) resolve slot 1 → `None`, so the **narrowed b2 guard (both slot 0 and slot 1 resolve to Some distinct objects)** does NOT intercept them. (The round-1 "6 incumbents" list was grep-over-counted — kraul harpooner / hans eriksson / skophos maze-warden have NO `Fight` node at all.)
- **b1 scope (R2-3, corrected):** `parse_fight_target` rewrites the `Fight` target `Typed{empty}`→`ParentTarget` for **10** cards: `arena, blizzard brawl, duel for dominance, hog-monkey rampage, joust, longstalk brawl, magus of the arena, malamet, rivals' duel, tail swipe`. Only **3** of these (Malamet/Longstalk/Duel) also fire b2. For the other **7** (≤1 slot), b1 removes the cast-panic but the fight then resolves via the single-target fall-through (source = spell vs first target → spell not `fight_eligible` → no damage) — semantically imperfect but **NOT a coverage regression** (b1 adds/removes no `Unimplemented` nodes; it does not flip any card's supported status by itself). The claim "b1 fires for exactly 3" is FALSE — it touches all 10.
- **10-card gate (mandatory):** run `cargo coverage` + `cargo semantic-audit` over all 10 b1-touched cards and confirm (a) **none falsely flips to `supported`** (the Unimplemented→supported trap — the 7 non-tranche fight cards must keep their existing supported/gap status unchanged) and (b) the ONLY intended `supported:true gap_count:0` flips are **Malamet / Longstalk / Duel**.
- Run existing `s07_longstalk_brawl_counter_gated_on_gift_promised` + the card-data coverage-regression check. Executor re-runs these enumerations and pastes the counts.

---

## 6. Files the executor will touch
| File | Change |
|------|--------|
| `crates/engine/src/parser/oracle_effect/conditions.rs` | (a) add `parse_target_entered_this_turn_condition[_text]` (sibling of the attacked-this-turn fn) + register at `:4302` |
| `crates/engine/src/parser/oracle_effect/mod.rs` | (b1) add `parse_fight_target` helper ("each other"→`ParentTarget`, else delegate to `parse_target_with_ctx`) |
| `crates/engine/src/parser/oracle_effect/imperative.rs` | (b1) call `parse_fight_target` from the `:1842` fight arm |
| `crates/engine/src/parser/oracle_effect/mod.rs` :11629 | (b1) call `parse_fight_target` from the compound/sequence fight arm |
| `crates/engine/src/game/effects/fight.rs` | (b2) `resolve_fight_fighters`: dual-fight divert ONLY when both `resolve_parent_slot_from_root(0)`+`(1)` are `Some` distinct objects; else fall through |
| `crates/engine/src/parser/oracle_effect/lower.rs` | (c)+(d) post-lowering rewrite in `lower_effect_chain_ir`: `PutCounter{ParentTarget}`→`ParentTargetSlot{0}` + set `subject_slot:Some(0)` on the node's `TargetMatchesFilter` |
| `crates/engine/src/types/ability.rs` | (d) add `subject_slot: Option<usize>` (serde default/skip) to `AbilityCondition::TargetMatchesFilter` |
| `crates/engine/src/game/effects/mod.rs` | (d) `TargetMatchesFilter` eval honors `subject_slot` via `resolve_parent_slot_from_root` |
| `crates/mtgish-import/src/convert/condition.rs` | (d) 3 cross-crate sites: constructions `:152`, `:688`; destructure `:3756` |
| ~67 engine sites (~34 constructions + ~33 destructures) | (d) compile-enforced `subject_slot: None` / `..` |
| test files (`crates/engine/tests/` + colocated) | §5 full-cast tests |

Increment A owns `counters.rs` + the 3 s25 files + `targeting.rs` helper — do NOT re-edit them here beyond calling `resolve_parent_slot_from_root`. Re-read shared files (`effects/mod.rs`, `lower.rs`) before editing (multi-agent safety).

---

## 7. STOP-AND-RETURN / escalation (NO-DEFERRALS: Malamet MUST ship)
1. **Increment-A not landed.** If `resolve_parent_slot_from_root` / the counters.rs arm are not committed, STOP — this plan depends on them.
2. **Slot ordering.** If a chain dump shows `try_parse_two_targets` emits you-don't-control at index 0, flip the `index`/`subject_slot` value; confirm before hardcoding `0`.
3. **`add-engine-variant` gate fails** for the (d) field (inventory reveals a conflicting slot semantic) → STOP and escalate to the coordinator with the specific blocker. Do NOT silently leave Malamet RED.
4. **(b2) both-fight not achieved.** If after b1+b2 the T-cast-panic test still shows only one creature damaged, escalate with the measured chain dump — do NOT fall back to a broad `effects/mod.rs:7382-7391` propagation change without re-planning.
Model B is NOT itself a stop — items (a)–(d) ship Malamet under model B.

---

## 8. Verification cadence (direct cargo — Tilt unwatched)
`cargo fmt --all` → `cargo engine-inventory` (before the (d) field) → parser combinator gate on touched parser code → `cargo test -p engine` (§5) → `cargo clippy -p engine -D warnings` → **`cargo coverage` + `cargo semantic-audit` over the 10 b1-touched cards** (arena, blizzard brawl, duel for dominance, hog-monkey rampage, joust, longstalk brawl, magus of the arena, malamet, rivals' duel, tail swipe): confirm no false `supported` flip and only Malamet/Longstalk/Duel flip to `supported:true gap_count:0`; Tail Swipe stays RED. Report: cast-panic gone (Malamet/Longstalk/Duel), per-creature counter + fight-damage deltas, detector (a) AST, the synthetic + real-card fight-guard test results, the 10-card coverage/semantic-audit table, the (d) blast-radius count, and any escalation.
