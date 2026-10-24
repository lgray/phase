//! CR 608.2c + CR 614.6: an "instead" override's selector chooses the
//! replacement; a guard the replacement body prints on its own instruction still
//! gates that instruction and never restores the replaced one.

use engine::game::scenario::{GameScenario, P0};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaCost, ManaType, ManaUnit};
use engine::types::phase::Phase;

/// Accepted grammar with no printed card: a multi-instruction "instead" body
/// whose first instruction carries its own "if" guard.
const KICKED_GUARDED_BODY: &str =
    "Kicker {2} (You may pay an additional {2} as you cast this spell.)\n\
Draw a card. If this spell was kicked, instead draw two cards if you control three or more \
creatures, then gain 1 life.";

/// The single-instruction "If <state>, <body> instead" form with a guarded body.
const STATE_GUARDED_BODY: &str = "Draw a card. If you have 10 or less life, draw two cards if \
you control three or more creatures instead.";

struct Board {
    kicked: Option<bool>,
    life: i32,
    creatures: usize,
}

/// P0 casts `oracle` on `board`; returns (cards drawn, life gained) once the stack is empty.
fn cast(oracle: &str, board: Board) -> (i64, i32) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, board.life);
    let spell = scenario
        .add_spell_to_hand(P0, "Guarded Instead", false)
        .from_oracle_text_with_keywords(&["Kicker"], oracle)
        .with_mana_cost(ManaCost::generic(0))
        .id();
    for index in 0..board.creatures {
        scenario.add_creature(P0, &format!("Bear {index}"), 2, 2);
    }
    for index in 0..5 {
        scenario.add_card_to_library_top(P0, &format!("Library Card {index}"));
    }
    scenario.with_mana_pool(
        P0,
        (0..2)
            .map(|_| ManaUnit::new(ManaType::Colorless, ObjectId(0), false, vec![]))
            .collect(),
    );
    let mut runner = scenario.build();
    let cast = runner.cast(spell);
    let cast = match board.kicked {
        Some(true) => cast.accept_optional(),
        Some(false) => cast.decline_optional(),
        None => cast,
    };
    let outcome = cast.resolve();
    assert!(outcome.state().stack.is_empty(), "the spell resolved");
    (outcome.hand_drawn(P0), outcome.life_delta(P0))
}

/// Kicked, inner guard false: "draw a card" never happens (CR 614.6) and "draw
/// two cards if you control three or more creatures" draws nothing.
#[test]
fn kicked_guarded_body_with_false_guard_draws_nothing() {
    let (drawn, _) = cast(
        KICKED_GUARDED_BODY,
        Board {
            kicked: Some(true),
            life: 20,
            creatures: 0,
        },
    );
    assert_eq!(drawn, 0);
}

/// Kicked, inner guard true: draw two cards, then gain 1 life.
#[test]
fn kicked_guarded_body_with_true_guard_draws_two_and_gains_life() {
    let outcome = cast(
        KICKED_GUARDED_BODY,
        Board {
            kicked: Some(true),
            life: 20,
            creatures: 3,
        },
    );
    assert_eq!(outcome, (2, 1));
}

/// Unkicked: only "Draw a card." happens.
#[test]
fn unkicked_guarded_body_draws_one() {
    let outcome = cast(
        KICKED_GUARDED_BODY,
        Board {
            kicked: Some(false),
            life: 20,
            creatures: 3,
        },
    );
    assert_eq!(outcome, (1, 0));
}

/// Selector holds, inner guard false: neither "draw a card" nor "draw two cards".
#[test]
fn state_selected_guarded_body_with_false_guard_draws_nothing() {
    let outcome = cast(
        STATE_GUARDED_BODY,
        Board {
            kicked: None,
            life: 10,
            creatures: 0,
        },
    );
    assert_eq!(outcome, (0, 0));
}

/// Selector holds, inner guard true: draw two cards.
#[test]
fn state_selected_guarded_body_with_true_guard_draws_two() {
    let outcome = cast(
        STATE_GUARDED_BODY,
        Board {
            kicked: None,
            life: 10,
            creatures: 3,
        },
    );
    assert_eq!(outcome, (2, 0));
}

/// Selector fails: only "Draw a card." happens.
#[test]
fn unselected_guarded_body_draws_one() {
    let outcome = cast(
        STATE_GUARDED_BODY,
        Board {
            kicked: None,
            life: 20,
            creatures: 3,
        },
    );
    assert_eq!(outcome, (1, 0));
}
