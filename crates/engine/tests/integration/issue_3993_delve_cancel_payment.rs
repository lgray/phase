//! Regression for issue #3993: Cancelling during delve mana payment must return
//! delved graveyard cards from exile instead of leaving them stranded.
//!
//! CR 601.2i: If the player is unable or unwilling to complete a cast, the
//! process is reversed and any choices made (including delve exiles) are undone.
//!
//! https://github.com/phase-rs/phase/issues/3993

use engine::ai_support::legal_actions_full;
use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::ability::{
    CastPermissionConstraint, CastingPermission, Comparator, ExileGrantCostProvenance, PlayerScope,
    QuantityExpr, QuantityRef,
};
use engine::types::actions::GameAction;
use engine::types::card_type::CoreType;
use engine::types::game_state::{CastPaymentMode, ConvokeMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard, ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const DELVE_DRAW_ORACLE: &str =
    "Delve (Each card you exile from your graveyard while casting this spell pays for {1}.)\n\
Draw a card.";

fn mana_pool(generic: usize, red: usize) -> Vec<ManaUnit> {
    let mut pool = Vec::new();
    for _ in 0..generic {
        pool.push(ManaUnit::new(
            ManaType::Colorless,
            ObjectId(0),
            false,
            vec![],
        ));
    }
    for _ in 0..red {
        pool.push(ManaUnit::new(ManaType::Red, ObjectId(0), false, vec![]));
    }
    pool
}

#[test]
fn issue_3993_cancel_during_delve_payment_returns_graveyard_cards() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Delve Draw", false, DELVE_DRAW_ORACLE)
        .id();
    let delve_a = scenario.add_spell_to_graveyard(P0, "Old Bolt", true).id();
    let delve_b = scenario.add_spell_to_graveyard(P0, "Old Shock", true).id();
    scenario.with_mana_pool(P0, mana_pool(3, 1));

    let mut runner = scenario.build();

    let card_id = runner.state().objects[&spell].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: spell,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("begin casting delve spell");

    assert!(matches!(
        runner.state().waiting_for,
        WaitingFor::ManaPayment {
            convoke_mode: Some(ConvokeMode::Delve),
            ..
        }
    ));

    for gy_id in [delve_a, delve_b] {
        runner
            .act(GameAction::TapForConvoke {
                object_id: gy_id,
                mana_type: ManaType::Colorless,
            })
            .expect("delve graveyard card");
        assert_eq!(
            runner.state().objects[&gy_id].zone,
            Zone::Exile,
            "delved card should be exiled before cancel"
        );
    }

    runner
        .act(GameAction::CancelCast)
        .expect("cancel during delve payment");

    for gy_id in [delve_a, delve_b] {
        assert_eq!(
            runner.state().objects[&gy_id].zone,
            Zone::Graveyard,
            "cancelled delve payment must return the card to the graveyard"
        );
    }
    assert_eq!(
        runner.state().objects[&spell].zone,
        Zone::Hand,
        "cancelled spell must return to hand"
    );
    assert!(
        runner.state().stack.is_empty(),
        "cancelled spell must be removed from the stack"
    );
    assert!(
        !runner
            .state()
            .cards_exiled_with_source_this_turn
            .get(&spell)
            .is_some_and(|ids| ids.contains(&delve_a) || ids.contains(&delve_b)),
        "delve exile-with-source tracking must be cleared on cancel"
    );
    assert!(
        runner.state().players[P0.0 as usize]
            .mana_pool
            .mana
            .iter()
            .all(|unit| !unit.is_convoke_payment()),
        "delve mana markers must be removed from the pool on cancel"
    );
}

const CRUISE_ORACLE: &str =
    "Delve (Each card you exile from your graveyard while casting this spell pays for {1}.)\n\
Draw three cards.";

fn cruise_cost() -> ManaCost {
    ManaCost::Cost {
        shards: vec![ManaCostShard::Blue],
        generic: 7,
    }
}

fn graveyard_names(runner: &GameRunner) -> Vec<String> {
    runner.state().players[P0.0 as usize]
        .graveyard
        .iter()
        .map(|id| runner.state().objects[id].name.clone())
        .collect()
}

fn delve_marker_count(runner: &GameRunner) -> usize {
    runner.state().players[P0.0 as usize]
        .mana_pool
        .mana
        .iter()
        .filter(|unit| unit.is_convoke_payment())
        .count()
}

fn cast_manual(runner: &mut GameRunner, spell: ObjectId) {
    let card_id = runner.state().objects[&spell].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: spell,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Manual,
        })
        .expect("begin casting Treasure Cruise");
}

fn delve(runner: &mut GameRunner, card: ObjectId) {
    runner
        .act(GameAction::TapForConvoke {
            object_id: card,
            mana_type: ManaType::Colorless,
        })
        .expect("delve graveyard card");
}

fn assert_cancel_restored(runner: &GameRunner, spell: ObjectId, spell_zone: Zone, gy: &[&str]) {
    assert_eq!(graveyard_names(runner), gy);
    assert_eq!(runner.state().objects[&spell].zone, spell_zone);
    assert!(runner.state().stack.is_empty());
    assert_eq!(delve_marker_count(runner), 0);
}

/// Treasure Cruise in hand, `gy` in the graveyard (in order), optionally a Forest and a Dimir Signet.
fn cruise_in_hand(
    gy: &[&str],
    with_signet: bool,
) -> (
    GameRunner,
    ObjectId,
    Vec<ObjectId>,
    Option<(ObjectId, ObjectId)>,
) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let cruise = scenario
        .add_spell_to_hand_from_oracle(P0, "Treasure Cruise", false, CRUISE_ORACLE)
        .from_oracle_text_with_keywords(&["Delve"], CRUISE_ORACLE)
        .with_mana_cost(cruise_cost())
        .id();
    let ids = gy
        .iter()
        .map(|name| scenario.add_spell_to_graveyard(P0, name, true).id())
        .collect();
    let lands = with_signet.then(|| {
        let forest = scenario.add_basic_land(P0, ManaColor::Green);
        let signet = scenario
            .add_artifact_from_oracle(P0, "Dimir Signet", "{1}, {T}: Add {U}{B}.")
            .id();
        (forest, signet)
    });
    (scenario.build(), cruise, ids, lands)
}

#[test]
fn cancel_delve_restores_graveyard_order_after_signet_activation() {
    let (mut runner, cruise, ids, lands) = cruise_in_hand(&["Lightning Bolt", "Shock"], true);
    let (forest, signet) = lands.expect("signet fixture");
    cast_manual(&mut runner, cruise);

    let (_, _, grouped) = legal_actions_full(runner.state());
    let selection = grouped
        .get(&forest)
        .into_iter()
        .flatten()
        .find_map(|action| match action {
            GameAction::TapLandForMana { selection } => Some(selection.clone()),
            _ => None,
        })
        .expect("engine offers a Forest tap");
    runner
        .act(GameAction::TapLandForMana { selection })
        .expect("tap Forest");
    delve(&mut runner, ids[0]);
    delve(&mut runner, ids[1]);
    runner
        .act(GameAction::ActivateAbility {
            source_id: signet,
            ability_index: 0,
        })
        .expect("activate Dimir Signet");

    let marker_sources: Vec<ObjectId> = runner.state().players[P0.0 as usize]
        .mana_pool
        .mana
        .iter()
        .filter(|unit| unit.is_convoke_payment())
        .map(|unit| unit.source_id)
        .collect();
    assert_eq!(
        marker_sources,
        [ids[1], ids[0]],
        "Signet activation must have reordered the delve markers"
    );

    runner.act(GameAction::CancelCast).expect("cancel cast");
    assert_cancel_restored(&runner, cruise, Zone::Hand, &["Lightning Bolt", "Shock"]);
}

#[test]
fn cancel_delve_restores_graveyard_order_around_undelved_card() {
    let (mut runner, cruise, ids, _) =
        cruise_in_hand(&["Lightning Bolt", "Island", "Shock"], false);
    cast_manual(&mut runner, cruise);
    delve(&mut runner, ids[0]);
    delve(&mut runner, ids[2]);
    for id in [ids[0], ids[2]] {
        assert_eq!(runner.state().objects[&id].zone, Zone::Exile);
    }

    runner.act(GameAction::CancelCast).expect("cancel cast");
    assert_cancel_restored(
        &runner,
        cruise,
        Zone::Hand,
        &["Lightning Bolt", "Island", "Shock"],
    );
}

/// CR 733.1 + CR 700.11: an undone delve exile puts no permanent card into the
/// graveyard, so the player has not descended.
#[test]
fn cancel_delve_of_permanent_card_does_not_mark_descended() {
    let (mut runner, cruise, ids, _) = cruise_in_hand(&["Grizzly Bears", "Shock"], false);
    runner
        .state_mut()
        .objects
        .get_mut(&ids[0])
        .unwrap()
        .card_types
        .core_types
        .push(CoreType::Creature);
    cast_manual(&mut runner, cruise);
    delve(&mut runner, ids[0]);
    assert_eq!(runner.state().objects[&ids[0]].zone, Zone::Exile);
    assert!(!runner.state().players[P0.0 as usize].descended_this_turn);
    let rows = runner.state().zone_changes_this_turn.len();

    runner.act(GameAction::CancelCast).expect("cancel cast");

    assert_cancel_restored(&runner, cruise, Zone::Hand, &["Grizzly Bears", "Shock"]);
    assert!(!runner.state().players[P0.0 as usize].descended_this_turn);
    assert_eq!(runner.state().zone_changes_this_turn.len(), rows);
}

/// Treasure Cruise in exile castable only while its mana value is at most the
/// graveyard size, so delving the graveyard away fails the finalize re-check.
fn cruise_in_exile(constraint_value: QuantityExpr) -> (GameRunner, ObjectId, Vec<ObjectId>) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let cruise = scenario
        .add_spell_to_exile(P0, "Treasure Cruise", false)
        .from_oracle_text_with_keywords(&["Delve"], CRUISE_ORACLE)
        .with_mana_cost(cruise_cost())
        .id();
    let ids = ["Lightning Bolt", "Island", "Shock"]
        .iter()
        .map(|name| scenario.add_spell_to_graveyard(P0, name, true).id())
        .collect();
    let mut pool = mana_pool(5, 0);
    pool.push(ManaUnit::new(ManaType::Blue, ObjectId(0), false, vec![]));
    scenario.with_mana_pool(P0, pool);
    let mut runner = scenario.build();
    runner
        .state_mut()
        .objects
        .get_mut(&cruise)
        .expect("cruise exists")
        .casting_permissions
        .push(CastingPermission::ExileWithAltCost {
            source_id: None,
            cost_provenance: ExileGrantCostProvenance::Alternative,
            cost: cruise_cost(),
            cast_transformed: false,
            constraint: Some(CastPermissionConstraint::ManaValue {
                comparator: Comparator::LE,
                value: constraint_value,
            }),
            granted_to: Some(P0),
            resolution_cleanup: None,
            duration: None,
            graveyard_replacement: None,
            mana_spend_permission: None,
            enters_with_counter: None,
            enters_with_modifications: Vec::new(),
            cast_cost_modifier: None,
        });
    (runner, cruise, ids)
}

#[test]
fn cancel_after_rejected_finalize_keeps_delve_record_and_restores_order() {
    let (mut runner, cruise, ids) = cruise_in_exile(QuantityExpr::Ref {
        qty: QuantityRef::GraveyardSize {
            player: PlayerScope::Controller,
        },
    });
    cast_manual(&mut runner, cruise);
    delve(&mut runner, ids[0]);
    delve(&mut runner, ids[2]);

    runner
        .act(GameAction::PassPriority)
        .expect_err("finalize re-check rejects the shrunken graveyard");
    for id in [ids[0], ids[2]] {
        assert_eq!(runner.state().objects[&id].zone, Zone::Exile);
    }
    assert_eq!(delve_marker_count(&runner), 2);
    assert_eq!(
        runner
            .state()
            .pending_cast
            .as_ref()
            .and_then(|pending| pending.delve.as_ref())
            .map(|delve| delve.cards.len()),
        Some(2)
    );

    runner.act(GameAction::CancelCast).expect("cancel cast");
    assert_cancel_restored(
        &runner,
        cruise,
        Zone::Exile,
        &["Lightning Bolt", "Island", "Shock"],
    );
}

#[test]
fn rejected_finalize_control_fixed_constraint_casts() {
    let (mut runner, cruise, ids) = cruise_in_exile(QuantityExpr::Fixed { value: 8 });
    cast_manual(&mut runner, cruise);
    delve(&mut runner, ids[0]);
    delve(&mut runner, ids[2]);

    runner
        .act(GameAction::PassPriority)
        .expect("constraint satisfied, cast completes");
    assert_eq!(runner.state().objects[&cruise].zone, Zone::Stack);
}
