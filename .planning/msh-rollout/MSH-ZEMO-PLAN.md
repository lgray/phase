# Baron Helmut Zemo (MSH) — Boast Implementation Plan

Source: zemo-planner (Plan/opus, xhigh). All building-block claims independently verified by lead (file:line greps confirmed). Complexity: **MEDIUM**, proceed-full (no staging). Worktree: `/home/lgray/vibe-coding/wt-msh-zemo` (feat/msh-zemo off upstream/main).

## Card & current parse state
- Line 1 "Whenever you cast a black spell from your hand, ~ connives." → `SpellCast` trigger / `Connive{SelfRef}`. CORRECT — **do not touch.**
- Boast keyword + restrictions parse correctly: `ability_tag:Boast`, `[OnlyOnceEachTurn, RequiresCondition{SourceAttackedThisTurn}]`. CR **702.142a** (verified docs/MagicCompRules.txt:5044). **Do not touch.**
- BROKEN cost+effect (current AST):
  - COST `EffectCost{ChangeZone{Graveyard→Exile, Typed(Card,You,[HasColor:Black,InZone:Graveyard])}}` — **non-functional**: EffectCost payment only handles `PutCounter{SelfRef}`; else returns "Effect-as-cost not yet resolvable" (costs.rs:724-748, VERIFIED). No count, no aggregate.
  - EFFECT `CopySpell{Any,KeepOriginalTargets}` + sub `CastFromZone{Any, without_paying:true}` — anaphors "those exiled cards"/"the copies" unbound (parse_warning TargetFallback); no "up to three" cap.

## Verified existing building blocks (lead-confirmed)
- `ObjectProperty::ManaSymbolCount(ManaColor)` — ability.rs:4654 (hybrid counts per color, CR 107.4e). Resolver-backed quantity.rs.
- `QuantityRef::Aggregate{function,property,filter}` + `TrackedSetAggregate{function,property}` — quantity.rs (Sum over filtered set / tracked set). Test `resolve_aggregate_mana_symbol_count_over_graveyard`.
- `AbilityCost::CollectEvidence` (ability.rs:6529) — standalone aggregate-exile sibling (Sum ManaValue ≥ amount, CR 701.59a). Precedent for a NEW sibling cost (not parameterizing `Exile` @ ability.rs:6508 / ~132 call sites). `OneOf`-vs-`Composite` precedent blesses sibling-at-two.
- `Effect::CastCopyOfCard{target,cost}` (ability.rs:8555) — resolver cast_copy_of_card.rs: with `target:TrackedSet`, opens `WaitingFor::ChooseFromZoneChoice{count, up_to:true}` (:58-62, CR 707.12a per-card choice). Currently count=all. Test `tracked_set_opens_up_to_choice_for_copies_to_cast` (:313).
- `TargetFilter::TrackedSet{id}` + sentinel `TrackedSetId(0)` resolved via `resolve_tracked_set_sentinel`/`latest_tracked_set_id` (targeting.rs). `publish_fresh_tracked_set` (effects/mod.rs:3793). `set_resolved_source_recursive` (copy_spell.rs:698) — precedent for the concrete-id rewrite.
- Parser: `fold_cast_copy_of_card_defs` (oracle_effect/mod.rs:17216) folds CopySpell{ParentTarget}+CastFromZone{ParentTarget,without_paying,Cast} → CastCopyOfCard{TrackedSet{0}}. `rewrite_parent_targets_to_tracked_set`/`contains_explicit_tracked_set_pronoun` (:17085) recognize "those cards"/"the exiled card" but NOT "those exiled cards"/"the copies".

## Gaps & design (per-gap; CR grep-verified before coding)
### Gap A — aggregate-threshold variable-count exile COST publishing a tracked set
- NEW standalone sibling `AbilityCost::ExileWithAggregate { filter, function:AggregateFunction, property:ObjectProperty, comparator:Comparator, value:i32, zone:Zone }`. Zemo = `{Typed(Card,You,[HasColor:Black,InZone:Graveyard]), Sum, ManaSymbolCount(Black), GE, 15, Graveyard}`. CR 117.1/118.3 + 601.2b/602.2b + 107.4a/202.1.
- NEW `PayCostKind::ExileAggregate{zone,function,property,comparator,value,filter}` (game_state.rs) — model on `PayCostKind::TapCreatures{aggregate}` + `ExileFromZone{zone}`.
- Payability (cost_payability.rs): legal iff `Aggregate(function,property, all eligible in zone) cmp value` (Sum/GE = exile-all max). Extract shared `aggregate_property_over(state,&[ObjectId],function,property)` = single summation authority (cost + effect + CollectEvidence).
- Payment (casting path): `WaitingFor::PayCost{kind:ExileAggregate}`; handler validates uniqueness + still-in-zone + threshold (mirror collect_evidence::handle_choice), exiles via zones::move_to_zone, publishes fresh set + binds effect (Gap C), resumes activation.
- AI: new legal-action producer (greedy minimal subset reaching threshold). add-interactive-effect requires AI legal actions.

### Gap B — threshold value: SOLVED by Gap A params + ManaSymbolCount resolver. "fifteen"→15 (oracle_nom/primitives word-number).

### Gap C — "those exiled cards"/"the copies" anaphor + cost→effect tracked-set binding (HARDEST, novel: cost is the publisher)
- Runtime (recommended concrete-id rewrite): after cost exiles chosen cards, `publish_fresh_tracked_set(state, chosen)` → `TrackedSetId C`; recursively rewrite effect chain `TrackedSet{0}`→`{C}` BEFORE pushing ability to stack (new helper mirroring set_resolved_source_recursive). Robust across activation→resolution gap (sentinel/latest path is NOT — intervening instant-speed tracked sets). Non-emptiness invariant: threshold ≥15 ⇒ set non-empty.
  - Fallback (documented, not preferred): `ResolvedAbility.cost_tracked_set_id:Option<TrackedSetId>` consulted first by cast_copy_of_card::resolve. Same correctness, more plumbing. NEVER drop threshold/anaphor.
- Parser: add "those exiled cards"/"the copies" to `contains_explicit_tracked_set_pronoun` (:17085); bind CopySpell/CastFromZone targets to `tracked_set_filter()` (:17197). Nom only.

### Gap D — "copy those cards, cast UP TO THREE"
- Types: add `count:Option<QuantityExpr>` to `Effect::CastCopyOfCard` (`#[serde(default)]` → None=all, preserves 13 existing cards + JSON). CR 707.12a.
- Resolver (cast_copy_of_card.rs:50): `cap = count.map(resolve).unwrap_or(len); choose = cap.min(source_ids.len());` set ChooseFromZoneChoice{count:choose, up_to:true}.
- Parser: extend `fold_cast_copy_of_card_defs` to match TrackedSet/anaphor-bound targets (not only ParentTarget) + capture "up to three" → count. Zemo AST: `CastCopyOfCard{target:TrackedSet{0}, cost:zero, count:Some(Fixed 3)}`.

## Build-for-the-class verdict
All gaps GENERALIZE. ExileWithAggregate covers "exile any number with [agg][cmp][N]" (CollectEvidence is a special case). The 15-black-symbol cost is 1 card today but modeled as a fully parameterized building block — NO verbatim string matching. New sibling (not Exile parameterization) avoids ~132-call-site churn + multi-agent collision; consistent with CollectEvidence + OneOf/Composite precedent.

## Complexity ranking
1. Gap C runtime binding (HARDEST, MEDIUM) — cost-published set surviving activation→resolution + concrete-id rewrite + PayCost activation-resume.
2. Gap A cost variant + interactive payment (MEDIUM) — variant + PayCostKind + payability + handler + AI + ~30-40 exhaustive arms.
3. Gap C/D parser (SMALL-MEDIUM). 4. Gap D effect (SMALL).
Total MEDIUM. No very-large piece. No correctness staging.

## Tests (non-vacuous + discriminating + revert probe) — test the building block
1. `aggregate_property_over(Sum,ManaSymbolCount(Black),set)`: {B}{B}+{B}{U}=3; hybrid {B/R}=black (CR 107.4e); bf/opp-gy excluded. Revert: return 0 → fail.
2. Cost payability: gy 15 black symbols → payable, exactly-15 accepted; 14 → unpayable; 14-subset → rejected; non-black ineligible. Revert: comparator GT or value 14 → flips.
3. Tracked-set binding: after cost exiles S (id C), effect resolves to exactly S even with intervening tracked set between activation/resolution. Revert: use sentinel → wrong set.
4. Copy+cast-up-to-N: exile 5 black (≥15) → ChooseFromZoneChoice.count==3, up_to; cast 3 free OK, 4th not offered; 0 legal. Revert: count None → ==5.
5. End-to-end (card-test skill): Zemo attacked-this-turn, gy ≥15 black symbols, Boast once/turn, copies cast free; 2nd activation same turn illegal.
6. Parser: Zemo → cost ExileWithAggregate{Sum,ManaSymbolCount(Black),GE,15}, effect CastCopyOfCard{TrackedSet{0}, count:Some(Fixed 3)}, ZERO parse_warnings. Revert: old AST fails.

## Critical files
types/ability.rs (ExileWithAggregate, CastCopyOfCard.count); game/effects/cast_copy_of_card.rs (count cap, bound set); game/costs.rs + cost_payability.rs (payability+payment, aggregate helper); parser/oracle_cost.rs + parser/oracle_effect/mod.rs (cost parse, anaphor, fold+count); types/game_state.rs (PayCostKind::ExileAggregate, publish+bind at activation resume).

## Skills/gates
add-engine-effect + add-interactive-effect + oracle-parser checklists. Run add-engine-variant gate for: `ExileWithAggregate`, `PayCostKind::ExileAggregate`, `CastCopyOfCard.count`. Worktree wt-msh-zemo is ISOLATED → cargo DIRECT (Tilt watches only main checkout). Rebase-before-push + full CI-equiv before shipping.
