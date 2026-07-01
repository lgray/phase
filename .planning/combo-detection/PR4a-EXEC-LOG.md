# PR-4a execution log (worktree wt-combo-pr4, base c1b61ded5)

## Status: implementing ability_graph.rs

## Key measured facts (verified in worktree)
- analysis/mod.rs: re-exports at :36-41; `pub mod sim;` at :31. Add `pub mod ability_graph;`.
- resource.rs: ResourceVector (pub fields), ResourceAxis 16 variants (:482), CounterClass 7 variants (Plus1Plus1,Minus1Minus1,Loyalty,Defense,Poison,Energy,Other), ObjectClass 5 (Creature,Planeswalker,Battle,Player,Other), TriggerKind 5 (Proliferate,Magecraft,Constellation,Landfall,Other). `CounterClass::from_counter_type` PRIVATE :97 -> make pub(crate). MANA_INDEX order [W,U,B,R,G,C] private.
- loop_check.rs: WinKind 6 variants. PRIVATE fns: is_progress(:291), unbounded_axes_for(:313 free fn), classify_win_kind(:343). Plan §7.3 primary: extract net_progress_for/unbounded_axes_for to resource.rs as methods (pub(crate)), loop_check delegates; classify_win_kind -> pub(crate). Tests call detect_loop/classify_win_kind/live_mandatory_loop_winner only.
- Effect enum 7767-10491 = 207 variants (confirmed). AbilityCost 6431-6624 = 29 variants (confirmed). TriggerMode 210-561 = 169 variants (confirmed). ManaProduction 1252 = 14 variants.
- AbilityDefinition recursion fields: effect:Box<Effect>, cost:Option<AbilityCost>, sub_ability, else_ability, mode_abilities:Vec.
- Nested-effect variants for walker: Vote{per_choice_effect:Vec<Box<AbilityDefinition>>}, SeparateIntoPiles{chosen_pile_effect:Box}, RevealFromHand{on_decline:Option<Box>}, CreateDelayedTrigger{effect:Box}, RollDie{results:Vec<DieResultBranch{effect:Box}>}, FlipCoin{win_effect,lose_effect:Option<Box>}, FlipCoins{win,lose}, FlipCoinUntilLose{win_effect:Box}, ChooseOneOf{branches:Vec<AbilityDefinition>}.
- CardFace: name, abilities:Vec, triggers:Vec<TriggerDefinition{mode,execute:Option<Box>}>, static_abilities:Vec<StaticDefinition{modifications:Vec<ContinuousModification>}>, replacements:Vec<ReplacementDefinition{execute:Option<Box>}>.
- ContinuousModification::GrantAbility{definition:Box}, GrantTrigger{trigger:Box}.
- ManaProduction variants + fields read (Fixed{colors},Colorless{count},Mixed{colorless_count,colors},AnyOneColor{count,color_options},AnyCombination{count,color_options},ChosenColor{count}(NO color_options),OpponentLandColors{count},AnyTypeProduceableBy{count},ChoiceAmongExiledColors,ChoiceAmongCombinations{options},AnyInCommandersColorIdentity{count},DistinctColorsAmongPermanents{filter},AnyOneColorAmongPermanents{count,filter},TriggerEventManaType).
- ManaColor: White,Blue,Black,Red,Green (no Colorless). ManaType adds Colorless. From<ManaColor> for ManaType exists.
- CounterMatch: Any | OfType(CounterType) (counter.rs:270).

## Design decisions
- Projection EXTENDED beyond plan's illustrative struct: Modeled{vector, magnitudes, produces:BTreeSet<AxisKey>, requires:BTreeSet<AxisKey>} so SetTapState can inject field-less Tap and RemoveCounter(None) inject AnyCounter (plan §3.4 names these injection sites; vector cannot carry them). DEVIATION noted.
- Modeled effect set (representative of 5 families; rest Unmodeled, recall-safe, breadth=4b):
  mana: Mana, GainEnergy. counter: PutCounter, PutCounterAll, MultiplyCounter, RemoveCounter, Proliferate, ProliferateTarget. damage: DealDamage, DamageAll, DamageEachPlayer, EachDealsDamageEqualToPower. tap: SetTapState. cast/copy: CopySpell, CastCopyOfCard, EpicCopy, CastFromZone, FreeCastFromZones, Cascade, Ripple, MiracleCast, MadnessCast, Encore, Myriad.
- Mana cost seeding -> colorless slot (idx5) by max(mana_value,1) magnitude (collapse means color irrelevant for cost). Costs never mark Unbounded.
- Sacrifice cost -> always produces {Sac,Ltb,Death} (recall-first over-approx of Death).
- PerCounter cost -> no-op (FOLLOW PLAN; base recursion would be trivial consistency win, noted not done).
- trigger_axis Some-set (4a={cast,counter,tap,mana}): SpellCast/SpellCopy/SpellCastOrCopy/SpellAbilityCast/SpellAbilityCopy->Casts; CounterAdded/CounterAddedOnce/CounterAddedAll/CounterPlayerAddedAll/CounterTypeAddedAll->AnyCounter; Taps/TapAll->Tap; TapsForMana/ManaAdded->Mana. All other 155 -> None.

## RESULTS (complete)
- Build clean; clippy -D warnings clean (boxed Projection.vector to clear large_enum_variant).
- ability_graph: 19 tests pass. Full analysis module: 115 pass (Engine A intact). Full `cargo test -p engine`: 15836 pass / 0 fail.
- 4 drift gates verified exhaustive no-wildcard: effect_projection(207), trigger_axis(169), From<&ResourceAxis>(16), fold_cost(29). Walker has intentional `_=>{}`.
- Revert probes (all measured fail-when-reverted, pass-when-restored):
  P1 override→ unbounded_production_covers/untap_cost/colored_mana/corpus FAIL.
  P2 veto→ net_negative FAIL (after redesign adding token gained-axis to isolate veto).
  P9 cycle-gate→ disjoint FAIL.
  P3 Untap no-op→ untap_cost+colored_mana+corpus FAIL.
  P4 collapse(colorless distinct)→ colored_mana+corpus+untap_cost FAIL.
  P5 Sacrifice→requires→ sacrifice_cost FAIL.
  P6 Loyalty no-op→ cost_position FAIL.
  P7 AnyCounter→eq→ any_counter FAIL.
  P8 add_damage→CONTROLLER→ deal_damage+opponent_damage FAIL (after rebuilding opponent_damage through DealDamage/add_damage).
  P10 ChooseOneOf descent removed→ collect_effects_descends FAIL.
- CR diff gate: 0 UNVERIFIED. Parser diff gate: no parser files touched (N/A).
- Two test rewrites for genuine discrimination: net_negative (token isolates veto), opponent_damage (routed through add_damage), colored_mana (built via build_node so From-collapse is exercised, not hardcoded).
- Files: NEW analysis/ability_graph.rs (2012 lines incl tests); edited loop_check.rs (-60/+~3 delegation), resource.rs (+55 methods), mod.rs (+8).
