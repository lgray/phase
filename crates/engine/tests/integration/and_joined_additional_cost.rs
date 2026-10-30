//! An additional cost of two " and "-joined costs requires paying both.
//!
//! CR 601.2f + CR 601.2h: every additional cost is part of the total cost, and
//! the total cost is paid in full. Ruthless Disposal: "discard a card and
//! sacrifice a creature".

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard, ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::zones::Zone;
use engine::types::TargetRef;

const RUTHLESS_DISPOSAL: &str = "As an additional cost to cast this spell, discard a card and sacrifice a creature.\nTwo target creatures each get -13/-13 until end of turn.";

struct Board {
    runner: GameRunner,
    spell: ObjectId,
    fodder: ObjectId,
    targets: [ObjectId; 2],
}

/// Ruthless Disposal and one other card in P0's hand, two 2/2s for P1, and
/// {B}{B}{B}{B}{B} floating; `with_own_creature` adds P0's own 2/2.
fn ruthless_board(with_own_creature: bool) -> (Board, Option<ObjectId>) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Ruthless Disposal", false, RUTHLESS_DISPOSAL)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Black],
            generic: 4,
        })
        .id();
    let fodder = scenario.add_card_to_hand(P0, "Fodder");
    let own = with_own_creature.then(|| scenario.add_vanilla(P0, 2, 2));
    let targets = [
        scenario.add_vanilla(P1, 2, 2),
        scenario.add_vanilla(P1, 2, 2),
    ];
    scenario.with_mana_pool(
        P0,
        (0..5)
            .map(|_| ManaUnit::new(ManaType::Black, ObjectId(0), false, vec![]))
            .collect(),
    );
    (
        Board {
            runner: scenario.build(),
            spell,
            fodder,
            targets,
        },
        own,
    )
}

#[test]
fn ruthless_disposal_pays_the_discard_and_the_sacrifice() {
    let (mut board, own) = ruthless_board(true);
    let bear = own.expect("own creature");

    let outcome = board
        .runner
        .cast(board.spell)
        .target_objects(&board.targets)
        .pay_cost_with(&[board.fodder, bear])
        .resolve();

    outcome.assert_zone(&[board.fodder, bear], Zone::Graveyard);
    outcome.assert_zone(&board.targets, Zone::Graveyard);
    outcome.assert_zone(&[board.spell], Zone::Graveyard);
    assert!(matches!(
        outcome.final_waiting_for(),
        WaitingFor::Priority { .. }
    ));
    assert!(outcome.state().stack.is_empty());
}

/// Ruling: Ruthless Disposal can't be cast without a creature to sacrifice, so
/// the cast is reversed (CR 733.1) rather than completed on the discard alone.
#[test]
fn ruthless_disposal_without_a_creature_is_not_cast() {
    let (mut board, _) = ruthless_board(false);
    let runner = &mut board.runner;
    let card_id = runner.state().objects[&board.spell].card_id;

    runner
        .act(GameAction::CastSpell {
            object_id: board.spell,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("the cast is offered");
    assert!(
        matches!(
            runner.state().waiting_for,
            WaitingFor::TargetSelection { .. }
        ),
        "reach guard: the cast reaches target selection, got {:?}",
        runner.state().waiting_for
    );
    let _ = runner.act(GameAction::SelectTargets {
        targets: board
            .targets
            .iter()
            .map(|&t| TargetRef::Object(t))
            .collect(),
    });

    let state = runner.state();
    assert_eq!(state.objects[&board.fodder].zone, Zone::Hand);
    assert_eq!(state.objects[&board.spell].zone, Zone::Hand);
    assert!(state.stack.is_empty());
    assert!(matches!(state.waiting_for, WaitingFor::Priority { .. }));
}

const KAERVEKS_SPITE: &str = "As an additional cost to cast this spell, sacrifice all permanents you control and discard your hand.\nTarget player loses 5 life.";

#[test]
fn kaerveks_spite_sacrifices_everything_and_discards_the_hand() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Kaervek's Spite", true, KAERVEKS_SPITE)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Black; 3],
            generic: 0,
        })
        .id();
    let hand = [
        scenario.add_card_to_hand(P0, "H1"),
        scenario.add_card_to_hand(P0, "H2"),
    ];
    let swamps: Vec<ObjectId> = (0..3)
        .map(|_| scenario.add_basic_land(P0, ManaColor::Black))
        .collect();
    let bear = scenario.add_vanilla(P0, 2, 2);
    let theirs = scenario.add_vanilla(P1, 2, 2);
    let mut runner = scenario.build();

    let outcome = runner
        .cast(spell)
        .target_player(P1)
        .pay_cost_with(&hand)
        .resolve();

    // CR 601.2g + CR 601.2h: the Swamps tapped for {B}{B}{B} are still
    // controlled when the cost is paid, so they are sacrificed too.
    outcome.assert_zone(&swamps, Zone::Graveyard);
    outcome.assert_zone(&[bear], Zone::Graveyard);
    outcome.assert_zone(&hand, Zone::Graveyard);
    outcome.assert_zone(&[spell], Zone::Graveyard);
    assert_eq!(outcome.life_delta(P1), -5);
    outcome.assert_zone(&[theirs], Zone::Battlefield);
    assert!(matches!(
        outcome.final_waiting_for(),
        WaitingFor::Priority { .. }
    ));
    assert!(outcome.state().stack.is_empty());
}
