# S07-condition-if-bespoke — Implementation Plan (28 cards, NO deferrals)

Base: `acd2f5e6b`. Worktree: `/home/lgray/vibe-coding/s07-impl-wt`.
Produced via `/engine-planner`. All CR numbers below were grep-verified against
`docs/MagicCompRules.txt` (read from the sibling checkout; the worktree copy is gitignored).

---

## 0. Core finding (why this tranche is small)

The engine already owns almost every condition/quantity/replacement building block these
28 cards need. The coverage gap is mostly **parser wiring** — mapping specific Oracle
phrasings onto *existing* typed variants — plus **two** new enum variants (`PlayerFilter::
DamagedThisWay`, `QuantityRef::DistinctBendTypesThisTurn`), **two** new nom combinators, and
**four** targeted non-variant runtime/parameterization changes exposed by the architectural gate:
B2 (Cloud → shared edit to `apply_trigger_doubling`), B3 (Agency Coroner → suspected-through-LKI
plumbing), A Killer (constrained secret creature-type choice), and B5 (Celestial Reunion → behold-
choose-a-creature-type cost, the single genuinely-hard card). There is **no new subsystem**; every
card lands by extending an existing pattern. Concretely, the following already exist and were traced:

| Building block | Location | Covers |
|---|---|---|
| `AbilityCondition::ZoneChangedThisWay { filter }` (reads `state.last_zone_changed_ids`) | `game/effects/mod.rs:8009`; field `types/game_state.rs:7019` | all "if X was destroyed/exiled/returned/put-into-hand this way" |
| `strip_if_you_do_conditional` + `parse_zone_changed_this_way_clause` | `parser/oracle_effect/conditions.rs:655`; `oracle_nom/condition.rs:6743` | the "this way" parser entry |
| `AbilityCondition::CostPaidObjectMatchesFilter { filter }` (reads `ResolvedAbility::cost_paid_object` LKI) | `game/effects/mod.rs:8017` | "the sacrificed/discarded X was …" |
| `AbilityCondition::AdditionalCostPaid { subject, … }` + `additional_cost_paid_any()` | `types/ability.rs:14620`, `:15055` | Gift/Blight/Behold/additional-cost "if it was paid" |
| `AbilityCondition::ControllerControlledMatchingAsCast { filter }` | `game/effects/mod.rs:7983`; parser `conditions.rs:2749` | "if you controlled a Mount as you cast" |
| `AbilityCondition::QuantityCheck { lhs, comparator, rhs }` | `types/ability.rs:14768` | all count/X comparisons |
| `AbilityCondition::TargetMatchesFilter { filter, use_lki }` | `types/ability.rs:14813` | "if an opponent controls that creature", "target is the chosen type" |
| `QuantityRef::TrackedSetSize` / `FilteredTrackedSetSize` | `types/ability.rs:4507`, `:4522` | "if you exiled four or more / no cards this way" |
| `QuantityRef::EventContextAmount` / `PreviousEffectAmount` | parser `oracle_nom/quantity.rs`; `game_state.last_effect_amount` | "if you draw one or more cards this way" |
| `QuantityRef::DistinctCardTypes { source: Zone{Graveyard, Controller} }` | `types/ability.rs:4482`; delirium parse `oracle.rs:1636` | delirium (4+ card types in graveyard) |
| `QuantityRef::ManaSpentToCast { metric: DistinctColors }` | `types/ability.rs:4864` | Converge X (colors of mana spent) |
| `ReplacementDefinition{ damage_modification: Double, damage_source_filter, damage_target_filter, combat_scope: NoncombatOnly, condition: OnlyIfQuantity }` | `types/ability.rs:16835-16850`; `ReplacementCondition::OnlyIfQuantity` `:15825` | double-damage (Furnace-of-Rath class) |
| `StaticMode::DoubleTriggers { cause }` + `AbilityDefinition.affected` filter + `condition: StaticCondition::SourceIsEquipped` | `types/statics.rs:1054`; snapshot `oracle_static/snapshot_tests.rs:636` | Panharmonicon-class trigger doubling |
| `FilterProp::IsChosenCreatureType` + `GameObject.chosen_creature_type`; `ChosenAttribute::CreatureType` | `types/ability.rs:2849`, `:867` | "is the chosen type" |
| `TargetFilter::LastCreated` + `state.last_created_token_ids` | `types/ability.rs:3799` | "if the token is an Aura" |
| `Effect::Unsuspect { target, scope }`; `FilterProp::Suspected`; `GameObject.is_suspected` | `game/effects/suspect.rs:155`; `types/ability.rs:3159` | suspect / un-suspect |
| Bending: `game/bending.rs`, `Player.bending_types_this_turn: HashSet<BendingType>`, `Effect::RegisterBending`, `TriggerMode::ElementalBend`, transform (`CardLayout::Transform`) | live | Avatar Aang |
| `FilterProp::AttachedToSource`; `QuantityExpr::{Offset, Multiply, ClampMin}` | `types/ability.rs:2985`, `:5262` | Cloud source filter; Slumbering Trudge "3−X" |

**Canonical analogous trace (used throughout Batch 1):** *Renegade Reaper* — "Mill four cards.
If at least one Angel card is milled this way, you gain 4 life." Traced end-to-end:
`Effect::Mill` (emits `ZoneChanged`, populates `last_zone_changed_ids`) →
`AbilityDefinition.sub_ability` with `.condition = Some(AbilityCondition::ZoneChangedThisWay { Angel })` →
`evaluate_condition` (`game/effects/mod.rs:8009`) → `GainLife`. Test:
`game/effects/mill.rs:703-744` + runtime assertion `:746`. *Winter Soldier* (`tests/integration/issue_4560_winter_soldier.rs`)
is the return-to-battlefield twin. These two are the templates for every Batch-1 card.

---

## 1. Batch grouping — all 28 cards accounted for

| # | Batch | Cards (count) | Shared collision file |
|---|-------|---------------|-----------------------|
| 1 | "this way" tracked-set parser coverage | Oviya, Spelunking, Town Greeter, Cache Grab, Nashi, Arid Archway, Break the Spell, Portent of Calamity, Transcendent Archaic (9) | `oracle_nom/condition.rs`, `oracle_effect/conditions.rs` |
| 2 | cost-object / additional-cost / gift conditions | Agency Coroner, Grab the Prize, Cinder Strike, Celestial Reunion, Coiling Rebirth, Longstalk Brawl (6) | `oracle_effect/conditions.rs`; +B3 `game_object.rs`/`types/game_state.rs`/`game/filter.rs` (Agency Coroner suspected-LKI); +B5 `casting_costs.rs`/`oracle_casting.rs`/`types/ability.rs` (Celestial behold-choose-type) |
| 3 | target / identity / misc `AbilityCondition` | Steer Clear, Malamet Battle Glyph, Fear of Immobility, Charging Hooligan, A Killer Among Us, Yenna Redtooth Regent, Eliminate the Impossible (7) | `oracle_effect/conditions.rs`; +A Killer `types/ability.rs` (`ChoiceType`)/`game/effects/choose.rs`/frontend/AI (interactive). Malamet = pure condition wiring, no new variant (B1) |
| 4 | replacement / static-mode | Trance Kuja, The Rollercrusher Ride, Cloud Midgar Mercenary (3) | `oracle_static/*`, `oracle_replacement.rs`, `types/statics.rs`; +Cloud `game/triggers.rs` `apply_trigger_doubling` (B2 shared-runtime self-exclusion gate) |
| 5 | new-runtime / arithmetic | Sonic Shrieker, Slumbering Trudge, Avatar Aang (3) | `types/game_state.rs`, `game/effects/*`, `game/bending.rs` |

Total 9+6+7+3+3 = **28**.

### Card → group → mechanism (exhaustive)

**Batch 1 — "this way" (`ZoneChangedThisWay` / tracked-set count):**
- **Oviya, Automech Artisan** — "If you put an artifact onto the battlefield this way, put two +1/+1 counters on it." → `ZoneChangedThisWay { Artifact }`. Gap: 2nd-person "you put … onto the battlefield" combinator is only reached under the `when` prefix; must also be reached under `if`.
- **Spelunking** — "If you put a Cave onto the battlefield this way, you gain 4 life." → `ZoneChangedThisWay { Subtype("Cave") }`. Same `if`-prefix gap.
- **Town Greeter** — "You may put a land card from among them into your hand. If you put a Town card into your hand this way, you gain 2 life." → `ZoneChangedThisWay { Subtype("Town") }`. Gap: new **put-into-hand-this-way** combinator.
- **Cache Grab** — "You may put a permanent card … into your hand. If you control a Squirrel **or** returned a Squirrel card to your hand this way, create a Food token." → `Or([ControllerControlsMatching{Squirrel}, ZoneChangedThisWay{Subtype("Squirrel")}])`. Gap: new **returned-to-hand-this-way** combinator + disjunction with a control-presence condition.
- **Nashi, Searcher in the Dark** — "You may put any number of legendary and/or enchantment cards … into your hand. If you put no cards into your hand this way, put a +1/+1 counter on Nashi." → `QuantityCheck { TrackedSetSize, EQ, 0 }` (put-into-hand set empty). Uses the same put-into-hand plumbing; condition is a count == 0.
- **Arid Archway** — "return a land you control to its owner's hand. If another Desert was returned this way, surveil 1." → `ZoneChangedThisWay { Subtype("Desert") + "another" (exclude source) }`. `parse_zone_changed_this_way_clause` already handles the passive "returned this way"; verify the `another` (other-than-source) qualifier maps to an exclude-self filter.
- **Break the Spell** — "Destroy target enchantment. If a permanent you controlled or a token was destroyed this way, draw a card." → `ZoneChangedThisWay { Or([ControlledByYou-permanent, Token]) }`. "destroyed this way" verb exists; gap is the disjunctive subject filter ("a permanent you controlled or a token").
- **Portent of Calamity** — "Reveal the top X cards … For each card type, you may exile a card of that type … Put the rest into your graveyard. You may cast a spell with mana value X or less from among the exiled cards without paying its mana cost if you exiled four or more cards this way." → `QuantityCheck { FilteredTrackedSetSize{ caused_by: Exiled }, GE, 4 }`. **B4-verified: all non-condition clauses ALREADY resolve** — `choose_from_zone.rs` has dedicated Portent handling (per-card-type exile loop `:161/:260`, "put the rest" tail `:2077`), `publish_fresh_tracked_set` (`:164`) rebinds the "cards exiled this way" set the condition reads, and the free-cast is `Effect::FreeCastFromZones` (`free_cast_from_zones.rs`). Condition wiring only.
- **Transcendent Archaic** — "you may draw X cards, where X is the number of colors of mana spent … If you draw one or more cards this way, discard two cards." → X = `ManaSpentToCast{ DistinctColors }`; condition = `QuantityCheck { EventContextAmount, GE, 1 }`. (Converge X + draw-count are both existing; verify `Converge —` ability-word prefix strips cleanly.)

**Batch 2 — cost-object / additional-cost / gift:**
- **Agency Coroner** — "Sacrifice another creature: Draw a card. If the sacrificed creature was suspected, draw two cards instead." → `CostPaidObjectMatchesFilter { filter: TargetFilter::Typed(creature + properties[FilterProp::Suspected]) }` (inverted-instead form). **Two-part edit, NOT pure parser wiring:**
  - *Parser seam (corrected — B3):* `parse_cost_paid_object_predicate` (`conditions.rs:4986`) is a two-branch dispatcher returning `Option<CostPaidPredicate>`, **not** an `alt()`. `enum CostPaidPredicate` (`:4953`) has `Color(FilterProp)` and `TypeMatch(TypeFilter)`; the `Color` arm already applies its `FilterProp` via `typed.properties(vec![prop])` (`:4936-4938`). Rename `Color(FilterProp)` → `Property(FilterProp)` (honest name: both color-set and status are `FilterProp` properties applied through `TypedFilter::properties`; this is a parameterization, not a new sibling — avoids a `Color`/`Status`/… cluster), update the single match arm at `:4936`, and add a status branch to the dispatcher: `if let Some(prop) = parse_status_property_predicate(rest) { return Some(CostPaidPredicate::Property(prop)); }` where `parse_status_property_predicate` is `value(FilterProp::Suspected, tag("suspected")).parse(rest)`. Do **NOT** label it `Color(FilterProp::Suspected)`.
  - *Runtime seam (new — B3):* `CostPaidObjectMatchesFilter` is evaluated by `matches_target_filter_on_lki_snapshot` (`game/effects/mod.rs:8019`) against the sacrificed creature's LKI (it's gone from the battlefield, and `is_suspected` is reset to `false` on zone change — `game_object.rs:1510/1609`). But `LKISnapshot` (`snapshot_public_characteristics`, `game_object.rs:1432`) does **not** capture `is_suspected`, and the `ZoneChangeRecord` (`types/game_state.rs:408`) synthesized by `matches_target_filter_on_lki_snapshot` (`game/filter.rs:1309`) has no suspected field either, so `FilterProp::Suspected` would read nothing. Plumb it: add `is_suspected: bool` to `LKISnapshot` (populate in `snapshot_public_characteristics`), to `ZoneChangeRecord` (populate in the synth at `filter.rs:1309` and wherever `ZoneChangeRecord`s are built for LKI), and add a `FilterProp::Suspected => record.is_suspected` arm to `matches_target_filter_on_zone_change_record` (`filter.rs:1156`). Snapshot is taken at cost payment (`cost_paid_object` `CostPaidObjectSnapshot { lki: obj.snapshot_for_mana_spent() }`), so it latches the suspected status BEFORE the sacrifice resets it.
- **Grab the Prize** — "discard a card. Draw two cards. If the discarded card wasn't a land card, … deals 2 damage to each opponent." → `Not(CostPaidObjectMatchesFilter { Land })`. **Already parses** per trace; this card needs a *runtime* discriminating test only (verify, don't re-implement).
- **Cinder Strike** — "Cinder Strike deals 2 damage to target creature. It deals 4 damage to that creature instead if this spell's additional cost was paid." → `AdditionalCostPaid` via inverted `" instead if "` (`conditions.rs:3017` `split_inverted_instead_clause` → `build_instead_def`). Verify `build_instead_def` routes "this spell's additional cost was paid".
- **Celestial Reunion** — "As an additional cost … you may choose a creature type and behold two creatures of that type. Search … reveal it, put it into your hand, then shuffle. If this spell's additional cost was paid **and** the revealed card is the chosen type, put that card onto the battlefield **instead** of putting it into your hand." → forward-instead with `And([AdditionalCostPaid, TargetMatchesFilter{IsChosenCreatureType}])`; the "instead" swaps the destination of the put (hand → battlefield). **GENUINELY HARD — the chosen-type write path does NOT exist (B5-verified). In scope, no deferral (see §5).**
  - *Verified gap:* `AbilityCost::Behold { count, filter, action }` (`types/ability.rs:6974`) has **no** creature-type parameter, and `handle_behold_for_cost` (`casting_costs.rs:1885`) sets `additional_cost_paid = true` + a cost-paid-object snapshot but **never writes any `ChosenAttribute::CreatureType`**. Worse, this behold shape ("choose a creature type and behold two creatures **of that type**") is exactly what `is_choose_behold_prefix` (`oracle_casting.rs:141`) deliberately keeps RED (declines to model) so the card doesn't go falsely green — so Celestial Reunion does not parse its cost today. `FilterProp::IsChosenCreatureType` reads `source.chosen_creature_type` (`filter.rs:3713`), sourced from the spell object's `chosen_attributes` (`game_object.rs:1737`), so the chosen type MUST be written onto the spell object at cost time.
  - *In-scope sub-plan (§5 detail):* model the "choose-a-creature-type-then-behold-N-of-that-type" cost so it (1) prompts a `ChoiceType::CreatureType` choice, pushing `ChosenAttribute::CreatureType` onto the spell object via the existing `choose.rs` persist path (`choose.rs:241-245`), and (2) beholds `count` creatures whose eligibility is `FilterProp::IsChosenCreatureType`. The library search + reveal + shuffle already resolve (`search_library.rs`); the forward-instead destination swap uses existing put-to-hand/put-to-battlefield effects. The chosen type persists on the spell object through resolution, so the "revealed card is the chosen type" check reads it.
- **Coiling Rebirth** — "Return target creature card … to the battlefield. Then if the gift was promised **and** that creature isn't legendary, create a token that's a copy …" → `And([AdditionalCostPaid, Not(TargetMatchesFilter{Legendary})])` on the token sub_ability. Gift = additional cost (CR 702.174b); "if the gift was promised" already parses.
- **Longstalk Brawl** — "Put a +1/+1 counter on the creature you control if the gift was promised. Then those creatures fight each other." → suffix `AdditionalCostPaid` gating the counter sub_ability.

**Batch 3 — target / identity / misc `AbilityCondition`:**
- **Steer Clear** — "…deals 4 damage to that creature instead if you controlled a Mount as you cast this spell." → `ControllerControlledMatchingAsCast { Mount }` via inverted-instead. Verify `build_instead_def` routes the as-cast condition.
- **Malamet Battle Glyph** — "Choose target creature you control and target creature you don't control. If the creature you control entered this turn, put a +1/+1 counter on it. Then those creatures fight." → pure condition wiring, **no new engine surface**. `FilterProp::EnteredThisTurn` ALREADY EXISTS (`types/ability.rs:3242`, evaluated `game/filter.rs:3847` via `obj.entered_battlefield_turn == Some(state.turn_number)`, parsed `oracle_target.rs:5491`). `AbilityCondition::TargetMatchesFilter { filter, use_lki }` ALREADY EXISTS (`types/ability.rs:14813`). Attach `TargetMatchesFilter { filter: Typed(Creature + controller You + FilterProp::EnteredThisTurn), use_lki: false }` onto the +1/+1-counter sub_ability. **Disambiguation mechanism (verified):** `evaluate_condition`'s `TargetMatchesFilter` arm (`game/effects/mod.rs:7863`) tests the ability's **FIRST object target** (`ability.targets.iter().find_map(TargetRef::Object)`), not all targets. Malamet declares "target creature you control" FIRST and "target creature you don't control" second, so `targets[0]` is the you-control creature — the arm checks exactly it. The `controller You` term in the filter is a defensive assertion (it also makes the wiring robust if declaration order ever changes); `use_lki: false` is correct because the target is still live on the battlefield at resolution (`EnteredThisTurn` reads `entered_battlefield_turn` on the live object, `filter.rs:3847`). Parser gap: recognize "if the creature you control entered this turn" → this condition via `parse_inner_condition`; no new variant. Implementation must confirm target-declaration order maps text order (grep the two-target lowering); if not, the filter's `controller You` guard still selects correctly only when target[0] happens to be you-control — so the lowering MUST preserve order (verify with a test that seeds the opponent creature as entered-this-turn and the you-control creature as NOT, asserting no counter).
- **Fear of Immobility** — "tap up to one target creature. If an opponent controls that creature, put a stun counter on it." → `TargetMatchesFilter { controller: Opponent }` gating the stun-counter sub_ability (CR 122.1d).
- **Charging Hooligan** — "Whenever this creature attacks, it gets +1/+0 … for each attacking creature. If a Rat is attacking, this creature gains trample …" → sub_ability with `QuantityCheck { ObjectCount{ Subtype("Rat") + FilterProp::Attacking }, GE, 1 }`.
- **A Killer Among Us** — "When this enchantment enters, create a 1/1 white Human, a 1/1 blue Merfolk, and a 1/1 red Goblin token, then secretly choose Human, Merfolk, or Goblin. … If target attacking creature token is the chosen type, put three +1/+1 counters …" → attack-trigger sub_ability gated on `TargetMatchesFilter { IsChosenCreatureType }` (existing). Token creation resolves (existing). **The ETB "secretly choose … from a fixed 3-list" needs new interactive work (B4/non-blocking — resolves the §3-vs-§5 contradiction: it DOES need an interactive addition, §5 governs).** `ChoiceType::CreatureType` (`types/ability.rs:300`) has no candidate restriction (unlike sibling `Color { excluded }`, `CardType { excluded }`, `Opponent { restriction }`). Add a candidate restriction to `ChoiceType::CreatureType` (mirroring that same axis — parameterization, not a new sibling) so the offered set is `{Human, Merfolk, Goblin}`; the existing `choose.rs` path writes `ChosenAttribute::CreatureType` onto the source enchantment (`choose.rs:241-245`). "Secretly" = the choice emits no public reveal event (hidden information); the chosen type is read only when the counter clause resolves. Follow `/add-interactive-effect` (WaitingFor round-trip + AI legal actions + frontend overlay) — see §5.
- **Yenna, Redtooth Regent** — "Create a token that's a copy of it … If the token is an Aura, untap Yenna, then scry 2." → sub_ability gated on `TargetMatchesFilter { LastCreated ∧ Aura }` (`And([LastCreated, Typed(Aura)])`).
- **Eliminate the Impossible** — "Creatures your opponents control get -2/-0 … If any of them are suspected, they're no longer suspected." → this is not a gate; it is an unconditional `Effect::Unsuspect { target: opponents' creatures scope }` (CR 701.60a: "a spell or ability causes it to no longer be suspected"). Applying Unsuspect to non-suspected creatures is a no-op, so no condition is needed.

**Batch 4 — replacement / static-mode:**
- **Trance Kuja, Fate Defied** — "If a Wizard you control would deal damage to a permanent or player, it deals double that damage instead." → `ReplacementDefinition { damage_modification: Double, damage_source_filter: Typed(Subtype("Wizard") + controller You), damage_target_filter: PlayerOrPermanent, combat_scope: None }` (CR 614.1a).
- **The Rollercrusher Ride** — "Delirium — If a source you control would deal noncombat damage to a permanent or player while there are four or more card types among cards in your graveyard, it deals double that damage instead." → same shape with `combat_scope: NoncombatOnly`, `damage_source_filter: controller You`, `condition: OnlyIfQuantity { lhs: DistinctCardTypes{Zone Graveyard Controller}, GE, rhs: 4 }` (delirium, CR 207.2c + 205.2a).
- **Cloud, Midgar Mercenary** — 2nd ability: "As long as Cloud is equipped, if a triggered ability of Cloud or an Equipment attached to it triggers, that ability triggers an additional time." → `StaticMode::DoubleTriggers { cause: Any }`, `affected: Or([SelfRef, Typed(Subtype("Equipment") + FilterProp::AttachedToSource)])`, `condition: StaticCondition::SourceIsEquipped`. (The ETB search half already parses.) **SHARED-RUNTIME edit (B2), not just parser wiring:** `apply_trigger_doubling` (`game/triggers.rs:4610`) applies a blanket self-exclusion `if trigger.source_id == *doubler_id { continue; }` at `:4641` — which runs BEFORE the `affected`-filter check at `:4655`. So a doubler can never double its OWN triggers, and Cloud's `SelfRef` branch can never fire. Fix in §4/§5.

**Batch 5 — new-runtime / arithmetic:**
- **Sonic Shrieker** — "it deals 2 damage to any target and you gain 2 life. If a player is dealt damage this way, they discard a card." → `DiscardCard { player: PlayerFilter::DamagedThisWay }` as a sub_ability. **New `PlayerFilter::DamagedThisWay` + resolution-context damaged-players tracking** (mirrors `PlayerFilter::ZoneChangedThisWay`; see §4).
- **Slumbering Trudge** — "enters with a number of stun counters … equal to three minus X. If X is 2 or less, it enters tapped." → enters-with quantity `ClampMin(Offset{ base: Multiply(Variable("X"), -1), offset: 3 }, 0)` stun counters; enter-tapped conditional gated on `QuantityCheck { Variable("X"), LE, 2 }` (or the enters-with replacement's `OnlyIfQuantity`). Pure `QuantityExpr` composition; no new variant. (CR 122.1d stun; CR 614.12 enters-with.)
- **Avatar Aang** — "Whenever you waterbend, earthbend, firebend, or airbend, draw a card. Then if you've done all four this turn, transform Avatar Aang." → draw trigger (`TriggerMode::ElementalBend`) with a `transform` sub_ability gated on "all four bend types this turn". Condition = `QuantityCheck { <distinct-bends-this-turn>, GE, 4 }`. Verify `PlayerActionsThisTurn` can count distinct bends; if not, add `QuantityRef::DistinctBendTypesThisTurn { player }` reading `bending_types_this_turn.len()` (see §4). Transform effect + DFC already live (CR 701.27).

---

## 2. Dispatch order & collision-file rationale

**One batch in flight at a time.** Order chosen to keep all edits to the single hottest file
(`parser/oracle_effect/conditions.rs`) consecutive, so its structure is loaded once and churn is
serialized:

1. **Batch 1** — new nom combinators land in `oracle_nom/condition.rs` and are wired into
   `strip_if_you_do_conditional` (`conditions.rs`). Do this first: it establishes the
   put-into-hand / returned-to-hand plumbing that Nash/Cache Grab/Town Greeter share, and it is the
   largest group (best ROI). Also touches `types/ability.rs` only if the `another`-exclusion needs a
   filter tweak (it doesn't — reuse `FilterProp` exclude-self).
2. **Batch 2** — parser: renames `CostPaidPredicate::Color`→`Property` + adds the suspected branch in
   `parse_cost_paid_object_predicate`, verifies instead-if routing (all in `conditions.rs`). Runtime:
   B3 `is_suspected` plumbing (`game_object.rs`, `types/game_state.rs`, `game/filter.rs` record
   matcher) for Agency Coroner; B5 behold-choose-a-type cost (`casting_costs.rs`, `oracle_casting.rs`,
   `types/ability.rs`) for Celestial Reunion — the one genuinely-hard card, sequenced last within the
   batch.
3. **Batch 3** — target/identity conditions in `conditions.rs`. Malamet is **pure condition wiring**
   (reuses existing `FilterProp::EnteredThisTurn` + `TargetMatchesFilter`; NO `types/ability.rs`
   change — B1). A Killer Among Us adds the `ChoiceType::CreatureType` candidate restriction
   (`types/ability.rs`) + interactive plumbing (`choose.rs`, AI legal actions, frontend overlay).
4. **Batch 4** — statics/replacement: `oracle_static/*`, `oracle_replacement.rs`, `types/statics.rs`,
   plus Cloud's **shared-runtime** edit to `game/triggers.rs` `apply_trigger_doubling` (B2). Disjoint
   from `conditions.rs`; runs after the condition batches so the shared file is quiescent. The
   `triggers.rs` self-exclusion gate is isolated from `types/ability.rs`, so it does not collide with
   Batch 3's `ChoiceType` edit.
5. **Batch 5** — runtime: `types/game_state.rs` (+ `PlayerFilter`), `game/effects/deal_damage.rs`,
   `game/bending.rs`/`game/effects` for the bend count, and the enters-with quantity in
   `oracle_replacement.rs`. Most isolated; last.

`types/ability.rs` is touched by Batch 2 (B5 behold cost model, B3 `CostPaidPredicate` rename),
Batch 3 (A Killer `ChoiceType::CreatureType` restriction — Malamet no longer touches it, B1), and
Batch 5 (`PlayerFilter::DamagedThisWay`, `QuantityRef::DistinctBendTypesThisTurn`). Because batches
run sequentially these are non-overlapping edits; each is a single variant/field addition with its
match arms.

---

## 3. Mandatory architectural sections

### Pattern Coverage
Every group targets a *class*, not a card:
- **`ZoneChangedThisWay` combinators (Batch 1)** unlock every "if you put/returned/exiled/destroyed a
  <type> <zone> this way" card — dozens across Bloomburrow/OTJ/MKM (mill-to-hand, bounce riders,
  exile-cast payoffs). The put-into-hand and returned-to-hand combinators generalize by
  `parse_type_phrase` subtype, not by literal card name.
- **cost-object predicate (Batch 2)** extends to every "if the sacrificed/discarded/exiled <noun>
  was <property>" card once the `FilterProp::Property(_)` axis carries status props like `Suspected`
  (plus the B3 LKI plumbing so the read works on the gone object).
- **existing `FilterProp::EnteredThisTurn` (Batch 3, reused not new — B1)** already covers all
  "if that/the target creature entered this turn" text (Malamet + fight-support + flicker payoffs);
  Malamet only needs the parser to route the phrase onto the existing `TargetMatchesFilter`.
- **double-damage `ReplacementDefinition` (Batch 4)** is the Furnace-of-Rath class parameterized by
  source filter / combat scope / gate — Trance Kuja and Rollercrusher are two points in that space.
- **`PlayerFilter::DamagedThisWay` (Batch 5)** covers every "if a player is dealt damage this way,
  they …" rider (mirrors the existing `ZoneChangedThisWay` player filter).

No group resolves to a single card. The only per-card items (Cloud's affected filter, Aang's bend
count) are still parameterized building blocks (`AttachedToSource`, a `…ThisTurn` count).

### Building Blocks
Reused, by name: `parse_zone_changed_this_way_clause`, `parse_you_put_onto_battlefield_this_way_clause`,
`parse_type_phrase`, `strip_if_you_do_conditional`, `split_inverted_instead_clause` /
`split_forward_instead_clause` / `build_instead_def`, `parse_cost_paid_object_predicate`,
`additional_cost_paid_any()`, `parse_controller_controlled_as_cast_condition`,
`parse_inner_condition` (the condition authority in `oracle_nom/condition.rs`),
`QuantityRef::{TrackedSetSize, FilteredTrackedSetSize, EventContextAmount, DistinctCardTypes,
ManaSpentToCast, ObjectCount}`, `ReplacementDefinition` builder (`.combat_scope()`,
`.damage_source_filter`), `StaticMode::DoubleTriggers`, `AbilityDefinition.affected`,
`FilterProp::{Suspected, IsChosenCreatureType, AttachedToSource, Attacking}`,
`TargetFilter::LastCreated`, `Effect::Unsuspect`, `Effect::RegisterBending`,
`QuantityExpr::{Offset, Multiply, ClampMin}`, `evaluate_condition`, `matches_target_filter`.
New helpers justified below (§4) — each is the minimal missing leaf, not a re-implementation.

### Logic Placement
- **Parser** (`oracle_nom/condition.rs`, `oracle_effect/conditions.rs`, `oracle_static/*`): phrase →
  typed variant only. No game logic.
- **Types** (`types/ability.rs`, `types/statics.rs`, `types/game_state.rs`): new enum leaves and the
  one new runtime field.
- **Resolver / effects** (`game/effects/mod.rs` `evaluate_condition`, `deal_damage.rs`,
  `game/filter.rs`, `game/quantity.rs`, `game/replacement.rs`, `game/bending.rs`): evaluation of the
  new leaves. `evaluate_condition` already handles `ZoneChangedThisWay`, `CostPaidObjectMatchesFilter`,
  `QuantityCheck`, `TargetMatchesFilter` — only the *new* `PlayerFilter`/`QuantityRef` arms are added.
- **Shared runtime (gate-exposed):** `game/triggers.rs` `apply_trigger_doubling` (B2 self-exclusion
  gate), `game/game_object.rs` + `types/game_state.rs` (B3 `is_suspected` on `LKISnapshot`/
  `ZoneChangeRecord`) + `game/filter.rs` record matcher arm, `game/casting_costs.rs` (B5 behold-
  choose-type cost). These are localized edits to existing authorities, not new subsystems.
- **Multiplayer filter** (`game/coverage.rs` handled/unhandled tables, APNAP-neutral): each new
  variant gets a coverage row so the audit stays honest.
- **Frontend / AI**: no new WaitingFor round-trips (A Killer Among Us's secret choice reuses the
  existing choose-creature-type flow if present; otherwise §5). New conditions are pure gates → AI
  reads them through existing `evaluate_condition`; no new legal-action surface.

### Rust Idioms
- New leaves are typed enum variants, never bools: `PlayerFilter::DamagedThisWay` (mirrors sibling
  `PlayerFilter::ZoneChangedThisWay`), `QuantityRef::DistinctBendTypesThisTurn` (a `…ThisTurn` count).
  B3's `is_suspected` is a struct field mirroring the existing `GameObject.is_suspected`, latched into
  the LKI snapshot — not a bool flag standing in for a typed concept.
- Slumbering Trudge's "3−X" is `QuantityExpr` composition (`Offset`/`Multiply`/`ClampMin`), not a
  bespoke `three_minus_x` field.
- Combinators use `alt()`/`tag()`/`preceded()` and delegate subtype to `parse_type_phrase`; no
  `contains`/`split_once` for dispatch.
- Exhaustive `match` arms added for every new variant in `evaluate_condition`,
  `matches_player_filter`, `game/coverage.rs`, and Display/hash impls — compiler enforces completeness.

### Nom Compliance (parser files change → mandatory)
- **put-into-hand-this-way** (`oracle_nom/condition.rs`, new `parse_you_put_into_hand_this_way_clause`):
  `preceded(alt((tag("you put "), tag("you've put "))), …)` then
  `alt((parse_quantifier, parse_type_phrase))` then `tag(" into your hand this way")`. Mirrors
  `parse_you_put_onto_battlefield_this_way_clause:6823`.
- **returned-to-hand-this-way** (new `parse_returned_to_hand_this_way_clause`): reuse
  `parse_zone_changed_this_way_clause` verb table if "returned … to your hand this way" already
  matches (it matches "returned this way"); otherwise a thin `alt()` arm adding the
  `" to your hand"` infix. Verify first with `parse_zone_changed_this_way_clause("a squirrel card
  was returned to your hand this way")`; add the arm only if it fails.
- **`if`-prefix for 2nd-person combinators**: in `strip_if_you_do_conditional`, hoist the
  `parse_you_put_onto_battlefield_this_way_clause` / put-into-hand / returned-to-hand attempts out of
  the `prefix == "when "` guard so they also run under `if ` (Oviya/Spelunking/Town Greeter say
  "If you put …"). This is a control-flow move of existing combinator calls, not new string logic.
- **cost-object suspected**: `parse_cost_paid_object_predicate` (`conditions.rs:4986`) is a two-branch
  dispatcher (NOT an `alt()`). Rename `CostPaidPredicate::Color`→`Property`, then add a dispatcher
  branch `parse_status_property_predicate = value(FilterProp::Suspected, tag("suspected"))` — no
  `contains`. (B3: also plumb `is_suspected` through the LKI snapshot so the runtime read works.)
- **delirium / double-damage / DoubleTriggers**: already have nom parsers; wiring only.
- Detector-is-the-parser: e.g. Charging Hooligan's "if a Rat is attacking" dispatches via
  `parse_inner_condition` → `IsPresent`/`ObjectCount` combinator, not a `contains("is attacking")`.

### Extension vs Creation
All extensions. New enum *leaves* (**2**) parameterize existing axes: `PlayerFilter::DamagedThisWay`
(the damage-channel sibling of `PlayerFilter::ZoneChangedThisWay`) and
`QuantityRef::DistinctBendTypesThisTurn` (a `…ThisTurn` count sibling of `CardsDrawnThisTurn`/
`SacrificedThisTurn`; unconditional — `PlayerActionKind` has no bend variants). The former
`FilterProp::EnteredThisTurn` proposal is **withdrawn** — it already exists and is reused verbatim
(B1). Four non-variant changes: B3 struct fields (`is_suspected` on `LKISnapshot`/`ZoneChangeRecord`)
+ `CostPaidPredicate::Color`→`Property` rename; B2 self-exclusion gate in `apply_trigger_doubling`;
A Killer candidate-restriction on `ChoiceType::CreatureType`; B5 behold-choose-a-type cost model.
No new pattern, no new subsystem. Two new WaitingFor round-trips reuse the existing `Choose`
continuation (A Killer secret choice; Celestial Reunion behold type choice).

### Analogous Trace
Primary: **Renegade Reaper** — `game/effects/mill.rs:703-746` → `evaluate_condition`
(`game/effects/mod.rs:8009`) → `AbilityCondition::ZoneChangedThisWay` reading
`state.last_zone_changed_ids` (`types/game_state.rs:7019`), populated from `ZoneChanged` events at
`game/effects/mod.rs:6512`. Secondary traces: **Winter Soldier**
(`tests/integration/issue_4560_winter_soldier.rs`) for return-to-battlefield; **Furnace of Rath**
(`game/replacement.rs`, `ReplacementDefinition.damage_modification`) for Batch 4 damage doubling;
**Panharmonicon / Splinter / Harmonic Prodigy** (`oracle_static/snapshot_tests.rs:636-685`) for
Cloud's `DoubleTriggers { cause: Any } + affected` filter.

### Variant Discoverability
`cargo engine-inventory` was run (`data/engine-inventory.json`, 40k lines) and grepped: confirmed
`AdditionalCostPaid`, `ZoneChangedThisWay`, `CostPaidObjectMatchesFilter`,
`ControllerControlledMatchingAsCast`, `FilteredTrackedSetSize`, `PerformedActionThisWay`,
`ThisWayCause`, `FilterProp::EnteredThisTurn` (`ability.rs:3242`), `TargetMatchesFilter`
(`ability.rs:14813`) all pre-exist (no duplication — B1 confirmed `EnteredThisTurn` is NOT new). The
**two** proposed leaves were checked for sibling-cluster smell: `PlayerFilter::DamagedThisWay` is an
explicit sibling of the existing `PlayerFilter::ZoneChangedThisWay` (same "…ThisWay" axis, different
CR-120 channel — no categorical-boundary violation; damage is CR 120, they live on the same
`PlayerFilter` enum by design precedent); `QuantityRef::DistinctBendTypesThisTurn` sits beside
`CardsDrawnThisTurn`/`SacrificedThisTurn` (a `…ThisTurn` count axis). The `ChoiceType::CreatureType`
candidate-restriction is a leaf parameterization of the same axis used by `Color { excluded }` /
`CardType { excluded }` / `Opponent { restriction }`. Each proposal must pass the `/add-engine-
variant` checklist at implementation time (mandatory gate).

### Verification Matrix (per claim: seam → entry → test → revert-assertion → hostiles)
For **every** card the discriminating test is a **cast → resolve → measure delta** runtime test
(GameScenario + GameRunner::cast().resolve(), per `card-test` skill), never a parse-shape assertion.
Representative rows (full list in §6):
- *Town Greeter* — seam: put-into-hand combinator + `ZoneChangedThisWay`. Entry: cast ETB. Test:
  stack a Town land in top-4, cast → assert +2 life **and** land in hand; revert (rename combinator
  arm) → life delta 0. Hostiles: (a) non-Town land put into hand → no life; (b) Town card *milled to
  graveyard* but not put to hand → no life (proves it reads the put set, not the mill set);
  (c) decline the optional put → no life. First production branch: `last_zone_changed_ids.iter().any`.
- *Agency Coroner* — seam: `FilterProp::Suspected` in cost-object predicate. Test: sacrifice a
  suspected creature → draw 2; revert → draw 1. Hostiles: sacrifice a *non*-suspected creature → draw
  1; sacrifice a suspected *noncreature* is impossible (701.60b permanents/creatures) — prove
  unreachable from the sac filter ("another creature").
- *Trance Kuja* — seam: `ReplacementDefinition` damage doubling. Test: a controlled Wizard deals 3 →
  target takes 6; revert (drop `damage_source_filter`) still 6 so instead assert a **non-Wizard**
  controlled source deals 3 → target takes 3 (source filter discriminates). Hostiles: opponent's
  Wizard deals 3 → 3 (controller scope); combat vs noncombat both double (no combat_scope).
- *Rollercrusher* — hostile that isolates the delirium gate: with 3 card types in graveyard, noncombat
  3 damage → 3 (gate false); with 4 types → 6. Also combat damage with 4 types → unchanged (NoncombatOnly).
- *Cloud* — seam: `DoubleTriggers` + `affected(Or[SelfRef, AttachedToSource])` + `SourceIsEquipped` +
  **B2 self-exclusion gate**. Test: equip Cloud, trigger **Cloud's own** ability → fires twice (this
  is the case the old self-exclusion broke — revert the gate → fires once, fails); trigger an attached
  Equipment's ability → twice; **unequip** Cloud → once (condition gate). Hostiles: non-attached
  Equipment's trigger → once (affected excludes it); unrelated creature you control → once.
  **Mandatory negative regression:** a Panharmonicon-shaped doubler (`affected = Typed(permanent you
  control)`, no `SelfRef`) with a self ETB trigger → fires **once** (self NOT doubled — proves the
  gate keys on structural `SelfRef`, not incidental filter match).
- *Sonic Shrieker* — seam: `PlayerFilter::DamagedThisWay`. Test: 2 damage to a *player* → that player
  discards; 2 damage to a *creature* → no discard (only players in the damaged-player set). Revert
  (don't populate damaged set) → no discard even when a player is hit.
- *Malamet* — `FilterProp::EnteredThisTurn`: target a creature you control that entered this turn →
  +1/+1 before fight; target one that entered a prior turn → no counter.
- *Avatar Aang* — do 3 distinct bends → no transform; do the 4th → transform. Revert (count ≥3
  instead of ≥4) → transforms after 3 (fails the discriminating assertion).

**Coverage-honesty (B4):** A card flips green ONLY when EVERY clause parses AND resolves with no
residual `Effect::unimplemented`. The per-card residual-clause audit is in **§8** (added). Three cards
carry non-condition work verified in scope: **Celestial Reunion** (behold-choose-a-type cost has no
write path — B5, §5), **A Killer Among Us** (constrained secret creature-type choice — §5), **Cloud**
(self-exclusion runtime fix — B2, §5). **Portent of Calamity** was flagged but is B4-verified
already-modeled (dedicated `choose_from_zone.rs` Portent paths + `FreeCastFromZones`) — condition
wiring only. Grab the Prize is expected already-green at parse; its row is a *runtime* verification
(guard against a silent misparse), not a coverage flip. No Oracle text is accepted-but-deferred.

### Identity / Provenance Contract
- **"this way" sets** (`ZoneChangedThisWay`, `TrackedSetSize`, `FilteredTrackedSetSize`): authority =
  `state.last_zone_changed_ids` (ids), bound at each `ZoneChanged`-emitting effect within the current
  resolution, **snapshotted by id** (zone-independent — members read in place even after moving),
  cleared at depth-0 chain start (`resolve_ability_chain`). Consuming fn: `evaluate_condition` /
  `resolve_quantity`. Hostile: two zone-change effects in one chain (mill then put-to-hand) — the
  condition must read the *last* set (put-to-hand), proven by Town Greeter hostile (b).
- **cost-object** (`CostPaidObjectMatchesFilter`): authority = `ResolvedAbility::cost_paid_object`
  (`CostPaidObjectSnapshot { object_id, lki }`), bound at cost payment (`snapshot_for_mana_spent`),
  **LKI-latched** (read via `matches_target_filter_on_lki_snapshot`) so a sacrificed creature's
  suspected status is read from its last-known state, not the empty battlefield. **B3 gap:** the LKI
  snapshot currently does NOT carry `is_suspected` (`snapshot_public_characteristics` omits it, and
  the synthesized `ZoneChangeRecord` has no such field), and `is_suspected` is reset on the sacrifice
  zone-change — so `FilterProp::Suspected` must be plumbed onto both structs + the record matcher (§4
  item 3) or the read returns false. Snapshot is taken BEFORE the sacrifice resets the flag, so once
  plumbed the value is correct. Hostile: sacrifice a *non*-suspected creature → filter false → draw 1.
- **additional cost / gift** (`AdditionalCostPaid`): authority = `SpellContext.additional_cost_paid`
  (bool set at cast), live for the spell's resolution. Multi-authority hostile: promise-gift vs
  decline-gift → Coiling Rebirth token only on promise.
- **as-cast control** (`ControllerControlledMatchingAsCast`): authority =
  `SpellContext.controller_controlled_as_cast: Vec<TargetFilter>`, **snapshotted at cast**
  (`stamp_controller_controlled_as_cast`), not re-evaluated at resolution. Hostile: control a Mount
  at cast, lose it before resolution → Steer Clear still deals 4.
- **chosen type** (`IsChosenCreatureType`): authority = `GameObject.chosen_creature_type` (read from
  `chosen_attributes` `ChosenAttribute::CreatureType`, `game_object.rs:1737`), evaluated with the
  ability's **source** object providing the chosen type (`filter.rs:3713`). **B5 gap:** behold does
  NOT write this today (`handle_behold_for_cost` writes no `ChosenAttribute`; `AbilityCost::Behold`
  has no type param) — the behold-choose-a-type cost model must push `ChosenAttribute::CreatureType`
  onto the spell object at cost time (§5). A Killer Among Us writes it onto the enchantment via the
  ETB secret choice (§5). Both bind at choice time and stay live through resolution. Hostile: revealed
  card not of chosen type → Celestial Reunion puts to hand, not battlefield.
- **created token** (`LastCreated`): authority = `state.last_created_token_ids`, bound at the
  `Effect::Token` immediately prior, consumed by the next sub_ability. Hostile: token is not an Aura
  → Yenna's untap/scry does not fire.
- **damaged players** (new `DamagedThisWay`): authority = a resolution-context damaged-player id set
  populated from `DamageDealt` events (same lifecycle as `last_zone_changed_ids`), consumed by
  `matches_player_filter`. Hostile: damage to a creature only → set empty → no discard.

---

## 4. New typed pieces (each gated by `/add-engine-variant`)

**B1 removed the former `FilterProp::EnteredThisTurn` proposal — that variant ALREADY EXISTS
(`types/ability.rs:3242`) and is reused as-is by Malamet.** Net new enum variants: **2**
(`PlayerFilter::DamagedThisWay`, `QuantityRef::DistinctBendTypesThisTurn`). Plus three
non-variant runtime/parameterization changes (B3 struct fields, B2 self-exclusion gate, A Killer
`ChoiceType::CreatureType` restriction, B5 behold cost model) enumerated below.

### New enum variants (2)

1. **`PlayerFilter::DamagedThisWay`** + resolution-context damaged-player set (`types/game_state.rs`,
   `types/ability.rs` `PlayerFilter`). — Sonic Shrieker.
   - CR 120.3 (damage dealt) + CR 608.2c ("this way" = this resolution). New field
     `last_damaged_player_ids: Vec<PlayerId>` (twin of `last_zone_changed_ids`), populated from
     `GameEvent::DamageDealt` where the target is a player, in the same post-effect scan at
     `game/effects/mod.rs:6512`, cleared at the same points. Evaluated in `matches_player_filter`
     (`game/effects/deal_damage.rs` already has `PlayerFilter::ZoneChangedThisWay` arms at :1437/:1661
     to mirror).
   - Justification: explicit sibling of `PlayerFilter::ZoneChangedThisWay` — same "…ThisWay"
     resolution-context axis on the same enum, different CR-120 channel. Not a new subsystem.
   - Parser: extend the reflexive-connector table so "if a player is dealt damage this way, they …"
     → sub_ability with `DiscardCard { player: DamagedThisWay }`.

2. **`QuantityRef::DistinctBendTypesThisTurn { player }`** — *unconditional* (Avatar Aang).
   Non-blocking-item resolved: `PlayerActionKind` (`types/events.rs:96`) has **no** bend variants
   (only `SearchedLibrary`/`Scry`/`Surveil`/`CollectEvidence`/`ShuffledLibrary`/`Proliferate`/
   `Investigate`), so `QuantityRef::PlayerActionsThisTurn` **cannot** count bends — the leaf is
   required. `BendingType` (`types/events.rs:88`) has exactly 4 variants (`Fire`/`Air`/`Earth`/
   `Water`), so `QuantityCheck { DistinctBendTypesThisTurn, GE, 4 }` == "all four bent this turn".
   - Reads `player.bending_types_this_turn.len()` — field at **`types/player.rs:167`** (corrected
     from the earlier `types/game_state.rs` mislocation), `HashSet<BendingType>`, store already live.
   - Sibling of `CardsDrawnThisTurn`/`SacrificedThisTurn` (a `…ThisTurn` count) — clean
     parameterization, no cluster smell. CR: bending is a set-specific keyword-action subsystem
     (no single CR number; annotate against the FF-set bending rules the existing `game/bending.rs`
     cites — do NOT invent a CR).

### Non-variant runtime / parameterization changes (4)

3. **B3 — suspected-through-LKI plumbing** (Agency Coroner). NOT a new enum variant; struct fields +
   one match arm. Add `is_suspected: bool` to `LKISnapshot` (`game_object.rs`, populate in
   `snapshot_public_characteristics:1432`) and to `ZoneChangeRecord` (`types/game_state.rs:408`,
   populate in the LKI synth at `filter.rs:1309` and other `ZoneChangeRecord` constructors), and a
   `FilterProp::Suspected => record.is_suspected` arm in `matches_target_filter_on_zone_change_record`
   (`filter.rs:1156`). Rename `CostPaidPredicate::Color(FilterProp)` → `Property(FilterProp)` and add
   the `suspected` parser branch (§1 Agency Coroner). CR 701.60 (suspect) + CR 608.2c (LKI).

4. **B2 — trigger-doubling self-exclusion gate** (Cloud). NOT a new variant; a guarded change to the
   existing self-exclusion at `game/triggers.rs:4641`. Replace the blanket
   `if trigger.source_id == *doubler_id { continue; }` with: skip self ONLY when the doubler's
   `affected` filter does **not** structurally reference itself. Add a small recursive helper
   `affected_references_self(&TargetFilter) -> bool` that returns true iff the filter tree contains a
   self-referential leaf (`TargetFilter::SelfRef`, and `SourceOrPaired`); then
   `if trigger.source_id == *doubler_id && !affected.as_ref().is_some_and(affected_references_self) { continue; }`.
   - Rationale + scope matrix: a doubler whose text **explicitly names itself** (Cloud: `affected =
     Or([SelfRef, AttachedToSource])`) doubles its own triggers; a doubler that matches self only
     **incidentally through a category filter** (Panharmonicon/Isshin: `affected = Typed(…you control)`,
     no `SelfRef`) still does NOT — preserving current, tested behavior. Enumerate every reachable
     `StaticMode::DoubleTriggers` user at implementation via `active_static_definitions`; the
     discriminator is "does `affected` contain a `SelfRef` leaf", which is exactly the Cloud-vs-rest
     boundary. CR: reuse the annotations already on `apply_trigger_doubling` (CR 702.26b + CR 604.1 at
     `triggers.rs:4611`; CR 603.2d at `:4644`) — all in-code (verified); add a one-line note on the
     new self-inclusion branch. Do NOT invent a new sub-number without grepping the fetched CR text.

5. **A Killer Among Us — `ChoiceType::CreatureType` candidate restriction** (non-blocking; see §1 and
   §5). Add a restriction field to `ChoiceType::CreatureType` (mirroring `Color { excluded }`,
   `CardType { excluded }`, `Opponent { restriction }`) so the offered set is a fixed candidate list;
   the existing `choose.rs` persist path already writes `ChosenAttribute::CreatureType`. `/add-engine-
   variant` gate applies (leaf parameterization of an existing variant's axis). "Secret" = no public
   reveal event.

6. **B5 — behold-choose-a-creature-type cost model** (Celestial Reunion; see §5). Model the "choose a
   creature type and behold N creatures of that type" cost so it prompts a `ChoiceType::CreatureType`
   choice writing `ChosenAttribute::CreatureType` onto the spell object and beholds creatures matching
   `FilterProp::IsChosenCreatureType`. May extend `AbilityCost::Behold` with an inline creature-type
   choice or compose `Choose`-as-cost (`AbilityCost::Effect`) + `Behold` under `Composite`; the exact
   seam is decided at implementation (both are existing building blocks). This lifts the deliberate
   `is_choose_behold_prefix` red-guard (`oracle_casting.rs:141`) for this shape.

No new `AbilityCondition`, `Effect`, `ReplacementCondition`, `StaticMode`, or `TargetFilter` variants
are required — all consumed variants pre-exist.

---

## 5. Genuinely-hard cards — concrete sub-plans (NOT deferrals)

- **Celestial Reunion — behold-choose-a-creature-type cost (B5, genuinely hard).** Verified: no
  write path for `chosen_creature_type` exists from behold (`handle_behold_for_cost`
  `casting_costs.rs:1885` never writes a `ChosenAttribute`), `AbilityCost::Behold` has no type
  parameter, and `is_choose_behold_prefix` (`oracle_casting.rs:141`) deliberately keeps this shape
  RED. Sub-plan: (a) parse "you may choose a creature type and behold two creatures of that type" as
  a cost carrying a creature-type choice; (b) at cost payment, prompt `ChoiceType::CreatureType`,
  push `ChosenAttribute::CreatureType` onto the spell object via the existing `choose.rs` persist
  path (`:241-245`), and behold `count` creatures whose eligibility filter is
  `FilterProp::IsChosenCreatureType`; (c) resolution: search/reveal/put-to-hand already work; the
  forward-instead `And([AdditionalCostPaid, TargetMatchesFilter{IsChosenCreatureType}])` swaps the
  destination to battlefield. Seam decision (extend `AbilityCost::Behold` with an inline type choice
  vs. `Composite[Effect(Choose CreatureType), Behold{IsChosenCreatureType}]`) is made at
  implementation — both use existing building blocks. Discriminating test: pay behold + reveal a
  card of the chosen type → enters battlefield; pay but reveal a different type → hand; decline behold
  → hand. This is the one card whose cost-layer needs genuinely new plumbing; it is in scope.
- **Cloud, Midgar Mercenary (B2 — shared-runtime edit, not pure wiring).** Build the 2nd ability as
  `StaticMode::DoubleTriggers { cause: Any }` with `AbilityDefinition.affected =
  Or([TargetFilter::SelfRef, Typed(Subtype("Equipment") + FilterProp::AttachedToSource)])` and
  `condition: Some(StaticCondition::SourceIsEquipped)`. Parser work in `oracle_static`: recognize
  "if a triggered ability of ~ or an Equipment attached to it triggers, that ability triggers an
  additional time" (self-ref `~` + attached-equipment disjunct) and the leading "As long as ~ is
  equipped, " gate (→ static `condition`). **Runtime fix required:** `apply_trigger_doubling`
  (`triggers.rs:4610`) blanket-excludes the doubler's own triggers at `:4641` BEFORE the `affected`
  check at `:4655`, so Cloud's `SelfRef` branch can never fire. Change the self-exclusion to skip self
  ONLY when `affected` does not structurally reference self (helper `affected_references_self`, §4
  item 4). Scope matrix — enumerate all `StaticMode::DoubleTriggers` users:
  - **Cloud** (`affected` contains `SelfRef`) → **doubles its own** trigger. ✓ new behavior.
  - **Panharmonicon / Yarok / Isshin / Harmonic Prodigy / Splinter** (`affected = Typed(category
    you control)`, no `SelfRef`) → still do **NOT** double their own ETB/attack triggers. ✓ preserved.
  Negative regression tests (both mandatory): (1) Panharmonicon does NOT double its own hypothetical
  ETB — seed a Panharmonicon-shaped doubler with a self ETB trigger, assert single fire; (2) Cloud
  DOES double its own trigger when equipped — equip Cloud, fire a Cloud trigger, assert double fire;
  unequip → single (condition gate). CR: reuse the in-code annotations on `apply_trigger_doubling`
  (CR 702.26b + CR 604.1 + CR 603.2d); grep the fetched CR text before adding any new sub-number.
- **Slumbering Trudge** — computed enters-with quantity + X gate. Sub-plan: (a) parse "enters with a
  number of stun counters on it equal to three minus X" via the enters-with replacement, quantity =
  `ClampMin(Offset{ Multiply(Variable("X"), -1), 3 }, 0)`; (b) parse "If X is 2 or less, it enters
  tapped" as an ETB-tapped replacement gated on `OnlyIfQuantity{ Variable("X"), LE, 2 }`. Verify the
  enters-with parser accepts a `QuantityExpr` (not just a literal) for the counter count; if it is
  literal-only today, widen it to `parse_number_or_x`-backed `QuantityExpr` — a parser widening, in
  scope. Discriminating test: cast with X=1 → 2 stun counters + tapped; X=3 → 0 counters + untapped.
- **Avatar Aang** — bending + transform both live; the only gap is the all-four gate (§4 item 2).
  Sub-plan: draw trigger already routes via `ElementalBend`; add the transform sub_ability with
  `QuantityCheck { DistinctBendTypesThisTurn, GE, 4 }`. Confirmed: `PlayerActionKind` has NO bend
  variants, so the new `QuantityRef::DistinctBendTypesThisTurn` leaf reading
  `player.bending_types_this_turn.len()` (`types/player.rs:167`) is required (unconditional). Test in §3.
- **A Killer Among Us — secret constrained creature-type choice (verified: DOES need interactive
  work).** The ETB "secretly choose Human, Merfolk, or Goblin" must write `ChosenAttribute::
  CreatureType` onto the source enchantment. Verified: `ChoiceType::CreatureType`
  (`types/ability.rs:300`) offers all creature types with **no** candidate restriction. Add a
  candidate-restriction field to `ChoiceType::CreatureType` (mirroring the `excluded`/`restriction`
  axis on sibling `Color`/`CardType`/`Opponent`) fixing the offered set to `{Human, Merfolk, Goblin}`;
  the existing `choose.rs` interactive path (`GameAction::ChooseOption` → `choose.rs:241-245`) then
  writes `ChosenAttribute::CreatureType`. "Secretly" = suppress any public reveal event so opponents
  don't see the pick; the type is disclosed only when the attack-trigger counter clause resolves.
  Follow `/add-interactive-effect`: WaitingFor round-trip (already exists for `Choose`), AI legal
  actions (the AI picks among the 3), frontend overlay (reuse the creature-type choice overlay with a
  restricted list). The `IsChosenCreatureType` consumer already exists. Discriminating test: choose
  Goblin, target the Goblin token → three +1/+1 counters; target the Merfolk token → no counters
  (revert: unconstrain the choice or drop the write → target-Goblin case gets 0 counters).

No card is escalated. Every card has a concrete, in-scope implementation path.

---

## 6. Per-card discriminating runtime tests (cast → measure)

All tests use `GameScenario`/`GameRunner::cast().resolve()` and assert on `CastOutcome` deltas
(never AST shape). Each names the revert-failing assertion.

**Batch 1**
- Oviya: activate ability, put an **artifact** creature from hand → it gets two +1/+1 counters;
  put a **non-artifact** creature → no counters. (revert: drop `if`-prefix hoist → artifact case gets 0.)
- Spelunking: ETB, put a **Cave** land from hand → +4 life; put a non-Cave land → no life.
- Town Greeter: §3 row.
- Cache Grab: control a Squirrel → Food created even if no Squirrel returned; control none but return
  a Squirrel to hand → Food; control none & return none → no Food. (disjunction discriminates both arms.)
- Nashi: combat damage → mill N; put ≥1 legendary/enchantment to hand → **no** +1/+1 counter; put
  none → +1/+1 counter. (revert: `EQ 0` → `GE 0` makes counter always fire, fails the "put ≥1" case.)
- Arid Archway: ETB return a **Desert** (another) → surveil 1; return a non-Desert land → no surveil;
  return the source itself is excluded by "another".
- Break the Spell: destroy an enchantment **you controlled** → draw; destroy an opponent's
  (non-token) enchantment → no draw; destroy a **token** enchantment (any controller) → draw.
- Portent: exile 4 cards (one per type) → free cast enabled; exile 3 → not enabled.
- Transcendent Archaic: cast with 2 colors of mana → draw 2 then discard 2; cast with 0 colors
  (all generic via rocks) → draw 0, **no** discard. (revert: `GE 1` → `GE 0` forces discard at 0.)

**Batch 2**
- Agency Coroner / Grab the Prize: §3 rows.
- Cinder Strike: pay the blight additional cost → 4 damage; decline it → 2 damage.
- Celestial Reunion: pay behold + revealed card is chosen type → enters battlefield; decline behold →
  goes to hand even if revealed matches type; pay but revealed ≠ chosen type → hand.
- Coiling Rebirth: promise gift + returned creature nonlegendary → token copy created; promise gift +
  returned creature **legendary** → no token; decline gift → no token.
- Longstalk Brawl: promise gift → +1/+1 counter before fight; decline → no counter.

**Batch 3**
- Steer Clear: control a Mount at cast → 4 damage; control none → 2 damage; control a Mount at cast
  then lose it pre-resolution → still 4 (as-cast snapshot).
- Malamet: §3 row.
- Fear of Immobility: tap an **opponent's** creature → stun counter; tap **your own** creature → no
  stun counter.
- Charging Hooligan: attack with a Rat present → trample granted; attack with no Rat → no trample
  (but still +1/+0 per attacker).
- A Killer Among Us: §5 row.
- Yenna: copy an **Aura** enchantment → untap + scry 2; copy a non-Aura enchantment → no untap/scry.
- Eliminate the Impossible: opponents' suspected creatures → become non-suspected + get −2/−0;
  non-suspected opponents' creatures unaffected by the unsuspect (still −2/−0). (revert: no-op the
  Unsuspect → suspected creatures stay suspected.)

**Batch 4**
- Trance Kuja / Rollercrusher / Cloud: §3 rows.

**Batch 5**
- Sonic Shrieker / Slumbering Trudge / Avatar Aang: §3/§5 rows.

Each test is non-vacuous (the negative sibling produces a *different* measured delta) and
discriminating (the revert flips exactly one assertion).

---

## 7. Per-card checklist coverage (skill-gated)

Batches 1–3: `/oracle-parser` + `/add-engine-variant` (for the 2 leaves + the `ChoiceType`
restriction) + `/card-test`. Batch 2 also: `/add-interactive-effect` (Celestial Reunion behold-
choose-a-type cost + write path) and the B3 suspected-LKI plumbing. Batch 3 also:
`/add-interactive-effect` (A Killer Among Us secret constrained choice — WaitingFor + AI + frontend).
Batch 4: `/add-static-ability` (Cloud, incl. the B2 `apply_trigger_doubling` self-exclusion gate) +
`/add-replacement-effect` (Trance Kuja, Rollercrusher). Batch 5: `/add-trigger` (Avatar Aang) +
`/card-test`. Every rules-touching edit carries a grep-verified `CR` annotation from
the verified set in §0/§1. `cargo fmt --all` after each batch; Tilt `clippy`/`test-engine`/`card-data`
consulted (not run directly) per the risk-scaled cadence; full card-data coverage-regression check
before marking any card fixed (parser changes can swallow clauses on other cards — the standing CI gate).

---

## 8. Per-card residual-clause audit (B4 — coverage-flip honesty)

A card flips green ONLY when EVERY clause parses AND resolves without a residual
`Effect::unimplemented`. This table separates the S07 condition-clause (the tranche's target) from
each card's OTHER clauses and states, per card, whether those already resolve (handler cited) or need
in-scope additional work (sub-plan cited). Verified by tracing effect handlers in
`crates/engine/src/game/effects/` and the parser (the worktree has no built `card-data.json`; Tilt is
up but a per-text parse dump would contend on cargo locks — dispatch was traced instead per the
gate's allowance). **3 cards carry non-condition work; all in scope, no deferrals.**

| Card | Non-condition clauses | Status |
|---|---|---|
| Oviya | static "attacking creatures have trample" (grant static); `{G},{T}` put creature/Vehicle from hand → battlefield (`ChangeZone`); two +1/+1 counters (`AddCounters`) | ✅ resolves |
| Spelunking | draw a card (`Draw`); put land from hand → battlefield (`ChangeZone`) | ✅ resolves |
| Town Greeter | mill four (`Mill`); put land from among → hand (`ChooseFromZone`→hand) | ✅ resolves |
| Cache Grab | mill four; put permanent from milled → hand; create Food (`CreateToken`) | ✅ resolves |
| Nashi | combat-damage trigger mill-that-many (`Mill`); put any number → hand; +1/+1 counter | ✅ resolves |
| Arid Archway | enters tapped (replacement); return a land → hand (`ReturnToHand`); surveil 1 (`Surveil`) | ✅ resolves |
| Break the Spell | destroy target enchantment (`Destroy`); draw (`Draw`) | ✅ resolves |
| Portent of Calamity | reveal top X; per-card-type exile; put rest → graveyard; free-cast MV≤X from exiled | ✅ resolves — B4-verified dedicated Portent paths (`choose_from_zone.rs:161/260/2077`, `publish_fresh_tracked_set:164`) + `FreeCastFromZones` |
| Transcendent Archaic | Vigilance (kw); Converge X = `ManaSpentToCast{DistinctColors}`; draw X (`Draw`); discard two (`Discard`) | ✅ resolves |
| Agency Coroner | `{2}{B}`, Sac another creature (sac cost); draw a card / draw two (`Draw`) | ✅ resolves; **condition needs B3 suspected-LKI plumbing** (§4 item 3) |
| Grab the Prize | additional-cost discard (cost); draw two (`Draw`); 2 damage each opponent (`DealDamage`) | ✅ resolves (already-green; runtime verify only) |
| Cinder Strike | additional-cost blight 1 (`BlightEffect`, `ability.rs:10873`); 2/4 damage (`DealDamage`) | ✅ resolves |
| **Celestial Reunion** | **behold-choose-a-creature-type cost (NO write path — B5)**; library search (`search_library.rs`); reveal+put hand; conditional battlefield-vs-hand | ⚠️ **needs B5 behold-choose-type cost model (§5)** — otherwise the cost stays RED (`is_choose_behold_prefix` guard) and the chosen type is never written |
| Coiling Rebirth | Gift (additional cost); return creature card → battlefield (`ChangeZone`); create token copy (`CreateTokenCopy`) | ✅ resolves |
| Longstalk Brawl | Gift a tapped Fish (cost); +1/+1 counter; fight (`Fight`) | ✅ resolves |
| Steer Clear | 2/4 damage to attacking/blocking creature (`DealDamage`) | ✅ resolves |
| Malamet Battle Glyph | two targets; +1/+1 counter; fight (`Fight`) | ✅ resolves; condition reuses existing `FilterProp::EnteredThisTurn` (B1 — no new surface) |
| Fear of Immobility | ETB tap up to one target (`Tap`); stun counter (`AddCounters` stun) | ✅ resolves |
| Charging Hooligan | attack trigger +1/+0 per attacker (pump w/ `ObjectCount`); grant trample (kw grant) | ✅ resolves |
| **A Killer Among Us** | create 3 tokens (`CreateToken`); **secret constrained creature-type choice (NEEDS work)**; attack trigger +3 counters | ⚠️ **needs `ChoiceType::CreatureType` candidate restriction + secret interactive choice (§5)** |
| Yenna | choose target enchantment; create token copy; untap (`Untap`); scry 2 (`Scry`) | ✅ resolves |
| Eliminate the Impossible | Investigate (`Investigate`); -2/-0 EOT (pump static); unsuspect (`Effect::Unsuspect`) | ✅ resolves |
| Trance Kuja | double-damage `ReplacementDefinition` | ✅ resolves |
| The Rollercrusher Ride | double-noncombat-damage `ReplacementDefinition` + delirium gate | ✅ resolves |
| **Cloud** | ETB search Equipment/reveal/hand/shuffle (`search_library.rs`); DoubleTriggers static | ⚠️ parse resolves; **DoubleTriggers self-double needs B2 runtime fix** (`triggers.rs:4641`) — not an `unimplemented` clause but a correctness gap that makes the SelfRef branch dead |
| Sonic Shrieker | Flying (kw); 2 damage any target (`DealDamage`); gain 2 life (`GainLife`); discard (`DiscardCard`) | ✅ resolves; condition needs new `PlayerFilter::DamagedThisWay` (§4 item 1) |
| Slumbering Trudge | enters-with stun counters = 3−X (`QuantityExpr`); enters-tapped conditional | ✅ resolves — **verify enters-with parser accepts `QuantityExpr` not just a literal; widen if literal-only (§5, in scope)** |
| Avatar Aang | Flying + firebending 2 (kw); bend trigger draw (`ElementalBend`); transform (DFC) | ✅ resolves; condition needs new `QuantityRef::DistinctBendTypesThisTurn` (§4 item 2) |

**Summary:** 25/28 have all non-condition clauses already resolving; only Celestial Reunion (B5),
A Killer Among Us (secret constrained choice), and Cloud (B2 runtime) carry non-condition work — all
planned in-scope in §5. Slumbering Trudge's enters-with widening is a small verify-and-widen. No card
is deferred.
