//! Gifts Given: the targeted opponent chooses which revealed cards go to their
//! graveyard; the rest go to their hand (CR 400.3), and they shuffle.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::actions::GameAction;
use engine::types::events::{GameEvent, PlayerActionKind};
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

/// Verbatim Scryfall Oracle text.
const GIFTS_GIVEN: &str = "Search target opponent's library for four cards with different \
names and reveal them. That player chooses two of those cards. Put the chosen cards into the \
player's graveyard and the rest into your hand. Then that player shuffles their library.";

/// Verbatim Scryfall Oracle text.
const GIFTS_UNGIVEN: &str = "Search your library for up to four cards with different names \
and reveal them. Target opponent chooses two of those cards. Put the chosen cards into your \
graveyard and the rest into your hand. Then shuffle.";

const LIBRARY: [&str; 5] = ["Card A", "Card B", "Card C", "Card D", "Card E"];

struct Resolved {
    runner: GameRunner,
    found: Vec<ObjectId>,
    chosen: Vec<ObjectId>,
    rest: Vec<ObjectId>,
    shuffled: Vec<PlayerId>,
}

/// Casts `oracle` from P0 targeting `target`, has P0 find the first four
/// cards of `library_owner`'s library, and has `expected_chooser` pick
/// `found[pick.0]` and `found[pick.1]`.
fn cast_and_choose(
    mut scenario: GameScenario,
    name: &str,
    oracle: &str,
    library_owner: PlayerId,
    target: PlayerId,
    expected_chooser: PlayerId,
    pick: (usize, usize),
) -> Resolved {
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_library_top(library_owner, &LIBRARY);
    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, name, false, oracle)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();
    runner.cast(spell).target_player(target).resolve();

    let WaitingFor::SearchChoice {
        player,
        library_owner: searched,
        cards,
        count,
        ..
    } = runner.state().waiting_for.clone()
    else {
        panic!(
            "expected SearchChoice, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(player, P0);
    assert_eq!(searched, Some(library_owner));
    assert_eq!(count, 4);
    let found: Vec<_> = cards.into_iter().take(4).collect();
    let mut events = runner
        .act(GameAction::SelectCards {
            cards: found.clone(),
        })
        .expect("search selection resumes the spell")
        .events;

    let WaitingFor::ChooseFromZoneChoice {
        player,
        cards: offered,
        count,
        ..
    } = runner.state().waiting_for.clone()
    else {
        panic!(
            "expected ChooseFromZoneChoice, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(player, expected_chooser);
    assert_eq!(count, 2);
    let mut offered_sorted = offered.clone();
    offered_sorted.sort();
    let mut found_sorted = found.clone();
    found_sorted.sort();
    assert_eq!(offered_sorted, found_sorted);

    let chosen = vec![found[pick.0], found[pick.1]];
    let rest: Vec<_> = found
        .iter()
        .copied()
        .filter(|id| !chosen.contains(id))
        .collect();
    events.extend(
        runner
            .act(GameAction::SelectCards {
                cards: chosen.clone(),
            })
            .expect("the choice resumes the spell")
            .events,
    );
    assert!(runner.state().stack.is_empty());
    assert!(matches!(
        runner.state().waiting_for,
        WaitingFor::Priority { .. }
    ));

    let shuffled = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::PlayerPerformedAction {
                player_id,
                action: PlayerActionKind::ShuffledLibrary,
                ..
            } => Some(*player_id),
            _ => None,
        })
        .collect();
    Resolved {
        runner,
        found,
        chosen,
        rest,
        shuffled,
    }
}

fn assert_in_zone(runner: &GameRunner, ids: &[ObjectId], owner: PlayerId, zone: Zone) {
    let player = &runner.state().players[owner.0 as usize];
    for id in ids {
        assert_eq!(runner.state().objects[id].zone, zone, "{id:?}");
        let held = match zone {
            Zone::Graveyard => player.graveyard.contains(id),
            Zone::Hand => player.hand.contains(id),
            other => panic!("unexpected zone {other:?}"),
        };
        assert!(held, "{id:?} not in {owner:?}'s {zone:?}");
    }
}

#[test]
fn gifts_given_targeted_opponent_chooses_and_cards_stay_with_their_owner() {
    let r = cast_and_choose(
        GameScenario::new(),
        "Gifts Given",
        GIFTS_GIVEN,
        P1,
        P1,
        P1,
        (3, 1),
    );
    assert_in_zone(&r.runner, &r.chosen, P1, Zone::Graveyard);
    assert_in_zone(&r.runner, &r.rest, P1, Zone::Hand);
    let p0 = &r.runner.state().players[0];
    assert!(r.found.iter().all(|id| !p0.hand.contains(id)));
    assert_eq!(r.shuffled, vec![P1]);
}

#[test]
fn gifts_given_binds_the_announced_opponent_among_several() {
    let p2 = PlayerId(2);
    let mut scenario = GameScenario::new_n_player(3, 42);
    scenario.with_library_top(P1, &["Bystander card"]);
    let r = cast_and_choose(scenario, "Gifts Given", GIFTS_GIVEN, p2, p2, p2, (0, 2));
    assert_in_zone(&r.runner, &r.chosen, p2, Zone::Graveyard);
    assert_in_zone(&r.runner, &r.rest, p2, Zone::Hand);
    assert_eq!(r.runner.state().players[1].library.len(), 1);
    assert_eq!(r.shuffled, vec![p2]);
}

#[test]
fn gifts_ungiven_partition_routes_the_casters_cards() {
    let r = cast_and_choose(
        GameScenario::new(),
        "Gifts Ungiven",
        GIFTS_UNGIVEN,
        P0,
        P1,
        P1,
        (0, 2),
    );
    assert_in_zone(&r.runner, &r.chosen, P0, Zone::Graveyard);
    assert_in_zone(&r.runner, &r.rest, P0, Zone::Hand);
    assert_eq!(r.shuffled, vec![P0]);
}
