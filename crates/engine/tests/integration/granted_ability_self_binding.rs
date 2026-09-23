//! Runtime + grant-clone proofs for the granted-ability self-reference dual
//! binding (S25). CR 201.5a: when an ability's effect grants another ability
//! that refers to the granting object BY NAME, the name refers only to the
//! granting object — never to the host it was granted to.
//!
//! Three independent channels are exercised, and must stay separate. Each now
//! has a TYPED half (the AST the engine resolves) and a DISPLAY half (the
//! `description` string the client renders), and the two must agree:
//!   1. Granter-referential ("Exile/Sacrifice/Return <granter-name>") →
//!      TYPED: `TargetFilter::GrantingObject` → concretized to
//!      `SpecificObject{granter}`. DISPLAY: the granting card's PRINTED name
//!      (`oracle_util::render_granting_self_reference`, CR 201.5a + CR 201.5c).
//!   2. Host-referential ("Sacrifice this permanent") → TYPED: stays `SelfRef` →
//!      host. DISPLAY: stays the host token `~`, which the client substitutes
//!      with the object's own name (CR 201.5b).
//!   3. Host power read ("where X is this creature's power") → TYPED:
//!      `QuantityRef::Power` (never a `TargetFilter`) → unchanged. DISPLAY:
//!      unchanged.
//!
//! A display half that disagreed with its typed half would be strictly worse
//! than a consistent error: the UI would say "sacrifice the Equipment" while the
//! engine sacrificed the creature.
//!
//! Every behavioral test drives the production Layer-6 grant path
//! (`evaluate_layers`) and, for Deconstruction Hammer, the full activate/resolve
//! pipeline asserting which object left the battlefield.

use std::sync::Arc;

use engine::game::game_object::AttachTarget;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameScenario, P0, P1};
use engine::parser::oracle::parse_oracle_text;
use engine::parser::oracle_util::normalize_card_name_refs;
use engine::types::ability::{
    AbilityCondition, AbilityCost, AbilityDefinition, Comparator, ContinuousModification, Effect,
    ObjectScope, QuantityExpr, QuantityRef, StaticDefinition, TargetFilter,
};
use engine::types::card_type::CoreType;
use engine::types::counter::CounterType;
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

fn equipment_types() -> (Vec<String>, Vec<String>) {
    (vec!["Artifact".to_string()], vec!["Equipment".to_string()])
}

/// The `AbilityDefinition` an equipment grants via its "Equipped creature has …"
/// static (the parse-time, pre-concretization body).
fn granted_activated_def(oracle: &str, name: &str) -> AbilityDefinition {
    let (types, subtypes) = equipment_types();
    let parsed = parse_oracle_text(oracle, name, &[], &types, &subtypes);
    grant_ability_static(&parsed.statics)
        .modifications
        .iter()
        .find_map(|m| match m {
            ContinuousModification::GrantAbility { definition } => Some((**definition).clone()),
            _ => None,
        })
        .expect("equipment must grant an activated ability")
}

fn grant_ability_static(statics: &[StaticDefinition]) -> StaticDefinition {
    statics
        .iter()
        .find(|s| {
            s.modifications
                .iter()
                .any(|m| matches!(m, ContinuousModification::GrantAbility { .. }))
        })
        .expect("equipment must have a GrantAbility static")
        .clone()
}

/// Install `grant_static` on a fresh artifact-equipment attached to `host`, then
/// run the production layer engine so the granted ability is cloned onto the
/// host with its granter self-references concretized.
fn equip_and_layer(
    scenario: GameScenario,
    equipment: ObjectId,
    host: ObjectId,
    grant_static: StaticDefinition,
) -> engine::game::scenario::GameRunner {
    let mut runner = scenario.build();
    {
        let st = runner.state_mut();
        let obj = st.objects.get_mut(&equipment).unwrap();
        obj.card_types.core_types = vec![CoreType::Artifact];
        obj.card_types.subtypes = vec!["Equipment".to_string()];
        obj.base_card_types = obj.card_types.clone();
        obj.power = None;
        obj.toughness = None;
        obj.base_power = None;
        obj.base_toughness = None;
        obj.attached_to = Some(AttachTarget::Object(host));
        obj.static_definitions.push(grant_static.clone());
        Arc::make_mut(&mut obj.base_static_definitions).push(grant_static);
        st.layers_dirty.mark_full();
    }
    evaluate_layers(runner.state_mut());
    runner
}

fn granted_ability_index(
    runner: &engine::game::scenario::GameRunner,
    host: ObjectId,
    pred: impl Fn(&AbilityDefinition) -> bool,
) -> usize {
    runner.state().objects[&host]
        .abilities
        .iter()
        .position(pred)
        .expect("host must carry the granted ability after evaluate_layers")
}

// ---------------------------------------------------------------------------
// Direction A — granter-referential COST/EFFECT resolves to the GRANTING object.
// ---------------------------------------------------------------------------

/// A1: Deconstruction Hammer's sacrifice cost sacrifices THE HAMMER (the granting
/// equipment), not the equipped creature. Full activate/resolve pipeline; asserts
/// which object left the battlefield.
///
/// Revert-to-red: remove the `layers.rs` GrantingObject→SpecificObject rewrite →
/// the cost stays `GrantingObject`, the defensive runtime arm resolves it to the
/// ability source (host) → the CREATURE is sacrificed and the Hammer survives →
/// both the concretization `assert_eq!` and the zone assertions flip.
#[test]
fn deconstruction_hammer_sacrifice_hits_the_equipment_not_the_host() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_mana_pool(
        P0,
        vec![
            ManaUnit::new(ManaType::White, ObjectId(0), false, vec![]),
            ManaUnit::new(ManaType::White, ObjectId(0), false, vec![]),
            ManaUnit::new(ManaType::White, ObjectId(0), false, vec![]),
        ],
    );
    let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
    let hammer = scenario
        .add_creature(P0, "Deconstruction Hammer", 0, 0)
        .id();
    let victim = scenario.add_creature(P1, "Relic", 0, 0).id();

    let (types, subtypes) = equipment_types();
    let grant_static = grant_ability_static(
        &parse_oracle_text(
            DECONSTRUCTION_HAMMER,
            "Deconstruction Hammer",
            &[],
            &types,
            &subtypes,
        )
        .statics,
    );

    let mut runner = {
        // Make the victim a destructible artifact target BEFORE the layer pass.
        let mut runner = equip_and_layer(scenario, hammer, host, grant_static);
        {
            let v = runner.state_mut().objects.get_mut(&victim).unwrap();
            v.card_types.core_types = vec![CoreType::Artifact];
            v.base_card_types = v.card_types.clone();
            v.power = None;
            v.toughness = None;
            v.base_power = None;
            v.base_toughness = None;
        }
        runner
    };

    let idx = granted_ability_index(&runner, host, |a| {
        a.cost.as_ref().and_then(sacrifice_target).is_some()
    });

    // Concretization proof (the layers.rs seam): the sacrifice cost (inside the
    // `{3},{T},Sacrifice` Composite) targets the Hammer, not `SelfRef`/`GrantingObject`.
    assert_eq!(
        runner.state().objects[&host].abilities[idx]
            .cost
            .as_ref()
            .and_then(sacrifice_target),
        Some(&TargetFilter::SpecificObject { id: hammer }),
        "CR 201.5a: sacrifice cost must target the granting Hammer, not the host"
    );

    // DISPLAY half of the same seam (matrix rows 1 and 3). This MUST run before
    // the activate below: the Hammer is sacrificed, the grant ends, and
    // `objects[&host].abilities` is empty afterwards (measured: index out of
    // bounds, len 0).
    let desc = runner.state().objects[&host].abilities[idx]
        .description
        .clone()
        .expect("the granted ability carries a display description");
    assert_eq!(
        desc, "{3}, {T}, Sacrifice Deconstruction Hammer: Destroy target artifact or enchantment.",
        "CR 201.5a: the granted body's description must name the GRANTING Hammer, \
         not collapse to the host token `~`"
    );
    // CLIENT PARITY, weaker form. `renderDescription(desc, object.name)` on the
    // host must not put the host's name anywhere in this body. This card's
    // effect half carries no `~`, so this proves only "the host name appears
    // NOWHERE"; the discriminating both-halves fixture is
    // `game::effects::token::tests::catalog_toggo_rock_sacrifice_cost_binds_to_rock_not_host`
    // (Rock's printed body carries a CR 201.5a granter reference in the cost AND
    // a CR 201.5b host `~` in the effect).
    let rendered = desc.replace('~', "Bearer");
    assert!(
        rendered.starts_with("{3}, {T}, Sacrifice Deconstruction Hammer:"),
        "CR 201.5a: a blanket `~`-replace would render `Sacrifice Bearer:`; got {rendered}"
    );
    assert_eq!(
        rendered.matches("Bearer").count(),
        0,
        "the host's name must not appear anywhere in this granted body; got {rendered}"
    );

    // Runtime proof: activate the granted ability, paying the sacrifice cost with
    // the Hammer and targeting the artifact, then assert which permanents left the
    // battlefield.
    let outcome = runner
        .activate(host, idx)
        .target_object(victim)
        .pay_with(&[hammer])
        .resolve();
    assert_eq!(
        outcome.zone_of(hammer),
        Zone::Graveyard,
        "CR 701.21a: the Hammer (granting object) is sacrificed to its owner's graveyard"
    );
    assert_eq!(
        outcome.zone_of(host),
        Zone::Battlefield,
        "the equipped creature survives — it is NOT the object named in the cost"
    );
    assert_eq!(
        outcome.zone_of(victim),
        Zone::Graveyard,
        "the targeted artifact is destroyed by the resolved effect"
    );
}

/// A2 + B1: The Dominion Bracelet. The `{15}, Exile <self>` cost exiles THE
/// BRACELET (granter-referential → GrantingObject → SpecificObject{bracelet}),
/// while the `{X} less … this creature's power` reduction stays host-referential
/// (`QuantityRef::Power{Source}`, an untouched third channel).
///
/// Parse-shape supplement proves the two `~`-collapsed referents split; the
/// `evaluate_layers` assertion proves the production concretization. Full {15}
/// activation is impractical, but the Exile-cost runtime resolution reuses the
/// exact `SpecificObject` machinery the Hammer test drives end-to-end.
#[test]
fn the_dominion_bracelet_exile_hits_the_bracelet_reduction_reads_the_host() {
    // Parse-shape: cost = Exile{GrantingObject}; reduction = Power{Source}; no
    // residual Unimplemented reduction node.
    let def = granted_activated_def(THE_DOMINION_BRACELET, "The Dominion Bracelet");
    assert_eq!(
        def.cost.as_ref().and_then(exile_filter),
        Some(&TargetFilter::GrantingObject),
        "the Exile cost names the Bracelet (granter) → GrantingObject, not SelfRef"
    );
    let reduction = def
        .cost_reduction
        .as_ref()
        .expect("the {X}-less reduction must fold into cost_reduction, not stay Unimplemented");
    assert_eq!(
        reduction.count,
        QuantityExpr::Ref {
            qty: QuantityRef::Power {
                scope: ObjectScope::Source
            }
        },
        "the reduction reads the equipped creature's power (host) — untouched third channel"
    );
    assert!(
        find_effect(&def, |e| matches!(e, Effect::Unimplemented { .. })).is_none(),
        "no residual Unimplemented cost-reduction node should remain"
    );

    // Production concretization: after grant-clone the host's Exile cost is
    // SpecificObject{bracelet}.
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let host = scenario.add_creature(P0, "Bearer", 3, 3).id();
    let bracelet = scenario
        .add_creature(P0, "The Dominion Bracelet", 0, 0)
        .id();
    let (types, subtypes) = equipment_types();
    let grant_static = grant_ability_static(
        &parse_oracle_text(
            THE_DOMINION_BRACELET,
            "The Dominion Bracelet",
            &[],
            &types,
            &subtypes,
        )
        .statics,
    );
    let runner = equip_and_layer(scenario, bracelet, host, grant_static);
    let idx = granted_ability_index(&runner, host, |a| {
        a.cost.as_ref().and_then(exile_filter).is_some()
    });
    assert_eq!(
        runner.state().objects[&host].abilities[idx]
            .cost
            .as_ref()
            .and_then(exile_filter),
        Some(&TargetFilter::SpecificObject { id: bracelet }),
        "CR 201.5a: the concretized Exile cost targets the Bracelet, not the host"
    );
    // Host power read is unchanged by concretization.
    assert_eq!(
        runner.state().objects[&host].abilities[idx]
            .cost_reduction
            .as_ref()
            .map(|r| &r.count),
        Some(&QuantityExpr::Ref {
            qty: QuantityRef::Power {
                scope: ObjectScope::Source
            }
        }),
        "the power reduction remains host-referential after grant-clone"
    );
}

/// A3 (effect-target channel): Trusty Boomerang's "Return <self> to its owner's
/// hand" bounces THE EQUIPMENT. After grant-clone the Bounce effect target is
/// `SpecificObject{boomerang}`, proving the effect channel (parse_self_reference)
/// concretizes just like the cost channel.
///
/// Revert-to-red: without the layers.rs rewrite the Bounce target stays
/// `GrantingObject` (≠ SpecificObject{boomerang}) → assertion fails.
#[test]
fn trusty_boomerang_return_bounces_the_equipment_not_the_host() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
    let boomerang = scenario.add_creature(P0, "Trusty Boomerang", 0, 0).id();
    let (types, subtypes) = equipment_types();
    let grant_static = grant_ability_static(
        &parse_oracle_text(TRUSTY_BOOMERANG, "Trusty Boomerang", &[], &types, &subtypes).statics,
    );
    let runner = equip_and_layer(scenario, boomerang, host, grant_static);

    let idx = granted_ability_index(&runner, host, |a| {
        find_effect(a, |e| matches!(e, Effect::Bounce { .. })).is_some()
    });
    let bounce_target = find_effect(&runner.state().objects[&host].abilities[idx], |e| {
        matches!(e, Effect::Bounce { .. })
    })
    .and_then(|e| match e {
        Effect::Bounce { target, .. } => Some(target.clone()),
        _ => None,
    })
    .expect("granted ability must carry a Bounce effect");
    assert_eq!(
        bounce_target,
        TargetFilter::SpecificObject { id: boomerang },
        "CR 201.5a: the granted Return bounces the Boomerang (granter), not the host"
    );
}

// ---------------------------------------------------------------------------
// Direction B — host-referential "this permanent" stays on the HOST.
// ---------------------------------------------------------------------------

const ACIDIC_SLIVER: &str =
    "All Slivers have \"{2}, Sacrifice this permanent: This permanent deals 2 damage to any target.\"";

/// B2: An Acidic-Sliver-style grant to a SECOND Sliver keeps its "Sacrifice this
/// permanent" cost bound to the HOST (`SelfRef`), never rebound to the granting
/// Sliver. This is the discriminating proof that "this permanent" (a
/// `SELF_REF_TYPE_PHRASES` self-ref, never the card name) is NOT masked to a
/// granter reference — a blanket "SelfRef-in-granted → granter" rewrite would
/// make this `SpecificObject{granter}` and fail.
#[test]
fn sliver_host_ref_sacrifice_stays_on_the_host_not_the_granter() {
    let (types, subtypes) = (vec!["Creature".to_string()], vec!["Sliver".to_string()]);
    let parsed = parse_oracle_text(ACIDIC_SLIVER, "Acidic Sliver", &[], &types, &subtypes);
    let granted = grant_ability_static(&parsed.statics)
        .modifications
        .iter()
        .find_map(|m| match m {
            ContinuousModification::GrantAbility { definition } => Some((**definition).clone()),
            _ => None,
        })
        .expect("Slivers grant an activated ability");
    assert_eq!(
        granted.cost.as_ref().and_then(sacrifice_target),
        Some(&TargetFilter::SelfRef),
        "\"Sacrifice this permanent\" is host-referential (SelfRef), never GrantingObject"
    );
    assert!(
        !contains_granting_object(&granted),
        "a host-ref Sliver ability must contain no GrantingObject reference"
    );

    // DISPLAY half. CR 201.5b: a host reference stays the host token `~` and must
    // NOT gain the granting card's name — the render is sentinel-driven, not a
    // blanket name substitution. Reach-guard: the `SelfRef` assertion above proves
    // this body really is the host-referential shape.
    let desc = granted
        .description
        .as_deref()
        .expect("the granted Sliver ability carries a display description");
    assert!(
        desc.contains('~'),
        "CR 201.5b: the host reference must stay `~`; got {desc}"
    );
    assert!(
        !desc.contains("Acidic Sliver"),
        "CR 201.5b: a host reference must never render as the GRANTER's name; got {desc}"
    );
}

// ---------------------------------------------------------------------------
// Direction C — R1 regression guard: `named <self>` name-FILTERS are preserved.
// ---------------------------------------------------------------------------

const FOOD_FIGHT: &str = "Artifacts you control have \"{2}, Sacrifice this artifact: \
It deals damage to any target equal to 1 plus the number of permanents named Food Fight you control.\"";

/// C (R1 negative): Food Fight's "permanents named Food Fight" is a name-FILTER,
/// not a self-reference. The quote masker must SKIP the `named <self>` position,
/// so the name survives to the count filter (and never becomes GrantingObject or
/// the raw placeholder char).
///
/// Revert-to-red: remove the `named`-position skip in
/// `mask_granting_self_reference_in_quotes` → "Food Fight" after `named` is
/// masked to the placeholder, the `named ~`→`named Food Fight` restoration never
/// fires, and the structural AST loses "Food Fight" (gains the placeholder char)
/// → this assertion fails.
#[test]
fn food_fight_named_self_filter_is_not_masked() {
    let (types, subtypes) = (vec!["Artifact".to_string()], Vec::<String>::new());
    let parsed = parse_oracle_text(FOOD_FIGHT, "Food Fight", &[], &types, &subtypes);
    let mut granted = grant_ability_static(&parsed.statics)
        .modifications
        .iter()
        .find_map(|m| match m {
            ContinuousModification::GrantAbility { definition } => Some((**definition).clone()),
            _ => None,
        })
        .expect("Food Fight grants an activated ability");

    // The host self-sacrifice cost is unaffected (positive reach-guard: the body
    // parsed past the cost separator into a real granted ability).
    assert_eq!(
        granted.cost.as_ref().and_then(sacrifice_target),
        Some(&TargetFilter::SelfRef),
        "\"Sacrifice this artifact\" is host-referential (SelfRef)"
    );

    // Structural (description-independent) check: the name survives in the count
    // filter; no GrantingObject and no leaked placeholder char. (The parser
    // lower-cases filter names, so match case-insensitively.)
    granted.description = None;
    let structural = format!("{granted:?}");
    assert!(
        structural.to_lowercase().contains("food fight"),
        "the `named Food Fight` name-filter must preserve the card name; got {structural}"
    );
    let json = serde_json::to_string(&granted).expect("the granted definition serializes");
    assert!(
        !json.contains(PLACEHOLDER),
        "the granting-object placeholder must never leak into the AST"
    );
    assert!(
        !contains_granting_object(&granted),
        "a name-FILTER position must not become a GrantingObject self-reference"
    );
}

// ---------------------------------------------------------------------------
// Recursive AST walkers used by the assertions above.
// ---------------------------------------------------------------------------

fn find_effect(def: &AbilityDefinition, pred: impl Fn(&Effect) -> bool + Copy) -> Option<&Effect> {
    if pred(&def.effect) {
        return Some(&def.effect);
    }
    for child in def
        .sub_ability
        .iter()
        .chain(def.else_ability.iter())
        .map(|b| b.as_ref())
        .chain(def.mode_abilities.iter())
    {
        if let Some(found) = find_effect(child, pred) {
            return Some(found);
        }
    }
    None
}

/// The Sacrifice cost's target filter, searching inside `Composite`/`OneOf`
/// (activation costs like `{3},{T},Sacrifice <x>` parse to a Composite).
fn sacrifice_target(cost: &AbilityCost) -> Option<&TargetFilter> {
    match cost {
        AbilityCost::Sacrifice(sac) => Some(&sac.target),
        AbilityCost::Composite { costs } | AbilityCost::OneOf { costs } => {
            costs.iter().find_map(sacrifice_target)
        }
        _ => None,
    }
}

/// The Exile cost's filter, searching inside `Composite`/`OneOf`.
fn exile_filter(cost: &AbilityCost) -> Option<&TargetFilter> {
    match cost {
        AbilityCost::Exile { filter, .. } => filter.as_ref(),
        AbilityCost::Composite { costs } | AbilityCost::OneOf { costs } => {
            costs.iter().find_map(exile_filter)
        }
        _ => None,
    }
}

/// Sound presence test for the fieldless `TargetFilter::GrantingObject` variant:
/// its debug repr is exactly `GrantingObject`, and no other AST node's debug
/// output contains that substring. Used only for the negative assertions here.
fn contains_granting_object(def: &AbilityDefinition) -> bool {
    format!("{def:?}").contains("GrantingObject")
}

/// The target filter of a single target-bearing effect (subset used here).
fn effect_target(effect: &Effect) -> Option<&TargetFilter> {
    match effect {
        Effect::PutCounter { target, .. }
        | Effect::GainControl { target, .. }
        | Effect::Bounce { target, .. }
        | Effect::Destroy { target, .. } => Some(target),
        _ => None,
    }
}

/// The GrantAbility body an equipment/aura grants via its "…has \"…\"" static.
fn granted_def_from(
    oracle: &str,
    name: &str,
    types: &[&str],
    subtypes: &[&str],
) -> AbilityDefinition {
    let types: Vec<String> = types.iter().map(|s| s.to_string()).collect();
    let subtypes: Vec<String> = subtypes.iter().map(|s| s.to_string()).collect();
    let parsed = parse_oracle_text(oracle, name, &[], &types, &subtypes);
    grant_ability_static(&parsed.statics)
        .modifications
        .iter()
        .find_map(|m| match m {
            ContinuousModification::GrantAbility { definition } => Some((**definition).clone()),
            _ => None,
        })
        .expect("card must grant an activated ability")
}

/// The private-use masker placeholder (U+E0002). Must NEVER survive into the AST.
const PLACEHOLDER: char = '\u{E0002}';

// ---------------------------------------------------------------------------
// CR 201.5a class corpus: exported cards whose quoted granted body names the
// card itself in a `GRANTER_SELF_REF_VERB_PREFIXES` position and whose parse
// carries a granter symbol. Every text is the verbatim Oracle text, reminder text
// and all, because a paraphrase can take a different parser branch.
//
// The predefined token Rock reaches the parser through
// `game::effects::token::catalog_rules_text_abilities`; its arm of this corpus
// property lives in
// `game::effects::token::tests::catalog_rules_text_abilities_never_leaks_the_placeholder`.
// ---------------------------------------------------------------------------

const BLAZING_TORCH: &str =
    "Equipped creature can't be blocked by Vampires or Zombies.\nEquipped creature has \"{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target.\"\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)";
const CITIZENS_CROWBAR: &str =
    "When this Equipment enters, create a 1/1 green and white Citizen creature token, then attach this Equipment to it.\nEquipped creature gets +1/+1 and has \"{W}, {T}, Sacrifice Citizen's Crowbar: Destroy target artifact or enchantment.\"\nEquip {2} ({2}: Attach to target creature you control. Equip only as a sorcery.)";
const DECONSTRUCTION_HAMMER: &str =
    "Equipped creature gets +1/+1 and has \"{3}, {T}, Sacrifice Deconstruction Hammer: Destroy target artifact or enchantment.\"\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)";
const FISHING_POLE: &str =
    "Equipped creature has \"{1}, {T}, Tap Fishing Pole: Put a bait counter on Fishing Pole.\"\nWhenever equipped creature becomes untapped, remove a bait counter from this Equipment. If you do, create a 1/1 blue Fish creature token.\nEquip {2} ({2}: Attach to target creature you control. Equip only as a sorcery.)";
const HANKYU: &str =
    "Equipped creature has \"{T}: Put an aim counter on Hankyu\" and \"{T}, Remove all aim counters from Hankyu: This creature deals damage to any target equal to the number of aim counters removed this way.\"\nEquip {4} ({4}: Attach to target creature you control. Equip only as a sorcery.)";
const MEANDERED_TOWERSHELL: &str =
    "Enchant creature\nEnchanted creature has islandwalk and \"Whenever this creature attacks, exile it and Meandered Towershell. Return it to the battlefield under your control tapped and attacking at the beginning of the declare attackers step on your next turn, then return Meandered Towershell to the battlefield under its owner's control attached to that creature.\"";
const NINJAS_KUNAI: &str =
    "Equipped creature has \"{1}, {T}, Sacrifice Ninja's Kunai: Ninja's Kunai deals 3 damage to any target.\"\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)";
const RAKDOS_RITEKNIFE: &str =
    "Equipped creature gets +1/+0 for each blood counter on this Equipment and has \"{T}, Sacrifice a creature: Put a blood counter on Rakdos Riteknife.\"\n{B}{R}, Sacrifice this Equipment: Target player sacrifices a permanent of their choice for each blood counter on this Equipment.\nEquip {2}";
const RAZOR_BOOMERANG: &str =
    "Equipped creature has \"{T}, Unattach Razor Boomerang: It deals 1 damage to any target. Return Razor Boomerang to its owner's hand.\"\nEquip {2}";
const SAKASHIMA_THE_IMPOSTOR: &str =
    "You may have Sakashima the Impostor enter as a copy of any creature on the battlefield, except its name is Sakashima the Impostor, it's legendary in addition to its other types, and it has \"{2}{U}{U}: Return Sakashima the Impostor to its owner's hand at the beginning of the next end step.\"";
const SPARE_DAGGER: &str =
    "Equipped creature gets +1/+0 and has \"Whenever this creature attacks, you may sacrifice Spare Dagger. When you do, this creature deals 1 damage to any target.\"\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)";
const SUNFIRE_TORCH: &str =
    "Equipped creature gets +1/+0 and has \"Whenever this creature attacks, you may sacrifice Sunfire Torch. When you do, this creature deals 2 damage to any target.\"\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)";
const THE_DOMINION_BRACELET: &str =
    "Equipped creature gets +1/+1 and has \"{15}, Exile The Dominion Bracelet: You control target opponent during their next turn. This ability costs {X} less to activate, where X is this creature's power. Activate only as a sorcery.\" (You see all cards that player could see and make all decisions for them.)\nEquip {1}";
const TORALFS_HAMMER: &str =
    "Equipped creature has \"{1}{R}, {T}, Unattach Toralf's Hammer: It deals 3 damage to any target. Return Toralf's Hammer to its owner's hand.\"\nEquipped creature gets +3/+0 as long as it's legendary.\nEquip {1}{R}";
const TRICKSTERS_TALISMAN: &str =
    "Invoke Duplicity \u{2014} Equipped creature gets +1/+1 and has \"Whenever this creature deals combat damage to a player, you may sacrifice Trickster's Talisman. If you do, create a token that's a copy of this creature.\"\nEquip {2}";
const TRUSTY_BOOMERANG: &str =
    "Equipped creature has \"{1}, {T}: Tap target creature. Return Trusty Boomerang to its owner's hand.\"\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)";
const GUTTER_GRIME: &str = "Whenever a nontoken creature you control dies, put a slime \
counter on this enchantment, then create a green Ooze creature token with \"This token's power \
and toughness are each equal to the number of slime counters on Gutter Grime.\"";
const DIRE_BLUNDERBUSS: &str = "Equipped creature gets +3/+0 and has \"Whenever this creature \
attacks, you may sacrifice an artifact other than Dire Blunderbuss. When you do, this creature \
deals damage equal to its power to target creature.\"\nEquip {1}";
const NETTLEVINE_BLIGHT: &str = "Enchant creature or land\nEnchanted permanent has \"At the \
beginning of your end step, sacrifice this permanent and attach Nettlevine Blight to a creature \
or land you control.\"";
const HELIODS_PUNISHMENT: &str = "Enchant creature\nThis Aura enters with four task counters \
on it.\nEnchanted creature can't attack or block. It loses all abilities and has \"{T}: Remove a \
task counter from Heliod's Punishment. Then if it has no task counters on it, destroy Heliod's \
Punishment.\"";
const SAPROLING_BURST: &str = "Fading 7 (This enchantment enters with seven fade counters on it. \
At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)\n\
Remove a fade counter from this enchantment: Create a green Saproling creature token. It has \
\"This token's power and toughness are each equal to the number of fade counters on Saproling \
Burst.\"\nWhen this enchantment leaves the battlefield, destroy all tokens created with this \
enchantment. They can't be regenerated.";
const GROTHAMA: &str = "Other creatures have \"Whenever this creature attacks, you may have it \
fight Grothama, All-Devouring.\"\nWhen Grothama leaves the battlefield, each player draws cards \
equal to the amount of damage dealt to Grothama this turn by sources they controlled.";
const THE_AETHERSPARK: &str = "As long as The Aetherspark is attached to a creature, The \
Aetherspark can't be attacked and has \"Whenever equipped creature deals combat damage during \
your turn, put that many loyalty counters on The Aetherspark.\"\n[+1]: Attach The Aetherspark to \
up to one target creature you control. Put a +1/+1 counter on that creature.\n[\u{2212}5]: Draw \
two cards.\n[\u{2212}10]: Add ten mana of any one color.";
const SHIFTING_SHADOW: &str = "Enchant creature\nEnchanted creature has haste and \"At the \
beginning of your upkeep, destroy this creature. Reveal cards from the top of your library until \
you reveal a creature card. Put that card onto the battlefield and attach Shifting Shadow to it, \
then put all other cards revealed this way on the bottom of your library in a random order.\"";

/// `(oracle text, printed name, core types, subtypes)` for the exported class
/// members.
const CLASS_CORPUS: &[(&str, &str, &[&str], &[&str])] = &[
    (
        BLAZING_TORCH,
        "Blazing Torch",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        CITIZENS_CROWBAR,
        "Citizen's Crowbar",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        DECONSTRUCTION_HAMMER,
        "Deconstruction Hammer",
        &["Artifact"],
        &["Equipment"],
    ),
    (FISHING_POLE, "Fishing Pole", &["Artifact"], &["Equipment"]),
    (HANKYU, "Hankyu", &["Artifact"], &["Equipment"]),
    (
        MEANDERED_TOWERSHELL,
        "Meandered Towershell",
        &["Enchantment"],
        &["Aura"],
    ),
    (NINJAS_KUNAI, "Ninja's Kunai", &["Artifact"], &["Equipment"]),
    (
        RAKDOS_RITEKNIFE,
        "Rakdos Riteknife",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        RAZOR_BOOMERANG,
        "Razor Boomerang",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        SAKASHIMA_THE_IMPOSTOR,
        "Sakashima the Impostor",
        &["Creature"],
        &["Human", "Rogue"],
    ),
    (SPARE_DAGGER, "Spare Dagger", &["Artifact"], &["Equipment"]),
    (
        SUNFIRE_TORCH,
        "Sunfire Torch",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        THE_DOMINION_BRACELET,
        "The Dominion Bracelet",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        TORALFS_HAMMER,
        "Toralf's Hammer",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        TRICKSTERS_TALISMAN,
        "Trickster's Talisman",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        TRUSTY_BOOMERANG,
        "Trusty Boomerang",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        ARCHERY_TRAINING,
        "Archery Training",
        &["Enchantment"],
        &["Aura"],
    ),
    (GUTTER_GRIME, "Gutter Grime", &["Enchantment"], &[]),
    (SAPROLING_BURST, "Saproling Burst", &["Enchantment"], &[]),
    (
        DIRE_BLUNDERBUSS,
        "Dire Blunderbuss",
        &["Artifact"],
        &["Equipment"],
    ),
    (
        THE_AETHERSPARK,
        "The Aetherspark",
        &["Artifact", "Planeswalker"],
        &["Equipment"],
    ),
    (
        HELIODS_PUNISHMENT,
        "Heliod's Punishment",
        &["Enchantment"],
        &["Aura"],
    ),
    (
        GROTHAMA,
        "Grothama, All-Devouring",
        &["Creature"],
        &["Wurm"],
    ),
    (
        NETTLEVINE_BLIGHT,
        "Nettlevine Blight",
        &["Enchantment"],
        &["Aura"],
    ),
];

/// CR 201.5a: no raw U+E0002 may survive into ANY string reachable from
/// `ParsedAbilities`' four top-level vectors through the render net's descend
/// set — including the outer static/trigger DESCRIPTION strings that embed the
/// raw quoted text (a granted body's "…has \"…Sacrifice <self>…\"" description).
/// `parser::oracle::render_granting_self_descriptions` renders every residual
/// marker to the granting card's printed name.
///
/// TWO REPAIRS to the round-1 form of this guard, both of which were measured
/// vacuous:
///
/// 1. **`serde_json`, not `format!("{:?}")`.** `Debug` ESCAPES the raw
///    private-use char to the literal text `\u{e0002}`, so searching a `Debug`
///    dump for the real character was ALWAYS false — the guard could not fail.
///    `serde_json` emits it raw, at every `String`, at every depth, which is
///    strictly stronger than any hand-written `visit_*` walk.
/// 2. **The whole measured class, not four constants.** Four cards cannot see a
///    copy-family regression; Sakashima is the only shipped card whose granted
///    description lives inside an `Effect::BecomeCopy` payload.
///
/// SCOPE NOTE: the serde ORACLE is WIDER than the net's REPAIR. It serializes
/// `def.cost` too, so a cost-borne marker would red here even though the net
/// deliberately does not walk the `AbilityCost` axis (the named excluded axis —
/// see `parser::oracle::tests::granted_cost_axis_is_not_walked_and_no_parse_shape_reaches_it`).
/// That is the correct polarity: this guard should red if a marker ever reaches
/// a cost, because nothing downstream would render it.
///
/// Non-vacuity is proved by `placeholder_leak_guard_reports_a_planted_marker`.
///
/// Revert-to-red: remove the render net from `parse_oracle_text` → every card's
/// outer static description carries the raw U+E0002 char.
#[test]
fn placeholder_never_leaks_into_any_description() {
    for &(oracle, name, types, subtypes) in CLASS_CORPUS {
        let types: Vec<String> = types.iter().map(|s| s.to_string()).collect();
        let subtypes: Vec<String> = subtypes.iter().map(|s| s.to_string()).collect();
        let p = parse_oracle_text(oracle, name, &[], &types, &subtypes);
        let json = serde_json::to_string(&p).expect("ParsedAbilities serializes");
        // PER-CARD POSITIVE REACH-GUARD: this card must actually be a class
        // member in the parsed tree — the masker fired and the typed channel
        // consumed the marker as `TargetFilter::GrantingObject`. Without it, a
        // card that silently stopped parsing its granted body would pass the
        // negative below on an empty tree.
        assert!(
            json.contains("GrantingObject"),
            "reach-guard: {name} must carry a granter self-reference in the typed \
             channel, or its leak assertion below is vacuous"
        );
        assert!(
            !json.contains(PLACEHOLDER),
            "{name}: the masker placeholder must render to the granting card's \
             printed name in every description; a raw U+E0002 leaked"
        );
    }
}

/// CR 201.5a — NON-VACUITY PROOF for `placeholder_never_leaks_into_any_description`.
///
/// A negative assertion is only worth what its ability to fail is worth. This
/// plants a marker into a real parsed tree AFTER the net has run and asserts the
/// same `serde_json` oracle DOES report it.
///
/// Revert-to-red: delete the injection — the guard passes on a clean tree and
/// this test's own assertion flips, which is the point.
#[test]
fn placeholder_leak_guard_reports_a_planted_marker() {
    let (types, subtypes) = equipment_types();
    let mut p = parse_oracle_text(
        DECONSTRUCTION_HAMMER,
        "Deconstruction Hammer",
        &[],
        &types,
        &subtypes,
    );
    assert!(
        !serde_json::to_string(&p)
            .expect("ParsedAbilities serializes")
            .contains(PLACEHOLDER),
        "reach-guard: the tree must be clean BEFORE the injection, or this test \
         proves nothing about the guard's sensitivity"
    );
    p.statics[0].description = Some(format!("x{PLACEHOLDER}y"));
    assert!(
        serde_json::to_string(&p)
            .expect("ParsedAbilities serializes")
            .contains(PLACEHOLDER),
        "the `serde_json` leak oracle must REPORT a planted marker — if it cannot \
         fail, `placeholder_never_leaks_into_any_description` is vacuous (which is \
         exactly what the round-1 `format!(\"{{:?}}\")` form was)"
    );
}

/// CR 201.5a — HOSTILE FIXTURE: two self-name occurrences in ONE granted body,
/// in DIFFERENT positions, bound independently.
///
/// Meandered Towershell's granted trigger body says, in order:
///   * "Whenever this creature attacks"  → a HOST reference (CR 201.5b) → `~`
///   * "exile it and Meandered Towershell" → lookbehind `and `, a refused
///     masker position, so it stays `~`.
///   * "return Meandered Towershell to the battlefield" → an ALLOWLISTED
///     (`return `) granter reference → masked → rendered as the printed name.
///
/// Revert-to-red: replace the sentinel render with a blanket
/// `text.replace('~', card_name)` → (b) fails, which is precisely the failure a
/// naive implementation produces.
#[test]
fn meandered_towershell_binds_each_occurrence_independently() {
    let parsed = parse_oracle_text(
        MEANDERED_TOWERSHELL,
        "Meandered Towershell",
        &[],
        &["Enchantment".to_string()],
        &["Aura".to_string()],
    );
    let json = serde_json::to_string(&parsed).expect("ParsedAbilities serializes");
    // POSITIVE REACH-GUARD: the allowlisted `return <granter>` occurrence really
    // reached the typed channel.
    assert!(
        json.contains("GrantingObject"),
        "reach-guard: the `return <granter>` occurrence must reach the typed channel"
    );

    let trigger = parsed
        .statics
        .iter()
        .flat_map(|s| s.modifications.iter())
        .find_map(|m| match m {
            ContinuousModification::GrantTrigger { trigger } => Some(trigger.as_ref()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the quoted body must parse to a GrantTrigger: {parsed:#?}"));
    let desc = trigger
        .description
        .as_deref()
        .expect("the granted trigger carries a display description");

    // (a) CR 201.5a: the allowlisted occurrence renders as the GRANTER's printed
    // name.
    assert!(
        desc.contains("Meandered Towershell"),
        "CR 201.5a: the `return <granter>` occurrence must render the printed \
         name; got {desc}"
    );
    // (b) CR 201.5b: the LEADING occurrence is a host reference and stays `~`.
    assert!(
        desc.starts_with("Whenever ~ attacks"),
        "CR 201.5b: the leading host reference must stay `~` — a blanket \
         `~`-replace renders `Whenever Meandered Towershell attacks`; got {desc}"
    );
}

/// CR 201.5a (last sentence: "This is also true if the second ability is copied
/// onto a new object") + CR 707.2 — HOSTILE FIXTURE: granter == host, via
/// copy-except.
///
/// Sakashima the Impostor is the ONLY shipped card whose granted description
/// lives inside an `Effect::BecomeCopy` payload. `types::ability_visit` treats
/// `BecomeCopy`/`CopySpell`/`CopyTokenOf` as LEAVES, so no walker in the tree
/// reaches this description without the render net's copy-family arm — which is
/// why this card is a load-bearing structural fixture, not a footnote. (Its
/// sibling `SetName` modification means the rendered output is the same before
/// and after this change; the STRUCTURAL claim is what this test pins.)
///
/// Revert-to-red: delete the `BecomeCopy | CopySpell | CopyTokenOf` arm from
/// `render_effect_descriptions` — the nested description retains the raw marker.
#[test]
fn sakashima_copy_except_grant_description_renders_the_granter() {
    let parsed = parse_oracle_text(
        SAKASHIMA_THE_IMPOSTOR,
        "Sakashima the Impostor",
        &[],
        &["Creature".to_string()],
        &["Human".to_string(), "Rogue".to_string()],
    );
    let json = serde_json::to_string(&parsed).expect("ParsedAbilities serializes");
    // POSITIVE REACH-GUARD: the self-grant's `Return <self> to its owner's hand`
    // really reached the typed channel as a granter reference.
    assert!(
        json.contains("GrantingObject"),
        "reach-guard: the copy-except self-grant must reach the typed channel"
    );
    assert!(
        !json.contains(PLACEHOLDER),
        "a raw CR 201.5a marker survived inside an `Effect::BecomeCopy` payload — \
         the copy-family descend arm is missing"
    );
    assert!(
        json.contains("Sakashima the Impostor to its owner"),
        "CR 201.5a: the granted body nested in the copy payload must name the \
         granting object: {json}"
    );
}

/// R4 (counter channel): the `put a … counter on <self>` (PutCounter target)
/// verb-object position emits `GrantingObject`, exactly like the
/// sacrifice/exile/return channels — Fishing Pole (multi-word) and Hankyu
/// (single-word, case-sensitive masking). Proves the position-aware masker's
/// allowlist still covers the counter target after the HIGH narrowing.
///
/// Revert-to-red: drop `counter on ` from `GRANTER_SELF_REF_VERB_PREFIXES` →
/// these bodies host-bind (`~`/SelfRef) → the `GrantingObject` assertion flips.
#[test]
fn r4_counter_channel_targets_the_granter() {
    for (oracle, name) in [(FISHING_POLE, "Fishing Pole"), (HANKYU, "Hankyu")] {
        let def = granted_def_from(oracle, name, &["Artifact"], &["Equipment"]);
        let target = find_effect(&def, |e| effect_target(e).is_some())
            .and_then(effect_target)
            .unwrap_or_else(|| {
                panic!("{name}: expected a target-bearing effect in the granted body")
            });
        assert_eq!(
            target,
            &TargetFilter::GrantingObject,
            "{name}: the PutCounter target names the granting equipment → GrantingObject"
        );
        // `serde_json`, not `format!("{:?}")`: `Debug` ESCAPES the raw private-use
        // char to the literal text `\u{e0002}`, so a Debug search for the real
        // character is always false and this negative would be vacuous.
        assert!(
            !serde_json::to_string(&def)
                .expect("the granted definition serializes")
                .contains(PLACEHOLDER),
            "{name}: no raw placeholder may survive into the AST"
        );
    }
}

fn granted_modifications(
    oracle: &str,
    name: &str,
    types: &[&str],
    subtypes: &[&str],
) -> Vec<ContinuousModification> {
    let types: Vec<String> = types.iter().map(|s| s.to_string()).collect();
    let subtypes: Vec<String> = subtypes.iter().map(|s| s.to_string()).collect();
    parse_oracle_text(oracle, name, &[], &types, &subtypes)
        .statics
        .into_iter()
        .flat_map(|s| s.modifications)
        .collect()
}

fn granted_trigger_effect(oracle: &str, name: &str, types: &[&str], subtypes: &[&str]) -> Effect {
    granted_modifications(oracle, name, types, subtypes)
        .into_iter()
        .find_map(|m| match m {
            ContinuousModification::GrantTrigger { trigger } => trigger.execute.map(|e| *e.effect),
            _ => None,
        })
        .expect("a granted trigger body")
}

fn cost_parts(cost: &AbilityCost) -> Vec<&AbilityCost> {
    match cost {
        AbilityCost::Composite { costs } => costs.iter().collect(),
        other => vec![other],
    }
}

/// CR 201.5a: the token's granted CDA reads the granting Saproling Burst.
#[test]
fn saproling_burst_granted_cda_reads_the_granter() {
    let parsed = parse_oracle_text(
        SAPROLING_BURST,
        "Saproling Burst",
        &[],
        &["Enchantment".to_string()],
        &[],
    );
    let activated = parsed
        .abilities
        .iter()
        .find(|a| matches!(a.cost, Some(AbilityCost::RemoveCounter { .. })))
        .expect("the fade-counter ability");
    assert!(matches!(*activated.effect, Effect::Unimplemented { .. }));
    let grant = match activated.sub_ability.as_deref().map(|s| s.effect.as_ref()) {
        Some(Effect::GenericEffect {
            static_abilities, ..
        }) => static_abilities[0]
            .modifications
            .iter()
            .find_map(|m| match m {
                ContinuousModification::GrantStaticAbility { definition } => Some(definition),
                _ => None,
            })
            .expect("GrantStaticAbility"),
        other => panic!("expected a GenericEffect grant, got {other:?}"),
    };
    let fade = QuantityExpr::Ref {
        qty: QuantityRef::CountersOn {
            scope: ObjectScope::GrantingObject,
            counter_type: Some(CounterType::Fade),
        },
    };
    assert_eq!(
        grant.modifications,
        vec![
            ContinuousModification::SetDynamicPower {
                value: fade.clone()
            },
            ContinuousModification::SetDynamicToughness { value: fade },
        ]
    );
}

/// CR 201.5a: "Remove all aim counters from Hankyu" is a cost on Hankyu.
#[test]
fn hankyu_remove_all_cost_names_the_granter() {
    let mods = granted_modifications(HANKYU, "Hankyu", &["Artifact"], &["Equipment"]);
    let defs: Vec<&AbilityDefinition> = mods
        .iter()
        .filter_map(|m| match m {
            ContinuousModification::GrantAbility { definition } => Some(definition.as_ref()),
            _ => None,
        })
        .collect();
    assert_eq!(defs.len(), 2);
    assert!(matches!(
        defs[0].effect.as_ref(),
        Effect::PutCounter {
            target: TargetFilter::GrantingObject,
            ..
        }
    ));
    let remove = cost_parts(defs[1].cost.as_ref().expect("a cost"))
        .into_iter()
        .find_map(|c| match c {
            AbilityCost::RemoveCounter { target, .. } => Some(target.clone()),
            _ => None,
        })
        .expect("a remove-counter cost");
    assert_eq!(remove, Some(TargetFilter::GrantingObject));
}

/// CR 201.5a: the granted "fight Grothama" fights the granting Grothama.
#[test]
fn grothama_granted_fight_names_the_granter() {
    match granted_trigger_effect(
        GROTHAMA,
        "Grothama, All-Devouring",
        &["Creature"],
        &["Wurm"],
    ) {
        Effect::Fight { target, .. } => assert_eq!(target, TargetFilter::GrantingObject),
        other => panic!("expected Fight, got {other:?}"),
    }
}

/// CR 201.5a: "Tap Fishing Pole" is a cost on Fishing Pole.
#[test]
fn fishing_pole_tap_cost_names_the_granter() {
    let def = granted_def_from(FISHING_POLE, "Fishing Pole", &["Artifact"], &["Equipment"]);
    assert!(matches!(
        def.effect.as_ref(),
        Effect::PutCounter {
            target: TargetFilter::GrantingObject,
            ..
        }
    ));
    let tap = cost_parts(def.cost.as_ref().expect("a cost"))
        .into_iter()
        .find_map(|c| match c {
            AbilityCost::EffectCost { effect } => match effect.as_ref() {
                Effect::SetTapState { target, .. } => Some(target.clone()),
                _ => None,
            },
            _ => None,
        })
        .expect("a tap effect cost");
    assert_eq!(tap, TargetFilter::GrantingObject);
}

fn foo_bar_body_condition(body: &str) -> (AbilityDefinition, AbilityCondition) {
    let oracle = format!("Enchant creature\nEnchanted creature has \"{body}\"");
    let def = granted_def_from(&oracle, "Foo Bar", &["Enchantment"], &["Aura"]);
    let sub = def.sub_ability.as_deref().expect("the destroy clause");
    assert!(matches!(
        sub.effect.as_ref(),
        Effect::Destroy {
            target: TargetFilter::GrantingObject,
            ..
        }
    ));
    let condition = sub.condition.clone().expect("the counter gate");
    (def, condition)
}

fn task_gate(scope: ObjectScope) -> AbilityCondition {
    AbilityCondition::QuantityCheck {
        lhs: QuantityExpr::Ref {
            qty: QuantityRef::CountersOn {
                scope,
                counter_type: Some(CounterType::Generic("task".to_string())),
            },
        },
        comparator: Comparator::EQ,
        rhs: QuantityExpr::Fixed { value: 0 },
    }
}

/// CR 608.2c + CR 201.5a: the bare "it" of a leading counter gate reads the
/// granter only when the prior clause names the granter, conditioned or not.
#[test]
fn counter_gate_pronoun_follows_its_antecedent() {
    let (_, host) = foo_bar_body_condition(
        "{T}: Remove a task counter from this creature. Then if it has no task counters on it, destroy Foo Bar.",
    );
    assert_eq!(host, task_gate(ObjectScope::Source));

    let (def, granter) = foo_bar_body_condition(
        "{T}: If you control an artifact, remove a task counter from Foo Bar. Then if it has no task counters on it, destroy Foo Bar.",
    );
    assert!(def.condition.is_some());
    assert_eq!(granter, task_gate(ObjectScope::GrantingObject));
}

/// CR 201.5a: Heliod's Punishment's body removes from, counts and destroys the
/// granting Aura.
#[test]
fn heliods_punishment_parse_reads_the_granter() {
    let def = granted_def_from(
        HELIODS_PUNISHMENT,
        "Heliod's Punishment",
        &["Enchantment"],
        &["Aura"],
    );
    assert!(matches!(
        def.effect.as_ref(),
        Effect::RemoveCounter {
            target: TargetFilter::GrantingObject,
            ..
        }
    ));
    let sub = def.sub_ability.as_deref().expect("the destroy clause");
    assert!(matches!(
        sub.effect.as_ref(),
        Effect::Destroy {
            target: TargetFilter::GrantingObject,
            ..
        }
    ));
    assert_eq!(sub.condition, Some(task_gate(ObjectScope::GrantingObject)));
}

// ---------------------------------------------------------------------------
// CR 201.5a masker positions: an allowlisted position masks the granter name, a
// refused one (`by `) stays `~`.
// ---------------------------------------------------------------------------

/// Assert the masker leaves a refused position unmasked while the name still
/// normalizes to `~`.
fn assert_masker_noop(oracle: &str, name: &str) {
    let normalized = normalize_card_name_refs(oracle, name);
    assert!(
        !normalized.contains(PLACEHOLDER),
        "{name}: a refused self-name position must NOT be masked"
    );
    assert!(
        normalized.contains('~'),
        "{name}: the self-name/self-ref must still normalize to ~ (reach-guard); got {normalized}"
    );
}

const ARCHERY_TRAINING: &str = "Enchant creature\nAt the beginning of your upkeep, you may put an \
arrow counter on this Aura.\nEnchanted creature has \"{T}: This creature deals X damage to target \
attacking or blocking creature, where X is the number of arrow counters on Archery Training.\"";

/// CR 201.5a: Archery Training's "number of arrow counters on <self>" reads the
/// granter, in both the typed and the display channel.
#[test]
fn archery_training_quantity_ref_channel_binds_the_granter() {
    assert!(normalize_card_name_refs(ARCHERY_TRAINING, "Archery Training").contains(PLACEHOLDER));
    let def = granted_def_from(
        ARCHERY_TRAINING,
        "Archery Training",
        &["Enchantment"],
        &["Aura"],
    );
    match def.effect.as_ref() {
        Effect::DealDamage { amount, .. } => assert_eq!(
            *amount,
            QuantityExpr::Ref {
                qty: QuantityRef::CountersOn {
                    scope: ObjectScope::GrantingObject,
                    counter_type: Some(CounterType::Generic("arrow".to_string())),
                },
            }
        ),
        other => panic!("expected DealDamage, got {other:?}"),
    }
    let desc = def
        .description
        .as_deref()
        .expect("the granted Archery Training ability carries a display description");
    assert!(
        desc.contains("arrow counters on Archery Training"),
        "{desc}"
    );
}

const ANIMAL_FRIEND: &str = "Enchant creature\nEnchanted creature has \"Whenever this creature \
attacks, create a 1/1 green Squirrel creature token. Put a +1/+1 counter on that token for each \
Aura and Equipment attached to this creature other than Animal Friend.\"";

/// CR 201.5a: Animal Friend's "other than <self>" and Shifting Shadow's
/// "attach <self>" are masked, and no placeholder survives their dropped clauses.
#[test]
fn animal_friend_exclusion_channel_masks_without_leaking() {
    for (oracle, name) in [
        (ANIMAL_FRIEND, "Animal Friend"),
        (SHIFTING_SHADOW, "Shifting Shadow"),
    ] {
        assert!(
            normalize_card_name_refs(oracle, name).contains(PLACEHOLDER),
            "{name}"
        );
        let parsed = parse_oracle_text(
            oracle,
            name,
            &[],
            &["Enchantment".to_string()],
            &["Aura".to_string()],
        );
        assert!(
            parsed
                .statics
                .iter()
                .flat_map(|s| s.modifications.iter())
                .any(|m| matches!(m, ContinuousModification::GrantTrigger { .. })),
            "{name}"
        );
        assert!(
            !serde_json::to_string(&parsed)
                .expect("ParsedAbilities serializes")
                .contains(PLACEHOLDER),
            "{name}"
        );
    }
}

const TORRENT_OF_LAVA: &str = "Torrent of Lava deals X damage to each creature without flying.\n\
As long as Torrent of Lava is on the stack, each creature has \"{T}: Prevent the next 1 damage \
that would be dealt to this creature by Torrent of Lava this turn.\"";

/// Torrent of Lava — damage-source channel ("dealt … by <self>"). Revert-to-red:
/// re-widen the masker → `by <placeholder>` in the normalized string → red.
#[test]
fn torrent_of_lava_damage_source_channel_not_masked() {
    assert_masker_noop(TORRENT_OF_LAVA, "Torrent of Lava");
}

/// CR 201.5a + CR 400.7 + CR 608.2h: `ObjectScope::SpecificObject` reads one exact
/// object incarnation; the unbound `ObjectScope::GrantingObject` reads as `Source` in
/// counter reads.
mod object_scope_reads {
    use engine::game::layers::evaluate_layers;
    use engine::game::quantity::{resolve_quantity, resolve_quantity_with_targets};
    use engine::game::scenario::{GameScenario, P0};
    use engine::game::zones::move_to_zone;
    use engine::types::ability::{
        ContinuousModification, Effect, ObjectScope, QuantityExpr, QuantityRef, ResolvedAbility,
        StaticDefinition, TargetFilter,
    };
    use engine::types::counter::CounterType;
    use engine::types::game_state::GameState;
    use engine::types::identifiers::{ObjectId, ObjectIncarnationRef};
    use engine::types::mana::{ManaColor, ManaCost, ManaCostShard};
    use engine::types::zones::Zone;

    fn slime() -> CounterType {
        CounterType::Generic("slime".to_string())
    }

    fn qty(qty: QuantityRef) -> QuantityExpr {
        QuantityExpr::Ref { qty }
    }

    fn counters(scope: ObjectScope) -> QuantityExpr {
        qty(QuantityRef::CountersOn {
            scope,
            counter_type: Some(slime()),
        })
    }

    fn bound(object: ObjectIncarnationRef) -> ObjectScope {
        ObjectScope::SpecificObject { object }
    }

    /// Host 1/1 {1}{W} white with 1 slime; granter 2/2 {2}{G}{U} green-blue with 3
    /// slime. With `bind_cda`, the host's P/T is a CDA counting slime on the granter.
    fn setup(bind_cda: bool) -> (GameState, ObjectId, ObjectId) {
        let mut scenario = GameScenario::new();
        let granter = {
            let mut b = scenario.add_creature(P0, "Granter", 2, 2);
            b.with_mana_cost(ManaCost::Cost {
                shards: vec![ManaCostShard::Green, ManaCostShard::Blue],
                generic: 2,
            })
            .with_color(vec![ManaColor::Green, ManaColor::Blue]);
            b.id()
        };
        let host = {
            let mut b = scenario.add_creature(P0, "Host", 1, 1);
            b.with_mana_cost(ManaCost::Cost {
                shards: vec![ManaCostShard::White],
                generic: 1,
            })
            .with_color(vec![ManaColor::White]);
            b.id()
        };
        scenario.with_counter(host, slime(), 1);
        scenario.with_counter(granter, slime(), 3);
        let mut state = scenario.build().state().clone();
        if bind_cda {
            let value = counters(bound(ObjectIncarnationRef::from_object(
                &state.objects[&granter],
            )));
            let def = StaticDefinition::continuous()
                .affected(TargetFilter::SelfRef)
                .cda()
                .modifications(vec![
                    ContinuousModification::SetDynamicPower {
                        value: value.clone(),
                    },
                    ContinuousModification::SetDynamicToughness { value },
                ]);
            let obj = state.objects.get_mut(&host).unwrap();
            obj.static_definitions.push(def.clone());
            std::sync::Arc::make_mut(&mut obj.base_static_definitions).push(def);
        }
        recompute(&mut state);
        (state, host, granter)
    }

    fn recompute(state: &mut GameState) {
        state.layers_dirty.mark_full();
        evaluate_layers(state);
    }

    fn resolving(host: ObjectId) -> ResolvedAbility {
        ResolvedAbility::new(
            Effect::unimplemented("granted", "granted ability read"),
            vec![],
            host,
            P0,
        )
    }

    fn host_pt(state: &GameState, host: ObjectId) -> (Option<i32>, Option<i32>) {
        let obj = &state.objects[&host];
        (obj.power, obj.toughness)
    }

    fn current(state: &GameState, id: ObjectId) -> ObjectScope {
        bound(ObjectIncarnationRef::from_object(&state.objects[&id]))
    }

    #[test]
    fn bound_object_scope_cda_reads_the_granter_not_the_host() {
        let (state, host, granter) = setup(true);
        let g0 = current(&state, granter);

        assert_eq!(host_pt(&state, host), (Some(3), Some(3)));
        let read = |q| resolve_quantity(&state, &qty(q), P0, host);
        assert_eq!(read(QuantityRef::Power { scope: g0 }), 2);
        assert_eq!(read(QuantityRef::ObjectManaValue { scope: g0 }), 4);
        assert_eq!(read(QuantityRef::ObjectColorCount { scope: g0 }), 2);
        assert_eq!(
            read(QuantityRef::ManaSymbolsInManaCost {
                scope: g0,
                color: None
            }),
            2
        );
    }

    #[test]
    fn bound_object_scope_departed_reads_zero_statically_lki_when_resolving() {
        let (mut state, host, granter) = setup(true);
        let g0 = current(&state, granter);
        move_to_zone(&mut state, granter, Zone::Graveyard, &mut Vec::new());
        recompute(&mut state);

        assert_eq!(host_pt(&state, host), (Some(0), Some(0)));
        assert_eq!(resolve_quantity(&state, &counters(g0), P0, host), 0);

        let ability = resolving(host);
        assert_eq!(
            resolve_quantity_with_targets(&state, &counters(g0), &ability),
            3
        );
        assert_eq!(
            resolve_quantity_with_targets(&state, &qty(QuantityRef::Power { scope: g0 }), &ability),
            2
        );

        let g_gy = current(&state, granter);
        assert_eq!(
            resolve_quantity(&state, &qty(QuantityRef::Power { scope: g_gy }), P0, host),
            2
        );
    }

    #[test]
    fn bound_object_scope_blinked_granter_is_a_new_object() {
        let (mut state, host, granter) = setup(true);
        let g0 = current(&state, granter);
        move_to_zone(&mut state, granter, Zone::Exile, &mut Vec::new());
        move_to_zone(&mut state, granter, Zone::Battlefield, &mut Vec::new());
        state
            .objects
            .get_mut(&granter)
            .unwrap()
            .counters
            .insert(slime(), 5);
        recompute(&mut state);
        let returned = current(&state, granter);
        let color_count = |scope| qty(QuantityRef::ObjectColorCount { scope });

        assert_eq!(resolve_quantity(&state, &counters(returned), P0, host), 5);
        assert_eq!(
            resolve_quantity(&state, &color_count(returned), P0, host),
            2
        );

        assert_eq!(host_pt(&state, host), (Some(0), Some(0)));
        assert_eq!(resolve_quantity(&state, &counters(g0), P0, host), 0);
        assert_eq!(
            resolve_quantity_with_targets(&state, &counters(g0), &resolving(host)),
            3
        );
        assert_eq!(resolve_quantity(&state, &color_count(g0), P0, host), 0);
        let pips = qty(QuantityRef::ManaSymbolsInManaCost {
            scope: g0,
            color: None,
        });
        assert_eq!(resolve_quantity(&state, &pips, P0, host), 0);
    }

    #[test]
    fn unbound_granting_object_counters_read_the_source() {
        let (mut state, host, _granter) = setup(false);
        let unbound = counters(ObjectScope::GrantingObject);

        assert_eq!(resolve_quantity(&state, &unbound, P0, host), 1);
        assert_eq!(
            resolve_quantity_with_targets(&state, &unbound, &resolving(host)),
            1
        );

        move_to_zone(&mut state, host, Zone::Graveyard, &mut Vec::new());
        assert_eq!(
            resolve_quantity_with_targets(&state, &unbound, &resolving(host)),
            1
        );
    }

    #[test]
    fn object_scope_granter_values_round_trip() {
        let g0 = bound(ObjectIncarnationRef::of(ObjectId(7), 2));
        let json = serde_json::to_string(&g0).unwrap();
        assert!(json.contains("\"SpecificObject\""), "{json}");
        assert_eq!(serde_json::from_str::<ObjectScope>(&json).unwrap(), g0);

        let json = serde_json::to_string(&ObjectScope::GrantingObject).unwrap();
        assert_eq!(
            serde_json::from_str::<ObjectScope>(&json).unwrap(),
            ObjectScope::GrantingObject
        );
    }
}

// ---------------------------------------------------------------------------
// CR 201.5a concretizer seams: every channel through which a granted body names
// its granter, bound at each attachment seam (Layer-6 grants, token creation).
// Card fixtures use verbatim Oracle text.
// ---------------------------------------------------------------------------

mod concretizer_seams {
    use std::sync::Arc;

    use engine::game::combat::AttackTarget;
    use engine::game::effects::attach::attach_to;
    use engine::game::effects::resolve_ability_chain;
    use engine::game::filter::{matches_target_filter, FilterContext};
    use engine::game::game_object::AttachTarget;
    use engine::game::layers::evaluate_layers;
    use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
    use engine::game::zones::move_to_zone;
    use engine::parser::oracle::parse_oracle_text;
    use engine::types::ability::{
        AbilityCondition, AbilityCost, AbilityDefinition, AbilityKind, Comparator,
        ContinuousModification, Effect, EffectKind, FilterProp, ObjectScope, PtValue, QuantityExpr,
        QuantityRef, ResolvedAbility, StaticDefinition, TargetFilter, TargetRef, TypeFilter,
        TypedFilter,
    };
    use engine::types::actions::GameAction;
    use engine::types::card_type::CoreType;
    use engine::types::counter::CounterType;
    use engine::types::events::GameEvent;
    use engine::types::game_state::{GameState, WaitingFor};
    use engine::types::identifiers::{ObjectId, ObjectIncarnationRef};
    use engine::types::keywords::Keyword;
    use engine::types::mana::{ManaCost, ManaType, ManaUnit};
    use engine::types::phase::Phase;
    use engine::types::zones::Zone;

    use super::{
        ARCHERY_TRAINING, DIRE_BLUNDERBUSS, GROTHAMA, GUTTER_GRIME, HANKYU, HELIODS_PUNISHMENT,
        NETTLEVINE_BLIGHT, SPARE_DAGGER, THE_AETHERSPARK, TRUSTY_BOOMERANG,
    };

    fn counter(kind: &str) -> CounterType {
        CounterType::Generic(kind.to_string())
    }

    fn counters_on(scope: ObjectScope, kind: &str) -> QuantityExpr {
        QuantityExpr::Ref {
            qty: QuantityRef::CountersOn {
                scope,
                counter_type: Some(counter(kind)),
            },
        }
    }

    fn incarnation(state: &GameState, id: ObjectId) -> ObjectIncarnationRef {
        ObjectIncarnationRef::from_object(&state.objects[&id])
    }

    fn bound(state: &GameState, id: ObjectId) -> ObjectScope {
        ObjectScope::SpecificObject {
            object: incarnation(state, id),
        }
    }

    fn grant_static(oracle: &str, name: &str, core: &str, subtype: &str) -> StaticDefinition {
        parse_oracle_text(
            oracle,
            name,
            &[],
            &[core.to_string()],
            &[subtype.to_string()],
        )
        .statics
        .into_iter()
        .find(|s| {
            s.modifications.iter().any(|m| {
                matches!(
                    m,
                    ContinuousModification::GrantAbility { .. }
                        | ContinuousModification::GrantTrigger { .. }
                )
            })
        })
        .expect("a static granting an ability or trigger")
    }

    fn granted_ability(grant: &mut StaticDefinition) -> &mut AbilityDefinition {
        grant
            .modifications
            .iter_mut()
            .find_map(|m| match m {
                ContinuousModification::GrantAbility { definition } => Some(definition.as_mut()),
                _ => None,
            })
            .expect("GrantAbility")
    }

    fn granted_execute(grant: &mut StaticDefinition) -> &mut AbilityDefinition {
        grant
            .modifications
            .iter_mut()
            .find_map(|m| match m {
                ContinuousModification::GrantTrigger { trigger } => trigger.execute.as_deref_mut(),
                _ => None,
            })
            .expect("GrantTrigger execute")
    }

    fn relayer(state: &mut GameState) {
        state.layers_dirty.mark_full();
        evaluate_layers(state);
    }

    /// Makes `granter` a `core` `subtype` attached to `host` that carries `grant`.
    fn attach(
        runner: &mut GameRunner,
        granter: ObjectId,
        host: ObjectId,
        core: CoreType,
        subtype: &str,
        grant: StaticDefinition,
    ) {
        let st = runner.state_mut();
        let obj = st.objects.get_mut(&granter).unwrap();
        obj.card_types.core_types = vec![core];
        obj.card_types.subtypes = vec![subtype.to_string()];
        obj.base_card_types = obj.card_types.clone();
        obj.power = None;
        obj.toughness = None;
        obj.base_power = None;
        obj.base_toughness = None;
        obj.attached_to = Some(AttachTarget::Object(host));
        obj.static_definitions.push(grant.clone());
        Arc::make_mut(&mut obj.base_static_definitions).push(grant);
        relayer(st);
    }

    fn make_artifact(state: &mut GameState, id: ObjectId) {
        let obj = state.objects.get_mut(&id).unwrap();
        obj.card_types.core_types = vec![CoreType::Artifact];
        obj.base_card_types = obj.card_types.clone();
        obj.power = None;
        obj.toughness = None;
        obj.base_power = None;
        obj.base_toughness = None;
    }

    fn pass_to_declare_attackers(runner: &mut GameRunner) {
        for _ in 0..8 {
            if matches!(
                runner.state().waiting_for,
                WaitingFor::DeclareAttackers { .. }
            ) {
                return;
            }
            runner.act(GameAction::PassPriority).unwrap();
        }
        panic!(
            "never reached DeclareAttackers: {:?}",
            runner.state().waiting_for
        );
    }

    fn attack_with(runner: &mut GameRunner, attacker: ObjectId) {
        pass_to_declare_attackers(runner);
        runner
            .declare_attackers(&[(attacker, AttackTarget::Player(P1))])
            .unwrap();
    }

    /// Drives priority, "you may" prompts and trigger ordering until the engine asks
    /// for anything else, answering a trigger target with `target`.
    fn drive(runner: &mut GameRunner, target: Option<TargetRef>) {
        for _ in 0..40 {
            let action = match &runner.state().waiting_for {
                WaitingFor::Priority { .. } if !runner.state().stack.is_empty() => {
                    GameAction::PassPriority
                }
                WaitingFor::OptionalEffectChoice { .. } => {
                    GameAction::DecideOptionalEffect { accept: true }
                }
                WaitingFor::OrderTriggers { triggers, .. } => GameAction::OrderTriggers {
                    order: (0..triggers.len()).collect(),
                },
                WaitingFor::TriggerTargetSelection { .. } if target.is_some() => {
                    GameAction::ChooseTarget {
                        target: target.clone(),
                    }
                }
                _ => return,
            };
            runner.act(action).unwrap();
        }
        panic!("drive did not settle: {:?}", runner.state().waiting_for);
    }

    fn activate(runner: &mut GameRunner, host: ObjectId, index: usize, target: Option<ObjectId>) {
        runner
            .act(GameAction::ActivateAbility {
                source_id: host,
                ability_index: index,
            })
            .unwrap();
        if matches!(
            runner.state().waiting_for,
            WaitingFor::TargetSelection { .. }
        ) {
            runner
                .act(GameAction::SelectTargets {
                    targets: target.into_iter().map(TargetRef::Object).collect(),
                })
                .unwrap();
        }
        if let Some(t) = target {
            let top = runner.state().stack.last().and_then(|e| e.ability());
            assert_eq!(
                top.map(|a| a.targets.clone()),
                Some(vec![TargetRef::Object(t)])
            );
        }
    }

    /// Resolves the whole stack, returning every event it produced.
    fn resolve_stack(runner: &mut GameRunner) -> Vec<GameEvent> {
        let mut events = Vec::new();
        for _ in 0..40 {
            if runner.state().stack.is_empty() {
                return events;
            }
            events.extend(runner.act(GameAction::PassPriority).unwrap().events);
        }
        panic!("stack never emptied: {:?}", runner.state().waiting_for);
    }

    /// Host carries 1 arrow counter; each Archery Training carries `arrows[i]`.
    fn archery_training(arrows: &[u32]) -> (GameRunner, ObjectId, Vec<ObjectId>, ObjectId) {
        let mut grant = grant_static(ARCHERY_TRAINING, "Archery Training", "Enchantment", "Aura");
        match granted_ability(&mut grant).effect.as_ref() {
            Effect::DealDamage { amount, .. } => {
                assert_eq!(*amount, counters_on(ObjectScope::GrantingObject, "arrow"))
            }
            other => panic!("expected DealDamage, got {other:?}"),
        }
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let raider = scenario.add_creature(P0, "Raider", 1, 20).id();
        scenario.with_counter(host, counter("arrow"), 1);
        let trainings: Vec<ObjectId> = arrows
            .iter()
            .map(|&n| {
                let id = scenario.add_creature(P0, "Archery Training", 0, 0).id();
                scenario.with_counter(id, counter("arrow"), n);
                id
            })
            .collect();
        let mut runner = scenario.build();
        for &at in &trainings {
            attach(
                &mut runner,
                at,
                host,
                CoreType::Enchantment,
                "Aura",
                grant.clone(),
            );
        }
        attack_with(&mut runner, raider);
        drive(&mut runner, None);
        (runner, host, trainings, raider)
    }

    fn granted_damage_indices(runner: &GameRunner, host: ObjectId) -> Vec<usize> {
        runner.state().objects[&host]
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, a)| matches!(*a.effect, Effect::DealDamage { .. }))
            .map(|(i, _)| i)
            .collect()
    }

    #[test]
    fn archery_training_damage_reads_the_granter_counters() {
        let (mut runner, host, trainings, raider) = archery_training(&[2]);
        let idx = granted_damage_indices(&runner, host);
        assert_eq!(idx.len(), 1);
        match runner.state().objects[&host].abilities[idx[0]]
            .effect
            .as_ref()
        {
            Effect::DealDamage { amount, .. } => assert_eq!(
                *amount,
                counters_on(bound(runner.state(), trainings[0]), "arrow")
            ),
            other => panic!("{other:?}"),
        }
        assert_ne!(trainings[0], host);
        activate(&mut runner, host, idx[0], Some(raider));
        runner.advance_until_stack_empty();
        assert_eq!(runner.state().objects[&raider].damage_marked, 2);
    }

    #[test]
    fn archery_training_two_granters_each_read_their_own() {
        let (mut runner, host, _trainings, raider) = archery_training(&[3, 2]);
        let idx = granted_damage_indices(&runner, host);
        assert_eq!(idx.len(), 2, "one granted ability per granter");
        let mut dealt = Vec::new();
        for i in idx {
            let before = runner.state().objects[&raider].damage_marked;
            runner.state_mut().objects.get_mut(&host).unwrap().tapped = false;
            activate(&mut runner, host, i, Some(raider));
            runner.advance_until_stack_empty();
            dealt.push(runner.state().objects[&raider].damage_marked - before);
        }
        dealt.sort_unstable();
        assert_eq!(dealt, vec![2, 3]);
    }

    #[test]
    fn archery_training_removed_in_response_uses_last_known_counters() {
        let (mut runner, host, trainings, raider) = archery_training(&[2]);
        let idx = granted_damage_indices(&runner, host);
        activate(&mut runner, host, idx[0], Some(raider));
        assert_eq!(runner.state().stack.len(), 1);
        move_to_zone(
            runner.state_mut(),
            trainings[0],
            Zone::Graveyard,
            &mut Vec::new(),
        );
        runner.advance_until_stack_empty();
        assert_eq!(runner.state().objects[&raider].damage_marked, 2);
    }

    fn add_gutter_grime(scenario: &mut GameScenario, slime: u32) -> ObjectId {
        let id = scenario
            .add_enchantment_from_oracle(P0, "Gutter Grime", GUTTER_GRIME)
            .id();
        scenario.with_counter(id, counter("slime"), slime);
        id
    }

    #[test]
    fn gutter_grime_parse_reads_the_granter() {
        let trigger = parse_oracle_text(
            GUTTER_GRIME,
            "Gutter Grime",
            &[],
            &["Enchantment".to_string()],
            &[],
        )
        .triggers
        .remove(0);
        let slime = counters_on(ObjectScope::GrantingObject, "slime");
        match trigger
            .execute
            .as_ref()
            .and_then(|e| e.sub_ability.as_ref())
            .map(|s| s.effect.as_ref())
        {
            Some(Effect::Token {
                power,
                toughness,
                static_abilities,
                ..
            }) => {
                assert_eq!(*power, PtValue::Quantity(slime.clone()));
                assert_eq!(*toughness, PtValue::Quantity(slime.clone()));
                assert_eq!(
                    static_abilities[0].modifications,
                    vec![
                        ContinuousModification::SetDynamicPower {
                            value: slime.clone()
                        },
                        ContinuousModification::SetDynamicToughness { value: slime },
                    ]
                );
            }
            other => panic!("expected Token, got {other:?}"),
        }
    }

    fn oozes(state: &GameState) -> Vec<ObjectId> {
        state
            .battlefield
            .iter()
            .copied()
            .filter(|id| state.objects[id].name == "Ooze")
            .collect()
    }

    /// Kills a nontoken creature and stops with the Gutter Grime trigger(s) on the stack.
    fn kill_victim_to_triggers(
        scenario: GameScenario,
        victim: ObjectId,
        bolt: ObjectId,
    ) -> GameRunner {
        let mut runner = scenario.build();
        {
            let _cast = runner.cast(bolt).free_cast().target_object(victim).commit();
        }
        for _ in 0..8 {
            if matches!(runner.state().waiting_for, WaitingFor::OrderTriggers { .. }) {
                drive(&mut runner, None);
            }
            if runner.state().objects[&victim].zone == Zone::Graveyard
                && !runner.state().stack.is_empty()
            {
                return runner;
            }
            runner.act(GameAction::PassPriority).unwrap();
        }
        panic!(
            "trigger never reached the stack: {:?}",
            runner.state().waiting_for
        );
    }

    #[test]
    fn gutter_grime_token_survives_at_the_granters_count() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let gg = add_gutter_grime(&mut scenario, 2);
        let victim = scenario.add_creature(P0, "Victim", 1, 1).id();
        let bolt = scenario.add_bolt_to_hand(P0);
        let mut runner = kill_victim_to_triggers(scenario, victim, bolt);
        runner.advance_until_stack_empty();
        let tokens = oozes(runner.state());
        assert_eq!(tokens.len(), 1);
        let token = &runner.state().objects[&tokens[0]];
        assert_eq!((token.power, token.toughness), (Some(3), Some(3)));
        assert_ne!(tokens[0], gg);
    }

    #[test]
    fn gutter_grime_two_granters_each_latch_their_own() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        add_gutter_grime(&mut scenario, 2);
        add_gutter_grime(&mut scenario, 5);
        let victim = scenario.add_creature(P0, "Victim", 1, 1).id();
        let bolt = scenario.add_bolt_to_hand(P0);
        let mut runner = kill_victim_to_triggers(scenario, victim, bolt);
        runner.advance_until_stack_empty();
        let mut sizes: Vec<_> = oozes(runner.state())
            .iter()
            .map(|id| runner.state().objects[id].power)
            .collect();
        sizes.sort();
        assert_eq!(sizes, vec![Some(3), Some(6)]);
    }

    #[test]
    fn gutter_grime_blinked_before_resolution_latches_the_old_object() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let gg = add_gutter_grime(&mut scenario, 2);
        let victim = scenario.add_creature(P0, "Victim", 1, 1).id();
        let bolt = scenario.add_bolt_to_hand(P0);
        let mut runner = kill_victim_to_triggers(scenario, victim, bolt);
        {
            let st = runner.state_mut();
            move_to_zone(st, gg, Zone::Exile, &mut Vec::new());
            move_to_zone(st, gg, Zone::Battlefield, &mut Vec::new());
            st.objects
                .get_mut(&gg)
                .unwrap()
                .counters
                .insert(counter("slime"), 4);
        }
        let events = resolve_stack(&mut runner);
        let token = events
            .iter()
            .find_map(|e| match e {
                GameEvent::TokenCreated {
                    object_id, name, ..
                } if name == "Ooze" => Some(*object_id),
                _ => None,
            })
            .expect("the trigger created the token");
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::ZoneChanged { object_id, from: Some(Zone::Battlefield), .. } if *object_id == token
        )));
        assert!(oozes(runner.state()).is_empty());
    }

    #[test]
    fn gutter_grime_ooze_stays_at_its_granters_count() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let gg = add_gutter_grime(&mut scenario, 2);
        let victim = scenario.add_creature(P0, "Victim", 1, 1).id();
        let bolt = scenario.add_bolt_to_hand(P0);
        let mut runner = kill_victim_to_triggers(scenario, victim, bolt);
        runner.advance_until_stack_empty();
        let tokens = oozes(runner.state());
        assert_eq!(tokens.len(), 1);
        assert_ne!(tokens[0], gg);
        let token = &runner.state().objects[&tokens[0]];
        assert_eq!((token.power, token.toughness), (Some(3), Some(3)));
        runner
            .state_mut()
            .objects
            .get_mut(&gg)
            .unwrap()
            .counters
            .insert(counter("slime"), 4);
        relayer(runner.state_mut());
        let token = &runner.state().objects[&tokens[0]];
        assert_eq!((token.power, token.toughness), (Some(4), Some(4)));
    }

    #[test]
    fn gutter_grime_existing_token_drops_to_zero_power_when_its_creator_is_blinked() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let gg = add_gutter_grime(&mut scenario, 2);
        let victim = scenario.add_creature(P0, "Victim", 1, 1).id();
        let bolt = scenario.add_bolt_to_hand(P0);
        let mut runner = kill_victim_to_triggers(scenario, victim, bolt);
        runner.advance_until_stack_empty();
        let token = oozes(runner.state())[0];
        assert_eq!(runner.state().objects[&token].power, Some(3));
        {
            let st = runner.state_mut();
            move_to_zone(st, gg, Zone::Exile, &mut Vec::new());
            move_to_zone(st, gg, Zone::Battlefield, &mut Vec::new());
            st.objects
                .get_mut(&gg)
                .unwrap()
                .counters
                .insert(counter("slime"), 4);
        }
        relayer(runner.state_mut());
        assert_eq!(runner.state().objects[&token].power, Some(0));
    }

    #[test]
    fn gutter_grime_leaving_makes_its_token_zero() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let gg = add_gutter_grime(&mut scenario, 2);
        let victim = scenario.add_creature(P0, "Victim", 1, 1).id();
        let bolt = scenario.add_bolt_to_hand(P0);
        let mut runner = kill_victim_to_triggers(scenario, victim, bolt);
        runner.advance_until_stack_empty();
        let token = oozes(runner.state())[0];
        assert_eq!(runner.state().objects[&token].power, Some(3));
        move_to_zone(runner.state_mut(), gg, Zone::Graveyard, &mut Vec::new());
        relayer(runner.state_mut());
        assert_eq!(runner.state().objects[&token].power, Some(0));
    }

    fn charge_power_grant() -> StaticDefinition {
        let inner = StaticDefinition::continuous()
            .affected(TargetFilter::SelfRef)
            .modifications(vec![ContinuousModification::SetDynamicPower {
                value: counters_on(ObjectScope::GrantingObject, "charge"),
            }]);
        StaticDefinition::continuous()
            .affected(TargetFilter::Typed(
                TypedFilter::creature().properties(vec![FilterProp::EquippedBy]),
            ))
            .modifications(vec![ContinuousModification::GrantStaticAbility {
                definition: Box::new(inner),
            }])
    }

    #[test]
    fn granted_static_reads_its_granters_counters() {
        let mut scenario = GameScenario::new();
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let other_host = scenario.add_creature(P0, "Other Bearer", 2, 2).id();
        let granter = scenario.add_creature(P0, "Charger", 0, 0).id();
        let other = scenario.add_creature(P0, "Other Charger", 0, 0).id();
        scenario.with_counter(host, counter("charge"), 1);
        scenario.with_counter(granter, counter("charge"), 3);
        scenario.with_counter(other, counter("charge"), 5);
        let mut runner = scenario.build();
        attach(
            &mut runner,
            granter,
            host,
            CoreType::Artifact,
            "Equipment",
            charge_power_grant(),
        );
        attach(
            &mut runner,
            other,
            other_host,
            CoreType::Artifact,
            "Equipment",
            charge_power_grant(),
        );
        let st = runner.state();
        assert_eq!(st.objects[&host].power, Some(3));
        assert_eq!(st.objects[&other_host].power, Some(5));
        let installed = st.objects[&host]
            .static_definitions
            .as_slice()
            .iter()
            .flat_map(|s| s.modifications.iter())
            .find_map(|m| match m {
                ContinuousModification::SetDynamicPower { value } => Some(value.clone()),
                _ => None,
            })
            .expect("the granted static is installed on the host");
        assert_eq!(installed, counters_on(bound(st, granter), "charge"));
    }

    fn blunderbuss_grant() -> StaticDefinition {
        let mut grant = grant_static(
            DIRE_BLUNDERBUSS,
            "Dire Blunderbuss",
            "Artifact",
            "Equipment",
        );
        match granted_execute(&mut grant).effect.as_ref() {
            Effect::Sacrifice {
                target: TargetFilter::Typed(typed),
                ..
            } => {
                assert!(typed.properties.contains(&FilterProp::DistinctFrom {
                    reference: Box::new(TargetFilter::GrantingObject),
                }));
                assert!(!typed.properties.contains(&FilterProp::Another));
            }
            other => panic!("expected a typed Sacrifice, got {other:?}"),
        }
        grant
    }

    #[test]
    fn dire_blunderbuss_cannot_sacrifice_itself() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let db = scenario.add_creature(P0, "Dire Blunderbuss", 0, 0).id();
        let relic = scenario.add_creature(P0, "Relic", 0, 0).id();
        let idol = scenario.add_creature(P0, "Idol", 0, 0).id();
        let mut runner = scenario.build();
        make_artifact(runner.state_mut(), relic);
        make_artifact(runner.state_mut(), idol);
        attach(
            &mut runner,
            db,
            host,
            CoreType::Artifact,
            "Equipment",
            blunderbuss_grant(),
        );
        attack_with(&mut runner, host);
        drive(&mut runner, None);
        let choices = match &runner.state().waiting_for {
            WaitingFor::EffectZoneChoice {
                cards,
                effect_kind: EffectKind::Sacrifice,
                ..
            } => cards.clone(),
            other => panic!("expected a sacrifice choice, got {other:?}"),
        };
        assert!(
            choices.contains(&relic) && choices.contains(&idol),
            "{choices:?}"
        );
        assert!(!choices.contains(&db), "{choices:?}");
    }

    #[test]
    fn distinct_from_a_bound_granter_excludes_it_across_trigger_batches() {
        let mut scenario = GameScenario::new();
        let host = scenario.add_creature(P0, "Host", 2, 2).id();
        let granter = scenario.add_creature(P0, "Granter", 0, 0).id();
        let other = scenario.add_creature(P0, "Other", 0, 0).id();
        let mut runner = scenario.build();
        make_artifact(runner.state_mut(), granter);
        make_artifact(runner.state_mut(), other);
        let st = runner.state_mut();
        st.current_trigger_events = vec![
            GameEvent::PermanentTapped {
                object_id: host,
                caused_by: None,
            },
            GameEvent::PermanentTapped {
                object_id: other,
                caused_by: None,
            },
        ];
        let st = runner.state();
        let specific = Box::new(TargetFilter::SpecificObject { id: granter });
        let distinct =
            TargetFilter::Typed(TypedFilter::new(TypeFilter::Artifact).properties(vec![
                FilterProp::DistinctFrom {
                    reference: specific.clone(),
                },
            ]));
        let control = TargetFilter::And {
            filters: vec![
                TargetFilter::Typed(TypedFilter::new(TypeFilter::Artifact)),
                TargetFilter::Not { filter: specific },
            ],
        };
        let ctx = FilterContext::from_source(st, host);
        for filter in [&distinct, &control] {
            assert!(!matches_target_filter(st, granter, filter, &ctx));
            assert!(matches_target_filter(st, other, filter, &ctx));
        }
    }

    #[test]
    fn nettlevine_blight_attach_moves_the_granter() {
        let mut grant = grant_static(
            NETTLEVINE_BLIGHT,
            "Nettlevine Blight",
            "Enchantment",
            "Aura",
        );
        match granted_execute(&mut grant)
            .sub_ability
            .as_mut()
            .map(|s| s.effect.as_mut())
        {
            Some(Effect::Attach { attachment, .. }) => {
                assert_eq!(*attachment, TargetFilter::GrantingObject);
            }
            other => panic!("expected Attach, got {other:?}"),
        }
        let mut scenario = GameScenario::new();
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let blight = scenario.add_creature(P0, "Nettlevine Blight", 0, 0).id();
        let mut runner = scenario.build();
        attach(
            &mut runner,
            blight,
            host,
            CoreType::Enchantment,
            "Aura",
            grant,
        );
        let attachment = runner.state().objects[&host]
            .trigger_definitions
            .as_slice()
            .iter()
            .find_map(|t| {
                match t
                    .definition
                    .execute
                    .as_ref()?
                    .sub_ability
                    .as_ref()?
                    .effect
                    .as_ref()
                {
                    Effect::Attach { attachment, .. } => Some(attachment.clone()),
                    _ => None,
                }
            })
            .expect("the granted trigger is on the host");
        assert_eq!(attachment, TargetFilter::SpecificObject { id: blight });
        assert_ne!(blight, host);
    }

    fn heliods_punishment_grant() -> StaticDefinition {
        grant_static(
            HELIODS_PUNISHMENT,
            "Heliod's Punishment",
            "Enchantment",
            "Aura",
        )
    }

    #[test]
    fn heliods_punishment_counts_and_destroys_the_granter() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let hp = scenario.add_creature(P0, "Heliod's Punishment", 0, 0).id();
        scenario.with_counter(hp, counter("task"), 2);
        scenario.with_counter(host, counter("task"), 1);
        let mut runner = scenario.build();
        attach(
            &mut runner,
            hp,
            host,
            CoreType::Enchantment,
            "Aura",
            heliods_punishment_grant(),
        );
        let index = runner.state().objects[&host].abilities.len() - 1;

        activate(&mut runner, host, index, None);
        runner.advance_until_stack_empty();
        let st = runner.state();
        assert_eq!(st.objects[&hp].counters.get(&counter("task")), Some(&1));
        assert_eq!(st.objects[&host].counters.get(&counter("task")), Some(&1));
        assert_eq!(st.objects[&hp].zone, Zone::Battlefield);

        runner.state_mut().objects.get_mut(&host).unwrap().tapped = false;
        activate(&mut runner, host, index, None);
        runner.advance_until_stack_empty();
        assert_eq!(runner.state().objects[&hp].zone, Zone::Graveyard);
        assert_eq!(runner.state().objects[&host].zone, Zone::Battlefield);
    }

    #[test]
    fn bound_condition_reads_the_granter_not_the_host() {
        for (granter_n, host_n, bound_runs, source_runs) in
            [(0, 2, true, false), (1, 0, false, true)]
        {
            let mut scenario = GameScenario::new();
            let host = scenario.add_creature(P0, "Host", 2, 2).id();
            let granter = scenario.add_creature(P0, "Granter", 0, 3).id();
            scenario.with_counter(host, counter("task"), host_n);
            scenario.with_counter(granter, counter("task"), granter_n);
            let mut runner = scenario.build();
            let granter_scope = bound(runner.state(), granter);
            for (scope, runs) in [
                (granter_scope, bound_runs),
                (ObjectScope::Source, source_runs),
            ] {
                let mut st = runner.state_mut().clone();
                let life = st.players[0].life;
                let gain = |n| Effect::GainLife {
                    amount: QuantityExpr::Fixed { value: n },
                    player: TargetFilter::Controller,
                };
                let mut sub = ResolvedAbility::new(gain(1), vec![], host, P0);
                sub.condition = Some(AbilityCondition::QuantityCheck {
                    lhs: counters_on(scope, "task"),
                    comparator: Comparator::EQ,
                    rhs: QuantityExpr::Fixed { value: 0 },
                });
                let mut root = ResolvedAbility::new(gain(10), vec![], host, P0);
                root.sub_ability = Some(Box::new(sub));
                resolve_ability_chain(&mut st, &root, &mut Vec::new(), 0).unwrap();
                assert_eq!(
                    st.players[0].life - life,
                    if runs { 11 } else { 10 },
                    "{scope:?}"
                );
            }
        }
    }

    const UPKEEP_GAIN: &str =
        "Equipped creature has \"At the beginning of your upkeep, you gain 1 life.\"";

    fn upkeep_grant(granter_counters: bool) -> StaticDefinition {
        let mut grant = grant_static(UPKEEP_GAIN, "Charger", "Artifact", "Equipment");
        if granter_counters {
            match granted_execute(&mut grant).effect.as_mut() {
                Effect::GainLife { amount, .. } => {
                    *amount = counters_on(ObjectScope::GrantingObject, "charge")
                }
                other => panic!("expected GainLife, got {other:?}"),
            }
        }
        grant
    }

    fn upkeep_runner(pairs: usize, granter_counters: bool) -> GameRunner {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::Untap);
        let pairs: Vec<(ObjectId, ObjectId)> = (0..pairs)
            .map(|i| {
                let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
                let granter = scenario.add_creature(P0, "Charger", 0, 0).id();
                scenario.with_counter(host, counter("charge"), 1);
                scenario.with_counter(granter, counter("charge"), 2 + i as u32);
                (host, granter)
            })
            .collect();
        let mut runner = scenario.build();
        for (host, granter) in pairs {
            attach(
                &mut runner,
                granter,
                host,
                CoreType::Artifact,
                "Equipment",
                upkeep_grant(granter_counters),
            );
        }
        runner.advance_to_upkeep();
        runner
    }

    #[test]
    fn granted_trigger_gains_the_granters_counters() {
        let mut runner = upkeep_runner(1, true);
        let life = runner.state().players[0].life;
        runner.advance_until_stack_empty();
        assert_eq!(runner.state().players[0].life - life, 2);
    }

    #[test]
    fn granted_triggers_bound_to_distinct_granters_need_ordering() {
        let runner = upkeep_runner(2, true);
        assert!(matches!(
            runner.state().waiting_for,
            WaitingFor::OrderTriggers { .. }
        ));
        let control = upkeep_runner(2, false);
        assert!(matches!(
            control.state().waiting_for,
            WaitingFor::Priority { .. }
        ));
        assert_eq!(control.state().stack.len(), 2);
    }

    #[test]
    fn spare_dagger_sacrifices_the_dagger_and_deals_damage() {
        let grant = grant_static(SPARE_DAGGER, "Spare Dagger", "Artifact", "Equipment");
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let dagger = scenario.add_creature(P0, "Spare Dagger", 0, 0).id();
        let mut runner = scenario.build();
        attach(
            &mut runner,
            dagger,
            host,
            CoreType::Artifact,
            "Equipment",
            grant,
        );
        let life = runner.state().players[1].life;
        attack_with(&mut runner, host);
        drive(&mut runner, Some(TargetRef::Player(P1)));
        let st = runner.state();
        assert_eq!(st.objects[&dagger].zone, Zone::Graveyard);
        assert_eq!(st.objects[&host].zone, Zone::Battlefield);
        assert_eq!(st.players[1].life, life - 1);
    }

    #[test]
    fn every_condition_and_quantity_arm_binds_the_granter() {
        let g = || counters_on(ObjectScope::GrantingObject, "charge");
        let check = |lhs, rhs| AbilityCondition::QuantityCheck {
            lhs,
            comparator: Comparator::GE,
            rhs,
        };
        let fixed = || QuantityExpr::Fixed { value: 0 };
        let condition = AbilityCondition::And {
            conditions: vec![
                check(
                    QuantityExpr::Difference {
                        left: Box::new(QuantityExpr::Offset {
                            inner: Box::new(g()),
                            offset: 1,
                        }),
                        right: Box::new(QuantityExpr::Sum { exprs: vec![g()] }),
                    },
                    fixed(),
                ),
                AbilityCondition::Or {
                    conditions: vec![
                        check(fixed(), g()),
                        AbilityCondition::Not {
                            condition: Box::new(check(g(), fixed())),
                        },
                    ],
                },
                AbilityCondition::ConditionInstead {
                    inner: Box::new(check(g(), fixed())),
                },
                AbilityCondition::PreviousEffectAmount {
                    comparator: Comparator::GE,
                    rhs: g(),
                    channel: Default::default(),
                },
            ],
        };
        let generic = Effect::GenericEffect {
            static_abilities: vec![StaticDefinition::continuous()
                .affected(TargetFilter::GrantingObject)
                .modifications(vec![ContinuousModification::SetDynamicPower { value: g() }])],
            duration: None,
            target: None,
            end_cost: None,
        };
        let mut def = AbilityDefinition::new(
            AbilityKind::Activated,
            Effect::GainLife {
                amount: QuantityExpr::Fixed { value: 1 },
                player: TargetFilter::Controller,
            },
        )
        .cost(AbilityCost::Tap)
        .sub_ability(AbilityDefinition::new(AbilityKind::Spell, generic));
        def.condition = Some(condition);
        let grant = StaticDefinition::continuous()
            .affected(TargetFilter::Typed(
                TypedFilter::creature().properties(vec![FilterProp::EquippedBy]),
            ))
            .modifications(vec![ContinuousModification::GrantAbility {
                definition: Box::new(def),
            }]);

        let mut scenario = GameScenario::new();
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let granter = scenario.add_creature(P0, "Charger", 0, 0).id();
        let mut runner = scenario.build();
        attach(
            &mut runner,
            granter,
            host,
            CoreType::Artifact,
            "Equipment",
            grant,
        );
        let st = runner.state();
        let copy = st.objects[&host]
            .abilities
            .last()
            .expect("the granted ability");
        let json = serde_json::to_string(copy).unwrap();
        let scope = serde_json::to_string(&bound(st, granter)).unwrap();
        let filter = serde_json::to_string(&TargetFilter::SpecificObject { id: granter }).unwrap();
        assert_eq!(json.matches(&scope).count(), 7, "{json}");
        assert_eq!(json.matches(&filter).count(), 1, "{json}");
        assert!(!json.contains("GrantingObject"), "{json}");
    }

    #[test]
    fn trusty_boomerang_taps_the_target_and_returns_itself() {
        let grant = grant_static(
            TRUSTY_BOOMERANG,
            "Trusty Boomerang",
            "Artifact",
            "Equipment",
        );
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        scenario.with_mana_pool(
            P0,
            vec![ManaUnit::new(
                ManaType::Colorless,
                ObjectId(0),
                false,
                vec![],
            )],
        );
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let boomerang = scenario.add_creature(P0, "Trusty Boomerang", 0, 0).id();
        let victim = scenario.add_creature(P1, "Victim", 2, 2).id();
        let mut runner = scenario.build();
        attach(
            &mut runner,
            boomerang,
            host,
            CoreType::Artifact,
            "Equipment",
            grant,
        );
        runner
            .state_mut()
            .objects
            .get_mut(&boomerang)
            .unwrap()
            .keywords
            .push(Keyword::Shroud);
        let index = runner.state().objects[&host].abilities.len() - 1;
        activate(&mut runner, host, index, Some(victim));
        runner.advance_until_stack_empty();
        let st = runner.state();
        assert!(st.objects[&victim].tapped);
        assert_eq!(st.objects[&boomerang].zone, Zone::Hand);
        assert_eq!(st.objects[&host].zone, Zone::Battlefield);
    }

    #[test]
    fn heliods_punishment_cast_enters_with_four_and_destroys_itself_on_the_fourth() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let hp = scenario
            .add_spell_to_hand(P0, "Heliod's Punishment", false)
            .as_enchantment()
            .with_subtypes(vec!["Aura"])
            .from_oracle_text(HELIODS_PUNISHMENT)
            .with_keyword(Keyword::Enchant(TargetFilter::Typed(
                TypedFilter::creature(),
            )))
            .with_mana_cost(ManaCost::generic(0))
            .id();
        let mut runner = scenario.build();
        runner.cast(hp).target_object(host).resolve();
        let st = runner.state();
        assert_eq!(st.objects[&hp].zone, Zone::Battlefield);
        assert_eq!(
            st.objects[&hp].attached_to,
            Some(AttachTarget::Object(host))
        );
        assert_eq!(st.objects[&hp].counters.get(&counter("task")), Some(&4));
        for remaining in [3, 2, 1] {
            runner.state_mut().objects.get_mut(&host).unwrap().tapped = false;
            let index = runner.state().objects[&host].abilities.len() - 1;
            activate(&mut runner, host, index, None);
            runner.advance_until_stack_empty();
            let st = runner.state();
            assert_eq!(
                st.objects[&hp].counters.get(&counter("task")),
                Some(&remaining)
            );
            assert_eq!(st.objects[&hp].zone, Zone::Battlefield);
            assert_eq!(st.objects[&host].zone, Zone::Battlefield);
        }
        runner.state_mut().objects.get_mut(&host).unwrap().tapped = false;
        let index = runner.state().objects[&host].abilities.len() - 1;
        activate(&mut runner, host, index, None);
        runner.advance_until_stack_empty();
        assert_eq!(runner.state().objects[&hp].zone, Zone::Graveyard);
        assert_eq!(runner.state().objects[&host].zone, Zone::Battlefield);
    }

    /// CR 201.5a + CR 601.2h: the cost removes Hankyu's counters, not the host's.
    #[test]
    fn hankyu_remove_all_reads_the_granter() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        scenario.with_counter(host, counter("aim"), 1);
        let hk = scenario
            .add_artifact_from_oracle(P0, "Hankyu", HANKYU)
            .with_subtypes(vec!["Equipment"])
            .id();
        scenario.with_counter(hk, counter("aim"), 2);
        let mut runner = scenario.build();
        assert_ne!(hk, host);
        attach_to(runner.state_mut(), hk, host);
        relayer(runner.state_mut());
        let index = runner.state().objects[&host].abilities.len() - 1;
        runner
            .act(GameAction::ActivateAbility {
                source_id: host,
                ability_index: index,
            })
            .unwrap();
        if matches!(
            runner.state().waiting_for,
            WaitingFor::TargetSelection { .. }
        ) {
            runner
                .act(GameAction::SelectTargets {
                    targets: vec![TargetRef::Player(P1)],
                })
                .unwrap();
        }
        assert!(
            !matches!(runner.state().waiting_for, WaitingFor::PayCost { .. }),
            "{:?}",
            runner.state().waiting_for
        );
        runner.advance_until_stack_empty();
        let st = runner.state();
        assert_eq!(st.objects[&hk].counters.get(&counter("aim")), None);
        assert_eq!(st.objects[&host].counters.get(&counter("aim")), Some(&1));
        assert!(st.objects[&host].tapped);
        assert!(st.stack.is_empty());
    }

    #[test]
    fn the_aetherspark_loyalty_lands_on_itself() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let sp = scenario
            .add_artifact_from_oracle(P0, "The Aetherspark", THE_AETHERSPARK)
            .with_subtypes(vec!["Equipment"])
            .id();
        scenario.with_counter(sp, CounterType::Loyalty, 1);
        let mut runner = scenario.build();
        attach_to(runner.state_mut(), sp, host);
        relayer(runner.state_mut());
        let targets: Vec<TargetFilter> = runner.state().objects[&sp]
            .trigger_definitions
            .as_slice()
            .iter()
            .filter_map(|t| t.definition.execute.as_deref())
            .filter_map(|d| match d.effect.as_ref() {
                Effect::PutCounter { target, .. } => Some(target.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec![TargetFilter::SpecificObject { id: sp }]);
        let life = runner.state().players[1].life;
        attack_with(&mut runner, host);
        runner.combat_damage();
        runner.advance_until_stack_empty();
        let st = runner.state();
        assert_eq!(st.players[1].life, life - 2);
        assert_eq!(
            st.objects[&sp].counters.get(&CounterType::Loyalty),
            Some(&3)
        );
        assert_eq!(st.objects[&host].counters.get(&CounterType::Loyalty), None);
    }

    #[test]
    fn grothama_granted_fight_fights_grothama() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let attacker = scenario.add_creature(P0, "Bearer", 3, 5).id();
        let grothama = scenario
            .add_creature_from_oracle(P0, "Grothama, All-Devouring", 10, 8, GROTHAMA)
            .as_legendary()
            .id();
        let mut runner = scenario.build();
        relayer(runner.state_mut());
        attack_with(&mut runner, attacker);
        drive(&mut runner, None);
        let st = runner.state();
        assert_ne!(grothama, attacker);
        assert_eq!(st.objects[&grothama].damage_marked, 3);
        assert_eq!(st.objects[&attacker].damage_marked, 10);
        assert_eq!(st.objects[&attacker].zone, Zone::Graveyard);
    }

    #[test]
    fn nettlevine_blight_moves_to_another_permanent() {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PostCombatMain);
        let host = scenario.add_creature(P0, "Bearer", 2, 2).id();
        let other = scenario.add_creature(P0, "Other", 1, 1).id();
        let blight = scenario
            .add_enchantment_from_oracle(P0, "Nettlevine Blight", NETTLEVINE_BLIGHT)
            .with_subtypes(vec!["Aura"])
            .id();
        let mut runner = scenario.build();
        attach_to(runner.state_mut(), blight, host);
        relayer(runner.state_mut());
        runner.advance_to_end_step();
        let legal = match &runner.state().waiting_for {
            WaitingFor::TriggerTargetSelection { selection, .. } => {
                selection.current_legal_targets.clone()
            }
            other => panic!("expected the attach target choice, got {other:?}"),
        };
        assert!(legal.contains(&TargetRef::Object(host)), "{legal:?}");
        drive(&mut runner, Some(TargetRef::Object(other)));
        let st = runner.state();
        assert_eq!(st.objects[&host].zone, Zone::Graveyard);
        assert_eq!(st.objects[&blight].zone, Zone::Battlefield);
        assert_eq!(
            st.objects[&blight].attached_to,
            Some(AttachTarget::Object(other))
        );
    }
}
