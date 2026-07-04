# S07 Target-Model Increment — Plan

**Worktree:** `/home/lgray/vibe-coding/s07-impl-wt` · branch `feat/std-s07-condition-if` · HEAD `49b62441a` (rebased onto upstream/main `6cefafb21`).
**Verification:** Tilt does NOT watch this worktree. Executor runs **direct cargo** in the worktree (`cargo test -p engine`, `cargo clippy -p engine`, `cargo fmt --all`).
**Card-data:** read-only from `/home/lgray/vibe-coding/phase-rs-workdir/data/card-data.json` (`.[name]` scalar `text`). NEVER edit under the main workdir.

## Objective

One increment, two consumers:
- **(a) s07** — a class-wide correctness bug: in a "two `TargetOnly` + conditional +1/+1 counter + fight" chain, the counter's target anaphor does NOT reference the **first** declared target (the *you-control* creature). Re-enable **Malamet Battle Glyph** (currently honest-RED) and fix the latent bug in already-committed **Longstalk Brawl** (`48b0d4f5d`).
- **(b) s25 critical path** — make `TargetFilter::ParentTargetSlot { index }` resolve correctly at **three delayed-trigger/snapshot sites** that today are `ParentTarget`-only and silently drop the index.

Building-block discipline: reuse `ParentTargetSlot` and `flatten_targets_in_chain`. No new engine **variant** (sibling) is added; under model B (the likely propagation model, §1.6) one optional **field** — `subject_slot` on the existing `TargetMatchesFilter` — is added via parameterize-don't-proliferate + the `add-engine-variant` gate (§3.1.1).

---

## 1. Trace-first (measured, file:line)

### 1.1 The card shape (Oracle text — verified in card-data.json)
- **Malamet Battle Glyph:** "Choose target creature you control and target creature you don't control. **If the creature you control entered this turn, put a +1/+1 counter on it.** Then those creatures fight each other."
- **Longstalk Brawl** (2nd sentence on): "Choose target creature you control and target creature you don't control. **Put a +1/+1 counter on the creature you control if the gift was promised.** Then those creatures fight each other."
- Out-of-scope-to-flip but must-not-regress: **Duel for Dominance** ("...put a +1/+1 counter on the chosen creature you control..."), **Tail Swipe** ("...the creature you control gets +1/+1...").

### 1.2 Parsed chain (high certainty)
"Choose target … and target …" is detected by `try_parse_two_targets` and lowered to two chained `Effect::TargetOnly` nodes (`parser/oracle_effect/imperative.rs:3902-3943`, `:9821-9843`). The conditional-counter clause becomes a downstream `Effect::PutCounter` node carrying the condition; the fight clause becomes `Effect::Fight` (subject rebound to `ParentTarget`, `parser/oracle_effect/mod.rs:14093-14114`). Chain assembled in `lower_effect_chain_ir` (`parser/oracle_effect/lower.rs:1133`).

### 1.3 The counter's TARGET — "parser emits the wrong ref" (CONFIRMED, this is the primary s07 defect)
The counter target is produced by `resolve_counter_placement_target` (`parser/oracle_effect/counter.rs:160-250`):
- **Malamet "on it":** `is_it_pronoun("it")` true → `resolve_it_pronoun(ctx)` (`counter.rs:196-197`). In spell context (`ctx.subject` is `None`/`SelfRef`/`Any`) `resolve_it_pronoun` returns **`TargetFilter::SelfRef`** (`parser/oracle_effect/mod.rs:175-182`). At runtime `SelfRef` resolves to `ability.source_id` (`game/effects/counters.rs:1406-1408`) — the **spell object itself**, not a creature. Definitively wrong.
- **Longstalk "on the creature you control":** `parse_target_with_ctx("the creature you control")` (`counter.rs:248`) returns a **fresh re-resolving `Typed { controller: You, … }`** filter — independent of the two declared slots.

Neither path emits `ParentTargetSlot { index: 0 }`. **This is a parser-emits-wrong-ref defect** (not the engine dropping an index): the parser never expresses "the first declared target."

### 1.4 The counter's runtime RESOLUTION also can't index (CONFIRMED, secondary defect)
`resolve_defined_or_targets` (`game/effects/counters.rs:1367`, the `PutCounter`/`RemoveCounter`/`MultiplyCounter` target resolver) has **no `ParentTargetSlot` arm**. Its fall-through (`counters.rs:1487-1497`) `filter_map`s **all** object refs in the node's local `ability.targets` with no index and no chain flatten. Contrast the two resolvers that DO index correctly: `game/effects/mod.rs::effect_object_targets:217-238` and `game/targeting.rs::resolved_targets:693-706` (the latter flattens the whole chain from the root stack entry via `flatten_targets_in_chain`, then `effect_object_targets` applies the index — comment at `targeting.rs:679`). So even if the parser emitted `ParentTargetSlot{0}`, the counter resolver would ignore the index. **Both the parser ref and the counter resolver must be fixed.**

### 1.5 The condition SUBJECT (measured constraint — drives §3)
Malamet's condition parses to `AbilityCondition::TargetMatchesFilter { filter: Creature∧You∧EnteredThisTurn, use_lki:false }`; Longstalk's to the gift gate (`AdditionalCostPaid`, unaffected). `TargetMatchesFilter` evaluation (`game/effects/mod.rs:7903-7972`) tests the **first object** in the node's `ability.targets` (`find_map` at `:7911-7917`) — it has **no slot field**. So the condition is correct **iff the counter node's first object target is the you-control creature.**

### 1.6 The two propagation models — genuinely UNKNOWN, evidence LEANS model-B
The counter node and the chained `Fight` node share one target vector, so measuring the fight settles the counter node too:
- **Chained `Fight` inherits, never declares slots.** `fight_subject_needs_target_slot(ParentTarget) == false` (`ability_utils.rs:1755-1761`) — the "those creatures fight each other" Fight (subject rebound to `ParentTarget`) is assigned NO local slots and inherits its fighters from its parent, the counter node. Therefore **the counter node's local `ability.targets` == exactly what Fight reads for its two fighters.**
- **Descent REPLACES child targets with the immediate parent's** (`effects/mod.rs:7382-7384`): `if sub.targets.is_empty() && !ability.targets.is_empty() { sub_with_targets.targets = ability.targets.clone() }` — a clone/replace of the *immediate* parent's vector, NOT an accumulate. `apply_parent_chain_context` (`effects/mod.rs:1763-1812`) copies context/`ability_index`/`chosen_players`/effect-context-object but **does NOT merge `targets`**. So if the two `TargetOnly` nodes are nested (root=`[you]` → sub=`[opp]` → PutCounter=`inherits [opp]` → Fight=`inherits [opp]`), every downstream node sees **most-recent-only `[opp]`** = **model B**.
- **Direct measurement leans B:** `.s07-b3-log.md:33` recorded a revert-probe with an unconditional counter landing on the **opponent only** (`theirs=1, mine=0`) — consistent with model B, not with model A (which, under the buggy `filter_map` fall-through, would land the counter on **both** creatures).

> **RETRACTED citation:** the earlier draft leaned on `fight.rs:361 dual_target_fight_uses_both_chosen_creatures_not_ability_source`. That test hand-builds a **flat single-node** `ResolvedAbility { Effect::Fight, targets: vec![bear, wolf] }` with both fighters injected directly (`fight.rs:369-377`) — it never exercises the real `TwoTargets→TargetOnly→TargetOnly→PutCounter→Fight` chain, so it proves nothing about what a *chained* Fight/counter node *inherits* (byte-identity ≠ runtime-equivalence). Do not rely on it.

**Model A** (counter node holds `[you, opp]` in order): §1.5 condition already correct; only the counter *ref* (§1.3/§1.4) is broken. **Model B** (counter node holds `[opp]`): the condition subject ALSO reads the wrong creature → needs the §3.1.1 fix. **Both models are covered below; the executor must disambiguate at step 0. Plan for model-B.**

> **Executor step 0 (mandatory, cheap, ~2 min) — DUAL FIGHT-DAMAGE discriminator:** parse Malamet + Longstalk, dump the lowered chain. Then cast Malamet (or Longstalk) end-to-end on a board with a you-control creature + an opponent creature and **assert resolved fight-damage on BOTH creatures**. `counters_on()` alone is AMBIGUOUS (under the buggy fall-through model A lands the counter on both, yet b3-log saw a single opponent landing). **Both creatures take fight damage ⇒ model A** (counter node holds `[you, opp]`). **Only one creature damaged ⇒ model B** (most-recent-only) ⇒ Malamet's condition subject reads the wrong creature ⇒ apply §3.1.1. Record the measured verdict in the executor report.

### 1.7 The three s25 no-op sites (CONFIRMED)
Each resolves `ParentTarget` but silently passes `ParentTargetSlot` through unchanged (index dropped → empty/wrong resolution). Note: the task's "does not appear in filter.rs" is stale — `ParentTargetSlot` IS handled in several filter.rs/effects functions already; the genuine gaps are exactly these three:

1. **`game/effects/delayed_trigger.rs::concrete_parent_target_filter:318-341`** — arm `TargetFilter::ParentTarget => parent_targets_filter(parent_targets)` (`:324`); `ParentTargetSlot{index}` falls to `other => other` (`:340`) = no-op. This binds delayed-trigger CONDITION filters (`WhenDies`/`WhenEntersBattlefield`/`WheneverEvent`, via `bind_contextual_filter_to_condition:251`) to the concrete parent object.
2. **`game/effects/mod.rs::filter_refs_parent_target:4106-4119`** — matches `ParentTarget`/`ParentTargetController`/`ParentTargetOwner` (`:4108-4110`) but NOT `ParentTargetSlot`. Consumed by `effect_refs_parent_target` at delayed-trigger creation (`delayed_trigger.rs:124`); if a delayed effect's target is `ParentTargetSlot`, the parent-target **snapshot is skipped** → `delayed_ability.targets = []` → the effect silently no-ops at firing. (The snapshot itself, `parent_target_snapshot:180`, already copies the full `ability.targets`, so once detection fires the index survives.)
3. **`game/filter.rs::normalize_contextual_filter:459-465`** — handles only `Not(ParentTarget)`; `Not(ParentTargetSlot{index})` (and positive `ParentTargetSlot` inside `And`/`Or`) are unhandled. Called first inside `concrete_parent_target_filter` (`delayed_trigger.rs:322`), so a `Not`-wrapped slot ref in a delayed condition never resolves.

---

## 2. Field-expressiveness check (S25 cross-pollination — non-negotiable)

**`TargetFilter::ParentTargetSlot { index: usize }` — counter target.** VERDICT: **EXPRESSIVE.** `index: 0` names the first declared chain slot; `flatten_targets_in_chain` (`game/ability_utils.rs:1404-1417`) already produces slots in declared order (`TargetOnly(you-control)` contributes index 0, then the opponent slot at index 1). The existing indexed consumers (`effect_object_targets`, `targeting.rs:803`, `filter.rs:3116`) prove the field carries the full semantics. No new field/variant required for the counter target.

**`AbilityCondition::TargetMatchesFilter { filter, use_lki }` — condition subject.** VERDICT: **CURRENT FIELDS CANNOT express "test slot N"** — the subject is hardwired to the first object of the resolving node's `ability.targets` (`mod.rs:7911`). It is correct **only** when that first object is the you-control creature.
- **Model A** (step 0 shows both creatures take fight damage → counter node holds `[you, opp]` in order): condition already correct → **no condition change, no new field.** ✅
- **Model B** (step 0 shows one creature damaged → counter node holds `[opp]`): the condition reads the wrong creature. This is NOT a STOP — the NO-DEFERRALS tranche rule requires Malamet to ship. Apply the **narrow slot-parameterization** in §3.1.1: add an optional slot field to `TargetMatchesFilter` resolved through the same `flatten_targets_in_chain(root)` helper the counter target uses. This is model-independent (root flatten is correct under A and B). Only if that narrow field addition is itself infeasible do you STOP-AND-RETURN and **escalate to the coordinator** (§7) — never silently leave Malamet RED. Do not paper over by matching Oracle text.

---

## 3. Fix design

### 3.1 s07 — counter target references the first declared slot (primary)
**Parser (chokepoint):** in `lower_effect_chain_ir` (`parser/oracle_effect/lower.rs:1133`) — per the memory note, post-lowering AST patches belong here, NOT in the `parse_effect_chain` wrappers (triggers/activated abilities call `lower_effect_chain_ir` directly and bypass the wrappers). Add a targeted post-lowering rewrite that recognizes the **class** shape "chain declares ≥2 `TargetOnly` object slots AND a later node whose effect is **`matches!(Effect::PutCounter { .. })`** and whose target is an unbound anaphor (`SelfRef` from `resolve_it_pronoun`, or a re-resolving `Typed{controller:You}` matching the first slot's filter)" and rewrites that `PutCounter` target to **`TargetFilter::ParentTargetSlot { index: 0 }`**.
- **A1 — hard `PutCounter`-only guard:** the rewrite MUST be gated on `matches!(node.effect, Effect::PutCounter { .. })`. Tail Swipe's "the creature you control gets +1/+1" lowers to `Effect::Pump` (not a counter) and MUST be excluded — do not generalize the rewrite to any you-control anaphor.
- Build for the class, not the card: key the rewrite off chain structure (count of leading `TargetOnly` slots + a downstream `PutCounter` with a you-control anaphor), never off card name or verbatim text. This covers Malamet ("on it"), Longstalk ("the creature you control"), Duel for Dominance ("the chosen creature you control"), and any future two-target fight/counter card with the same shape.
- The rewrite must map "the creature you control" / "it" → slot 0 (the *you-control* declaration, emitted first by `try_parse_two_targets`). Verify slot ordering against step 0's chain dump before hardcoding index 0.
- Do NOT touch the `Effect::Fight` node (its `ParentTarget` subject + both-target read already work — `fight.rs:73-84`).

**Engine (counter resolver):** in `resolve_defined_or_targets` (`game/effects/counters.rs:1367`) add a `ParentTargetSlot { index }` arm **before** the `:1487` fall-through.
- **A3 — flatten-from-root UNCONDITIONALLY.** Resolve against the **flattened chain from the root** and apply the index — reuse the exact building block at `targeting.rs::resolved_targets:693-706` (find the root stack entry via `resolving_stack_entry`/`state.stack`, `flatten_targets_in_chain(root)`, then index). Do this unconditionally — root flatten is correct under BOTH model A and model B, whereas indexing the node-local `ability.targets` is correct only under model A. Do NOT take the local-`ability.targets` shortcut. Extract the shared helper per §3.4.

**Condition subject:** §2 — determined by step 0. Model A → no change. **Model B → apply §3.1.1** (Malamet must ship; no deferral).

### 3.1.1 Model-B condition-subject fix (slot-parameterize `TargetMatchesFilter`)
Only implemented if step 0 confirms model B. Parameterize, don't proliferate: **add an optional slot field to the existing `AbilityCondition::TargetMatchesFilter`** rather than adding a sibling condition variant.
- Field: `subject_slot: Option<usize>` (default `None` = current behavior: test the node-local first object). `Some(0)` = test the you-control creature at chain slot 0.
  - **MANDATORY serde attribute (round-2 BLOCKING):** the field MUST carry `#[serde(default, skip_serializing_if = "Option::is_none")]`, matching the sibling `use_lki` (`types/ability.rs:14843`) and the codebase `Option<T>` idiom (`ability.rs:1385-1386`). `AbilityCondition` derives `Serialize, Deserialize` (`ability.rs:14628`): without `#[serde(default)]`, deserializing any previously-serialized `TargetMatchesFilter` condition (no `subject_slot` key) HARD-ERRORS; without `skip_serializing_if`, every ~100 existing `TargetMatchesFilter` conditions gain `"subject_slot":null` in card-data.json — contradicting the A4 behavior-neutral / no-coverage-regression claim. Both attributes are required for the behavior-neutrality guarantee to hold.
- Emission: the same `lower_effect_chain_ir` rewrite (§3.1) that sets the counter target to `ParentTargetSlot{0}` also sets `subject_slot: Some(0)` on that node's `TargetMatchesFilter` condition, for the same `PutCounter`-guarded class shape. **The `subject_slot` write MUST be guarded on `matches!(node.condition, Some(AbilityCondition::TargetMatchesFilter { .. }))`** — Longstalk's condition is `AdditionalCostPaid` (gift), so it receives the counter-target `ParentTargetSlot{0}` rewrite (condition-type-independent) but NO `subject_slot`. Do not force `subject_slot` onto a non-`TargetMatchesFilter` condition. Malamet's condition is a bare `TargetMatchesFilter{EnteredThisTurn}` → the guard matches directly.
- Resolution: in `TargetMatchesFilter` evaluation (`game/effects/mod.rs:7903-7928`), when `subject_slot` is `Some(n)`, resolve the tested object via the **same `flatten_targets_in_chain(root)` helper** (§3.4) indexed at `n`, instead of `ability.targets.iter().find_map(first object)`. Model-independent (root flatten correct under A and B). `None` keeps the existing `find_map`/`TriggeringSource` fallback untouched — zero behavior change for every existing consumer (measured: all current `TargetMatchesFilter` sites default `None`).
- **`/add-engine-variant` gate reasoning (field addition):** (1) Parameterization filter — this is a leaf-level parameterization of `TargetMatchesFilter`'s existing "which object does the filter test" axis (currently hardwired to node-local slot 0), not a new structural concept → parameterize the existing variant. ✅ (2) Categorical boundary — stays within CR 608.2c anaphora resolution (the same rule the variant already implements); does not cross rule sections. ✅ (3) Existence/sibling-smell — grep `data/engine-inventory.json` (run `cargo engine-inventory` first) to confirm no existing slot-bearing condition already covers this and no sibling-cluster smell is introduced. ✅ Executor runs the `add-engine-variant` skill checklist as the runnable gate before adding the field.
- If even this narrow field addition proves infeasible (e.g. the inventory reveals a conflicting slot semantic) → STOP-AND-RETURN and **escalate to the coordinator** with the specific blocker (§7). Do NOT silently leave Malamet RED.

### 3.2 s25 — `ParentTargetSlot{index}` resolves wherever `ParentTarget` does (three sites)
1. **`delayed_trigger.rs::concrete_parent_target_filter:318`** — add arm:
   `TargetFilter::ParentTargetSlot { index } =>` return `parent_targets.get(index)` mapped to `SpecificObject{id}` / `SpecificPlayer{id}` (mirror `parent_targets_filter:344-358` but single-slot); empty/out-of-range → `TargetFilter::Any` (matches the empty-slice behavior). This is the delayed-condition analogue of the `ParentTarget → parent_targets_filter` arm.
2. **`game/effects/mod.rs::filter_refs_parent_target:4106`** — add `TargetFilter::ParentTargetSlot { .. } => true`, so `effect_refs_parent_target` snapshots parent targets for a slot-referencing delayed effect (the snapshot already preserves the full vector; the index is honored at firing by `effect_object_targets`).
3. **`game/filter.rs::normalize_contextual_filter:459`** — extend the `Not(ParentTarget)` arm to also handle `Not(ParentTargetSlot{index})` (exclude only `parent_targets[index]`), and (if the executor finds a live consumer) positive `ParentTargetSlot`. Keep the documented intentional-scope comment accurate.

### 3.3 What NOT to do
- **No broad target-propagation change** to the replace-descent at `effects/mod.rs:7382-7391` / `resolve_chain_body` — too large a regression surface across every chained effect. The model-B fix (§3.1.1) deliberately routes the condition subject through the root-flatten helper INSTEAD of changing propagation.
- No new `AbilityCondition` **variant** (a sibling). The only permitted type change is the optional `subject_slot` **field** on the existing `TargetMatchesFilter` (§3.1.1), and only under model B, gated by the `add-engine-variant` checklist.

### 3.4 Building-block note — mandatory helper extraction (A3)
Extract the "resolve `ParentTargetSlot` from the flattened chain root" logic (currently inline at `targeting.rs:693-706`: root stack-entry lookup + `flatten_targets_in_chain(root)` + index) into ONE `pub(crate)` helper (e.g. `resolve_parent_slot_from_root(state, ability, index) -> Option<TargetRef>`). Call it from all three consumers: `targeting.rs` (refactor the inline block to call it), the new `counters.rs` `ParentTargetSlot` arm (§3.1 A3), and — under model B — the `TargetMatchesFilter` `subject_slot` resolution (§3.1.1). This is the single authority for slot-from-root resolution; do not duplicate the stack-lookup three times.

---

## 4. CR annotations (grep-verified against `docs/MagicCompRules.txt`)

Executor MUST cite these (all confirmed present):
- **CR 608.2c** (line 2793) — "read the whole text and apply the rules of English…"; canonical target-anaphora / later-text-refers-to-earlier propagation rule. Annotate the counter-slot rewrite, the counter resolver arm, and all three s25 sites.
- **CR 601.2c** (line 2461) — "the same object can be chosen once for each instance of the word 'target'"; justifies two distinct declared slots and index-0 vs index-1. Annotate the `try_parse_two_targets` rewrite.
- **CR 603.7c** (line 2618) — delayed triggered ability refers to a particular object; annotate the delayed-trigger slot binding (site 1) and the snapshot-detection fix (site 2).
- **CR 122.1** (line 1178) — a counter is a marker placed on an object; annotate the `PutCounter` target resolution.
- **CR 701.14a/b** (verify exact line before citing) — Fight; only if the executor touches fight-adjacent code (it should not).

Do NOT invent 701.x/702.x numbers from memory — grep each before writing.

---

## 5. Tests (discriminating + non-vacuous — mandatory)

Use the `card-test` recipe (`GameScenario` + `GameRunner::cast(...).resolve()`, assert on resolved deltas). Byte-identical AST proves nothing for a tree-changed card (memory: sub_ability vs else_ability inherit different target vectors) — every test below must **cast and measure resolved counters on specific objects.**

**T0 — dual fight-damage discriminator (step 0, gates the design).** Cast Malamet end-to-end (condition true); assert BOTH creatures have resolved fight-damage marked (you-control takes opponent's power, opponent takes you-control's power). Records model A vs B (§1.6). This test stays in the suite as a regression guard that the fight itself hits both fighters.
**T1 — Malamet re-enable (which-creature, discriminating).** Cast Malamet with a you-control creature that entered this turn + an opponent creature. Assert: `counters_on(you_control_creature) == 1` AND `counters_on(opponent_creature) == 0` AND both creatures took fight damage. Negative sibling: same board but the you-control creature entered a PRIOR turn → `counters_on == 0` on both (condition false), and the fight still happens (both damaged). Proves slot-0 target AND (critically for model B) the condition subject reads the you-control creature.
**T2 — Malamet counter lands on the correct creature even when the opponent creature would out-rank it.** A discriminating variant asserting that swapping which creature is "most recent" (declared 2nd) does not move the counter (guards against a most-recent-only regression). Under model B this is the test that fails pre-fix and passes post-fix.
**T3 — Longstalk Brawl which-creature (the missing discriminating test the commit lacked).** Gift promised → `counters_on(you_control) == 1` AND `counters_on(opponent) == 0` AND both creatures took fight damage. **Negative sibling proving the OLD model was wrong:** construct the pre-fix AST inline (counter target = `SelfRef`/`Typed{You}`) and show it mislands (source/opponent/both), so the test discriminates the fix from the bug. Gift NOT promised → `counters_on == 0` on both (preserve the committed gate test). Note: Longstalk's condition is `AdditionalCostPaid` (gift), which is target-independent, so Longstalk is fully fixable under BOTH models by the counter-target fix alone.
**T3b — Duel for Dominance (A2, in-scope regression via cast-and-measure).** The rewrite WILL fire on Duel (shares the 2-`TargetOnly` chain; slot 0 = you-control — correct). Cast Duel with the Coven condition satisfied; assert `counters_on(you_control) == 1`, `counters_on(opponent) == 0`, and both creatures took fight damage. This is a cast-and-measure regression, NOT a doesn't-crash check.
**T4 — counter resolver honors `ParentTargetSlot{index}` (building-block level).** Construct a `ResolvedAbility` chain with two object targets and a `PutCounter { target: ParentTargetSlot{0} }`; assert the counter lands on `targets[0]`; a sibling with `index:1` lands on `targets[1]`. Directly exercises the new flatten-from-root `resolve_defined_or_targets` arm.
**T4b — (model B only) `TargetMatchesFilter { subject_slot: Some(0) }` tests slot 0.** Build a two-target chain where slot 0 satisfies the filter and slot 1 does not; assert the condition is TRUE; swap so slot 0 fails → condition FALSE; `subject_slot: None` sibling proves the default still tests the node-local first object. Exercises the §3.1.1 field.
**T5 — s25 site 1 (delayed condition slot bind).** A delayed trigger whose condition filter is `ParentTargetSlot{1}` (or `WhenDies{ParentTargetSlot{1}}`) fires for the second parent target and NOT the first. Mirror the existing `single-target WhenDies must bind ParentTarget to the chosen victim` test (`delayed_trigger.rs:1352-1391`) but with two parents + an index.
**T6 — s25 site 2 (snapshot detection).** A delayed effect targeting `ParentTargetSlot{0}` snapshots the parent target (non-empty `delayed_ability.targets`) and affects the correct object at firing; without the fix the snapshot is empty. Model on the Flickerwisp/Grave-Betrayal snapshot tests already in `delayed_trigger.rs`.
**T7 — s25 site 3 (`Not(ParentTargetSlot)` normalization).** A mass effect excluding `Not(ParentTargetSlot{0})` excludes only that one parent object; the other parent is still affected. Mirror `normalize_contextual_filter_with_multiple_parent_targets_excludes_all_of_them` (`filter.rs:7502`).

Regression guard: run the existing Longstalk gate test (`s07_longstalk_brawl_counter_gated_on_gift_promised`) and **Tail Swipe** (must NOT be touched by the rewrite — it's a `Pump`, excluded by the A1 `PutCounter`-only guard; assert its +1/+1 still lands on the you-control creature). Run the card-data coverage-regression check (memory: parser coverage regressions are CI/coverage-only, invisible to `cargo test -p engine`) and confirm Malamet flips `supported:true gap_count:0` and Longstalk/Duel stay supported.

**A4 — measured no-regression basis (not assumed):** the counter-resolver `ParentTargetSlot` arm is new-code-only because **no existing `PutCounter` emits `ParentTargetSlot`** today (grep `ParentTargetSlot` under the parser — the only emitters are `oracle_target.rs:1014` "the second player" and `:1487` "the artifact", which are player/artifact anaphors that resolve through `targeting.rs`, NOT through the counter-specific `resolve_defined_or_targets`). So adding the arm changes zero existing card behavior. Likewise the `TargetMatchesFilter.subject_slot` field defaults `None` at every existing call site (measured: all current constructions omit it), so the model-B field addition is behavior-neutral for prior cards. The executor must re-run these greps and paste the counts as the measured basis.

Non-vacuity evidence to include in the executor report: for each of T1/T3/T5/T7 show the assertion FAILS on the pre-fix tree (paste the measured wrong-creature delta), then PASSES post-fix.

---

## 6. Files the executor will touch

| File | Change |
|------|--------|
| `crates/engine/src/parser/oracle_effect/lower.rs` | post-lowering rewrite in `lower_effect_chain_ir`: two-`TargetOnly` + you-control-anaphor `PutCounter` → target `ParentTargetSlot{0}` |
| `crates/engine/src/game/effects/counters.rs` | `resolve_defined_or_targets`: add `ParentTargetSlot{index}` arm (flatten-from-root + index) |
| `crates/engine/src/game/effects/delayed_trigger.rs` | `concrete_parent_target_filter`: add `ParentTargetSlot{index}` arm (s25 site 1) |
| `crates/engine/src/game/effects/mod.rs` | `filter_refs_parent_target`: add `ParentTargetSlot{..} => true` (s25 site 2) |
| `crates/engine/src/game/filter.rs` | `normalize_contextual_filter`: handle `Not(ParentTargetSlot{index})` (+positive if a consumer exists) (s25 site 3) |
| `crates/engine/src/game/targeting.rs` | extract shared `resolve_parent_slot_from_root` helper (§3.4, mandatory) and refactor the `:693-706` inline block to call it |
| `crates/engine/src/types/ability.rs` | **model B only:** add `subject_slot: Option<usize>` field (with `#[serde(default, skip_serializing_if = "Option::is_none")]`) to `AbilityCondition::TargetMatchesFilter` (§3.1.1, after `add-engine-variant` gate) |
| **~40 mechanical destructure/construction updates (compile-enforced, two crates)** | Adding the field fans out to **33 exact `{ filter, use_lki }` destructures** (no `..`) plus construction sites across `database/`, `game/`, `parser/`, AND the separate **`crates/mtgish-import/src/convert/condition.rs`** (constructs at `:152`, `:688`; destructures at `:3756`). All are compile-enforced (Rust struct literals require every field) → NO silent-default risk, but the executor must budget for the fan-out and add `subject_slot: None` at every non-emitting site. |
| test file(s) under `crates/engine/src/game/` | T0–T7 + regression guards (colocated `#[cfg(test)]` or the existing `s07_*` integration test files) |
| `data/card-data.json` in worktree? | NO — worktree lacks it; verify support flip via coverage tooling per project-reference |

Possibly-shared collision files (`effects/mod.rs`, `filter.rs`): edit surgically, re-read before editing (multi-agent safety). s25 does NOT touch these three files — this increment owns them.

---

## 7. STOP-AND-RETURN flags (NO silent card deferral — Malamet MUST ship)

1. **Model B is NOT a stop.** If step 0 shows most-recent-only propagation (the likely case per §1.6), do NOT stop — implement §3.1.1 (the `subject_slot` field on `TargetMatchesFilter`, resolved via the root-flatten helper). Malamet ships either way. A STOP here that leaves Malamet RED is unacceptable under the NO-DEFERRALS tranche rule.
2. **Only-if-infeasible escalation.** STOP-AND-RETURN **and escalate to the coordinator** ONLY if the §3.1.1 field addition is itself blocked (e.g. `data/engine-inventory.json` reveals a conflicting slot semantic, or the `add-engine-variant` gate fails). Return the specific blocker — never silently defer Malamet.
3. **Slot ordering.** If step 0's chain dump shows `try_parse_two_targets` emits the *you-don't-control* slot at index 0 (contrary to §1.2), flip the rewrite index; confirm before hardcoding `index: 0`.
4. **New sibling variant.** The plan permits exactly one type change: the optional `subject_slot` field on the existing `TargetMatchesFilter` (parameterize-don't-proliferate). If the executor believes a NEW sibling variant is required, STOP and run the `add-engine-variant` skill gate first.

---

## 8. Verification cadence (executor, direct cargo — Tilt unwatched here)
`cargo fmt --all` → `cargo engine-inventory` (before any variant/field reasoning) → parser combinator gate on touched parser code → `cargo test -p engine` (T0–T7 + regressions) → `cargo clippy -p engine -D warnings` → coverage/support-flip check for Malamet + Longstalk + Duel + no coverage-regression on other cards. Report: **which model step 0 confirmed (dual fight-damage result)**, before/after resolved-counter + fight-damage deltas for T1/T3/T3b, whether §3.1.1 was needed, the A4 grep counts, s25 T5–T7 results, and any escalation.
