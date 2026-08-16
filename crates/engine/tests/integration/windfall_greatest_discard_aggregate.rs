//! Windfall's cross-player MAX aggregate — "the greatest number of cards a
//! player discarded this way".
//!
//! Oracle (Scryfall, verified verbatim 2026-08-15):
//!   "Each player discards their hand, then draws cards equal to the greatest
//!    number of cards a player discarded this way."
//!
//! Class: Windfall, Jace's Archivist, Whispering Madness — identical text.
//!
//! CR 608.2e: the discard action is processed simultaneously for every player,
//!   then the draw action reads that completed action's result.
//! CR 608.2h: the draw count is determined ONCE, when the draw action is
//!   applied — not re-derived per player as the fan-out proceeds.
//! CR 608.2i: that determination is a look-back at the already-completed
//!   discard action, the exception to CR 608.2h this clause relies on.
//! CR 701.9a: to discard a card is to move it from hand to graveyard.
//! CR 121.2: drawing N cards is N individual card draws.
//!
//! The regression this pins: the engine reduces the per-player discard counts to
//! ONE untyped scalar whose aggregate lived on the PRODUCER. With the producer
//! set to a cross-player SUM, Windfall drew 8+7+3+3 = 21 for every player
//! instead of the greatest single player's 8.

use engine::game::scenario::{GameScenario, Outcome, P0, P1};
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const WINDFALL: &str = "Each player discards their hand, then draws cards equal to the greatest number of cards a player discarded this way.";

/// Syphon Mind's shape — the cross-player SUM sibling that must STAY a sum.
/// Guards against a "fix" that flips the shared aggregate back to MAX globally.
const SYPHON_MIND: &str =
    "Each other player discards a card. You draw a card for each card discarded this way.";

const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);
const SEATS: [PlayerId; 4] = [P0, P1, P2, P3];

/// Deep enough that no draw in these tests is library-limited.
const LIBRARY_DEPTH: usize = 60;

fn seed_library(scenario: &mut GameScenario, player: PlayerId, n: usize) {
    for i in 0..n {
        scenario.add_card_to_library_top(player, &format!("Filler {i}"));
    }
}

fn seed_hand(scenario: &mut GameScenario, player: PlayerId, n: usize) {
    for i in 0..n {
        scenario.add_card_to_hand(player, &format!("Hand Filler {i}"));
    }
}

fn zone_len(outcome: &Outcome, player: PlayerId, zone: Zone) -> usize {
    let p = outcome
        .state()
        .players
        .iter()
        .find(|p| p.id == player)
        .expect("player exists");
    match zone {
        Zone::Hand => p.hand.len(),
        Zone::Library => p.library.len(),
        Zone::Graveyard => p.graveyard.len(),
        other => panic!("zone_len does not cover {other:?}"),
    }
}

/// CR 608.2e + CR 121.2: four seats, hands 8/7/3/3 (the USER-reported board).
/// CR 608.2h: the greatest number of cards any one player discarded is 8 and is
/// determined once when the draw action is applied, so EVERY player draws
/// exactly 8.
///
/// P0's eight are the cards held BESIDE Windfall: CR 601.2a removes the spell
/// from hand when the cast commits to the stack, so it is not itself discarded.
///
/// Non-vacuous and discriminating: the four hand sizes make MAX (8), SUM (21),
/// MIN (3), and per-player (8/7/3/3) four mutually distinguishable outcomes, so
/// the assertion fails under every wrong aggregate, not merely the one that
/// shipped. The graveyard assertion is the reach guard — it proves the discard
/// step actually ran, so a spell that failed to parse or resolve cannot pass a
/// bare hand-size check for the wrong reason.
#[test]
fn windfall_draws_the_greatest_single_players_discard_not_the_cross_player_sum() {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    for (seat, hand) in SEATS.iter().zip([8usize, 7, 3, 3]) {
        seed_hand(&mut scenario, *seat, hand);
        seed_library(&mut scenario, *seat, LIBRARY_DEPTH);
    }
    let windfall = scenario
        .add_spell_to_hand_from_oracle(P0, "Windfall", false, WINDFALL)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();

    let outcome = runner.cast(windfall).resolve();

    let drawn: Vec<usize> = SEATS
        .iter()
        .map(|p| LIBRARY_DEPTH - zone_len(&outcome, *p, Zone::Library))
        .collect();
    let hands: Vec<usize> = SEATS
        .iter()
        .map(|p| zone_len(&outcome, *p, Zone::Hand))
        .collect();
    let graveyards: Vec<usize> = SEATS
        .iter()
        .map(|p| zone_len(&outcome, *p, Zone::Graveyard))
        .collect();
    eprintln!("PROBE windfall/cast: drawn={drawn:?} hands={hands:?} graveyards={graveyards:?}");

    // CR 701.9a reach guard: every player really did discard their whole hand.
    assert!(
        graveyards[0] >= 8 && graveyards[1] >= 7 && graveyards[2] >= 3 && graveyards[3] >= 3,
        "reach guard: each player's hand must have reached the graveyard, got {graveyards:?}"
    );
    assert_eq!(
        drawn,
        vec![8, 8, 8, 8],
        "each player draws the GREATEST single-player discard (8), not the cross-player sum (21)"
    );
    assert_eq!(
        hands,
        vec![8, 8, 8, 8],
        "each hand holds exactly the freshly drawn cards"
    );
}

/// The SUM sibling stays a sum. Syphon Mind in a four-player game: the three
/// other players each discard one card and the controller draws 3 — the
/// cross-player TOTAL. A global flip back to MAX would draw 1 here.
#[test]
fn syphon_mind_shape_still_draws_the_cross_player_total() {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    for seat in SEATS {
        seed_hand(&mut scenario, seat, 1);
        seed_library(&mut scenario, seat, LIBRARY_DEPTH);
    }
    let syphon = scenario
        .add_spell_to_hand_from_oracle(P0, "Syphon Mind", false, SYPHON_MIND)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();

    let outcome = runner.cast(syphon).resolve();

    let drawn = LIBRARY_DEPTH - zone_len(&outcome, P0, Zone::Library);
    let opponents_discarded: usize = [P1, P2, P3]
        .iter()
        .map(|p| zone_len(&outcome, *p, Zone::Graveyard))
        .sum();
    eprintln!("PROBE syphon/cast: drawn={drawn} opponents_discarded={opponents_discarded}");

    // Reach guard: the discard step ran for all three opponents (CR 701.9a).
    assert_eq!(
        opponents_discarded, 3,
        "reach guard: each of the three other players must discard one card"
    );
    assert_eq!(
        drawn, 3,
        "controller draws one per card discarded across all opponents (sum), not the max (1)"
    );
}

/// PROBE for the second, independent defect the code map surfaced: the draw
/// tail keeps `player_scope: All` and re-fans-out, and each player's completed
/// draw re-stamps the shared scalar with that player's DELIVERED count. So a
/// player whose library ran short does not just draw fewer cards — they
/// redefine how many every LATER player draws.
///
/// CR 608.2h: the draw action's count is determined only once, when the
/// effect is applied — one player's short library cannot change another
/// player's count. CR 608.2e: the whole fan-out is one action processed
/// simultaneously. CR 121.2c: the SERIALIZATION (the active player performs
/// all of their draws first, then each other player in turn order) is itself
/// rules-correct — only the leaked count is not.
///
/// Discriminating: P0's library holds 5, everyone else 60. Correct = [5,8,8,8].
/// Leaked-delivered-count = [5,5,5,5]. The two differ on three seats.
#[test]
fn windfall_short_library_does_not_shrink_later_players_draws() {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    for (seat, hand) in SEATS.iter().zip([8usize, 7, 3, 3]) {
        seed_hand(&mut scenario, *seat, hand);
        seed_library(
            &mut scenario,
            *seat,
            if *seat == P0 { 5 } else { LIBRARY_DEPTH },
        );
    }
    let windfall = scenario
        .add_spell_to_hand_from_oracle(P0, "Windfall", false, WINDFALL)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();
    let outcome = runner.cast(windfall).resolve();

    let drawn: Vec<usize> = SEATS
        .iter()
        .map(|p| {
            let depth = if *p == P0 { 5 } else { LIBRARY_DEPTH };
            depth - zone_len(&outcome, *p, Zone::Library)
        })
        .collect();
    eprintln!(
        "PROBE windfall/short-library: drawn={drawn:?} waiting={:?}",
        outcome.final_waiting_for()
    );
    assert_eq!(
        drawn,
        vec![5, 8, 8, 8],
        "P0's short library caps only P0; every later player still draws the greatest discard (8)"
    );
}
