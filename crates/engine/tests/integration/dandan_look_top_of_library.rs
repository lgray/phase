//! "Look at the top N cards of your library. You may cast a spell from among them ... Put the
//! rest on the bottom of your library": under the Dandân announcement "your library" is the one
//! shared pile whoever owns each card (CR 400.1, CR 401.2), so every looked-at card is offered
//! and every unchosen one goes to the bottom.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::actions::GameAction;
use engine::types::format::FormatConfig;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

use crate::support::shared_card_db;

/// Svella's second ability: "{6}{R}{G}, {T}: Look at the top four cards of your library. You may
/// cast a spell from among them without paying its mana cost. Put the rest on the bottom of your
/// library in a random order."
const SVELLA_LOOK_ABILITY: usize = 1;

struct Peek {
    runner: GameRunner,
    /// The four looked-at cards, top first, as `(id, owner)`.
    top_four: Vec<(ObjectId, PlayerId)>,
}

/// P0 controls Svella; the top four cards of P0's library are Brainstorm, Island, Divination and
/// Opt with the given owners, followed by six Islands.
fn peek(format: FormatConfig, owners: [PlayerId; 4]) -> Peek {
    let db = shared_card_db().expect("card db");
    let mut scenario = GameScenario::new_with_format(format, 2, 11);
    scenario.at_phase(Phase::PreCombatMain);
    let svella = scenario.add_real_card(P0, "Svella, Ice Shaper", Zone::Battlefield, db);
    let names = ["Brainstorm", "Island", "Divination", "Opt"];
    let top_four: Vec<_> = names
        .iter()
        .zip(owners)
        .map(|(name, owner)| {
            (
                scenario.add_real_card(owner, name, Zone::Library, db),
                owner,
            )
        })
        .collect();
    for _ in 0..6 {
        scenario.add_real_card(P0, "Island", Zone::Library, db);
    }
    scenario.with_mana_pool(
        P0,
        [
            (ManaType::Colorless, 6),
            (ManaType::Red, 1),
            (ManaType::Green, 1),
        ]
        .into_iter()
        .flat_map(|(color, n)| (0..n).map(move |_| ManaUnit::new(color, svella, false, vec![])))
        .collect(),
    );
    let mut runner = scenario.build();
    let state = runner.state();
    assert_eq!(
        state
            .library_of(P0)
            .iter()
            .take(4)
            .copied()
            .collect::<Vec<_>>(),
        top_four.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        "reach: the staged cards are the top four of P0's library"
    );
    let _ = runner
        .activate(svella, SVELLA_LOOK_ABILITY)
        .accept_optional()
        .resolve();
    Peek { runner, top_four }
}

fn offered(peek: &Peek) -> Vec<ObjectId> {
    let WaitingFor::EffectZoneChoice { cards, zone, .. } = &peek.runner.state().waiting_for else {
        panic!("the peek must park its library cast choice");
    };
    assert_eq!(*zone, Zone::Library);
    cards.clone()
}

fn id_of(peek: &Peek, index: usize) -> ObjectId {
    peek.top_four[index].0
}

fn library(peek: &Peek) -> Vec<ObjectId> {
    peek.runner.state().library_of(P0).iter().copied().collect()
}

/// The three spells among the four looked-at cards (indices 0, 2, 3); the Island is a land and
/// is never cast.
const SPELLS: [usize; 3] = [0, 2, 3];

#[test]
fn dandan_every_looked_at_spell_is_offered_whoever_owns_it() {
    if shared_card_db().is_none() {
        return;
    }
    let p = peek(FormatConfig::dandan(), [P1, P1, P1, P0]);
    assert!(
        p.runner.state().players[1].library.is_empty(),
        "reach: the pile is held by the canonical seat"
    );
    let mut expected: Vec<_> = SPELLS.iter().map(|&i| id_of(&p, i)).collect();
    let mut got = offered(&p);
    expected.sort();
    got.sort();
    assert_eq!(got, expected, "P1-owned spells are P0's library cards too");
}

#[test]
fn dandan_casting_an_opponent_owned_hit_bottoms_every_other_looked_at_card() {
    if shared_card_db().is_none() {
        return;
    }
    let mut p = peek(FormatConfig::dandan(), [P1, P1, P1, P0]);
    let chosen = id_of(&p, 0);
    p.runner
        .act(GameAction::SelectCards {
            cards: vec![chosen],
        })
        .expect("choosing a P1-owned hit must succeed");
    assert_eq!(p.runner.state().objects[&chosen].zone, Zone::Stack);
    let lib = library(&p);
    let tail: Vec<_> = lib[lib.len() - 3..].to_vec();
    let mut rest: Vec<_> = (1..4).map(|i| id_of(&p, i)).collect();
    let mut tail_sorted = tail.clone();
    rest.sort();
    tail_sorted.sort();
    assert_eq!(
        tail_sorted, rest,
        "the unchosen looked-at cards are the bottom three"
    );
}

#[test]
fn dandan_declining_bottoms_all_four_looked_at_cards() {
    if shared_card_db().is_none() {
        return;
    }
    let mut p = peek(FormatConfig::dandan(), [P1, P1, P1, P0]);
    p.runner
        .act(GameAction::SelectCards { cards: vec![] })
        .expect("declining the cast must succeed");
    let lib = library(&p);
    let mut tail: Vec<_> = lib[lib.len() - 4..].to_vec();
    let mut looked: Vec<_> = (0..4).map(|i| id_of(&p, i)).collect();
    tail.sort();
    looked.sort();
    assert_eq!(tail, looked, "all four looked-at cards moved to the bottom");
}

#[test]
fn standard_offers_the_controllers_own_spells_and_leaves_the_opponent_library_alone() {
    if shared_card_db().is_none() {
        return;
    }
    let mut p = peek(FormatConfig::standard(), [P0, P0, P0, P0]);
    let mut expected: Vec<_> = SPELLS.iter().map(|&i| id_of(&p, i)).collect();
    let mut got = offered(&p);
    expected.sort();
    got.sort();
    assert_eq!(got, expected, "reach: the three spells are offered");
    let opponent_library: Vec<_> = p.runner.state().library_of(P1).iter().copied().collect();
    p.runner
        .act(GameAction::SelectCards { cards: vec![] })
        .expect("declining the cast must succeed");
    assert_eq!(
        p.runner
            .state()
            .library_of(P1)
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        opponent_library,
        "P1's own library is not part of P0's look"
    );
}
