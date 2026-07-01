I now have a complete, evidence-based picture. Here is the plan.

---

# PLAN — S01-reflexive-if-rider building block

## 1. Summary & skills followed

**What this is.** The cluster's "Swallow:Condition_If" is not merely a coverage gap — it is a **correctness bug**. For every card I inspected in `client/public/card-data.json`, the rider *effect* parses and attaches as a `sub_ability` (`sub_link: "SequentialSibling"`), but the rider *condition* is dropped to `null`, so the rider fires **unconditionally**. Measured examples:
- Sold Out → `ChangeZone(Exile)` then sub `Token{Clue}` with `"condition":null` (the "If it was dealt damage this turn" gate is gone).
- Consuming Ashes → `ChangeZone(Exile)` then sub `Surveil{2}` with `"condition":null`.
- Brackish Blunder → `Bounce` then sub `Token{Map}` with `"condition":null`.

**Root cause (measured).** The chain-internal swallow site is `parse_clause_ast` (`crates/engine/src/parser/oracle_effect/mod.rs:10144-10163`): it splits the leading `"if …, "`, calls **only** `try_nom_condition_as_ability_condition` (mod.rs:10158), and on `None` returns `ClauseAst::Conditional { condition: None, clause }` — the prefix is stripped and the body parsed, but `lower_clause_ast` (mod.rs:10352-10359) only sets `result.condition` when `Some`, so an unrecognized condition is **silently discarded**. The chunk-loop strip cascade (mod.rs:20265-20426) runs the same recognizer family first and likewise fails on these predicates, then the structural fallback `strip_unrecognized_conditional_head_when_body_optional` (conditions.rs:299) is a no-op (body is not `"you may "`).

So the predicates simply aren't recognized. They mostly map to **already-existing** typed conditions; the gaps are missing verb-form/word-order arms in the recognizers.

**The fix is two-part:**
- **Part A — recognition** (parser, `conditions.rs`): extend the reflexive anaphoric recognizer that `try_nom_condition_as_ability_condition` dispatches, composing existing building blocks, so each rider predicate parses to an existing typed `AbilityCondition` that binds to the prior clause's target.
- **Part B — anaphor binding** (lowering, `lower_effect_chain_ir`): the mandated chokepoint for any AST patch (triggers/activated abilities call `lower_effect_chain_ir` directly, bypassing the `parse_effect_chain` wrappers — confirmed at `oracle_trigger.rs:1104`, `oracle.rs:1871`). The genuine lowering-time concern is the **declined-optional-target anaphor** (Faller's Faithful) and is the home for any future `ObjectScope` rebind.

**No new enum variants are required** — `TargetMatchesFilter`, `PreviousEffectAmount`, `ControllerControlsMatching`, `Or`/`Not` (all in `types/ability.rs:14133+`) and `FilterProp::{Tapped, Attacking, Blocking, WasDealtDamageThisTurn, Cmc, PtComparison}` (`types/ability.rs:2602-2972`) already exist. The `/add-engine-variant` gate is therefore **not triggered** (stated explicitly so reviewers don't expect it).

**Skills read & followed:** `engine-planner` (trace + architectural sections below), `oracle-parser` (Rule Zero nom mandate, the `parse_inner_condition` single-authority rule, the §11 swallow-detector workflow), `add-engine-effect` (no new effect — but its targeting/AI/frontend checklist confirms no downstream wiring is needed since the conditions and effects already exist), `validate-cr-annotations` (every CR below grep-verified), `unlock-set` (cluster-by-primitive framing, commit-between-clusters, coverage-regression gate). `add-engine-variant` consulted and found N/A (no new variants).

---

## 2. Trace of the closest analogous feature

**Primary reference — `patch_self_ref_head_tap_anaphor`** (`crates/engine/src/parser/oracle_effect/lower.rs:181-208`), the CR 608.2c "Incredible Hulk" anaphor work, traced end-to-end:

- **Where it runs:** inside `lower_effect_chain_ir` at `lower.rs:1621`, in the tail block of post-lowering AST patches (alongside `rewire_result_anchored_subchain`, `wire_optional_cast_decline_fallback`, etc., lines 1610-1621). This is *after* the chain is assembled into `result` (the linked `sub_ability` tree, lines 1509-1568), so it can see the head's subject and the chained sub together.
- **What it does:** `walk(def, carried_self_ref)` carries the active permanent antecedent down the `sub_ability` chain. It rebinds a chained `SetTapState { target: ParentTarget, scope: Single }` to `SelfRef` **only** when the head's `target_filter()` is `SelfRef` (lines 185-205). The discriminator (head subject) is *visible only at lowering* — by resolution time both Hulk and a declined-optional anaphor reach the resolver with the same empty target list (doc comment lines 176-180). This is exactly why anaphor binding must be a lowering-time concern.
- **Companion helpers** show the idioms I will reuse: `target_filter_is_player_scoped` (lines 219-238, a deliberately non-exhaustive allow-list — "an omission is safe; a false inclusion is not"), and the recursive `def + sub_ability + else_ability` walkers `rewrite_else_parent_target_to_self_ref` (lines 140-152) and `rewrite_player_anaphor_targets_in_definition` (lines 62-70).
- **Tests** colocated at `lower.rs:241-320+` (`self_ref_tap_anaphor_tests`): the discriminating pair `self_ref_head_tap_anaphor_rewrites_to_self_ref` (SelfRef head → rewrite) vs `typed_head_tap_anaphor_stays_parent_target` (typed head → no rewrite). I will mirror this discriminating-pair structure.

**Secondary reference — the runtime anaphor binding I am relying on (no new runtime code):**
- `evaluate_condition` (`game/effects/mod.rs:7058`) evaluates a `sub_ability`'s condition against the **parent** `ResolvedAbility` at `game/effects/mod.rs:6564` (`let condition_met = evaluate_condition(condition, state, ability);`, where `ability` is the parent). Sub-abilities with empty targets inherit the parent's targets (`game/effects/mod.rs:1035-1036, 5594-5595, 6531-6533, 6616-6618`).
- `TargetMatchesFilter` resolves "it"/"that creature" to `ability.targets[0]`, with LKI for past tense (`game/effects/mod.rs:7412-7480`); `use_lki:true` reads `lki_cache` / the `ZoneChanged` record.
- `PtComparison`/`Cmc`/`Tapped`/`WasDealtDamageThisTurn` are all evaluated on the LKI snapshot path (`game/filter.rs:132, 340, 2863, 2881, 2898, 3243, 3788`; `WasDealtDamageThisTurn` reads the turn-scoped `state.damage_dealt_this_turn` ledger which survives the object's zone change — `filter.rs:3788-3791`).
- `PreviousEffectAmount` reads `state.last_effect_amount` (`game/effects/mod.rs:7336-7345`) — the excess-damage channel.

**Existing recognizer building blocks I will compose (not duplicate):** `parse_target_anaphoric_subject` (conditions.rs:1303-1316: `it`/`that creature`/`that permanent`/`that card`), `parse_target_anaphoric_tense_polarity` (conditions.rs:1318-1356: returns `(negated, use_lki)` from `was/wasn't/is/isn't/'s`), `parse_target_demonstrative_subject` (conditions.rs:1234-1246), `parse_type_phrase` (`oracle_target.rs`), and the existing arms inside `try_nom_condition_as_ability_condition` (conditions.rs:3746) — e.g. `parse_target_type_membership_condition_text` (3772), `parse_target_attacked_this_turn_condition_text` (3763), `parse_previous_effect_excess_damage_condition` (4626).

---

## 3. Per-card classification (measured) & realistic flip count

Rider condition → representation → what's missing. Predicate representations verified against the enums above.

| Card | Rider predicate | Representation (existing types) | Gap |
|---|---|---|---|
| **Consuming Ashes** | "if it **had** mana value 3 or less" | `TargetMatchesFilter{Cmc LE 3, use_lki:true}` | recognizer: `strip_mana_value_conditional` (conditions.rs:1815) has `"if its mana value was"` + `"if it has mana value"` but **not** `"if it had mana value"` |
| **Brackish Blunder** | "if it **was tapped**" | `TargetMatchesFilter{FilterProp::Tapped, use_lki:true}` | recognizer: no anaphoric `was tapped` arm |
| **Sold Out** | "if it **was dealt damage this turn**" | `TargetMatchesFilter{FilterProp::WasDealtDamageThisTurn, use_lki:true}` | recognizer arm; + `Duration_ThisTurn` false-positive must clear |
| **Driftgloom Coyote** | "if that creature **had power 2 or less**" | `TargetMatchesFilter{PtComparison{Power,LE,2}, use_lki:true}` | recognizer: `strip_property_conditional` (conditions.rs:1459-1490) only handles `"if its power is"` and emits `QuantityCheck{Power{CostPaidObject}}` (wrong scope); need anaphoric `had power N` → `PtComparison` |
| **Wisecrack** | "if that creature **is attacking**" | `TargetMatchesFilter{FilterProp::Attacking, use_lki:false}` | recognizer arm (effect "deals 2 to that creature's controller" already supported — `ParentTargetController`, targeting.rs:854) |
| **Orbital Plunge** | "if **excess damage was dealt this way**" | `PreviousEffectAmount{GT, 0}` | recognizer word-order: `parse_previous_effect_excess_damage_condition` (conditions.rs:4626) only matches `"<noun> is dealt excess damage this way"`, not `"excess damage was dealt this way"` |
| **Torch the Witness** | "if **excess damage was dealt to that creature this way**" | `PreviousEffectAmount{GT, 0}` | same word-order gap |
| Reptilian Recruiter | "if that creature's power is 2 or less **or** if you control another Lizard" | `Or[TargetMatchesFilter{PtComparison{Power,LE,2}}, ControllerControlsMatching{Lizard, exclude self}]` | compound recognizer + complex effect (gain-control+untap+haste); **stretch** |
| Throw from the Saddle | "Put a +1/+1 counter on it **instead if it's a Mount**" | `ConditionInstead{TargetMatchesFilter{Subtype:Mount}}` (via `build_instead_def`, conditions.rs:2755) + `DynamicQty` | different sub-pattern (instead); **stretch** |
| Faller's Faithful | "if that creature **wasn't** dealt damage this turn" | `Not{TargetMatchesFilter{WasDealtDamageThisTurn, use_lki:true}}` | recognizer + **Part B optional-target guard** + `ParentTargetController` draws; **needs Part B** |
| Full Bore | "if that creature **was cast for its warp cost**" | no target-scoped cast-variant condition exists (`CastVariantPaid` is source-scoped) | **out** (would need a new condition variant → separate cluster) |
| Dose of Dawnglow | "if it isn't your main phase" + "blight 2" | phase condition (not target-anaphoric) | **out of class** (different anaphor + effect support) |
| Quistis Trepe | "if that spell **would** be put into a graveyard, exile it **instead**" | replacement effect | **out** (replacement class, not rider) |
| Kylox's Voltstrider | same "would … instead" | replacement | **out** |
| Summoner's Grimoire | "if that card is an enchantment card, it enters tapped and attacking" | nested inside a **granted quoted** ability | **out** (granted-static class) |
| Yarus | "if it's a permanent card" on a dies→return | face-down dies trigger | **out** |
| Amalia | "if its power is exactly 20" | **source** anaphor `QuantityCheck{Power{Source},EQ,20}` (not target) | **out of class** (source anaphor; separate fix) |

**Realistic flip estimate for this PR: 5–7 cards** — the recognizer-only TargetMatchesFilter / PreviousEffectAmount cases (Consuming Ashes, Brackish Blunder, Sold Out, Driftgloom Coyote, Wisecrack, Orbital Plunge, Torch the Witness). Reptilian Recruiter, Throw from the Saddle, and Faller's Faithful become reachable once the building block exists but carry extra effect-side work; the remaining 5 are a different class. I will land the clean set first, then attempt the three stretch cards in the same PR only if each passes review without architecture compromises.

---

## 4. Architectural sections (engine-planner mandate)

- **Pattern coverage.** The building block is "reflexive anaphoric rider condition": `<anaphor subject> <tense> <predicate>` where the subject is the object the prior clause acted on and the predicate is one of a *parameterized set* of object characteristics (tapped, dealt-damage-this-turn, mana value, power/toughness comparison, combat status, excess-damage-this-way). It covers the 7 clean cards now and the broad class of "removal/bounce/damage + conditional bonus" spells and ETB triggers across the corpus, not one card. The predicate axis is a single `FilterProp` `alt()` — one new branch per characteristic, composed, never a flat product of full-string tags.

- **Building blocks composed.** `parse_target_anaphoric_subject` + `parse_target_anaphoric_tense_polarity` (subject/tense), `parse_type_phrase` (types), `nom_primitives::parse_number`/`parse_comparison_suffix` (already used by `strip_property_conditional`/`strip_mana_value_conditional`), `FilterProp::{Tapped, Attacking, Blocking, WasDealtDamageThisTurn, Cmc, PtComparison}`, `AbilityCondition::{TargetMatchesFilter, PreviousEffectAmount, Not, Or, ControllerControlsMatching}`. Lowering reuses the `def + sub_ability + else_ability` recursive-walker idiom and the `target_filter()`/optional-target inspection from `patch_self_ref_head_tap_anaphor`.

- **Logic placement.** Recognition → `conditions.rs` (the `try_nom_condition_as_ability_condition` dispatch, the single recognizer both the chunk loop and `parse_clause_ast` route through). Anaphor binding / optional-target guard → `lower_effect_chain_ir` (the chain chokepoint reached by spells, triggers, and activated abilities). No engine/runtime changes (the conditions already evaluate correctly against the parent ability + LKI).

- **Rust idioms.** Typed `AbilityCondition`/`FilterProp` (no bools, no verbatim string matches); `use_lki` derived from the typed tense tuple, never a literal; predicate dispatch is one `alt()` of `value()`/`map()` combinators; lowering walker is a recursive `match` with a non-exhaustive *allow-list* of "establishes-a-target" effects (omission is safe — it only declines to rebind, never mis-binds, mirroring `target_filter_is_player_scoped`).

- **Nom compliance.** Every new recognizer arm is a nom combinator from the first line: `preceded`/`tag`/`alt`/`value`/`map` composed with the existing anaphoric-subject and tense combinators. No `contains`/`starts_with`/`split_once`/`find` for dispatch. The recognizer *is* the detector. `parse_inner_condition` remains the single authority for *game-state* `StaticCondition`s; these target-anaphoric predicates correctly live in `conditions.rs` as `AbilityCondition` producers (same layer as the existing `parse_target_type_membership_condition_text`), because they reference a target object, not a `StaticCondition` game fact — so no `static_condition_to_*` bridge changes are required.

- **Extension vs creation.** Pure extension: new `alt()` arms in an existing recognizer + extending two existing strippers' verb tables (`strip_mana_value_conditional` `"had"`, `parse_previous_effect_excess_damage_condition` word order) + one new lowering pass modeled on `patch_self_ref_head_tap_anaphor`.

- **Verification matrix.** Per claim: (1) recognizer arm → unit test in `conditions.rs` asserting the exact `AbilityCondition`, with a revert-probe showing the arm is load-bearing; (2) anaphor binding → `card-test` runtime test casting the spell and asserting the rider fires *only* when the condition holds (sibling negative case included); (3) coverage honesty → full-corpus `cargo coverage` + swallowed-clause delta. Hostile fixtures: the negated sibling ("wasn't"/"isn't"), the declined-optional-target path (Faller's), and a non-target card that must **not** be mis-folded (Section 5 guard test).

- **Identity / provenance contract.** "it"/"that creature"/"that card" = the **first object target of the resolving ability** (CR 115.1, `ObjectScope::Target` semantics). Binding time: the condition is evaluated at the rider's resolution against the **parent** ability's resolved targets (`game/effects/mod.rs:6564`), which were chosen at cast/trigger time and snapshotted to LKI on the primary effect's zone change. Live vs snapshot: present-tense predicates (`is attacking`) read live state (`use_lki:false`); past-tense (`was tapped`, `had mana value`, `was dealt damage`) read LKI (`use_lki:true`, CR 400.7 / 608.2h). Invalidation: if the optional antecedent target was declined, there is no referent — handled by Part B's guard. Multi-authority hostile fixture: Faller's "up to one" declined target must leave the rider's negated condition **false** (no draw), not true.

---

## 5. File-by-file changes

### A. `crates/engine/src/parser/oracle_effect/conditions.rs` — recognition (core)

**A1.** Add a single parameterized recognizer `parse_target_reflexive_property_condition_text(lower: &str) -> Option<AbilityCondition>` near the existing `parse_target_type_membership_condition_text` (conditions.rs ~1248-1290). It composes:
```
parse_target_anaphoric_subject  →  parse_target_anaphoric_tense_polarity (negated, use_lki)
  →  alt(( reflexive predicate ))  →  all_consuming tail
```
where the predicate `alt()` (one branch per characteristic — the parameterized axis, **not** a flat tag product) yields a `FilterProp`:
- `value(FilterProp::Tapped, tag("tapped"))` — Brackish Blunder
- `value(FilterProp::WasDealtDamageThisTurn, tag("dealt damage this turn"))` — Sold Out / Faller's
- `value(FilterProp::Attacking{..default}, tag("attacking"))`, `value(FilterProp::Blocking, tag("blocking"))` — Wisecrack
- `map((parse "power"/"toughness" → PtStat, parse_comparison_suffix), |…| FilterProp::PtComparison{ stat, scope: PtValueScope::Current, comparator, value })` — Driftgloom (handles `"had power N or less"`; tense from the subject parser already supplies `had`)

The result wraps in `TargetMatchesFilter{ filter: Typed(default().properties(vec![prop])), use_lki }`, then `Not{..}` when `negated`. **Tense → `use_lki`** comes directly from `parse_target_anaphoric_tense_polarity` (no literals).

Register this arm inside `try_nom_condition_as_ability_condition` (conditions.rs:3746), placed **after** the more specific `parse_target_attacked_this_turn_condition_text` (3763) and `parse_target_type_membership_condition_text` (3772) arms (specific-before-general), and **before** the bare-color/`parse_inner_condition` fallbacks.

**A2.** Extend `strip_mana_value_conditional` (conditions.rs:1815) with leading + suffix `"if it had mana value "` arms mirroring the existing `"if its mana value was"` (leading, `use_lki:true`) and `"if it has mana value"` (leading) arms — Consuming Ashes. Single new `alt` arm each; reuse `parse_leading_mana_value_condition_body` / `parse_mana_value_threshold`. (Both this stripper and A1 ultimately set the same `TargetMatchesFilter{Cmc}`; A1 also catches the `parse_clause_ast` fallback path. Add `"had"` to whichever the trace shows fires first for the corpus — both routes converge on the identical AST, and the swallow test in §6 will confirm coverage.)

**A3.** Extend `parse_previous_effect_excess_damage_condition` (conditions.rs:4626) to also match the printed word order `"excess damage was dealt this way"` and `"excess damage was dealt to <anaphor> this way"` (compose `parse_target_anaphoric_subject` for the optional `"to that creature"` slot) → `PreviousEffectAmount{GT, Fixed 0}` — Orbital Plunge, Torch the Witness. Keep the existing `"<noun> is dealt excess damage this way"` arm.

### B. `crates/engine/src/parser/oracle_effect/lower.rs` — anaphor binding (mandated chokepoint)

**B1.** Add `bind_reflexive_rider_anaphor(result: &mut AbilityDefinition)` modeled structurally on `patch_self_ref_head_tap_anaphor` (lower.rs:181), called from the post-lowering patch block inside `lower_effect_chain_ir` next to the existing `patch_self_ref_head_tap_anaphor(&mut result)` call at **lower.rs:1621**. It walks `def → sub_ability → else_ability`, carrying the active object-target antecedent down the chain (the head/prior clause's `target_filter()`), and performs two jobs:

- **Optional-target declined guard (the real correctness fix, Faller's Faithful):** when a sub-ability's `condition` is a reflexive target condition (`TargetMatchesFilter` or `Not{TargetMatchesFilter}` / `PreviousEffectAmount`) **and** the antecedent clause's target is optional (`optional_targeting`/"up to one"), ensure the rider does not fire on a declined target. Implementation: the negated form `Not{TargetMatchesFilter{…}}` currently evaluates true when `ability.targets` is empty (no referent) — wrap so the rider is gated on the antecedent target existing (compose with the existing optional-decline machinery; this mirrors the `patch_self_ref_head_tap_anaphor` doc's "declined optional target leaves the target list empty so the sub no-ops" contract, lines 170-173).
- **`ObjectScope` rebind seam (future-proofing / safety net):** if any rider condition carries a deferred `ObjectScope::Anaphoric`/`Demonstrative` quantity (none of the §5A representations do today — they use `FilterProp` inside `TargetMatchesFilter`, which auto-binds), rebind to `ObjectScope::Target` when the antecedent has a chosen object target, reusing `rebind_anaphoric_object_scope` (`mod.rs:14486`). This is the documented "rebound to `ObjectScope::Target` at the parser/lowering seam" pattern (`game/quantity.rs:3866-3867`).

Placing this in `lower_effect_chain_ir` (not a `parse_effect_chain` wrapper) is mandatory: `oracle_trigger.rs:1104` and `oracle.rs:1871` call `lower_effect_chain_ir` directly, so Driftgloom/Faller's (ETB triggers) get the same treatment as the instant/sorcery spells.

**Important:** for the 7 clean cards (mandatory single targets, affirmative or simple-negated conditions), B1 is a verified **no-op** — the recognition in §5A is sufficient because the runtime already binds `TargetMatchesFilter`/`PreviousEffectAmount` against the parent ability + LKI (Section 2). B1 earns its keep on the optional-antecedent class (Faller's) and as the single home for anaphor AST patches.

### C. CR annotations (all grep-verified against `docs/MagicCompRules.txt`)

I verified each before listing (the planner cannot write code, but these are the annotations the implementer must attach):
- `CR 608.2c` — "later text on a card may refer to an object mentioned earlier in the same effect" (the reflexive anaphor) ✓ present throughout lower.rs/conditions.rs.
- `CR 400.7` / `CR 608.2h` — past-tense / left-the-battlefield → last-known information (`use_lki:true`) ✓.
- `CR 115.1` — "the first object target" (`ObjectScope::Target`) ✓ (cited at `types/ability.rs:3865`).
- `CR 120.6` / `CR 120.9` — "was dealt damage this turn" as historical fact ✓ (cited at `filter.rs:3782-3787`).
- `CR 120.10` — excess damage (`PreviousEffectAmount`) ✓ (cited at `types/ability.rs:14278`).
- `CR 510.1c` / `CR 506.4` — combat status "is attacking" — to grep-verify at implementation (`grep -n "^510.1" docs/MagicCompRules.txt`); I have NOT confirmed the exact subsection number, so the implementer must verify before writing (flagged per the CR-annotation protocol).

---

## 6. Discriminating, non-vacuous tests + revert-probe

**Parser unit tests** (colocated `#[cfg(test)]` in `conditions.rs`), one discriminating pair per recognizer arm, asserting the *exact* `AbilityCondition`:
- `reflexive_was_tapped_emits_target_matches_filter_lki` → `Not`-free `TargetMatchesFilter{Tapped, use_lki:true}`; **sibling** `reflexive_isnt_tapped_emits_negated` → `Not{TargetMatchesFilter{Tapped, use_lki:false}}` (proves both the predicate AND the tense/polarity axis are load-bearing).
- `reflexive_had_mana_value_le3` → `TargetMatchesFilter{Cmc LE 3, use_lki:true}`; sibling `reflexive_had_mana_value_ge4` (proves the comparator axis).
- `reflexive_had_power_le2` → `TargetMatchesFilter{PtComparison{Power,LE,2}, use_lki:true}`; sibling toughness/GE.
- `reflexive_is_attacking` → `use_lki:false`; sibling `is_blocking`.
- `excess_damage_was_dealt_this_way_emits_previous_effect_amount` and the `"to that creature"` variant.

**Revert-probe (discrimination proof):** for each arm, the test file documents that reverting the single `alt` arm makes the corresponding assertion fail with the condition coming back as `None`/dropped (the pre-fix behavior reproduced from `card-data.json`). I will capture the *current* `condition:null` parse for Sold Out/Consuming Ashes/Brackish Blunder as the baseline the test discriminates against.

**Runtime `card-test` tests** (`game/effects` or a card-level test using `GameScenario`/`GameRunner::cast(...).resolve()` per the `card-test` skill), proving the anaphor binds and the gate actually gates:
- Brackish Blunder: bounce a **tapped** creature → Map token created; bounce an **untapped** creature → **no** Map token (the discriminating negative — proves the condition is not vacuously true). This is the exact behavior the current bug gets wrong.
- Consuming Ashes: exile MV-2 creature → surveil happens; exile MV-5 creature → no surveil.
- Driftgloom Coyote: ETB exile a 2-power creature → +1/+1 counter; a 4-power → none (proves LKI power read on the exiled creature).
- **Hostile / multi-authority fixture (Part B):** Faller's Faithful — decline the "up to one" target → the negated "wasn't dealt damage" rider must **not** draw (proves the optional-target guard; without B1 it would draw).

**Guard test against the coverage-regression hazard (non-target card NOT mis-folded):** a parser test asserting a *board-state* leading conditional on an unrelated card — e.g. `"If you control a creature, draw a card"` and `"If an opponent controls more lands than you, …"` — still parses to its existing condition (`ControllerControlsMatching` / unchanged) and is **not** swallowed or re-folded into a `TargetMatchesFilter`. This directly probes the Zemo/Isochron-class regression.

---

## 7. Verification & regression plan (Tilt-first)

1. `cargo fmt --all` (the one always-direct command).
2. Parser combinator gate: `./scripts/check-parser-combinators.sh` (Rule Zero) + `./scripts/check-skill-doc.sh` (no new slots/modules, should pass untouched).
3. Targeted semantic checks via Tilt (do **not** run cargo directly): `tilt logs test-engine --tail 80 --since 3m` after the parser tests land, or `./scripts/tilt-wait.sh --timeout 240 clippy test-engine`.
4. **Full-corpus coverage + swallowed-clause delta (mandatory for this cluster — the CI-only regression class):**
   - Baseline first: capture pre-change `cargo coverage` and the swallow drilldown for `--warning-detector Condition_If` (and `Duration_ThisTurn`) into the scratchpad.
   - After change: regenerate (`./scripts/gen-card-data.sh`) and rerun. Assert: (a) `Condition_If` count **drops** by the flipped set; (b) the per-category swallowed-clause totals for **every other** detector do **not** rise (no clause swallowed on other cards); (c) the 7 target cards move `supported: false → true`; (d) Sold Out's `Duration_ThisTurn` false-positive clears (the "this turn" is now inside the recognized condition).
   - Drill any net-negative category with `coverage-report -- data --warning-detector <X> --warning-limit 20` and bisect with `parse_oracle_text` on suspect cards (per the memory note: parser coverage regressions are CI-only and must be probed via the corpus, not `cargo test -p engine`).
5. Commit between sub-clusters (recognition arms first, lowering guard second) per `unlock-set`, so a regression bisects to a small change.

---

## 8. Risks & open questions

- **Coverage-regression (highest risk).** Broadening "if it/that …" recognition can swallow clauses corpus-wide. Mitigation: every arm is anchored to a *specific* predicate after the anaphoric subject+tense, registered *after* the more-specific existing arms, and gated by the §6 guard test + the §7 full-corpus delta. I will not touch `parse_inner_condition` or the `static_condition_to_*` bridges.
- **Sold Out target-id continuity post-exile.** `WasDealtDamageThisTurn` is keyed by object id in `state.damage_dealt_this_turn` (filter.rs:3788). After exile the object gets a new id (CR 400.7). The ability's `targets` hold the **battlefield** id, and `use_lki:true` routes through `lki_cache.get(&battlefield_id)` (effects/mod.rs:7454) — so the ledger lookup uses the battlefield id and should match. **Open question to confirm in the runtime test:** that `damage_dealt_this_turn` records the battlefield id and the LKI snapshot path queries it by that id. If not, Sold Out/Faller's drop to "recognizer lands, runtime needs a small ledger-by-LKI-id fix" — I'll keep them behind the clean set.
- **`PtComparison` LKI value scope.** `PtValueScope::Current` on an exiled creature: the LKI snapshot must capture the buffed battlefield power (CR 608.2h). `filter.rs:1318` comment indicates the base-scope arm reads LKI base; I must confirm `Current` reads the LKI's modified P/T (Driftgloom test is the probe). If only `Base` is reliable on LKI, "had power" maps to `PtValueScope::Base` — acceptable for these cards (no relevant continuous buffs in the test).
- **Part B necessity for the clean 7.** Honest assessment: B1 is a no-op for the mandatory-single-target cards; the recognition in §5A alone flips them (runtime already binds via parent-ability eval). B1 is *required* only for the optional-antecedent class (Faller's) and is the mandated home per the task. I will land §5A + the clean cards first; B1 + Faller's second. If reviewers prefer, B1 can ship as a guarded no-op-for-now with the Faller's test marked `#[ignore]`-with-reason until the guard is proven non-vacuous.
- **Stretch cards.** Reptilian Recruiter (compound `Or` + gain-control/untap/haste chain), Throw from the Saddle (`ConditionInstead` + DynamicQty), and Full Bore (needs a *new* target-scoped cast-variant condition) are explicitly **not** counted in the 5–7; Full Bore is out of this cluster entirely (new variant → its own `add-engine-variant` gate).
- **CR 510.1c "is attacking"** subsection number is unverified by me — the implementer must `grep -n "^510" docs/MagicCompRules.txt` before annotating (flagged per protocol; do not write an unverified CR number).

**Key files (absolute paths):**
- Recognition: `/home/lgray/vibe-coding/phase-rs-workdir/crates/engine/src/parser/oracle_effect/conditions.rs` (`try_nom_condition_as_ability_condition` :3746, `strip_mana_value_conditional` :1815, `strip_property_conditional` :1455, `parse_previous_effect_excess_damage_condition` :4626, anaphoric helpers :1303-1356)
- Swallow site: `/home/lgray/vibe-coding/phase-rs-workdir/crates/engine/src/parser/oracle_effect/mod.rs` (`parse_clause_ast` :10144, `lower_clause_ast` :10352, chunk-loop condition cascade :20265-20426)
- Lowering chokepoint: `/home/lgray/vibe-coding/phase-rs-workdir/crates/engine/src/parser/oracle_effect/lower.rs` (`lower_effect_chain_ir` :666, patch block :1610-1621, `patch_self_ref_head_tap_anaphor` reference :181)
- Runtime (read-only confirmation, no edits): `/home/lgray/vibe-coding/phase-rs-workdir/crates/engine/src/game/effects/mod.rs` (:6564, :7412-7480, :7336), `/home/lgray/vibe-coding/phase-rs-workdir/crates/engine/src/game/filter.rs` (:3788, :3243), `/home/lgray/vibe-coding/phase-rs-workdir/crates/engine/src/types/ability.rs` (`AbilityCondition` :14133, `FilterProp` :2602, `ObjectScope` :3858)

---
# BINDING AMENDMENTS (round-1 adversarial review — these OVERRIDE the plan above on any conflict)

1. **[HIGH] DROP Orbital Plunge and Torch the Witness from this PR entirely.** Do NOT implement plan section 5.A3 (the `parse_previous_effect_excess_damage_condition` word-order extension). Reason (verified): `previous_effect_amount_from_events` (game/effects/mod.rs:4754) sums TOTAL damage for `Effect::DealDamage` (only `Effect::Fight` sums `excess`), so `PreviousEffectAmount{GT,0}` would fire on ANY damage landing — a misparse, not a correct gate. Excess-damage-on-DealDamage needs a NEW excess channel (separate cluster, routed through the add-engine-variant gate). **Clean flip set is now 5 cards: Consuming Ashes, Brackish Blunder, Sold Out, Driftgloom Coyote, Wisecrack** (+ Faller's Faithful via Part B). Do NOT recognize "excess damage was dealt this way" anywhere in this PR.

2. **[MED] Driftgloom "had power 2 or less" needs a tense arm first.** `parse_target_anaphoric_tense_polarity` (conditions.rs:~1328-1345) has NO ` had ` arm, so the composed predicate path in 5.A1 never reaches "power 2 or less". Add `value((false /*not negated*/, true /*use_lki — CR 400.7 past tense*/), tag(" had "))` to that parser. None of the 5 target cards need a negated ` didn't have `/` hadn't ` form — omit it.

3. **[LOW] CR fix.** Do NOT annotate "is attacking" with CR 510.1c (that rule is blocked-creature damage assignment, docs:2399). Reuse **CR 508.1b** as already annotated on `FilterProp::Attacking` (types/ability.rs:2611), or CR 508.1a/508.4. Grep-verify against docs/MagicCompRules.txt before writing.

4. **Keep (reviewer-confirmed correct):** single registration point — adding the arm to `try_nom_condition_as_ability_condition` (conditions.rs:3746) covers BOTH the `parse_clause_ast` swallow (mod.rs:10158) and the chunk-loop path (`strip_leading_general_conditional`:258); chokepoint placement of Part B in `lower_effect_chain_ir` (lower.rs:1621 patch block); discriminating runtime tests with explicit negative cases; the coverage-regression guard test ("If you control a creature, draw a card" must NOT be mis-folded).

5. **Residual probes — keep each card behind a PASSING runtime card-test, not just a parser AST test:** (a) Sold Out `WasDealtDamageThisTurn` after exile resolves by battlefield ObjectId via the `use_lki:true` lki_cache path (effects/mod.rs:7420-7471) — confirm it matches; (b) Driftgloom `PtComparison` LKI value scope — if `PtValueScope::Current` is not reliably captured on the LKI snapshot for an exiled creature, fall back to `PtValueScope::Base` (acceptable; no buffs in test).

6. **Faller's Faithful / Part B** stays a SECOND commit (optional-target-declined guard). If the exact gating mechanism for `Not{TargetMatchesFilter}` on antecedent-existence isn't cleanly provable, ship the clean 5 first and mark Faller's test `#[ignore]`-with-reason; do not force it.
